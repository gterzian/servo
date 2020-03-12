/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::cell::DomRefCell;
use crate::dom::bindings::codegen::Bindings::ReadableStreamBinding;
use crate::dom::bindings::conversions::{ConversionBehavior, ConversionResult};
use crate::dom::bindings::error::Error;
use crate::dom::bindings::refcounted::Trusted;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use crate::dom::bindings::root::DomRoot;
use crate::dom::bindings::settings_stack::AutoIncumbentScript;
use crate::dom::bindings::utils::get_dictionary_property;
use crate::dom::globalscope::GlobalScope;
use crate::dom::promise::Promise;
use crate::js::conversions::FromJSValConvertible;
use crate::realms::{enter_realm, AlreadyInRealm, InRealm};
use crate::script_runtime::JSContext as SafeJSContext;
use crate::task::TaskCanceller;
use crate::task_source::dom_manipulation::DOMManipulationTaskSource;
use crate::task_source::TaskSource;
use crate::task_source::TaskSourceName;
use dom_struct::dom_struct;
use js::glue::{CreateReadableStreamUnderlyingSource, ReadableStreamUnderlyingSourceTraps};
use js::jsapi::HandleValue;
use js::jsapi::{
    HandleObject, Heap, IsReadableStream, JSContext, JSObject,
    NewReadableExternalSourceStreamObject, ReadableStreamClose, ReadableStreamDefaultReaderRead,
    ReadableStreamError, ReadableStreamGetReader, ReadableStreamIsDisturbed,
    ReadableStreamIsLocked, ReadableStreamIsReadable, ReadableStreamReaderMode,
    ReadableStreamReaderReleaseLock, ReadableStreamUpdateDataAvailableFromSource,
    UnwrapReadableStream,
};
use js::jsval::JSVal;
use js::jsval::UndefinedValue;
use js::rust::HandleValue as SafeHandleValue;
use js::rust::IntoHandle;
use std::cell::Cell;
use std::os::raw::c_void;
use std::ptr::{self, NonNull};
use std::rc::Rc;
use std::sync::Mutex;

#[dom_struct]
#[unrooted_must_root_lint::allow_unrooted_in_rc]
pub struct ReadableStream {
    reflector_: Reflector,
    #[ignore_malloc_size_of = "SM handles JS values"]
    js_stream: Heap<*mut JSObject>,
    #[ignore_malloc_size_of = "SM handles JS values"]
    js_reader: Heap<*mut JSObject>,
    has_reader: Cell<bool>,
    #[ignore_malloc_size_of = "Rc is hard"]
    external_underlying_source: Option<Rc<ExternalUnderlyingSourceController>>,
}

impl ReadableStream {
    fn new_inherited(
        external_underlying_source: Option<Rc<ExternalUnderlyingSourceController>>,
    ) -> ReadableStream {
        ReadableStream {
            reflector_: Reflector::new(),
            js_stream: Heap::default(),
            js_reader: Heap::default(),
            has_reader: Default::default(),
            external_underlying_source: external_underlying_source,
        }
    }

    fn new(
        global: &GlobalScope,
        external_underlying_source: Option<Rc<ExternalUnderlyingSourceController>>,
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
        source: Option<Rc<ExternalUnderlyingSourceController>>,
    ) -> DomRoot<ReadableStream> {
        let in_realm_proof = AlreadyInRealm::assert_for_cx(cx);
        let global = GlobalScope::from_safe_context(cx, InRealm::Already(&in_realm_proof));

        let stream = ReadableStream::new(&global, source);
        unsafe { stream.js_stream.set(UnwrapReadableStream(obj)) };

        stream
    }

    /// Build a stream backed by a Rust underlying source.
    #[allow(unsafe_code)]
    pub fn new_with_external_underlying_source(
        global: &GlobalScope,
        source: ExternalUnderlyingSource,
    ) -> DomRoot<ReadableStream> {
        let source = Rc::new(ExternalUnderlyingSourceController::new(source));

        let cx = global.get_cx();

        let stream = unsafe {
            let mut traps = ReadableStreamUnderlyingSourceTraps {
                requestData: Some(request_data),
                writeIntoReadRequestBuffer: Some(write_into_read_request_buffer),
                cancel: Some(cancel),
                onClosed: Some(close),
                onErrored: Some(error),
                finalize: Some(finalize),
            };
            let js_wrapper = CreateReadableStreamUnderlyingSource(
                &mut traps,
                &*source as *const _ as *const c_void,
            );

            rooted!(in(*cx)
                let js_stream = NewReadableExternalSourceStreamObject(
                    *cx,
                    js_wrapper,
                    // Prototype.
                    HandleObject::null(),
                )
            );

            ReadableStream::from_js_with_source(cx, js_stream.get(), Some(source.clone()))
        };

        // We want a task-source to run the finalize steps later,
        // the DOM manipulation one is as good as any.
        let task_source = global.dom_manipulation_task_source();
        let canceller = global.task_canceller(TaskSourceName::DOMManipulation);
        let trusted_stream = Trusted::new(&*stream);
        source.set_up_finalize(trusted_stream, task_source, canceller);

        stream
    }

    #[allow(unsafe_code)]
    pub fn finalize(&self) {
        // TODO: update SM.
        // ReadableStreamReleaseCCObject(self.js_stream.get());
    }

    /// Get a pointer to the underlying JS object.
    pub fn get_js_stream(&self) -> NonNull<JSObject> {
        NonNull::new(self.js_stream.get())
            .expect("Couldn't get a non-null pointer to JS stream object.")
    }

    /// Enqueue bytes to the underlying source(via the controller).
    #[allow(unsafe_code)]
    pub fn enqueue_native(&self, bytes: &[u8]) {
        let global = self.global();
        let ar = enter_realm(&*global);
        let cx = global.get_cx();

        let stream_handle = unsafe { self.js_stream.handle() };

        self.external_underlying_source
            .as_ref()
            .expect("No external source to enqueue bytes.")
            .enqueue_chunk(cx, stream_handle.clone(), bytes);
    }

    /// Error the stream.
    #[allow(unsafe_code)]
    pub fn error_native(&self, error: Error) {
        println!("Erroring stream");
        let global = self.global();
        let ar = enter_realm(&*global);
        let cx = global.get_cx();

        unsafe {
            rooted!(in(*cx) let mut js_error = UndefinedValue());
            error.to_jsval(*cx, &global, js_error.handle_mut());
            ReadableStreamError(
                *cx,
                self.js_stream.handle(),
                js_error.handle().into_handle(),
            );
        }
    }

    /// Close a stream via it's underlying source controller.
    #[allow(unsafe_code)]
    pub fn close_native(&self) {
        let global = self.global();
        let ar = enter_realm(&*global);
        let cx = global.get_cx();

        let handle = unsafe { self.js_stream.handle() };

        self.external_underlying_source
            .as_ref()
            .expect("No external source to close.")
            .close(cx, handle);
    }

    /// Acquires a reader and locks the stream,
    /// must be done before `read_a_chunk`,
    /// fails if the stream is already locked to a reader.
    #[allow(unsafe_code)]
    pub fn start_reading(&self) -> Result<(), ()> {
        if self.is_locked() || self.is_disturbed() {
            return Err(());
        }

        let global = self.global();
        let ar = enter_realm(&*global);
        let cx = global.get_cx();

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
        Ok(())
    }

    /// Read a chunk from the stream,
    /// must be called after `start_reading`,
    /// and before `stop_reading`.
    #[allow(unsafe_code)]
    pub fn read_a_chunk(&self) -> Rc<Promise> {
        if !self.has_reader.get() {
            panic!("Attempt to read stream chunk without having acquired a reader.");
        }

        println!("Reading a chunk from a stream");

        let global = self.global();
        let ar = enter_realm(&*global);
        AlreadyInRealm::assert(&*global);
        let _ais = AutoIncumbentScript::new(&*global);

        let cx = global.get_cx();

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
        if !self.has_reader.get() {
            println!("ReadableStream::stop_reading called on a readerless stream.");
        }

        self.has_reader.set(false);

        let global = self.global();
        let ar = enter_realm(&*global);
        let cx = global.get_cx();

        unsafe {
            ReadableStreamReaderReleaseLock(*cx, self.js_reader.handle());
            let _ = self.js_reader.get();
        }
    }

    #[allow(unsafe_code)]
    pub fn is_locked(&self) -> bool {
        // If we natively took a reader, we're locked.
        if self.has_reader.get() {
            return true;
        }

        // Otherwise, still double-check that script didn't lock the stream,
        // in case of a script-created stream.
        let cx = self.global().get_cx();
        let mut locked_or_disturbed = false;

        unsafe {
            rooted!(in(*cx) let stream = self.js_stream.get());
            ReadableStreamIsLocked(*cx, stream.handle().into_handle(), &mut locked_or_disturbed);
        }

        locked_or_disturbed
    }

    #[allow(unsafe_code)]
    pub fn is_disturbed(&self) -> bool {
        // If we natively took a reader, we're disturbed
        // (Note: or is that only once at least a chunk has been read?).
        if self.has_reader.get() {
            return true;
        }

        // Otherwise, still double-check that script didn't disturb the stream,
        // in case of a script-created stream.
        let cx = self.global().get_cx();
        let mut locked_or_disturbed = false;

        unsafe {
            rooted!(in(*cx) let stream = self.js_stream.get());
            ReadableStreamIsDisturbed(*cx, stream.handle().into_handle(), &mut locked_or_disturbed);
        }

        locked_or_disturbed
    }
}

#[allow(unsafe_code)]
unsafe extern "C" fn request_data(
    source: *const c_void,
    cx: *mut JSContext,
    stream: HandleObject,
    desired_size: usize,
) {
    let source = &*(source as *const ExternalUnderlyingSourceController);
    source.pull(SafeJSContext::from_ptr(cx), stream, desired_size);
}

#[allow(unsafe_code)]
unsafe extern "C" fn write_into_read_request_buffer(
    source: *const c_void,
    cx: *mut JSContext,
    stream: HandleObject,
    buffer: *mut c_void,
    length: usize,
    bytes_written: *mut usize,
) {
    let source = &*(source as *const ExternalUnderlyingSourceController);
    source.write_into_buffer(
        SafeJSContext::from_ptr(cx),
        stream,
        buffer,
        length,
        bytes_written,
    );
}

#[allow(unsafe_code)]
unsafe extern "C" fn cancel(
    _source: *const c_void,
    _cx: *mut JSContext,
    _stream: HandleObject,
    _reason: HandleValue,
) -> *mut JSVal {
    ptr::null_mut()
}

#[allow(unsafe_code)]
unsafe extern "C" fn close(_source: *const c_void, _cx: *mut JSContext, _stream: HandleObject) {}

#[allow(unsafe_code)]
unsafe extern "C" fn error(
    _source: *const c_void,
    _cx: *mut JSContext,
    _stream: HandleObject,
    _reason: HandleValue,
) {
}

#[allow(unsafe_code)]
unsafe extern "C" fn finalize(source: *const c_void) {
    let source = &*(source as *const ExternalUnderlyingSourceController);
    source.finalize();
}

/// Something representing the actual underlying source of data.
pub enum ExternalUnderlyingSource {
    /// Facilitate partial integration with sources
    /// that are currently read into memory.
    Memory(Vec<u8>),
    /// A blob as underlying source, with a known total size.
    Blob(usize),
    /// A fetch response as underlying source.
    FetchResponse,
    /// A fetch request as underlying source.
    FetchRequest,
}

/// When `finalize` is called, use this to schedule a task
/// on the relevant event-loop for the stream, and run the finalizing steps.
/// Must be via a queued task,
/// since `finalize` can be called on a SM background "clean-up" thread.
pub struct StreamFinalizer {
    stream: Trusted<ReadableStream>,
    task_source: DOMManipulationTaskSource,
    canceller: TaskCanceller,
}

impl StreamFinalizer {
    fn finalize(self) {
        let trusted_stream = self.stream;
        let canceller = self.canceller;
        let _ = self.task_source.queue_with_canceller(
            task!(reject_promise: move || {
                let stream = trusted_stream.root();
                let _ = enter_realm(&*stream.global());
                stream.finalize();
            }),
            &canceller,
        );
    }
}

#[derive(JSTraceable, MallocSizeOf)]
struct ExternalUnderlyingSourceController {
    /// Loosely matches the underlying queue,
    /// <https://streams.spec.whatwg.org/#internal-queues>
    buffer: DomRefCell<Vec<u8>>,
    /// Has the stream been closed by native code?
    closed: DomRefCell<bool>,
    /// An object that maybe be accessed from a background "clean-up" thread,
    /// and which can be used to queue a task to finalize the stream.
    /// The mutex is strictly speaking not required, since the option will not be used concurrently,
    /// it will be used once upon initialization, from the event-loop,
    /// and upon finalization, potentially on a background thread.
    #[ignore_malloc_size_of = "StreamFinalizer"]
    finalizer: Mutex<Option<StreamFinalizer>>,
}

impl ExternalUnderlyingSourceController {
    fn new(source: ExternalUnderlyingSource) -> ExternalUnderlyingSourceController {
        let buffer = match source {
            ExternalUnderlyingSource::Blob(size) => Vec::with_capacity(size),
            ExternalUnderlyingSource::Memory(bytes) => bytes,
            ExternalUnderlyingSource::FetchResponse | ExternalUnderlyingSource::FetchRequest => vec![],
        };
        ExternalUnderlyingSourceController {
            buffer: DomRefCell::new(buffer),
            closed: DomRefCell::new(false),
            finalizer: Mutex::new(None),
        }
    }

    fn set_up_finalize(
        &self,
        stream: Trusted<ReadableStream>,
        task_source: DOMManipulationTaskSource,
        canceller: TaskCanceller,
    ) {
        // Called right after `new` to pass the newly created stream that is using this source.
        *self.finalizer.lock().unwrap() = Some(StreamFinalizer {
            stream,
            task_source,
            canceller,
        });
    }

    fn finalize(&self) {
        self.finalizer
            .lock()
            .unwrap()
            .take()
            .expect("No StreamFinalizer.")
            .finalize();
    }

    /// Signal to SM that we have bytes ready.
    /// This will immediately call into `write_into_buffer` if a read request is currently pending,
    /// and if not it will call into it on the next read request.
    #[allow(unsafe_code)]
    fn signal_available_bytes(&self, cx: SafeJSContext, stream: HandleObject, available: usize) {
        // We have bytes available in memory, or from a blob push source.
        println!("Signal available bytes: {:?}", available);
        unsafe {
            ReadableStreamUpdateDataAvailableFromSource(*cx, stream, available as u32);
        }
    }

    /// Close the stream, if it is currently readable.
    #[allow(unsafe_code)]
    fn maybe_close_js_stream(&self, cx: SafeJSContext, stream: HandleObject) {
        println!("Maybe closing stream.");
        unsafe {
            let mut readable = false;
            if !ReadableStreamIsReadable(*cx, stream, &mut readable) {
                return;
            }
            if readable {
                println!("Indeed closing stream.");
                ReadableStreamClose(*cx, stream);
            }
        }
    }

    /// Set the closed flag, and close the stream if currently readbale.
    fn close(&self, cx: SafeJSContext, stream: HandleObject) {
        println!("Native close called");
        *self.closed.borrow_mut() = true;
        self.maybe_close_js_stream(cx, stream);
    }

    fn enqueue_chunk(&self, cx: SafeJSContext, stream: HandleObject, chunk: &[u8]) {
        println!("Enqueuing chunks: {:?}", chunk.len());
        let available = {
            let mut buffer = self.buffer.borrow_mut();
            *buffer = [chunk, buffer.as_slice()].concat().to_vec();
            buffer.len()
        };
        self.signal_available_bytes(cx, stream, available);
    }

    /// The "pull steps" for this controller.
    /// If we restructured fetch or file-reading to be pull-based, this hook could be used to pull a chunk over IPC,
    /// (via an async request for a new chunk).
    /// Since everything currently just pushes data at us, we simply look at the buffer and signal available bytes.
    #[allow(unsafe_code)]
    fn pull(&self, cx: SafeJSContext, stream: HandleObject, desired_size: usize) {
        println!(
            "Pull steps ExternalUnderlyingSourceController with buffer: {:?} closed: {:?} desired_size: {:?}",
            self.buffer.borrow().len(),
            *self.closed.borrow(),
            desired_size,
        );

        let closed = { *self.closed.borrow() };

        if closed {
            return self.maybe_close_js_stream(cx, stream);
        }

        let available = {
            let buffer = self.buffer.borrow();
            buffer.len()
        };

        if available > 0 {
            self.signal_available_bytes(cx, stream, desired_size);
        }
    }

    /// Called by SM after we've signalled bytes to be available.
    #[allow(unsafe_code)]
    fn write_into_buffer(
        &self,
        cx: SafeJSContext,
        stream: HandleObject,
        target_buffer: *mut c_void,
        length: usize,
        bytes_written: *mut usize,
    ) {

        let mut buffer = self.buffer.borrow_mut();
        let buffer_len = buffer.len();
        assert!(buffer_len >= length as usize);

        let (rest, chunk) = buffer.as_slice().split_at(buffer_len - length);

        unsafe {
            *bytes_written = chunk.len();
            println!(
                "Writing into buffer with length: {:?} a chunk of len: {:?}",
                length,
                chunk.len()
            );
            ptr::copy_nonoverlapping(chunk.as_ptr(), target_buffer as *mut u8, chunk.len());
        }

        *buffer = rest.to_vec();
    }
}

/// Get the `done` property of an object that a read promise resolved to.
#[allow(unsafe_code)]
pub fn get_read_promise_done(cx: SafeJSContext, v: &SafeHandleValue) -> Result<bool, Error> {
    unsafe {
        rooted!(in(*cx) let object = v.to_object());
        rooted!(in(*cx) let mut done = UndefinedValue());
        let has_done =
            get_dictionary_property(*cx, object.handle(), "done", done.handle_mut()).is_ok();

        if !has_done {
            return Err(Error::Type("".to_string()));
        }

        let is_done = match bool::from_jsval(*cx, done.handle(), ()) {
            Ok(ConversionResult::Success(val)) => val,
            _ => panic!("Couldn't convert jsval to boolean"),
        };

        Ok(is_done)
    }
}

/// Get the `value` property of an object that a read promise resolved to.
#[allow(unsafe_code)]
pub fn get_read_promise_bytes(cx: SafeJSContext, v: &SafeHandleValue) -> Result<Vec<u8>, Error> {
    unsafe {
        rooted!(in(*cx) let object = v.to_object());
        rooted!(in(*cx) let mut bytes = UndefinedValue());
        let has_value =
            get_dictionary_property(*cx, object.handle(), "value", bytes.handle_mut()).is_ok();

        if !has_value {
            return Err(Error::Type("".to_string()));
        }

        let chunk =
            match Vec::<u8>::from_jsval(*cx, bytes.handle(), ConversionBehavior::EnforceRange) {
                Ok(ConversionResult::Success(val)) => val,
                _ => panic!("Couldn't convert jsval to Vec<u8>"),
            };
        Ok(chunk)
    }
}
