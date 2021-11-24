// Copyright 2017 Red Hat, Inc.

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

use crate::{error::c_result, partition::Partition, BlkIdResult};
use blkid_sys::*;
use std::{ffi::CStr, str::FromStr};
use strum_macros::{Display, EnumString};

/// Information about a partition table
#[derive(Debug)]
pub struct PartTable(pub(crate) blkid_parttable);

impl PartTable {
    /// Returns partition table ID (for example GPT disk UUID).
    ///
    /// The ID is GPT disk UUID or DOS disk ID (in hex format).
    pub fn get_id(&self) -> Option<String> {
        let ptr = unsafe { blkid_parttable_get_id(self.0) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr).to_string_lossy().to_string() })
        }
    }

    /// Returns position (in bytes) of the partition table.
    ///
    /// # NOTE
    ///
    /// The position is relative to begin of the device as defined by `Prober::set_device` for
    /// primary partition table, and relative to parental partition for nested partition tables.
    pub fn get_offset(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_parttable_get_offset(self.0)) }
    }

    /// Returns parent for nested partition tables
    pub fn get_parent(&self) -> Option<Partition> {
        let part = unsafe { blkid_parttable_get_parent(self.0) };
        if part.is_null() {
            None
        } else {
            Some(Partition(part))
        }
    }

    /// Returns partition table type (type name, e.g. "dos", "gpt", ...)
    pub fn get_type(&self) -> Option<PartitionTableType> {
        let ptr = unsafe { blkid_parttable_get_type(self.0) };
        if ptr.is_null() {
            None
        } else {
            let part_table_type = unsafe { CStr::from_ptr(ptr).to_string_lossy() };
            let part_table_type = PartitionTableType::from_str(part_table_type.as_ref())
                .expect("BUG: strum is broken, it must use default");
            Some(part_table_type)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum PartitionTableType {
    Aix,
    Atari,
    Bsd,
    Dos,
    Gpt,
    Mac,
    Minix,
    Sgi,
    Solaris,
    Sun,
    Ultrix,
    Unixware,
    #[strum(default)]
    Unknown(String),
}
