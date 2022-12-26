use blkid_sys::*;

/// Device topology information
pub struct Topology(pub(crate) blkid_topology);

impl Topology {
    /// Alignment offset in bytes or 0.
    pub fn alignment_offset(&self) -> u64 {
        unsafe { blkid_topology_get_alignment_offset(self.0) }.try_into().unwrap()
    }

    /// Minimum io size in bytes or 0.
    pub fn minimum_io_size(&self) -> u64 {
        unsafe { blkid_topology_get_minimum_io_size(self.0) }.try_into().unwrap()
    }

    /// Optimal io size in bytes or 0.
    pub fn optimal_io_size(&self) -> u64 {
        unsafe { blkid_topology_get_optimal_io_size(self.0) }.try_into().unwrap()
    }

    /// Logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn logical_sector_size(&self) -> u64 {
        unsafe { blkid_topology_get_logical_sector_size(self.0) }.try_into().unwrap()
    }

    /// Logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn physical_sector_size(&self) -> u64 {
        unsafe { blkid_topology_get_physical_sector_size(self.0) }.try_into().unwrap()
    }

    /// Returns `true` if dax is supported
    #[cfg(blkid = "2.36")]
    pub fn dax(&self) -> bool {
        unsafe { blkid_topology_get_dax(self.0) == 1 }
    }
}
