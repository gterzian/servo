/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::cell::Ref;
use crate::dom::bindings::codegen::Bindings::BlobBinding::BlobBinding::BlobMethods;
use crate::dom::bindings::codegen::Bindings::FormDataBinding::FormDataMethods;
use crate::dom::bindings::codegen::Bindings::XMLHttpRequestBinding::BodyInit;
use crate::dom::bindings::error::{Error, Fallible};
use crate::dom::bindings::refcounted::{Trusted, TrustedPromise};
use crate::dom::bindings::reflector::DomObject;
use crate::dom::bindings::root::DomRoot;
use crate::dom::bindings::str::{is_token, ByteString, DOMString, USVString};
use crate::dom::bindings::trace::RootedTraceableBox;
use crate::dom::blob::{normalize_type_string, Blob};
use crate::dom::formdata::FormData;
use crate::dom::globalscope::GlobalScope;
use crate::dom::htmlformelement::{encode_multipart_form_data, generate_boundary};
use crate::dom::promise::Promise;
use crate::dom::promisenativehandler::{Callback, PromiseNativeHandler};
use crate::dom::readablestream::ReadableStream;
use crate::dom::urlsearchparams::URLSearchParams;
use crate::realms::{AlreadyInRealm, InRealm};
use crate::script_runtime::JSContext;
use crate::task::TaskCanceller;
use crate::task_source::networking::NetworkingTaskSource;
use crate::task_source::TaskSource;
use crate::task_source::TaskSourceName;
use encoding_rs::{Encoding, UTF_8};
use ipc_channel::ipc::{self, IpcSender};
use ipc_channel::router::ROUTER;
use js::jsapi::Heap;
use js::jsapi::JSContext as UnSafeJSContext;
use js::jsapi::JSObject;
use js::jsapi::JS_ClearPendingException;
use js::jsapi::Value as JSValue;
use js::jsval::JSVal;
use js::jsval::UndefinedValue;
use js::rust::wrappers::JS_GetPendingException;
use js::rust::wrappers::JS_ParseJSON;
use js::rust::HandleValue;
use js::typedarray::{ArrayBuffer, CreateWith};
use mime::{self, Mime};
use net_traits::request::{BodySource, RequestBody};
use script_traits::serializable::BlobImpl;
use std::ptr;
use std::rc::Rc;
use std::str;
use url::form_urlencoded;

struct BodyConnectHandler {
    stream: Option<Trusted<ReadableStream>>,
    global: Trusted<GlobalScope>,
    task_source: NetworkingTaskSource,
    canceller: TaskCanceller,
}

impl BodyConnectHandler {
    pub fn new(
        stream: Trusted<ReadableStream>,
        global: Trusted<GlobalScope>,
        task_source: NetworkingTaskSource,
        canceller: TaskCanceller,
    ) -> BodyConnectHandler {
        BodyConnectHandler {
            stream: Some(stream),
            global,
            task_source,
            canceller,
        }
    }

    pub fn start_reading_from_stream(&mut self, bytes_sender: IpcSender<Vec<u8>>) {
        let global = self.global.clone();
        let stream = match self.stream.take() {
            Some(stream) => stream,
            None => return warn!("start_reading_from_stream called multiple times."),
        };

        let _ = self.task_source.queue_with_canceller(
            task!(setup_native_body_promise_handler: move || {
                let rooted_stream = stream.root();
                let promise = rooted_stream.read_a_chunk();

                let promise_handler = Box::new(BodyPromiseHandler {
                    bytes_sender,
                    stream: rooted_stream,
                });

                let handler = PromiseNativeHandler::new(&global.root(), Some(promise_handler), None);
                promise.append_native_handler(&handler);
            }),
            &self.canceller,
        );
    }
}

#[derive(JSTraceable, MallocSizeOf)]
struct BodyPromiseHandler {
    #[ignore_malloc_size_of = "Channels are hard"]
    bytes_sender: IpcSender<Vec<u8>>,
    stream: DomRoot<ReadableStream>,
}

impl Callback for BodyPromiseHandler {
    fn callback(&self, _cx: *mut UnSafeJSContext, v: HandleValue) {
        // 1: Send chunks over IPC.
        // 2: If reader is not done, read another chunk from the stream.
    }
}

/// The result of https://fetch.spec.whatwg.org/#concept-bodyinit-extract
pub struct ExtractedBody {
    pub stream: DomRoot<ReadableStream>,
    pub source: BodySource,
    pub total_bytes: usize,
    pub content_type: Option<DOMString>,
}

impl ExtractedBody {
    /// Essentially infra for the parallel steps of
    /// <https://fetch.spec.whatwg.org/#concept-request-transmit-body>
    pub fn into_request_body(self, global: &GlobalScope) -> (RequestBody, DomRoot<ReadableStream>) {
        let ExtractedBody {
            stream,
            total_bytes,
            content_type,
            source,
        } = self;

        let (stream_connect_sender, stream_connect_receiver) = ipc::channel().unwrap();
        let trusted_stream = Trusted::new(&*stream);
        let trusted_global = Trusted::new(global);

        let task_source = global.networking_task_source();
        let canceller = global.task_canceller(TaskSourceName::Networking);

        let mut body_handler =
            BodyConnectHandler::new(trusted_stream, trusted_global, task_source, canceller);

        ROUTER.add_route(
            stream_connect_receiver.to_opaque(),
            Box::new(move |message| {
                let bytes_sender = message.to().unwrap();
                body_handler.start_reading_from_stream(bytes_sender);
            }),
        );

        let request_body = RequestBody {
            stream: stream_connect_sender,
            source,
            transmitted_bytes: 0,
            total_bytes,
        };

        (request_body, stream)
    }
}

/// <https://fetch.spec.whatwg.org/#concept-bodyinit-extract>
pub trait Extractable {
    fn extract(&self) -> ExtractedBody;
}

impl Extractable for BodyInit {
    // https://fetch.spec.whatwg.org/#concept-bodyinit-extract
    fn extract(&self) -> ExtractedBody {
        match self {
            BodyInit::String(ref s) => s.extract(),
            BodyInit::URLSearchParams(ref usp) => usp.extract(),
            BodyInit::Blob(ref b) => b.extract(),
            BodyInit::FormData(ref formdata) => formdata.extract(),
            BodyInit::ArrayBuffer(ref typedarray) => {
                let bytes = typedarray.to_vec();
                let total_bytes = bytes.len();
                ExtractedBody {
                    stream: ReadableStream::new_with_external_underlying_source(bytes),
                    total_bytes,
                    content_type: None,
                    source: BodySource::BufferSource,
                }
            },
            BodyInit::ArrayBufferView(ref typedarray) => {
                let bytes = typedarray.to_vec();
                let total_bytes = bytes.len();
                ExtractedBody {
                    stream: ReadableStream::new_with_external_underlying_source(bytes),
                    total_bytes,
                    content_type: None,
                    source: BodySource::BufferSource,
                }
            },
            BodyInit::ReadableStream(stream) => ExtractedBody {
                stream: stream.clone(),
                total_bytes: 0,
                content_type: None,
                source: BodySource::Null,
            },
        }
    }
}

impl Extractable for Vec<u8> {
    fn extract(&self) -> ExtractedBody {
        // TODO: use a stream with a native underlying source.
        let bytes = self.clone();
        let total_bytes = self.len();
        ExtractedBody {
            stream: ReadableStream::new_with_external_underlying_source(bytes),
            total_bytes,
            content_type: None,
            // A vec is used only in `submit_entity_body`.
            source: BodySource::FormData,
        }
    }
}

impl Extractable for Blob {
    fn extract(&self) -> ExtractedBody {
        // TODO: use a stream with a native underlying source.
        let content_type = if self.Type().as_ref().is_empty() {
            None
        } else {
            Some(self.Type())
        };
        let bytes = self.get_bytes().unwrap_or(vec![]);
        let total_bytes = bytes.len();
        ExtractedBody {
            stream: ReadableStream::new_with_external_underlying_source(bytes),
            total_bytes,
            content_type,
            source: BodySource::Blob,
        }
    }
}

impl Extractable for DOMString {
    fn extract(&self) -> ExtractedBody {
        // TODO: use a stream with a native underlying source.
        let bytes = self.as_bytes().to_owned();
        let total_bytes = bytes.len();
        let content_type = Some(DOMString::from("text/plain;charset=UTF-8"));
        ExtractedBody {
            stream: ReadableStream::new_with_external_underlying_source(bytes),
            total_bytes,
            content_type,
            source: BodySource::USVString,
        }
    }
}

impl Extractable for FormData {
    fn extract(&self) -> ExtractedBody {
        // TODO: use a stream with a native underlying source.
        let boundary = generate_boundary();
        let bytes = encode_multipart_form_data(&mut self.datums(), boundary.clone(), UTF_8);
        let total_bytes = bytes.len();
        let content_type = Some(DOMString::from(format!(
            "multipart/form-data;boundary={}",
            boundary
        )));
        ExtractedBody {
            stream: ReadableStream::new_with_external_underlying_source(bytes),
            total_bytes,
            content_type,
            source: BodySource::FormData,
        }
    }
}

impl Extractable for URLSearchParams {
    fn extract(&self) -> ExtractedBody {
        // TODO: use a stream with a native underlying source.
        let bytes = self.serialize_utf8().into_bytes();
        let total_bytes = bytes.len();
        let content_type = Some(DOMString::from(
            "application/x-www-form-urlencoded;charset=UTF-8",
        ));
        ExtractedBody {
            stream: ReadableStream::new_with_external_underlying_source(bytes),
            total_bytes,
            content_type,
            source: BodySource::URLSearchParams,
        }
    }
}

#[derive(Clone, Copy, JSTraceable, MallocSizeOf)]
pub enum BodyType {
    Blob,
    FormData,
    Json,
    Text,
    ArrayBuffer,
}

pub enum FetchedData {
    Text(String),
    Json(RootedTraceableBox<Heap<JSValue>>),
    BlobData(DomRoot<Blob>),
    FormData(DomRoot<FormData>),
    ArrayBuffer(RootedTraceableBox<Heap<*mut JSObject>>),
    JSException(RootedTraceableBox<Heap<JSVal>>),
}

// https://fetch.spec.whatwg.org/#concept-body-consume-body
#[allow(unrooted_must_root)]
pub fn consume_body<T: BodyOperations + DomObject>(object: &T, body_type: BodyType) -> Rc<Promise> {
    let in_realm_proof = AlreadyInRealm::assert(&object.global());
    let promise =
        Promise::new_in_current_realm(&object.global(), InRealm::Already(&in_realm_proof));

    // Step 1
    if object.get_body_used() || object.is_locked() {
        promise.reject_error(Error::Type(
            "The response's stream is disturbed or locked".to_string(),
        ));
        return promise;
    }

    object.set_body_promise(&promise, body_type);

    // Steps 2-4
    // TODO: Body does not yet have a stream.

    consume_body_with_promise(object, body_type, &promise);

    promise
}

// https://fetch.spec.whatwg.org/#concept-body-consume-body
#[allow(unrooted_must_root)]
pub fn consume_body_with_promise<T: BodyOperations + DomObject>(
    object: &T,
    body_type: BodyType,
    promise: &Promise,
) {
    // Step 5
    let body = match object.take_body() {
        Some(body) => body,
        None => return,
    };

    let pkg_data_results =
        run_package_data_algorithm(object, body, body_type, object.get_mime_type());

    match pkg_data_results {
        Ok(results) => {
            match results {
                FetchedData::Text(s) => promise.resolve_native(&USVString(s)),
                FetchedData::Json(j) => promise.resolve_native(&j),
                FetchedData::BlobData(b) => promise.resolve_native(&b),
                FetchedData::FormData(f) => promise.resolve_native(&f),
                FetchedData::ArrayBuffer(a) => promise.resolve_native(&a),
                FetchedData::JSException(e) => promise.reject_native(&e.handle()),
            };
        },
        Err(err) => promise.reject_error(err),
    }
}

// https://fetch.spec.whatwg.org/#concept-body-package-data
#[allow(unsafe_code)]
fn run_package_data_algorithm<T: BodyOperations + DomObject>(
    object: &T,
    bytes: Vec<u8>,
    body_type: BodyType,
    mime_type: Ref<Vec<u8>>,
) -> Fallible<FetchedData> {
    let global = object.global();
    let cx = global.get_cx();
    let mime = &*mime_type;
    match body_type {
        BodyType::Text => run_text_data_algorithm(bytes),
        BodyType::Json => run_json_data_algorithm(cx, bytes),
        BodyType::Blob => run_blob_data_algorithm(&global, bytes, mime),
        BodyType::FormData => run_form_data_algorithm(&global, bytes, mime),
        BodyType::ArrayBuffer => run_array_buffer_data_algorithm(cx, bytes),
    }
}

fn run_text_data_algorithm(bytes: Vec<u8>) -> Fallible<FetchedData> {
    Ok(FetchedData::Text(
        String::from_utf8_lossy(&bytes).into_owned(),
    ))
}

#[allow(unsafe_code)]
fn run_json_data_algorithm(cx: JSContext, bytes: Vec<u8>) -> Fallible<FetchedData> {
    let json_text = String::from_utf8_lossy(&bytes);
    let json_text: Vec<u16> = json_text.encode_utf16().collect();
    rooted!(in(*cx) let mut rval = UndefinedValue());
    unsafe {
        if !JS_ParseJSON(
            *cx,
            json_text.as_ptr(),
            json_text.len() as u32,
            rval.handle_mut(),
        ) {
            rooted!(in(*cx) let mut exception = UndefinedValue());
            assert!(JS_GetPendingException(*cx, exception.handle_mut()));
            JS_ClearPendingException(*cx);
            return Ok(FetchedData::JSException(RootedTraceableBox::from_box(
                Heap::boxed(exception.get()),
            )));
        }
        let rooted_heap = RootedTraceableBox::from_box(Heap::boxed(rval.get()));
        Ok(FetchedData::Json(rooted_heap))
    }
}

fn run_blob_data_algorithm(
    root: &GlobalScope,
    bytes: Vec<u8>,
    mime: &[u8],
) -> Fallible<FetchedData> {
    let mime_string = if let Ok(s) = String::from_utf8(mime.to_vec()) {
        s
    } else {
        "".to_string()
    };
    let blob = Blob::new(
        root,
        BlobImpl::new_from_bytes(bytes, normalize_type_string(&mime_string)),
    );
    Ok(FetchedData::BlobData(blob))
}

fn run_form_data_algorithm(
    root: &GlobalScope,
    bytes: Vec<u8>,
    mime: &[u8],
) -> Fallible<FetchedData> {
    let mime_str = if let Ok(s) = str::from_utf8(mime) {
        s
    } else {
        ""
    };
    let mime: Mime = mime_str
        .parse()
        .map_err(|_| Error::Type("Inappropriate MIME-type for Body".to_string()))?;

    // TODO
    // ... Parser for Mime(TopLevel::Multipart, SubLevel::FormData, _)
    // ... is not fully determined yet.
    if mime.type_() == mime::APPLICATION && mime.subtype() == mime::WWW_FORM_URLENCODED {
        let entries = form_urlencoded::parse(&bytes);
        let formdata = FormData::new(None, root);
        for (k, e) in entries {
            formdata.Append(USVString(k.into_owned()), USVString(e.into_owned()));
        }
        return Ok(FetchedData::FormData(formdata));
    }

    Err(Error::Type("Inappropriate MIME-type for Body".to_string()))
}

#[allow(unsafe_code)]
pub fn run_array_buffer_data_algorithm(cx: JSContext, bytes: Vec<u8>) -> Fallible<FetchedData> {
    rooted!(in(*cx) let mut array_buffer_ptr = ptr::null_mut::<JSObject>());
    let arraybuffer = unsafe {
        ArrayBuffer::create(
            *cx,
            CreateWith::Slice(&bytes),
            array_buffer_ptr.handle_mut(),
        )
    };
    if arraybuffer.is_err() {
        return Err(Error::JSFailed);
    }
    let rooted_heap = RootedTraceableBox::from_box(Heap::boxed(array_buffer_ptr.get()));
    Ok(FetchedData::ArrayBuffer(rooted_heap))
}

pub trait BodyOperations {
    fn get_body_used(&self) -> bool;
    fn set_body_promise(&self, p: &Rc<Promise>, body_type: BodyType);
    /// Returns `Some(_)` if the body is complete, `None` if there is more to
    /// come.
    fn take_body(&self) -> Option<Vec<u8>>;
    fn is_locked(&self) -> bool;
    fn get_mime_type(&self) -> Ref<Vec<u8>>;
}
