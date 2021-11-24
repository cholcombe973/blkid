// Copyright 2017 Red Hat, Inc.

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

use crate::{cache::Cache, tag::Tags};
use bitflags::bitflags;
use blkid_sys::*;
use std::{
    ffi::{CStr, OsStr},
    os::unix::ffi::OsStrExt,
    path::Path,
    ptr,
};

/// Wrapper around device iterator
pub struct Devs {
    pub(crate) iter: blkid_dev_iterate,
}

impl Drop for Devs {
    fn drop(&mut self) {
        unsafe { blkid_dev_iterate_end(self.iter) }
    }
}

impl Iterator for Devs {
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

impl Devs {
    /// Creates wrapper around device
    pub fn new(cache: &Cache) -> Devs {
        let iter = unsafe { blkid_dev_iterate_begin(cache.0) };
        assert_ne!(iter, ptr::null_mut());
        Devs { iter }
    }
}

/// The device object keeps information about one device
pub struct Dev(pub(crate) blkid_dev);

impl Dev {
    /// Creates a new device from raw pointer
    pub fn new(dev: blkid_dev) -> Dev {
        Dev(dev)
    }

    /// Returns device name. This name does not have to be canonical (real path) name, but for
    /// example symlink
    pub fn name(&self) -> &Path {
        let cstr = unsafe {
            let n_ptr = blkid_dev_devname(self.0);
            assert_ne!(n_ptr, ptr::null_mut());
            CStr::from_ptr(n_ptr)
        };
        Path::new(OsStr::from_bytes(cstr.to_bytes()))
    }

    /// Verify that the data in dev is consistent with what is on the actual block device (using the
    /// devname field only). Normally this will be called when finding items in the cache, but for
    /// long running processes is also desirable to revalidate an item before use.
    ///
    /// If we are unable to revalidate the data, we return the old data and do not set the
    /// `BLKID_BID_FL_VERIFIED` flag on it.
    pub fn verify(&self, cache: &Cache) -> bool {
        unsafe { !blkid_verify(cache.0, self.0).is_null() }
    }

    /// Returns device's tags
    pub fn tags(&self) -> Tags {
        Tags::new(self)
    }
}

bitflags! {
    pub struct GetDevFlags: i32 {
        /// Just look up a device entry, and return NULL if it is not found
        const FIND = 0x0000;
        /// Create an empty device structure if not found in the cache
        const CREATE = 0x0001;
        /// Make sure the device structure corresponds with reality
        const VERIFY = 0x0002;
        /// Get a valid device structure, either from the cache or by probing the device
        const NORMAL = Self::CREATE.bits | Self::VERIFY.bits;
    }
}
