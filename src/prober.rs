use crate::{
    error::{c_result, BlkIdError, BlkIdResult},
    part_list::PartList,
    path_to_cstring,
    topology::Topology,
    PartitionsFlags, SuperblocksFlags,
};
use blkid_sys::*;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    path::Path,
    ptr,
};

/// Low-level probing setting
///
/// The probing routines are grouped together into separate chains. Currently, the library provides
/// superblocks, partitions and topology chains.
///
/// The probing routines is possible to filter (enable/disable) by type (e.g. fstype "vfat" or
/// partype "gpt") or by usage flags (e.g. BLKID_USAGE_RAID). These filters are per-chain. Note that
/// always when you touch the chain filter the current probing position is reset and probing starts
/// from scratch. It means that the chain filter should not be modified during probing, for example
/// in loop where you call [`Self::do_probe`].
///
/// The probing routines inside the chain are mutually exclusive by default - only few probing
/// routines are marked as "tolerant". The "tolerant" probing routines are used for filesystem
/// which can share the same device with any other filesystem. The [`Self::safeprobe`] checks for
/// the "tolerant" flag.
///
/// The `superblocks` chain is enabled by default. The all others chains is necessary to enable by
/// `enable_'CHAINNAME'()`.
pub struct Prober(pub(crate) blkid_probe);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeState {
    Success,
    Done,
    NothingDetected,
    Ambivalent,
}

impl Drop for Prober {
    fn drop(&mut self) {
        unsafe { blkid_free_probe(self.0) }
    }
}

impl Prober {
    /// Create newly allocated `probe` struct.
    pub fn new() -> BlkIdResult<Self> {
        let probe = unsafe { c_result(blkid_new_probe()) }?;
        Ok(Self(probe))
    }

    /// Create newly allocated `probe` struct by filename.
    /// `filename` can be either regular file or device
    pub fn new_from_filename<P: AsRef<Path>>(filename: P) -> BlkIdResult<Self> {
        let path = path_to_cstring(filename)?;
        let probe = unsafe { c_result(blkid_new_probe_from_filename(path.as_ptr())) }?;
        Ok(Self(probe))
    }

    /// Calls probing functions in all enabled chains. The superblocks chain is enabled by default.
    /// Stores result from only one probing function.
    ///
    /// # Note
    ///
    /// It's necessary to call this routine in a loop to get results from all probing functions in
    /// all chains. The probing is reset by [`Self::reset`] or by filter functions.
    ///
    /// Returns the following possible states:
    /// * [`ProberState::Success`]
    /// * [`ProberState::Done`]
    ///
    /// # Exapmles
    ///
    /// * Basic case - use the first result only
    /// ```ignore, compile_fail
    /// let prober = Prober::new().unwrap();
    ///
    /// if prober.do_probe() == ProbeState::success {
    ///     let value_map = prober.get_values_map().unwrap();
    ///     println!("{:#?}", value_map);
    /// }
    /// ```
    /// * Advanced case - probe for all signatures
    /// ```ignore, compile_fail
    /// let prober = Prober::new().unwrap();
    ///
    /// while prober.do_probe() == ProbeState::Done {
    ///     let value_map = prober.get_values_map().unwrap();
    ///     println!("{:#?}", value_map);
    /// }
    /// ```
    pub fn do_probe(&self) -> BlkIdResult<ProbeState> {
        let ret_code = unsafe { blkid_do_probe(self.0) };

        match ret_code {
            0 => Ok(ProbeState::Success),
            1 => Ok(ProbeState::Done),
            _ => Err(BlkIdError::Io(std::io::Error::last_os_error())),
        }
    }

    /// This function gathers probing results from all enabled chains and checks for ambivalent
    /// results (e.g. more filesystems on the device).
    ///
    /// This is string-based `NAME=value` interface only.
    ///
    /// # Note
    ///
    /// Suberblocks chain -- the function does not check for filesystems when a `RAID` signature is
    /// detected. The function also does not check for collision between `RAID`s. The first detected
    /// `RAID` is returned. The function checks for collision between partition table and `RAID`
    /// signature - it's recommended to enable partitions chain ([`Self::enable_partitions`])
    /// together with superblocks chain (enabled by default).
    ///
    /// Returns the following possible states:
    /// * [`ProberState::Success`]
    /// * [`ProberState::NothingDetected`]
    /// * [`ProberState::Ambivalent`]
    pub fn do_safe_probe(&self) -> BlkIdResult<ProbeState> {
        let ret_code = unsafe { blkid_do_safeprobe(self.0) };

        match ret_code {
            0 => Ok(ProbeState::Success),
            1 => Ok(ProbeState::NothingDetected),
            -2 => Ok(ProbeState::Ambivalent),
            _ => Err(BlkIdError::Io(std::io::Error::last_os_error())),
        }
    }

    /// This function gathers probing results from all enabled chains. Same as
    /// [`Self::do_safe_probe`] but does not check for collision between probing result.
    ///
    /// Returns the following possible states:
    /// * [`ProberState::Success`]
    /// * [`ProberState::NothingDetected`]
    pub fn do_full_probe(&self) -> BlkIdResult<ProbeState> {
        let ret_code = unsafe { blkid_do_safeprobe(self.0) };

        match ret_code {
            0 => Ok(ProbeState::Success),
            1 => Ok(ProbeState::NothingDetected),
            _ => Err(BlkIdError::Io(std::io::Error::last_os_error())),
        }
    }

    /// Erases the current signature detected by prober. The prober has to be open in `O_RDWR` mode,
    /// `BLKID_SUBLKS_MAGIC` or/and `BLKID_PARTS_MAGIC` flags has to be enabled. That means that you
    /// should use [`Self::set_device`] with options above.
    ///
    /// After successful signature removing the prober will be moved one step back and the next
    /// [`Self::probe`] call will again call previously called probing function.
    ///
    /// # Examples
    ///
    /// ```ignore, compile_fail
    /// use std::os::unix::io::AsRawFd;
    /// use std::fs::OpenOptions;
    ///
    /// let mut file = OpenOptions::new()
    ///     .read(true)
    ///     .write(true)
    ///     .open("/dev/sda");
    /// let fd: RawFd = file.as_raw_fd();
    ///
    /// let mut prober = Prober::new().unwrap();
    /// prober.set_device(fd, 0, None).unwrap();
    ///
    /// while prober.do_probe() == ProbeState::Success {
    ///     prober.do_wipe(false).unwrap();
    /// }
    /// ```
    pub fn do_wipe(&self, dry_run: bool) -> BlkIdResult<ProbeState> {
        let ret_code = unsafe { blkid_do_wipe(self.0, dry_run as i32) };

        match ret_code {
            0 => Ok(ProbeState::Success),
            1 => Ok(ProbeState::Done),
            _ => Err(BlkIdError::Io(std::io::Error::last_os_error())),
        }
    }

    /// Retrieve the Nth item `(Name, Value)` in the probing result, (0..self.numof_values())
    pub fn get_value(&self, num: i32) -> BlkIdResult<(String, String)> {
        let mut name_ptr: *const ::libc::c_char = ptr::null();
        let mut data_ptr: *const ::libc::c_char = ptr::null();
        let mut len = 0;

        unsafe {
            c_result(blkid_probe_get_value(
                self.0,
                num,
                &mut name_ptr,
                &mut data_ptr,
                &mut len,
            ))
        }?;

        let name_value = unsafe { CStr::from_ptr(name_ptr).to_str()?.to_owned() };
        let data_value = unsafe { CStr::from_ptr(data_ptr).to_str()?.to_owned() };
        Ok((name_value, data_value))
    }

    /// Retrieve a `HashMap` of all the probed values
    pub fn get_values_map(&self) -> BlkIdResult<HashMap<String, String>> {
        let numof_values = self.numof_values()?;
        let mut map = HashMap::with_capacity(numof_values as usize);

        for i in 0..numof_values {
            let (key, value) = self.get_value(i)?;
            map.insert(key, value);
        }

        Ok(map)
    }

    /// Check if device has the specified value
    pub fn has_value(&self, name: &str) -> BlkIdResult<bool> {
        let name = CString::new(name)?;
        unsafe { c_result(blkid_probe_has_value(self.0, name.as_ptr())).map(|val| val == 1) }
    }

    /// Value by specified `name`
    ///
    /// # Note
    ///
    /// You should call [`Self::do_probe`] before using this
    pub fn lookup_value(&self, name: &str) -> BlkIdResult<String> {
        let name = CString::new(name)?;
        let mut data_ptr: *const ::libc::c_char = ptr::null();
        let mut len = 0;
        unsafe {
            c_result(blkid_probe_lookup_value(
                self.0,
                name.as_ptr(),
                &mut data_ptr,
                &mut len,
            ))
        }?;

        let data_value = unsafe { CStr::from_ptr(data_ptr).to_str()?.to_owned() };
        Ok(data_value)
    }

    /// Number of values in probing result
    pub fn numof_values(&self) -> BlkIdResult<i32> {
        unsafe { c_result(blkid_probe_numof_values(self.0)) }
    }

    /// Block device number, or 0 for regular file
    pub fn get_devno(&self) -> u64 {
        unsafe { blkid_probe_get_devno(self.0) }
    }

    /// File descriptor for assigned device/file
    pub fn get_fd(&self) -> i32 {
        unsafe { blkid_probe_get_fd(self.0) }
    }

    /// Block device logical sector size (`BLKSSZGET` ioctl, default 512)
    pub fn get_sector_size(&self) -> u32 {
        unsafe { blkid_probe_get_sectorsize(self.0) }
    }

    // /// Set logical sector size.
    // ///
    // /// Note that [`Self::set_device`] resets this setting. Use it after [`Self::set_device`] and
    // /// before any probing call.
    // pub fn set_sector_size(&self, size: u32) -> BlkIdResult<()> {
    //     unsafe { c_result(blkid_probe_set_sectorsize(self.0, size)).map(|_| ()) }
    // }

    /// 512-byte sector count
    pub fn get_sectors(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_probe_get_sectors(self.0)) }
    }

    /// Size of probing area in bytes as defined by [`Self::set_device`]. If the size of the probing
    /// area is unrestricted then this function returns the real size of device
    pub fn get_size(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_probe_get_size(self.0)) }
    }

    /// Offset of probing area as defined by [`Self::set_device`]
    pub fn get_offset(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_probe_get_offset(self.0)) }
    }

    /// Device number of the wholedisk, or 0 for regular files
    pub fn get_wholedisk_devno(&self) -> u64 {
        unsafe { blkid_probe_get_wholedisk_devno(self.0) }
    }

    /// If device is wholedisk
    pub fn is_wholedisk(&self) -> bool {
        unsafe { blkid_probe_is_wholedisk(self.0) == 1 }
    }

    // /// Modifies in-memory cached data from the device. The specified range is zeroized.
    // /// This is usable together with [`Self::step_back`]. The next [`Self::do_probe`] will not see
    // /// specified area.
    // ///
    // /// Note that this is usable for already (by library) read data, and this function is not a way
    // /// how to hide any large areas on your device.
    // ///
    // /// The [`Self::reset_buffers`] reverts all.
    // pub fn hide_range(&self, offset: u64, size: u64) -> BlkIdResult<()> {
    //     unsafe { c_result(blkid_probe_hide_range(self.0, offset, size)).map(|_| ()) }
    // }

    // /// Reuse all already read buffers from the device. The buffers may be modified by
    // /// [`Self::hide_range`]. This resets and free all cached buffers. The next [`Self::do_probe`]
    // /// will read all data from the device.
    // pub fn reset_buffers(&self) -> BlkIdResult<()> {
    //     unsafe { c_result(blkid_probe_reset_buffers(self.0)).map(|_| ()) }
    // }

    /// This function move pointer to the probing chain one step back - it means that the
    /// previously used probing function will be called again in the next [`Self::do_probe`] call.
    ///
    /// This is necessary for example if you erase or modify on-disk superblock according to the
    /// current libblkid probing result.
    ///
    /// Note that [`Self::hide_range`] changes semantic of this function and cached buffers are
    /// not reset, but library uses in-memory modified buffers to call the next probing function.
    ///
    /// # Examples
    ///
    /// ```ignore, compile_fail
    /// let prober = Prober::new_from_filename("/dev/sda");
    /// TODO: coplete this example
    /// ```
    pub fn step_back(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_step_back(self.0)).map(|_| ()) }
    }

    /// Assigns the device to probe control struct, resets internal buffers and resets the current
    /// probing.
    ///
    /// `fd`: device file descriptor
    /// `offset`: begin of probing area
    /// `size`: size of probing area (`None` means whole device/file)
    pub fn set_device(&mut self, fd: i32, offset: i64, size: Option<i64>) -> BlkIdResult<()> {
        let size = size.unwrap_or(0);
        unsafe { c_result(blkid_probe_set_device(self.0, fd, offset, size)).map(|_| ()) }
    }

    /// Zeroize probing results and resets the current probing (this has impact to [`Self::do_probe`]
    /// only). This function does not touch probing filters and keeps assigned device.
    pub fn reset_probe(&self) {
        unsafe { blkid_reset_probe(self.0) }
    }

    /// Enables/disables the superblocks probing for non-binary interface.
    pub fn enable_superblocks(&self, enable: bool) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_enable_superblocks(self.0, enable as i32)).map(|_| ()) }
    }

    /// If known filesystem type
    pub fn known_fstype(fstype: &str) -> BlkIdResult<bool> {
        let fstype = CString::new(fstype)?;
        Ok(unsafe { blkid_known_fstype(fstype.as_ptr()) == 1 })
    }

    // TODO: implement
    // pub fn superblocks_get_name() {}

    // TODO: implement
    // pub fn filter_superblocks_type() {}

    // TODO: implement
    // pub fn filter_superblocks_usage() {}

    /// Inverts superblocks probing filter
    pub fn invert_superblocks_filter(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_invert_superblocks_filter(self.0)).map(|_| ()) }
    }

    /// Resets superblocks probing filter
    pub fn reset_superblocks_filter(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_reset_superblocks_filter(self.0)).map(|_| ()) }
    }

    /// Sets probing flags to the superblocks prober. This function is optional, the default are
    /// [`Superblocks::DEFAULT`] flags.
    pub fn set_superblocks_flags(&self, flags: SuperblocksFlags) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_set_superblocks_flags(self.0, flags.bits())).map(|_| ()) }
    }

    /// Enables/disables the partitions probing for non-binary interface
    pub fn enable_partitions(&self, enable: bool) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_enable_partitions(self.0, enable as i32)).map(|_| ()) }
    }

    /// Sets probing flags to the partitions prober. This function is optional
    pub fn set_partitions_flags(&self, flags: PartitionsFlags) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_set_partitions_flags(self.0, flags.bits())).map(|_| ()) }
    }

    // TODO: implement
    // pub fn filter_partitions_type() {}

    /// Inverts partitions probing filter
    pub fn invert_partitions_filter(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_invert_partitions_filter(self.0)).map(|_| ()) }
    }

    /// Resets partitions probing filter
    pub fn reset_partitions_filter(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_reset_partitions_filter(self.0)).map(|_| ()) }
    }

    /// If known partition table type
    pub fn known_pttype(pttype: &str) -> BlkIdResult<bool> {
        let pttype = CString::new(pttype)?;
        Ok(unsafe { blkid_known_pttype(pttype.as_ptr()) == 1 })
    }

    // /// Returns name of a supported partition.
    // pub fn partitions_get_name(idx: u64) -> BlkIdResult<String> {
    //     let mut name: *const ::libc::c_char = ptr::null();
    //     unsafe { c_result(blkid_partitions_get_name(idx, &mut name)) }?;
    //     let name = unsafe { CStr::from_ptr(name).to_str()?.to_owned() };
    //     Ok(name)
    // }

    /// Returns [`PartList`] object.
    ///
    /// This is a binary interface for partitions.
    ///
    /// This is independent on `Self::do_[safe,full]_probe()` and [`Self::enable_partitions`] calls.
    ///
    /// # WARNING
    ///
    /// The returned object will be overwritten by the next [`Self::part_list`] call for the same
    /// prober. If you want to use more [`PartList`] objects in the same time you have to create
    /// more [`Prober`] handlers.
    pub fn part_list(&self) -> BlkIdResult<PartList> {
        unsafe { c_result(blkid_probe_get_partitions(self.0)).map(PartList) }
    }

    /// Enables/disables the topology probing for non-binary interface
    pub fn enable_topology(&self, enable: bool) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_enable_topology(self.0, enable as i32)).map(|_| ()) }
    }

    /// Returns topology.
    ///
    /// This is a binary interface for topology values.
    ///
    /// This is independent on `Self::do_[safe,full]_probe()` and [`Self::enable_partitions`] calls.
    ///
    /// # WARNING
    ///
    /// The returned object will be overwritten by the next [`Self::topology`] call for the same
    /// prober. If you want to use more [`Topology`] objects in the same time you have to create
    /// more [`Prober`] handlers.
    pub fn topology(&self) -> BlkIdResult<Topology> {
        unsafe { c_result(blkid_probe_get_topology(self.0)).map(Topology) }
    }

    // TODO: uncomments this when it will be possible
    // Sets extra hint for low-level prober. If the hint is set by NAME=value notation than value
    // is ignored. The [`Self::set_device`] and [`Self::reset_probe`] resets all hints.
    //
    // The hints are optional way how to force libblkid probing functions to check for example
    // another location.
    // pub fn set_hint(&self, hint_name: &str, offset: u64) -> BlkIdResult<()> {
    //     let name = CString::new(hint_name)?;
    //     unsafe { c_result(blkid_probe_set_hint(self.0, name.as_ptr(), offset)).map(|_| ()) }
    // }

    // TODO: uncomments this when it will be possible
    // Removes all previously defined probing hints. See also [`Self::set_hint`]
    // pub fn reset_hints(&self) -> BlkIdResult<()> {
    //    unsafe { c_result(blkid_probe_reset_hints(self.0)).map(|_| ()) }
    // }
}
