/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::cell::Cell;
use std::collections::VecDeque;
use std::mem;
use std::rc::Rc;

use dom_struct::dom_struct;
use js::jsapi::Heap;
use js::jsval::{JSVal, UndefinedValue};
use js::rust::{HandleObject as SafeHandleObject, HandleValue as SafeHandleValue};

use super::bindings::refcounted::Trusted;
use super::bindings::root::MutNullableDom;
use super::bindings::structuredclone;
use super::types::ReadableStreamDefaultController;
use super::underlyingsourcecontainer::TeeUnderlyingSource;
use crate::dom::bindings::cell::DomRefCell;
use crate::dom::bindings::codegen::Bindings::ReadableStreamDefaultReaderBinding::{
    ReadableStreamDefaultReaderMethods, ReadableStreamReadResult,
};
use crate::dom::bindings::error::Error;
use crate::dom::bindings::import::module::Fallible;
use crate::dom::bindings::reflector::{reflect_dom_object, DomObject, Reflector};
use crate::dom::bindings::root::{Dom, DomRoot};
use crate::dom::bindings::trace::RootedTraceableBox;
use crate::dom::globalscope::GlobalScope;
use crate::dom::promise::Promise;
use crate::dom::promisenativehandler::{Callback, PromiseNativeHandler};
use crate::dom::readablestream::ReadableStream;
use crate::microtask::Microtask;
use crate::realms::{enter_realm, InRealm};
use crate::script_runtime::{CanGc, JSContext as SafeJSContext};

#[derive(JSTraceable, MallocSizeOf)]
#[allow(crown::unrooted_must_root)]
pub struct TeeReadRequestMicrotask {
    #[ignore_malloc_size_of = "mozjs"]
    chunk: Box<Heap<JSVal>>,
    tee_read_request: Dom<TeeReadRequest>,
}

impl TeeReadRequestMicrotask {
    pub fn microtask_chunk_steps(&self) {
        self.tee_read_request.chunk_steps(&self.chunk)
    }
}

#[dom_struct]
pub struct TeeReadRequest {
    reflector_: Reflector,
    stream: Dom<ReadableStream>,
    branch_1: MutNullableDom<ReadableStream>,
    branch_2: MutNullableDom<ReadableStream>,
    #[ignore_malloc_size_of = "Rc"]
    reading: Rc<Cell<bool>>,
    #[ignore_malloc_size_of = "Rc"]
    read_again: Rc<Cell<bool>>,
    #[ignore_malloc_size_of = "Rc"]
    canceled_1: Rc<Cell<bool>>,
    #[ignore_malloc_size_of = "Rc"]
    canceled_2: Rc<Cell<bool>>,
    #[ignore_malloc_size_of = "Rc"]
    clone_for_branch_2: Rc<Cell<bool>>,
    #[ignore_malloc_size_of = "Rc"]
    cancel_promise: Rc<Promise>,
    #[ignore_malloc_size_of = "Rc"]
    tee_underlying_source: Dom<TeeUnderlyingSource>,
}

impl TeeReadRequest {
    #[allow(clippy::too_many_arguments)]
    #[allow(crown::unrooted_must_root)]
    pub fn new(
        stream: Dom<ReadableStream>,
        branch_1: MutNullableDom<ReadableStream>,
        branch_2: MutNullableDom<ReadableStream>,
        reading: Rc<Cell<bool>>,
        read_again: Rc<Cell<bool>>,
        canceled_1: Rc<Cell<bool>>,
        canceled_2: Rc<Cell<bool>>,
        clone_for_branch_2: Rc<Cell<bool>>,
        cancel_promise: Rc<Promise>,
        tee_underlying_source: Dom<TeeUnderlyingSource>,
    ) -> Self {
        println!("Branch 1: {:?}", branch_1.get().is_some());
        TeeReadRequest {
            reflector_: Reflector::new(),
            stream,
            branch_1,
            branch_2,
            reading,
            read_again,
            canceled_1,
            canceled_2,
            clone_for_branch_2,
            cancel_promise,
            tee_underlying_source,
        }
    }

    /// Call into error of the default controller of branch_1,
    /// <https://streams.spec.whatwg.org/#readable-stream-default-controller-error>
    pub fn branch_1_default_controller_error(&self, error: SafeHandleValue) {
        self.branch_1
            .get()
            .expect("branch_1 must be set")
            .get_default_controller()
            .error(error);
    }

    /// Call into error of the default controller of branch_2,
    /// <https://streams.spec.whatwg.org/#readable-stream-default-controller-error>
    pub fn branch_2_default_controller_error(&self, error: SafeHandleValue) {
        self.branch_2
            .get()
            .expect("branch_2 must be set")
            .get_default_controller()
            .error(error);
    }

    /// Call into cancel of the stream,
    /// <https://streams.spec.whatwg.org/#readable-stream-cancel>
    pub fn stream_cancel(&self, reason: SafeHandleValue, can_gc: CanGc) {
        self.stream.cancel(reason, can_gc);
    }

    /// Enqueue a microtask to perform the chunk steps
    /// <https://streams.spec.whatwg.org/#ref-for-read-request-chunk-steps%E2%91%A2>
    pub fn enqueue_chunk_steps(&self, chunk: RootedTraceableBox<Heap<JSVal>>) {
        // Queue a microtask to perform the following steps:
        let tee_read_request_chunk = TeeReadRequestMicrotask {
            chunk: Heap::boxed(*chunk.handle()),
            tee_read_request: Dom::from_ref(self),
        };
        let global = self.stream.global();
        let microtask_queue = global.microtask_queue();
        let cx = GlobalScope::get_cx();

        microtask_queue.enqueue(
            Microtask::ReadableStreamTeeReadRequest(tee_read_request_chunk),
            cx,
        );
    }

    /// <https://streams.spec.whatwg.org/#ref-for-read-request-chunk-steps%E2%91%A2>
    #[allow(unsafe_code)]
    pub fn chunk_steps(&self, chunk: &Box<Heap<JSVal>>) {
        // Set readAgain to false.
        self.read_again.set(false);
        // Let chunk1 and chunk2 be chunk.
        let chunk1 = chunk;
        let chunk2 = chunk;

        // If canceled_2 is false and cloneForBranch2 is true,
        if !self.canceled_2.get() && self.clone_for_branch_2.get() {
            let cx = GlobalScope::get_cx();
            // Let cloneResult be StructuredClone(chunk2).
            rooted!(in(*cx) let mut clone_result = UndefinedValue());
            let data = structuredclone::write(
                cx,
                unsafe { SafeHandleValue::from_raw(chunk2.handle()) },
                None,
            )
            .unwrap();

            // If cloneResult is an abrupt completion,
            if structuredclone::read(&self.stream.global(), data, clone_result.handle_mut())
                .is_err()
            {
                // Perform ! ReadableStreamDefaultControllerError(branch_1.[[controller]], cloneResult.[[Value]]).
                self.branch_1_default_controller_error(clone_result.handle());
                // Perform ! ReadableStreamDefaultControllerError(branch_2.[[controller]], cloneResult.[[Value]]).
                self.branch_2_default_controller_error(clone_result.handle());
                // Resolve cancelPromise with ! ReadableStreamCancel(stream, cloneResult.[[Value]]).
                self.stream_cancel(clone_result.handle(), CanGc::note());

                // Return.
                return;
            } else {
                // Otherwise, set chunk2 to cloneResult.[[Value]].
                chunk2.set(*clone_result);
            }
        }

        // If canceled_1 is false, perform ! ReadableStreamDefaultControllerEnqueue(branch_1.[[controller]], chunk1).
        if !self.canceled_1.get() {
            self.branch_1_default_controller_enqueue(unsafe {
                SafeHandleValue::from_raw(chunk1.handle())
            });
        }
        // If canceled_2 is false, perform ! ReadableStreamDefaultControllerEnqueue(branch_2.[[controller]], chunk2).
        if !self.canceled_2.get() {
            self.branch_2_default_controller_enqueue(unsafe {
                SafeHandleValue::from_raw(chunk2.handle())
            });
        }
        // Set reading to false.
        self.reading.set(false);

        // If readAgain is true, perform pullAlgorithm.
        if self.read_again.get() {
            self.pull_algorithm();
        }
    }

    /// <https://streams.spec.whatwg.org/#read-request-close-steps>
    pub fn close_steps(&self) {
        // Set reading to false.
        self.reading.set(false);

        // If canceled_1 is false, perform ! ReadableStreamDefaultControllerClose(branch_1.[[controller]]).
        if !self.canceled_1.get() {
            self.branch_1_default_controller_close();
        }
        // If canceled_2 is false, perform ! ReadableStreamDefaultControllerClose(branch_2.[[controller]]).
        if !self.canceled_2.get() {
            self.branch_2_default_controller_close();
        }
        // If canceled_1 is false or canceled_2 is false, resolve cancelPromise with undefined.
        if !self.canceled_1.get() || !self.canceled_2.get() {
            self.cancel_promise.resolve_native(&());
        }
    }

    /// <https://streams.spec.whatwg.org/#read-request-error-steps>
    pub fn error_steps(&self) {
        // Set reading to false.
        self.reading.set(false);
    }

    /// Call into enqueue of the default controller of branch_1,
    /// <https://streams.spec.whatwg.org/#readable-stream-default-controller-enqueue>
    pub fn branch_1_default_controller_enqueue(&self, chunk: SafeHandleValue) {
        let _ = self
            .branch_1
            .get()
            .expect("branch_1 must be set")
            .get_default_controller()
            .enqueue(GlobalScope::get_cx(), chunk, CanGc::note());
    }

    /// Call into enqueue of the default controller of branch_2,
    /// <https://streams.spec.whatwg.org/#readable-stream-default-controller-enqueue>
    pub fn branch_2_default_controller_enqueue(&self, chunk: SafeHandleValue) {
        let _ = self
            .branch_2
            .get()
            .expect("branch_2 must be set")
            .get_default_controller()
            .enqueue(GlobalScope::get_cx(), chunk, CanGc::note());
    }

    /// Call into close of the default controller of branch_1,
    /// <https://streams.spec.whatwg.org/#readable-stream-default-controller-close>
    pub fn branch_1_default_controller_close(&self) {
        self.branch_1
            .get()
            .expect("branch_1 must be set")
            .get_default_controller()
            .close();
    }

    /// Call into close of the default controller of branch_2,
    /// <https://streams.spec.whatwg.org/#readable-stream-default-controller-close>
    pub fn branch_2_default_controller_close(&self) {
        self.branch_2
            .get()
            .expect("branch_2 must be set")
            .get_default_controller()
            .close();
    }

    pub fn pull_algorithm(&self) {
        self.tee_underlying_source.pull_algorithm();
    }
}
