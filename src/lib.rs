// Copyright (c) 2017 Chris Holcombe

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

//! See https://mirrors.edge.kernel.org/pub/linux/utils/util-linux/v2.37/libblkid-docs/index.html
//! for the reference manual to the FFI bindings

pub mod cache;
pub mod dev;
pub mod error;
pub mod part_list;
pub mod part_table;
pub mod partition;
pub mod prober;
pub mod tag;
pub mod topology;

use bitflags::bitflags;
use std::{ffi::CString, path::Path};

pub use error::{BlkIdError, BlkIdResult};

pub(crate) fn path_to_cstring<P: AsRef<Path>>(path: P) -> BlkIdResult<CString> {
    Ok(CString::new(path.as_ref().to_string_lossy().as_ref())?)
}

bitflags! {
    pub struct SuperblocksFlags: i32 {
        /// Read LABEL from superblock
        const LABEL     = 1 << 1;
        /// Read and define LABEL_RAW result value
        const LABELRAW  = 1 << 2;
        /// Read UUID from superblock
        const UUID      = 1 << 3;
        /// Read and define UUID_RAW result value
        const UUIDRAW   = 1 << 4;
        /// Define TYPE result value
        const TYPE      = 1 << 5;
        /// Define compatible fs type (second type)
        const SECTYPE   = 1 << 6;
        /// Define USAGE result value
        const USAGE     = 1 << 7;
        /// Read FS type from superblock
        const VERSION   = 1 << 8;
        /// Define SBMAGIC and SBMAGIC_OFFSET
        const MAGIC     = 1 << 9;
        /// Allow a bad checksum
        const BADCSUM   = 1 << 10;
        /// Default flags
        const DEFAULT   = Self::LABEL.bits | Self::UUID.bits | Self::TYPE.bits | Self::SECTYPE.bits;
    }

    pub struct PartitionsFlags: i32 {
        const FORCE_GPT     = 1 << 1;
        const ENTRY_DETAILS = 1 << 2;
        const MAGIC         = 1 << 3;
    }
}

impl Default for SuperblocksFlags {
    fn default() -> Self {
        Self::DEFAULT
    }
}
