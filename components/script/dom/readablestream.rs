/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::codegen::Bindings::ReadableStreamBinding;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, MutDomObject, Reflector};
use crate::dom::bindings::root::DomRoot;
use crate::dom::bindings::utils::set_dictionary_property;
use crate::dom::bindings::utils::AsCCharPtrPtr;
use crate::dom::globalscope::GlobalScope;
use crate::dom::promise::Promise;
use crate::realms::{AlreadyInRealm, InRealm};
use crate::script_runtime::JSContext as SafeJSContext;
use dom_struct::dom_struct;
use js::glue::{CreateReadableStreamUnderlyingSource, ReadableStreamUnderlyingSourceTraps};
use js::jsapi::{
    AddRawValueRoot, HandleObject, Heap, IsReadableStream, JSContext, JSObject, JS_NewObject,
    NewReadableExternalSourceStreamObject, ReadableStreamDefaultReaderRead,
    ReadableStreamGetReader, ReadableStreamIsDisturbed, ReadableStreamIsLocked,
    ReadableStreamReaderMode, ReadableStreamReaderReleaseLock, RemoveRawValueRoot,
    UnwrapReadableStream,
};
use js::jsval::{JSVal, ObjectValue, UndefinedValue};
use js::rust::{IntoHandle, Runtime};
use std::cell::Cell;
use std::os::raw::c_void;
use std::ptr;
use std::rc::Rc;

#[dom_struct]
#[unrooted_must_root_lint::allow_unrooted_in_rc]
pub struct ReadableStream {
    reflector_: Reflector,
    #[ignore_malloc_size_of = "SM handles JS values"]
    js_stream: Heap<*mut JSObject>,
    #[ignore_malloc_size_of = "SM handles JS values"]
    js_reader: Heap<*mut JSObject>,
    has_reader: Cell<bool>,
    external_underlying_source: Option<ExternalUnderlyingSource>,
}

impl ReadableStream {
    fn new_inherited(
        external_underlying_source: Option<ExternalUnderlyingSource>,
    ) -> ReadableStream {
        ReadableStream {
            reflector_: Reflector::new(),
            js_stream: Heap::default(),
            js_reader: Heap::default(),
            has_reader: Default::default(),
            external_underlying_source,
        }
    }

    pub fn new(
        global: &GlobalScope,
        external_underlying_source: Option<ExternalUnderlyingSource>,
    ) -> DomRoot<ReadableStream> {
        reflect_dom_object(
            Box::new(ReadableStream::new_inherited(external_underlying_source)),
            global,
            ReadableStreamBinding::Wrap,
        )
    }

    /// Used from RustCodegen.py
    #[allow(unsafe_code)]
    pub fn from_js(cx: SafeJSContext, obj: *mut JSObject) -> Result<DomRoot<ReadableStream>, ()> {
        unsafe {
            if !IsReadableStream(obj) {
                return Err(());
            }

            let stream = ReadableStream::from_js_with_source(cx, obj, None);

            Ok(stream)
        }
    }

    #[allow(unsafe_code)]
    fn from_js_with_source(
        cx: SafeJSContext,
        obj: *mut JSObject,
        source: Option<ExternalUnderlyingSource>,
    ) -> DomRoot<ReadableStream> {
        unsafe {
            let in_realm_proof = AlreadyInRealm::assert_for_cx(cx);
            let global = GlobalScope::from_context(*cx, InRealm::Already(&in_realm_proof));

            let stream = ReadableStream::new(&global, source);
            stream.js_stream.set(UnwrapReadableStream(obj));

            stream
        }
    }

    /// Build a stream backed by a Rust underlying source.
    #[allow(unsafe_code)]
    pub fn new_with_external_underlying_source(data: Vec<u8>) -> DomRoot<ReadableStream> {
        let mut traps = ReadableStreamUnderlyingSourceTraps {
            requestData: Some(request_data),
            writeIntoReadRequestBuffer: None,
            cancel: None,
            onClosed: None,
            onErrored: None,
            finalize: None,
        };
        let mut source = ExternalUnderlyingSource::new_with_data(data);
        unsafe {
            let source_wrapper = CreateReadableStreamUnderlyingSource(
                &mut traps,
                &mut source as *mut _ as *mut c_void,
            );

            let global = GlobalScope::current().expect("No current global object.");
            let cx = global.get_cx();

            rooted!(in(*cx) let proto = UndefinedValue());
            rooted!(in(*cx) let proto_obj = proto.to_object());
            rooted!(in(*cx)
                let js_stream = NewReadableExternalSourceStreamObject(*cx, source_wrapper, proto_obj.handle().into_handle())
            );

            ReadableStream::from_js_with_source(cx, js_stream.get(), Some(source))
        }
    }

    /// Hack to make partial integration easier
    pub fn clone_body(&self) -> Option<Vec<u8>> {
        self.external_underlying_source
            .as_ref()
            .and_then(|source| Some(source.clone_body()))
    }

    /// Acquires a reader and locks the stream,
    /// must be done before `read_a_chunk`.
    #[allow(unsafe_code)]
    pub fn start_reading(&self) {
        if self.has_reader.get() {
            panic!("ReadableStream::start_reading called on a locked stream.");
        }

        let cx = self.global().get_cx();

        unsafe {
            rooted!(in(*cx) let stream = self.js_stream.get());

            rooted!(in(*cx) let reader = ReadableStreamGetReader(
                *cx,
                stream.handle().into_handle(),
                ReadableStreamReaderMode::Default,
            ));

            // Note: the stream is locked to the reader.
            self.js_reader.set(reader.get());
        }

        self.has_reader.set(true);
    }

    /// Read a chunk from the stream,
    /// must be called after `start_reading`,
    /// and before `stop_reading`.
    #[allow(unsafe_code)]
    pub fn read_a_chunk(&self) -> Rc<Promise> {
        if !self.has_reader.get() {
            panic!("ReadableStream::read_a_chunk called before start_reading.");
        }

        let cx = self.global().get_cx();

        unsafe {
            rooted!(in(*cx) let promise_obj = ReadableStreamDefaultReaderRead(
                *cx,
                self.js_reader.handle(),
            ));
            Promise::new_with_js_promise(promise_obj.handle(), cx)
        }
    }

    /// Releases the lock on the reader,
    /// must be done after `start_reading`.
    #[allow(unsafe_code)]
    pub fn stop_reading(&self) {
        if self.has_reader.get() {
            panic!("ReadableStream::stop_reading called on a readerless stream.");
        }

        self.has_reader.set(false);

        let cx = self.global().get_cx();

        unsafe {
            ReadableStreamReaderReleaseLock(*cx, self.js_reader.handle());
            let _ = self.js_reader.get();
        }
    }
}

#[allow(unsafe_code)]
unsafe extern "C" fn request_data(
    source: *const c_void,
    cx: *mut JSContext,
    stream: HandleObject,
    desired_size: usize,
) {
    let source = &mut *(source as *mut ExternalUnderlyingSource);
    source.request_data(SafeJSContext::from_ptr(cx), stream, desired_size);
}

#[derive(MallocSizeOf)]
pub struct ExternalUnderlyingSource {
    /// TODO: integrate with a streaming source.
    data: Vec<u8>,
}

impl ExternalUnderlyingSource {
    #[allow(unsafe_code)]
    pub fn new_with_data(data: Vec<u8>) -> ExternalUnderlyingSource {
        ExternalUnderlyingSource { data }
    }

    /// Hack to make partial integration easier
    pub fn clone_body(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn request_data(&mut self, cx: SafeJSContext, stream: HandleObject, desired_size: usize) {
        // TODO: start reading data asynchronously.
    }
}
