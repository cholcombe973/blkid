use crate::{error::c_result, part_table::PartTable, BlkIdResult};
use blkid_sys::*;
use std::{ffi::CStr, marker::PhantomData};

/// Information about a partition
#[derive(Debug)]
pub struct Partition<'a>(pub(crate) blkid_partition, PhantomData<&'a ()>);

impl<'a> Partition<'a> {
    pub(crate) fn new(part: blkid_partition) -> Partition<'a> {
        Partition(part, PhantomData)
    }

    /// Returns the partition name if supported by the partition table type (e.g. Mac), or `None`.
    pub fn name(&self) -> Option<String> {
        let name = unsafe { blkid_partition_get_name(self.0) };
        if name.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(name).to_string_lossy().to_string() })
        }
    }

    /// Returns partition flags (or attributes for GPT).
    pub fn flags(&self) -> u64 {
        unsafe { blkid_partition_get_flags(self.0) }
    }

    /// Returns the proposed partition number (e.g. `N` from `sdaN`). Note that the number is
    /// generated independently by libblkid, not by your OS.
    pub fn partno(&self) -> BlkIdResult<i32> {
        unsafe { c_result(blkid_partition_get_partno(self.0), "blkid_partition_get_partno") }
    }

    /// Returns size of the partition (in 512-sectors).
    ///
    /// # WARNING
    ///
    /// Be very careful when you work with MS-DOS extended partitions. The library always returns
    /// full size of the partition. If you want to add the partition to the Linux system
    /// (BLKPG_ADD_PARTITION ioctl) you need to reduce the size of the partition to 1 or 2 blocks.
    /// The rest of the partition has to be inaccessible for mkfs or mkswap programs, we need a
    /// small space for boot loaders only.
    ///
    /// For some unknown reason this (safe) practice is not to used for nested BSD, Solaris, ...,
    /// partition tables in Linux kernel.
    pub fn size(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_partition_get_size(self.0), "blkid_partition_get_size") }
    }

    /// Returns start of the partition (in 512-sectors).
    ///
    /// # NOTE
    ///
    /// Be careful if you _not_ probe whole disk:
    ///     1) the offset is usually relative to begin of the disk -- but if you probe a fragment of
    ///     the disk only -- then the offset could be still relative to the begin of the disk
    ///     rather that relative to the fragment.
    ///     2) the offset for nested partitions could be relative to parent (e.g. Solaris) _or_
    ///     relative to the begin of the whole disk (e.g. bsd).
    ///
    /// You don't have to care about such details if you probe whole disk. In such a case libblkid
    /// always returns the offset relative to the begin of the disk.
    pub fn start(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_partition_get_start(self.0), "blkid_partition_get_start") }
    }

    /// Returns partition table object.
    ///
    /// The "parttable" describes partition table. The table is usually the same for all partitions
    /// -- except nested partition tables.
    ///
    /// For example `bsd`, `solaris`, etc. use a nested partition table within standard primary `dos`
    /// partition:
    /// ```text
    /// -- dos partition table
    /// 0: sda1     dos primary partition
    /// 1: sda2     dos primary partition
    /// -- bsd partition table (with in sda2)
    /// 2: sda5  bsd partition
    /// 3: sda6  bsd partition
    /// ```
    ///
    /// The library does not to use a separate partition table object for dos logical partitions
    /// (partitions within extended partition). It's possible to differentiate between logical,
    /// extended and primary partitions by `Self::is_{extended, primary, logical}`.
    pub fn table(&self) -> BlkIdResult<PartTable<'a>> {
        unsafe { c_result(blkid_partition_get_table(self.0), "blkid_partition_get_table").map(PartTable::new) }
    }

    /// Returns the partition type code.
    pub fn typ(&self) -> i32 {
        unsafe { blkid_partition_get_type(self.0) }
    }

    /// Returns the partition type as a string, if available.
    ///
    /// The type string is supported by a small subset of partition tables (e.g. Mac and EFI GPT).
    /// Note that GPT uses type UUIDs and this function returns the UUID as a string.
    pub fn typ_string(&self) -> Option<String> {
        let ptr = unsafe { blkid_partition_get_type_string(self.0) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr).to_string_lossy().to_string() })
        }
    }

    /// Returns the partition UUID string if supported by the partition table type (e.g. GPT).
    pub fn uuid(&self) -> Option<String> {
        let ptr = unsafe { blkid_partition_get_uuid(self.0) };
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(ptr).to_string_lossy().to_string() })
        }
    }

    /// Returns `true` if the partition is an extended (DOS, Windows, or Linux) partition.
    pub fn is_extended(&self) -> bool {
        unsafe { blkid_partition_is_extended(self.0) == 1 }
    }

    /// Returns `true` if the partition is a logical partition.
    ///
    /// # NOTE
    ///
    /// Returns `true` for all partitions in all nested partition tables (e.g. BSD labels)
    pub fn is_logical(&self) -> bool {
        unsafe { blkid_partition_is_logical(self.0) == 1 }
    }

    /// Returns `true` if the partition is a primary partition.
    ///
    /// # NOTE
    ///
    /// Returns `false` for DOS extended partitions and all partitions in nested partition tables.
    pub fn is_primary(&self) -> bool {
        unsafe { blkid_partition_is_primary(self.0) == 1 }
    }
}
