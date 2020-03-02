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
use js::jsapi::{
    AddRawValueRoot, Heap, IsReadableStream, JSObject, JS_NewObject,
    ReadableStreamDefaultReaderRead, ReadableStreamGetReader, ReadableStreamIsDisturbed,
    ReadableStreamIsLocked, ReadableStreamReaderMode, RemoveRawValueRoot, UnwrapReadableStream,
};
use js::jsval::{JSVal, ObjectValue};
use js::rust::{IntoHandle, Runtime};
use std::rc::Rc;

#[dom_struct]
#[unrooted_must_root_lint::allow_unrooted_in_rc]
pub struct ReadableStream {
    reflector_: Reflector,
    #[ignore_malloc_size_of = "SM handles JS values"]
    js_stream: Heap<*mut JSObject>,
    /// This should be an object implementing `js::jsapi::ReadableStreamUnderlyingSource`.
    external_underlying_source: Option<Vec<u8>>,
}

impl ReadableStream {
    fn new_inherited(external_underlying_source: Option<Vec<u8>>) -> ReadableStream {
        ReadableStream {
            reflector_: Reflector::new(),
            js_stream: Heap::default(),
            external_underlying_source,
        }
    }

    pub fn new(
        global: &GlobalScope,
        external_underlying_source: Option<Vec<u8>>,
    ) -> DomRoot<ReadableStream> {
        reflect_dom_object(
            Box::new(ReadableStream::new_inherited(external_underlying_source)),
            global,
            ReadableStreamBinding::Wrap,
        )
    }

    #[allow(unsafe_code, unrooted_must_root)]
    pub fn from_js(cx: SafeJSContext, obj: *mut JSObject) -> Result<DomRoot<ReadableStream>, ()> {
        unsafe {
            if !IsReadableStream(obj) {
                return Err(());
            }

            let in_realm_proof = AlreadyInRealm::assert_for_cx(cx);
            let global = GlobalScope::from_context(*cx, InRealm::Already(&in_realm_proof));

            let stream = ReadableStream::new(&global, None);
            stream.js_stream.set(UnwrapReadableStream(obj));

            Ok(stream)
        }
    }

    /// Build a stream backed by a Rust underlying source.
    /// TODO: use an actual Rust underlying source to provide data asynchronously,
    /// see `js::jsapi::ReadableStreamUnderlyingSource`.
    pub fn new_with_external_underlying_source(source: Vec<u8>) -> DomRoot<ReadableStream> {
        let global = GlobalScope::current().expect("No current global object.");
        ReadableStream::new(&*global, Some(source))
    }

    /// Hack to make partial integration easier
    pub fn clone_body(&self) -> Option<Vec<u8>> {
        self.external_underlying_source.clone()
    }

    #[allow(unsafe_code)]
    pub fn read_a_chunk(&self) -> Rc<Promise> {
        let cx = self.global().get_cx();

        unsafe {
            rooted!(in(*cx) let stream = self.js_stream.get());

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
            rooted!(in(*cx) let stream = self.js_stream.get());
            ReadableStreamIsLocked(*cx, stream.handle().into_handle(), &mut locked_or_disturbed);
            ReadableStreamIsDisturbed(*cx, stream.handle().into_handle(), &mut locked_or_disturbed);
        }

        locked_or_disturbed
    }
}
