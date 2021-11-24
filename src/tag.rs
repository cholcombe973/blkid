// Copyright 2017 Red Hat, Inc.

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

use crate::dev::Dev;
use blkid_sys::*;
use std::{ffi::CStr, ptr, str::FromStr};
use strum_macros::{Display, EnumString};

pub struct Tags {
    pub(crate) iter: blkid_tag_iterate,
}

impl Tags {
    pub fn new(dev: &Dev) -> Tags {
        let iter = unsafe { blkid_tag_iterate_begin(dev.0) };
        assert_ne!(iter, ptr::null_mut());
        Tags { iter }
    }
}

impl Drop for Tags {
    fn drop(&mut self) {
        unsafe { blkid_tag_iterate_end(self.iter) }
    }
}

impl Iterator for Tags {
    type Item = Tag;

    fn next(&mut self) -> Option<Self::Item> {
        let mut k = ptr::null();
        let mut v = ptr::null();
        unsafe {
            match blkid_tag_next(self.iter, &mut k, &mut v) {
                0 => {
                    let name = TagType::from(CStr::from_ptr(k).to_string_lossy().as_ref());
                    let value = CStr::from_ptr(v).to_string_lossy().to_string();

                    Some(Tag { name, value })
                }
                _ => None,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    name: TagType,
    value: String,
}

impl Tag {
    pub fn new(name: impl Into<TagType>, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.to_owned(),
        }
    }

    pub fn typ(&self) -> TagType {
        self.name.clone()
    }

    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

/// This is unified form of tag types.
/// Each of inner enum value implement `From` trait, which allows to construct this enum using the
/// following syntax:
/// ```rust,text
/// let part_tag = PartitionTag::Ptuuid;
/// let tag_type: TagType = part_tag.into();
/// let tag_part_type = TagType::Partition(PartitionTag::Ptuuid);
///
/// assert_eq!(tag_type, tag_part_type);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagType {
    Superblock(SuperblockTag),
    Partition(PartitionTag),
    Topoligy(TopologyTag),
    Unknown(String),
}

impl std::fmt::Display for TagType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match &self {
            Self::Superblock(tag) => tag.to_string(),
            Self::Partition(tag) => tag.to_string(),
            Self::Topoligy(tag) => tag.to_string(),
            Self::Unknown(tag) => tag.clone(),
        };
        write!(f, "{}", name)
    }
}

impl From<&str> for TagType {
    fn from(name: &str) -> Self {
        if let Ok(tag) = SuperblockTag::from_str(name) {
            TagType::Superblock(tag)
        } else if let Ok(tag) = PartitionTag::from_str(name) {
            TagType::Partition(tag)
        } else if let Ok(tag) = TopologyTag::from_str(name) {
            TagType::Topoligy(tag)
        } else {
            TagType::Unknown(name.to_owned())
        }
    }
}

#[derive(Clone, Debug, Display, Eq, PartialEq, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum SuperblockTag {
    /// Filesystem type
    Type,
    /// Secondary filesystem type
    SecType,
    /// Filesystem label
    Label,
    /// Raw label from FS superblock
    LabelRaw,
    /// Filesystem UUID (lower case)
    Uuid,
    /// Subvolume uuid (e.g. btrfs)
    UuidSub,
    /// External log UUID (e.g. xfs)
    Loguuid,
    /// Raw UUID from FS superblock
    UuidRaw,
    /// External journal UUID
    ExtJournal,
    /// Usage string: "raid", "filesystem", ...
    Usage,
    /// Filesystem version
    Version,
    /// Cluster mount name (?) -- ocfs only
    Mount,
    /// Super block magic string
    Sbmagic,
    /// Offset of SBMAGIC
    SbmagicOffset,
    /// Size of filesystem [not-implemented yet]
    Fssize,
    /// ISO9660 system identifier
    SystemId,
    /// ISO9660 publisher identifier
    PublisherId,
    /// ISO9660 application identifier
    ApplicationId,
    /// ISO9660 boot system identifier
    BootSystemId,
    /// Block size
    BlockSize,
}

impl From<SuperblockTag> for TagType {
    fn from(tag: SuperblockTag) -> Self {
        Self::Superblock(tag)
    }
}

#[derive(Clone, Debug, Display, Eq, PartialEq, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum PartitionTag {
    /// Partition table type (dos, gpt, etc.)
    Pttype,
    /// Partition table id (uuid for gpt, hex for dos)
    Ptuuid,
    /// Partition table type
    PartEntrySchema,
    /// Partition name (gpt and mac only)
    PartEntryName,
    /// Partition UUID (gpt, or pseudo IDs for MBR)
    PartEntryUuid,
    /// Partition type, 0xNN (e.g. 0x82) or type UUID (gpt only) or type string (mac)
    PartEntryType,
    /// Partition flags (e.g. boot_ind) or attributes (e.g. gpt attributes)
    PartEntryFlags,
    /// Partition number
    PartEntryNumber,
    /// The begin of the partition
    PartEntryOffset,
    /// Size of the partition
    PartEntrySize,
    /// Whole-disk maj:min
    PartEntryDisk,
}

impl From<PartitionTag> for TagType {
    fn from(tag: PartitionTag) -> Self {
        Self::Partition(tag)
    }
}

#[derive(Clone, Debug, Display, Eq, PartialEq, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TopologyTag {
    /// The smallest unit the storage device can address. It is typically 512 bytes
    LogicalSectorSize,
    /// The smallest unit a physical storage device can write atomically. It is usually the same as
    /// the logical sector size but may be bigger.
    PhysicalSectorSize,
    /// Minimum size which is the device's preferred unit of I/O. For RAID arrays it is often the
    /// stripe chunk size
    MinimumIoSize,
    /// Usually the stripe width for RAID or zero. For RAID arrays it is usually the stripe width
    /// or the internal track size
    OptiomalIoSize,
    /// Indicates how many bytes the beginning of the device is offset from the disk's natural
    /// alignment
    AlignmentOffset,
}

impl From<TopologyTag> for TagType {
    fn from(tag: TopologyTag) -> Self {
        Self::Topoligy(tag)
    }
}
