use crate::{error::c_result, part_table::PartTable, partition::Partition, prober::Prober, BlkIdResult};
use blkid_sys::*;
use std::marker::PhantomData;

/// List of all detected partitions and partitions tables
pub struct PartList<'a>(pub(crate) blkid_partlist, PhantomData<&'a Prober>);

impl<'a> PartList<'a> {
    pub(crate) fn new(list: blkid_partlist) -> PartList<'a> {
        PartList(list, PhantomData)
    }

    /// Returns partition object.
    ///
    /// It's possible that the list of partitions is *empty*, but there is a valid partition table
    /// on the disk. This happen when on-disk details about partitions are unknown or the partition
    /// table is empty.
    ///
    /// See also [`Self::get_table`].
    pub fn get_partition(&self, part_num: i32) -> BlkIdResult<Partition<'a>> {
        unsafe { c_result(blkid_partlist_get_partition(self.0, part_num)).map(Partition::new) }
    }

    /// Returns partition object by the partiton number (e.g. `N` from sda`N`).
    ///
    /// This does not assume any order of the input blkid_partlist. And correctly handles "out of
    /// order" partition tables. partition N is located after partition N+1 on the disk.
    #[cfg(blkid = "2.25")]
    pub fn get_partition_by_parno(&self, partno: i32) -> BlkIdResult<Partition<'a>> {
        unsafe { c_result(blkid_partlist_get_partition_by_partno(self.0, partno)).map(Partition::new) }
    }

    /// Returns all partitions
    pub fn get_partitions(&self) -> BlkIdResult<Vec<Partition<'a>>> {
        let numof = self.numof_partitions()?;
        let mut partitions = Vec::with_capacity(numof as usize);

        for part_num in 0..numof {
            partitions.push(self.get_partition(part_num)?);
        }
        Ok(partitions)
    }

    /// Returns partition object by requested partition.
    ///
    /// This tries to get start and size for devno from `sysfs` and returns a partition from list
    /// which matches with the values from `sysfs`.
    ///
    /// This function is necessary when you want to make a relation between an entry in the
    /// partition table (list) and block devices in your system.
    pub fn devno_to_partition(&self, devno: u64) -> BlkIdResult<Partition<'a>> {
        unsafe { c_result(blkid_partlist_devno_to_partition(self.0, devno)).map(Partition::new) }
    }

    /// Returns [`PartTable`] or `None` if there is not a partition table on the device
    pub fn get_table(&self) -> Option<PartTable<'a>> {
        let table = unsafe { blkid_partlist_get_table(self.0) };
        if table.is_null() {
            None
        } else {
            Some(PartTable::new(table))
        }
    }

    /// Returns number of partitions in the list
    pub fn numof_partitions(&self) -> BlkIdResult<i32> {
        unsafe { c_result(blkid_partlist_numof_partitions(self.0)) }
    }
}
