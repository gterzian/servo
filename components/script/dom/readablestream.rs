/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::conversions::{ConversionBehavior, ConversionResult};
use crate::dom::bindings::error::Error;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use crate::dom::bindings::root::DomRoot;
use crate::dom::bindings::settings_stack::AutoIncumbentScript;
use crate::dom::bindings::utils::get_dictionary_property;
use crate::dom::globalscope::GlobalScope;
use crate::dom::promise::Promise;
use crate::js::conversions::FromJSValConvertible;
use crate::realms::{enter_realm, AlreadyInRealm, InRealm};
use crate::script_runtime::JSContext as SafeJSContext;
use dom_struct::dom_struct;
use js::glue::{
    CreateReadableStreamUnderlyingSource, DeleteReadableStreamUnderlyingSource,
    ReadableStreamUnderlyingSourceTraps,
};
use js::jsapi::{HandleObject, HandleValue, Heap, JSContext, JSObject};
use js::jsapi::{
    IsReadableStream, NewReadableExternalSourceStreamObject, ReadableStreamClose,
    ReadableStreamDefaultReaderRead, ReadableStreamError, ReadableStreamGetReader,
    ReadableStreamIsDisturbed, ReadableStreamIsLocked, ReadableStreamIsReadable,
    ReadableStreamReaderMode, ReadableStreamReaderReleaseLock, ReadableStreamUnderlyingSource,
    ReadableStreamUpdateDataAvailableFromSource, UnwrapReadableStream,
};
use js::jsval::JSVal;
use js::jsval::UndefinedValue;
use js::rust::HandleValue as SafeHandleValue;
use js::rust::IntoHandle;
use std::cell::{Cell, RefCell};
use std::os::raw::c_void;
use std::ptr::{self, NonNull};
use std::rc::Rc;

#[dom_struct]
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
        )
    }

    /// Used from RustCodegen.py
    #[allow(unsafe_code)]
    pub fn from_js(cx: SafeJSContext, obj: *mut JSObject) -> Result<DomRoot<ReadableStream>, ()> {
        unsafe {
            if !IsReadableStream(obj) {
                return Err(());
            }

            let in_realm_proof = AlreadyInRealm::assert_for_cx(cx);
            let global = GlobalScope::from_safe_context(cx, InRealm::Already(&in_realm_proof));

            let stream = ReadableStream::new(&global, None);
            stream.js_stream.set(UnwrapReadableStream(obj));

            Ok(stream)
        }
    }

    /// Build a stream backed by a Rust underlying source.
    #[allow(unsafe_code)]
    pub fn new_with_external_underlying_source(
        global: &GlobalScope,
        source: ExternalUnderlyingSource,
    ) -> DomRoot<ReadableStream> {
        let _ar = enter_realm(global);
        let cx = global.get_cx();

        let source = Rc::new(ExternalUnderlyingSourceController::new(source));

        let stream = ReadableStream::new(&global, Some(source.clone()));

        unsafe {
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
                    ptr::null_mut() as *mut c_void,
                    HandleObject::null(),
                )
            );

            stream.js_stream.set(UnwrapReadableStream(js_stream.get()));
        }

        stream
    }

    /// Get a pointer to the underlying JS object.
    pub fn get_js_stream(&self) -> NonNull<JSObject> {
        NonNull::new(self.js_stream.get())
            .expect("Couldn't get a non-null pointer to JS stream object.")
    }

    #[allow(unsafe_code)]
    pub fn enqueue_native(&self, bytes: Vec<u8>) {
        let global = self.global();
        let _ar = enter_realm(&*global);
        let cx = global.get_cx();

        let stream_handle = unsafe { self.js_stream.handle() };

        self.external_underlying_source
            .as_ref()
            .expect("No external source to enqueue bytes.")
            .enqueue_chunk(cx, stream_handle, bytes);
    }

    #[allow(unsafe_code)]
    pub fn error_native(&self, error: Error) {
        let global = self.global();
        let _ar = enter_realm(&*global);
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

    #[allow(unsafe_code)]
    pub fn close_native(&self) {
        let global = self.global();
        let _ar = enter_realm(&*global);
        let cx = global.get_cx();

        let handle = unsafe { self.js_stream.handle() };

        self.external_underlying_source
            .as_ref()
            .expect("No external source to close.")
            .close(cx, handle);
    }

    /// Acquires a reader and locks the stream,
    /// must be done before `read_a_chunk`.
    #[allow(unsafe_code)]
    pub fn start_reading(&self) -> Result<(), ()> {
        if self.is_locked() || self.is_disturbed() {
            return Err(());
        }

        let global = self.global();
        let _ar = enter_realm(&*global);
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

        let global = self.global();
        let _ar = enter_realm(&*global);
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
            panic!("ReadableStream::stop_reading called on a readerless stream.");
        }

        self.has_reader.set(false);

        let global = self.global();
        let _ar = enter_realm(&*global);
        let cx = global.get_cx();

        unsafe {
            ReadableStreamReaderReleaseLock(*cx, self.js_reader.handle());
            // Note: is this the way to nullify the Heap?
            self.js_reader.set(ptr::null_mut());
        }
    }

    #[allow(unsafe_code)]
    pub fn is_locked(&self) -> bool {
        // If we natively took a reader, we're locked.
        if self.has_reader.get() {
            return true;
        }

        // Otherwise, still double-check that script didn't lock the stream.
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
        // Check that script didn't disturb the stream.
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
    _resolve_to: *mut JSVal,
) {
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
unsafe extern "C" fn finalize(source: *mut ReadableStreamUnderlyingSource) {
    DeleteReadableStreamUnderlyingSource(source);
}

pub enum ExternalUnderlyingSource {
    /// Facilitate partial integration with sources
    /// that are currently read into memory.
    Memory(usize),
    /// A blob as underlying source, with a known total size.
    Blob(usize),
    /// A fetch response as underlying source.
    FetchResponse,
}

#[derive(JSTraceable, MallocSizeOf)]
struct ExternalUnderlyingSourceController {
    /// Loosely matches the underlying queue,
    /// <https://streams.spec.whatwg.org/#internal-queues>
    buffer: RefCell<Vec<u8>>,
    /// Has the stream been closed by native code?
    closed: RefCell<bool>,
}

impl ExternalUnderlyingSourceController {
    fn new(source: ExternalUnderlyingSource) -> ExternalUnderlyingSourceController {
        let buffer = match source {
            ExternalUnderlyingSource::Blob(size) | ExternalUnderlyingSource::Memory(size) => {
                Vec::with_capacity(size)
            },
            ExternalUnderlyingSource::FetchResponse => vec![],
        };
        ExternalUnderlyingSourceController {
            buffer: RefCell::new(buffer),
            closed: RefCell::new(false),
        }
    }

    /// Signal available bytes if the stream is currently readable.
    #[allow(unsafe_code)]
    fn maybe_signal_available_bytes(
        &self,
        cx: SafeJSContext,
        stream: HandleObject,
        available: usize,
    ) {
        if available == 0 {
            return;
        }
        unsafe {
            let mut readable = false;
            if !ReadableStreamIsReadable(*cx, stream, &mut readable) {
                return;
            }
            if readable {
                ReadableStreamUpdateDataAvailableFromSource(*cx, stream, available as u32);
            }
        }
    }

    /// Close a currently readable js stream.
    #[allow(unsafe_code)]
    fn maybe_close_js_stream(&self, cx: SafeJSContext, stream: HandleObject) {
        unsafe {
            let mut readable = false;
            if !ReadableStreamIsReadable(*cx, stream, &mut readable) {
                return;
            }
            if readable {
                ReadableStreamClose(*cx, stream);
            }
        }
    }

    fn close(&self, cx: SafeJSContext, stream: HandleObject) {
        *self.closed.borrow_mut() = true;
        self.maybe_close_js_stream(cx, stream);
    }

    fn enqueue_chunk(&self, cx: SafeJSContext, stream: HandleObject, mut chunk: Vec<u8>) {
        let available = {
            let mut buffer = self.buffer.borrow_mut();
            buffer.append(&mut chunk);
            buffer.len()
        };
        self.maybe_signal_available_bytes(cx, stream, available);
    }

    #[allow(unsafe_code)]
    fn pull(&self, cx: SafeJSContext, stream: HandleObject, _desired_size: usize) {
        // Note: for pull sources,
        // this would be the time to ask for a chunk.

        if { *self.closed.borrow() } {
            return self.maybe_close_js_stream(cx, stream);
        }

        let available = {
            let buffer = self.buffer.borrow();
            buffer.len()
        };

        self.maybe_signal_available_bytes(cx, stream, available);
    }

    fn get_chunk_with_length(&self, length: usize) -> Vec<u8> {
        let mut buffer = self.buffer.borrow_mut();
        let buffer_len = buffer.len();
        assert!(buffer_len >= length as usize);
        buffer.split_off(buffer_len - length)
    }

    #[allow(unsafe_code)]
    fn write_into_buffer(
        &self,
        _cx: SafeJSContext,
        _stream: HandleObject,
        buffer: *mut c_void,
        length: usize,
        bytes_written: *mut usize,
    ) {
        let chunk = self.get_chunk_with_length(length);

        unsafe {
            *bytes_written = chunk.len();
            ptr::copy_nonoverlapping(chunk.as_ptr(), buffer as *mut u8, chunk.len());
        }
    }
}

#[allow(unsafe_code)]
/// Get the `done` property of an object that a read promise resolved to.
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

#[allow(unsafe_code)]
/// Get the `value` property of an object that a read promise resolved to.
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
                _ => return Err(Error::Type("".to_string())),
            };
        Ok(chunk)
    }
}
