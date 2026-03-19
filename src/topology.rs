use blkid_sys::*;
use std::marker::PhantomData;

use crate::prober::Prober;

/// Device topology information
pub struct Topology<'a>(pub(crate) blkid_topology, PhantomData<&'a Prober>);

impl<'a> Topology<'a> {
    pub(crate) fn new(topo: blkid_topology) -> Topology<'a> {
        Topology(topo, PhantomData)
    }

    /// Alignment offset in bytes or 0.
    pub fn alignment_offset(&self) -> u64 {
        unsafe { blkid_topology_get_alignment_offset(self.0) as u64 }
    }

    /// Minimum io size in bytes or 0.
    pub fn minimum_io_size(&self) -> u64 {
        unsafe { blkid_topology_get_minimum_io_size(self.0) as u64 }
    }

    /// Optimal io size in bytes or 0.
    pub fn optimal_io_size(&self) -> u64 {
        unsafe { blkid_topology_get_optimal_io_size(self.0) as u64 }
    }

    /// Logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn logical_sector_size(&self) -> u64 {
        unsafe { blkid_topology_get_logical_sector_size(self.0) as u64 }
    }

    /// Logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn physical_sector_size(&self) -> u64 {
        unsafe { blkid_topology_get_physical_sector_size(self.0) as u64 }
    }

    /// Returns `true` if dax is supported
    #[cfg(blkid = "2.36")]
    pub fn dax(&self) -> bool {
        unsafe { blkid_topology_get_dax(self.0) == 1 }
    }

    /// Disk sequence number or 0.
    #[cfg(blkid = "2.39")]
    pub fn diskseq(&self) -> u64 {
        unsafe { blkid_topology_get_diskseq(self.0) }
    }
}
