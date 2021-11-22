// Copyright 2017 Red Hat, Inc.

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

use std::borrow::Cow;
use std::ffi::CStr;
use std::ptr;

use blkid_sys::*;
use dev::Dev;

pub struct Tags<'a> {
    pub dev: &'a Dev,
    pub iter: blkid_tag_iterate,
}

impl<'a> Tags<'a> {
    pub fn new(dev: &'a Dev) -> Tags<'a> {
        let iter = unsafe { blkid_tag_iterate_begin(dev.dev) };
        assert_ne!(iter, ptr::null_mut());
        Tags { dev, iter }
    }
}

impl<'a> Drop for Tags<'a> {
    fn drop(&mut self) {
        unsafe { blkid_tag_iterate_end(self.iter) }
    }
}

impl<'a> Iterator for Tags<'a> {
    type Item = (Cow<'a, str>, Cow<'a, str>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut k = ptr::null();
        let mut v = ptr::null();
        unsafe {
            match blkid_tag_next(self.iter, &mut k, &mut v) {
                0 => {
                    let k_cow = CStr::from_ptr(k).to_string_lossy();
                    let v_cow = CStr::from_ptr(v).to_string_lossy();
                    Some((k_cow, v_cow))
                }
                _ => None,
            }
        }
    }
}
