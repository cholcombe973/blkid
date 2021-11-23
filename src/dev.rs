// Copyright 2017 Red Hat, Inc.

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

use std::ffi::{CStr, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use blkid_sys::*;
use cache::Cache;
use tag::Tags;

pub struct Devs<'a> {
    pub cache: &'a Cache,
    pub iter: blkid_dev_iterate,
}

impl<'a> Drop for Devs<'a> {
    fn drop(&mut self) {
        unsafe { blkid_dev_iterate_end(self.iter) }
    }
}

impl<'a> Iterator for Devs<'a> {
    type Item = Dev;

    fn next(&mut self) -> Option<Self::Item> {
        let mut d: blkid_dev = ptr::null_mut();
        unsafe {
            match blkid_dev_next(self.iter, &mut d) {
                0 => Some(Dev::new(d)),
                _ => None,
            }
        }
    }
}

impl<'a> Devs<'a> {
    pub fn new(cache: &'a Cache) -> Devs<'a> {
        let iter = unsafe { blkid_dev_iterate_begin(cache.cache) };
        assert_ne!(iter, ptr::null_mut());
        Devs { cache, iter }
    }
}

pub struct Dev {
    pub dev: blkid_dev,
}

impl Dev {
    pub fn new(dev: blkid_dev) -> Dev {
        Dev { dev }
    }

    pub fn name(&self) -> &Path {
        let cstr = unsafe {
            let n_ptr = blkid_dev_devname(self.dev);
            assert_ne!(n_ptr, ptr::null_mut());
            CStr::from_ptr(n_ptr)
        };
        Path::new(OsStr::from_bytes(cstr.to_bytes()))
    }

    pub fn verify(&self, cache: &Cache) -> bool {
        unsafe { !blkid_verify(cache.cache, self.dev).is_null() }
    }

    pub fn tags(&self) -> Tags {
        Tags::new(self)
    }
}
