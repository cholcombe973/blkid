use crate::{cache::Cache, error::c_result, tag::Tags, BlkIdResult};
use bitflags::bitflags;
use blkid_sys::*;
use std::{
    ffi::{CStr, CString, OsStr},
    marker::PhantomData,
    os::unix::ffi::OsStrExt,
    path::Path,
    ptr,
};

/// Wrapper around device iterator
pub struct Devs<'a> {
    pub(crate) iter: blkid_dev_iterate,
    _marker: PhantomData<&'a Cache>,
}

impl Drop for Devs<'_> {
    fn drop(&mut self) {
        unsafe { blkid_dev_iterate_end(self.iter) }
    }
}

impl<'a> Iterator for Devs<'a> {
    type Item = Dev<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut d: blkid_dev = ptr::null_mut();
        unsafe {
            match blkid_dev_next(self.iter, &mut d) {
                0 => Some(Dev(d, PhantomData)),
                _ => None,
            }
        }
    }
}

impl<'a> Devs<'a> {
    /// Creates wrapper around device
    pub fn new(cache: &'a Cache) -> Devs<'a> {
        let iter = unsafe { blkid_dev_iterate_begin(cache.0) };
        assert_ne!(iter, ptr::null_mut());
        Devs {
            iter,
            _marker: PhantomData,
        }
    }

    /// Set search filter for the device iterator. Only devices matching the
    /// given `search_type` and `search_value` will be returned.
    pub fn set_search(&mut self, search_type: &str, search_value: &str) -> BlkIdResult<()> {
        let s_type = CString::new(search_type)?;
        let s_value = CString::new(search_value)?;
        unsafe {
            c_result(blkid_dev_set_search(self.iter, s_type.as_ptr(), s_value.as_ptr()), "blkid_dev_set_search")
                .map(|_| ())
        }
    }
}

/// The device object keeps information about one device
pub struct Dev<'a>(pub(crate) blkid_dev, PhantomData<&'a Cache>);

impl<'a> Dev<'a> {
    pub(crate) fn new(dev: blkid_dev) -> Dev<'a> {
        Dev(dev, PhantomData)
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

    /// Returns `true` if the device has the specified tag type and value.
    pub fn has_tag(&self, tag_type: &str, value: &str) -> bool {
        let Ok(tag_type) = CString::new(tag_type) else {
            return false;
        };
        let Ok(value) = CString::new(value) else {
            return false;
        };
        unsafe { blkid_dev_has_tag(self.0, tag_type.as_ptr(), value.as_ptr()) != 0 }
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
