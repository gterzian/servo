/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::cell::DomRefCell;
use crate::dom::bindings::codegen::Bindings::ReadableStreamBinding;
use crate::dom::bindings::conversions::{ConversionBehavior, ConversionResult};
use crate::dom::bindings::error::Error;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use crate::dom::bindings::root::DomRoot;
use crate::dom::bindings::utils::get_dictionary_property;
use crate::dom::globalscope::GlobalScope;
use crate::dom::promise::Promise;
use crate::js::conversions::FromJSValConvertible;
use crate::realms::{AlreadyInRealm, InRealm};
use crate::script_runtime::JSContext as SafeJSContext;
use dom_struct::dom_struct;
use js::glue::{CreateReadableStreamUnderlyingSource, ReadableStreamUnderlyingSourceTraps};
use js::jsapi::{
    HandleObject, Heap, IsReadableStream, JSContext, JSObject,
    NewReadableExternalSourceStreamObject, ReadableStreamDefaultReaderRead, ReadableStreamError,
    ReadableStreamGetReader, ReadableStreamIsDisturbed, ReadableStreamIsLocked,
    ReadableStreamReaderMode, ReadableStreamReaderReleaseLock, ReadableStreamUnderlyingSource,
    ReadableStreamUpdateDataAvailableFromSource, UnwrapReadableStream,
};
use js::jsval::UndefinedValue;
use js::rust::HandleValue as SafeHandleValue;
use js::rust::IntoHandle;
use std::cell::Cell;
use std::os::raw::c_void;
use std::ptr::{self, NonNull};
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
    external_underlying_source: Option<ExternalUnderlyingSourceWrapper>,
}

impl ReadableStream {
    fn new_inherited(
        external_underlying_source: Option<ExternalUnderlyingSourceWrapper>,
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
        external_underlying_source: Option<ExternalUnderlyingSourceWrapper>,
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
        source: Option<ExternalUnderlyingSourceWrapper>,
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
        source: ExternalUnderlyingSource,
    ) -> DomRoot<ReadableStream> {
        let mut source = ExternalUnderlyingSourceWrapper::new(source);
        unsafe {
            let global = GlobalScope::current().expect("No current global object.");
            let cx = global.get_cx();

            rooted!(in(*cx) let proto = UndefinedValue());
            rooted!(in(*cx) let proto_obj = proto.to_object());
            rooted!(in(*cx)
                let js_stream = NewReadableExternalSourceStreamObject(
                    *cx,
                    source.create_js_wrapper(),
                    proto_obj.handle().into_handle(),
                )
            );

            ReadableStream::from_js_with_source(cx, js_stream.get(), Some(source))
        }
    }

    /// Get a pointer to the underlying JS object.
    pub fn get_js_stream(&self) -> NonNull<JSObject> {
        NonNull::new(self.js_stream.get())
            .expect("Couldn't get a non-null pointer to JS stream object.")
    }

    /// Hack to make partial integration easier
    pub fn clone_body(&self) -> Option<Vec<u8>> {
        self.external_underlying_source
            .as_ref()
            .and_then(|source| source.clone_body())
    }

    #[allow(unsafe_code)]
    pub fn enqueue_native(&self, bytes: Vec<u8>) {
        let global = GlobalScope::current().expect("No current global object.");
        let cx = global.get_cx();

        let stream_handle = unsafe { self.js_stream.handle() };

        self.external_underlying_source
            .as_ref()
            .expect("No external source to enqueue bytes.")
            .enqueue_chunk(cx, stream_handle, bytes);
    }

    #[allow(unsafe_code)]
    pub fn error_native(&self, error: Error) {
        let global = GlobalScope::current().expect("No current global object.");
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
        // If we natively took a reader, we're disturbed(Note: or is that only if reading has started?).
        if self.has_reader.get() {
            return true;
        }

        // Otherwise, still double-check that script didn't disturb the stream.
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
    let source = &*(source as *const ExternalUnderlyingSourceWrapper);
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
    let source = &*(source as *const ExternalUnderlyingSourceWrapper);
    source.write_into_buffer(
        SafeJSContext::from_ptr(cx),
        stream,
        buffer,
        length,
        bytes_written,
    );
}

#[derive(Clone, JSTraceable, MallocSizeOf)]
pub enum ExternalUnderlyingSource {
    /// Facilitate partial integration with sources
    /// that are currently read into memory.
    Memory(Vec<u8>),
    /// A blob as underlying source, with a known total size.
    Blob(usize),
}

#[derive(JSTraceable, MallocSizeOf)]
struct ExternalUnderlyingSourceWrapper {
    source: DomRefCell<ExternalUnderlyingSource>,
    buffer: DomRefCell<Vec<u8>>,
}

impl ExternalUnderlyingSourceWrapper {
    fn new(source: ExternalUnderlyingSource) -> ExternalUnderlyingSourceWrapper {
        let buffer = match source {
            ExternalUnderlyingSource::Blob(size) => Vec::with_capacity(size),
            ExternalUnderlyingSource::Memory(_) => Vec::with_capacity(0),
        };
        ExternalUnderlyingSourceWrapper {
            source: DomRefCell::new(source),
            buffer: DomRefCell::new(buffer),
        }
    }

    #[allow(unsafe_code)]
    fn create_js_wrapper(&mut self) -> *mut ReadableStreamUnderlyingSource {
        let mut traps = ReadableStreamUnderlyingSourceTraps {
            requestData: Some(request_data),
            writeIntoReadRequestBuffer: Some(write_into_read_request_buffer),
            cancel: None,
            onClosed: None,
            onErrored: None,
            finalize: None,
        };
        unsafe { CreateReadableStreamUnderlyingSource(&mut traps, self as *mut _ as *mut c_void) }
    }

    #[allow(unsafe_code)]
    fn signal_available_bytes(&self, cx: SafeJSContext, stream: HandleObject) {
        let available = match &*self.source.borrow() {
            ExternalUnderlyingSource::Memory(vec) => vec.len(),
            ExternalUnderlyingSource::Blob(_) => self.buffer.borrow().len(),
        };
        if available > 0 {
            // We have bytes available in memory, or from a blob push source.
            unsafe {
                ReadableStreamUpdateDataAvailableFromSource(*cx, stream, available as u32);
            }
        }
    }

    fn enqueue_chunk(&self, cx: SafeJSContext, stream: HandleObject, mut chunk: Vec<u8>) {
        match &*self.source.borrow() {
            ExternalUnderlyingSource::Blob(_) => {
                self.buffer.borrow_mut().append(&mut chunk);
            },
            ExternalUnderlyingSource::Memory(_) => {
                panic!("Memory source should not enqueue chunks.");
            },
        }
        self.signal_available_bytes(cx, stream);
    }

    fn pull(&self, cx: SafeJSContext, stream: HandleObject, _desired_size: usize) {
        // Note: for pull sources,
        // this would be the time to ask for a chunk.
        self.signal_available_bytes(cx, stream);
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
        let source = match &mut *self.source.borrow_mut() {
            ExternalUnderlyingSource::Memory(vec) => {
                assert!(vec.len() >= length as usize);
                vec.split_off(length)
            },
            ExternalUnderlyingSource::Blob(_) => {
                let mut buffer = self.buffer.borrow_mut();
                assert!(buffer.len() >= length as usize);
                buffer.split_off(length)
            },
        };
        unsafe {
            ptr::copy_nonoverlapping(
                source.as_ptr() as *const _ as *const c_void,
                buffer,
                source.len(),
            );
            *bytes_written = length;
        };
    }

    /// Hack to enable partial integration
    /// for bodies that have already been read into memory.
    fn clone_body(&self) -> Option<Vec<u8>> {
        match &*self.source.borrow() {
            ExternalUnderlyingSource::Memory(vec) => Some(vec.clone()),
            ExternalUnderlyingSource::Blob(_) => None,
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
                _ => panic!("Couldn't convert jsval to Vec<u8>"),
            };
        Ok(chunk)
    }
}
