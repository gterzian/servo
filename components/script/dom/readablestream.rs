/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::dom::bindings::reflector::{DomObject, MutDomObject, Reflector};
use crate::dom::bindings::utils::set_dictionary_property;
use crate::dom::bindings::utils::AsCCharPtrPtr;
use crate::dom::promise::Promise;
use crate::script_runtime::JSContext as SafeJSContext;
use dom_struct::dom_struct;
use js::jsapi::{
    AddRawValueRoot, Heap, IsReadableStream, JSObject, JS_NewObject,
    ReadableStreamDefaultReaderRead, ReadableStreamGetReader, ReadableStreamIsDisturbed,
    ReadableStreamIsLocked, ReadableStreamReaderMode, RemoveRawValueRoot, UnwrapReadableStream,
};
use js::jsval::{JSVal, ObjectValue};
use js::rust::{IntoHandle, Runtime};
use std::rc::Rc;

/// Private helper to enable adding new methods to Rc<ReadableStream>.
trait ReadableStreamHelper {
    fn initialize(&self, cx: SafeJSContext);
}

impl ReadableStreamHelper for Rc<ReadableStream> {
    #[allow(unsafe_code)]
    fn initialize(&self, cx: SafeJSContext) {
        let obj = self.reflector().get_jsobject();
        unsafe {
            self.permanent_js_root
                .set(ObjectValue(UnwrapReadableStream(*obj)));
            assert!(AddRawValueRoot(
                *cx,
                self.permanent_js_root.get_unsafe(),
                b"ReadableStream::root\0".as_c_char_ptr()
            ));
        }
    }
}

impl Drop for ReadableStream {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        unsafe {
            let object = self.permanent_js_root.get().to_object();
            assert!(!object.is_null());
            let cx = Runtime::get();
            assert!(!cx.is_null());
            RemoveRawValueRoot(cx, self.permanent_js_root.get_unsafe());
        }
    }
}

#[dom_struct]
#[unrooted_must_root_lint::allow_unrooted_in_rc]
pub struct ReadableStream {
    reflector_: Reflector,
    #[ignore_malloc_size_of = "SM handles JS values"]
    permanent_js_root: Heap<JSVal>,
    /// This should be an object implementing `js::jsapi::ReadableStreamUnderlyingSource`.
    external_underlying_source: Option<Vec<u8>>,
}

impl ReadableStream {
    #[allow(unsafe_code, unrooted_must_root)]
    pub fn from_js(cx: SafeJSContext, obj: *mut JSObject) -> Result<Rc<ReadableStream>, ()> {
        unsafe {
            if !IsReadableStream(obj) {
                return Err(());
            }

            let stream = ReadableStream {
                reflector_: Reflector::new(),
                permanent_js_root: Heap::default(),
                external_underlying_source: None,
            };
            let mut stream = Rc::new(stream);

            Rc::get_mut(&mut stream).unwrap().init_reflector(obj);
            stream.initialize(cx);
            Ok(stream)
        }
    }

    /// Build a stream backed by a Rust underlying source.
    /// TODO: use an actual Rust underlying source to provide data asynchronously,
    /// see `js::jsapi::ReadableStreamUnderlyingSource`.
    pub fn new_with_external_underlying_source(source: Vec<u8>) -> Rc<ReadableStream> {
        Rc::new(ReadableStream {
            reflector_: Reflector::new(),
            permanent_js_root: Heap::default(),
            external_underlying_source: Some(source),
        })
    }

    /// Hack to make partial integration easier
    pub fn clone_body(&self) -> Option<Vec<u8>> {
        self.external_underlying_source.clone()
    }

    #[allow(unsafe_code)]
    pub fn read_a_chunk(&self) -> Rc<Promise> {
        let cx = self.global().get_cx();

        unsafe {
            rooted!(in(*cx) let stream = self.permanent_js_root.get().to_object());

            rooted!(in(*cx) let reader = ReadableStreamGetReader(
                *cx,
                stream.handle().into_handle(),
                ReadableStreamReaderMode::Default,
            ));

            rooted!(in(*cx) let promise_obj = ReadableStreamDefaultReaderRead(
                *cx,
                reader.handle().into_handle(),
            ));

            Promise::new_with_js_promise(promise_obj.handle(), cx)
        }
    }

    #[allow(unsafe_code)]
    pub fn is_locked_or_disturbed(&self) -> bool {
        let cx = self.global().get_cx();
        let mut locked_or_disturbed = false;

        unsafe {
            rooted!(in(*cx) let stream = self.permanent_js_root.get().to_object());
            ReadableStreamIsLocked(*cx, stream.handle().into_handle(), &mut locked_or_disturbed);
            ReadableStreamIsDisturbed(*cx, stream.handle().into_handle(), &mut locked_or_disturbed);
        }

        locked_or_disturbed
    }
}
