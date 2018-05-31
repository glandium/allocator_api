// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! dox

use alloc::Layout;

use core::sync::atomic::{AtomicPtr, Ordering};
use core::{mem, ptr};

static HOOK: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

/// Registers a custom OOM hook, replacing any that was previously registered.
///
/// The OOM hook is invoked when an infallible memory allocation fails.
/// The default hook prints a message to standard error and aborts the
/// execution, but this behavior can be customized with the [`set_oom_hook`]
/// and [`take_oom_hook`] functions.
///
/// The hook is provided with a `Layout` struct which contains information
/// about the allocation that failed.
///
/// The OOM hook is a global resource.
pub fn set_oom_hook(hook: fn(Layout) -> !) {
    HOOK.store(hook as *mut (), Ordering::SeqCst);
}

/// Unregisters the current OOM hook, returning it.
///
/// *See also the function [`set_oom_hook`].*
///
/// If no custom hook is registered, the default hook will be returned.
pub fn take_oom_hook() -> fn(Layout) -> ! {
    let hook = HOOK.swap(ptr::null_mut(), Ordering::SeqCst);
    if hook.is_null() {
        default_oom_hook
    } else {
        unsafe { mem::transmute(hook) }
    }
}

fn default_oom_hook(_layout: Layout) -> ! {
    loop {}
}

pub extern fn rust_oom(layout: Layout) -> ! {
    let hook = HOOK.load(Ordering::SeqCst);
    let hook: fn(Layout) -> ! = if hook.is_null() {
        default_oom_hook
    } else {
        unsafe { mem::transmute(hook) }
    };
    hook(layout)
}
