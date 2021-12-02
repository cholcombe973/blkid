use crate::{
    dev::{Dev, Devs, GetDevFlags},
    error::c_result,
    path_to_cstring,
    tag::{Tag, TagType},
    BlkIdResult,
};
use blkid_sys::*;
use std::{
    ffi::{CStr, CString},
    path::Path,
    ptr,
};

#[derive(Debug)]
pub struct Cache(pub(crate) blkid_cache);

impl Drop for Cache {
    fn drop(&mut self) {
        unsafe { blkid_put_cache(self.0) }
    }
}

impl Cache {
    /// Creates and initialize cache handler by default path. Default path can be overridden by the
    /// environment variable `BLKID_FILE`
    pub fn new() -> BlkIdResult<Self> {
        let mut cache: blkid_cache = ptr::null_mut();
        unsafe { c_result(blkid_get_cache(&mut cache, ptr::null())) }?;
        Ok(Self(cache))
    }

    /// Creates and initialize cache hadler by particular path
    pub fn new_by_path<P: AsRef<Path>>(path: P) -> BlkIdResult<Self> {
        let mut cache: blkid_cache = ptr::null_mut();
        let path = path_to_cstring(path)?;
        unsafe { c_result(blkid_get_cache(&mut cache, path.as_ptr())) }?;
        Ok(Self(cache))
    }

    /// Probes all block devices
    pub fn probe_all(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_all(self.0)).map(|_| ()) }
    }

    /// Probes all new block devices
    pub fn prob_all_new(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_all_new(self.0)).map(|_| ()) }
    }

    /// The `libblkid` probing is based on devices from `/proc/partitions` by default. This file
    /// usually does not contain removable devices (e.g. CDROMs) and this kind of devices are
    /// invisible for `libblkid`.
    ///
    /// This function adds removable block devices to cache (probing is based on information from
    /// the `/sys` directory). Don't forget that removable devices (floppies, CDROMs, ...) could be
    /// pretty slow. It's very bad idea to call this function by default.
    ///
    /// # Note
    ///
    /// Devices which were detected by this function won't be written to `blkid.tab` cache file
    pub fn probe_all_removable(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_all_removable(self.0)).map(|_| ()) }
    }

    /// Returns iterator over all devices are found by probe
    pub fn devs(&self) -> Devs {
        Devs::new(self)
    }

    /// Find a dev struct in the cache by device name, if available.
    ///
    /// If there is no entry with the specified device name, and the [`GetDevFlag::CREATE`] is set,
    /// then create an empty device entry
    pub fn get_dev(&self, name: &str, flags: GetDevFlags) -> BlkIdResult<Dev> {
        let devname = CString::new(name)?;
        let dev = unsafe { c_result(blkid_get_dev(self.0, devname.as_ptr(), flags.bits())) }?;
        Ok(Dev(dev))
    }

    /// Returns a device which matches a particular [`Tag`].
    ///
    /// If there is more than one device that matches the search specification, it returns the one
    /// with the highest priority value. This allows us to give preference to `EVMS` or `LVM` devices
    pub fn find_dev_with_tag(&self, tag: Tag) -> BlkIdResult<Option<Dev>> {
        let name = CString::new(tag.name())?;
        let value = CString::new(tag.value())?;
        let dev = unsafe { blkid_find_dev_with_tag(self.0, name.as_ptr(), value.as_ptr()) };

        if dev.is_null() {
            Ok(None)
        } else {
            Ok(Some(Dev(dev)))
        }
    }

    /// Find a tag name (e.g. [`TagType::Label`] or [`TagType::Uuid`]) on a specific device
    pub fn find_tag_value(&self, tag_type: TagType, dev_name: &str) -> BlkIdResult<Option<String>> {
        let tagname = CString::new(tag_type.to_string())?;
        let devname = CString::new(dev_name)?;
        let ptr = unsafe { blkid_get_tag_value(self.0, tagname.as_ptr(), devname.as_ptr()) };

        if ptr.is_null() {
            Ok(None)
        } else {
            let value = unsafe { CStr::from_ptr(ptr).to_str()?.to_owned() };
            Ok(Some(value))
        }
    }

    /// Removes garbage (non-existing devices) from the cache
    pub fn gc(&self) {
        unsafe { blkid_gc_cache(self.0) }
    }
}
