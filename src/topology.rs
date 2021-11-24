// Copyright 2017 Red Hat, Inc.

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

use blkid_sys::*;

/// Device topology information
pub struct Topology(pub(crate) blkid_topology);

impl Topology {
    /// Alignment offset in bytes or 0.
    pub fn alignment_offset(&self) -> u64 {
        unsafe { blkid_topology_get_alignment_offset(self.0) }
    }

    /// Minimum io size in bytes or 0.
    pub fn minimum_io_size(&self) -> u64 {
        unsafe { blkid_topology_get_minimum_io_size(self.0) }
    }

    /// Optimal io size in bytes or 0.
    pub fn optimal_io_size(&self) -> u64 {
        unsafe { blkid_topology_get_optimal_io_size(self.0) }
    }

    /// Logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn logical_sector_size(&self) -> u64 {
        unsafe { blkid_topology_get_logical_sector_size(self.0) }
    }

    /// Logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn physical_sector_size(&self) -> u64 {
        unsafe { blkid_topology_get_physical_sector_size(self.0) }
    }

    // TODO: uncomment this when will be available
    // /// Returns `true` if dax is supported
    // pub fn dax(&self) -> bool {
    //     unsafe { blkid_topogy_get_dax(self.0) == 1 }
    // }
}
