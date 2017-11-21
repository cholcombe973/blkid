// Copyright 2017 Red Hat, Inc.

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

use std::ptr;

use BlkidError;
use dev::Devs;
use blkid_sys::*;
use result;


#[derive(Debug)]
pub struct Cache {
    pub cache: blkid_cache,
}

impl Cache {
    pub fn new() -> Result<Cache, BlkidError> {
        let mut c: blkid_cache = ptr::null_mut();
        unsafe {
            result(blkid_get_cache(&mut c, ptr::null()))?;
            result(blkid_probe_all(c))?;
        }
        Ok(Cache { cache: c })
    }

    pub fn devs(&self) -> Devs {
        Devs::new(self)
    }
}

impl Drop for Cache {
    fn drop(&mut self) -> () {
        unsafe { blkid_put_cache(self.cache) }
    }
}
