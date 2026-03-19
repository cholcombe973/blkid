use crate::{
    error::{c_result, BlkIdError, BlkIdResult},
    part_list::PartList,
    path_to_cstring,
    topology::Topology,
    FilterMode, PartitionsFlags, SuperblocksFlags, SuperblocksUsage,
};
use blkid_sys::*;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    path::Path,
    ptr,
};

/// Low-level probing handle.
///
/// The probing routines are grouped together into separate chains. Currently, the library provides
/// superblocks, partitions, and topology chains.
///
/// Probing routines can be filtered (enabled/disabled) by type (e.g. fstype `"vfat"` or
/// partype `"gpt"`) or by usage flags (e.g. `BLKID_USAGE_RAID`). These filters are per-chain.
/// Note that modifying a chain filter resets the current probing position and probing starts
/// from scratch. The chain filter should not be modified during probing, for example
/// in a loop where you call [`Self::do_probe`].
///
/// The probing routines inside a chain are mutually exclusive by default -- only a few probing
/// routines are marked as "tolerant". The "tolerant" probing routines are used for filesystems
/// that can share the same device with any other filesystem. [`Self::do_safe_probe`] checks for
/// the "tolerant" flag.
///
/// The `superblocks` chain is enabled by default. All other chains must be enabled explicitly
/// (e.g. [`Self::enable_partitions`], [`Self::enable_topology`]).
pub struct Prober(pub(crate) blkid_probe);

/// Result state returned by probing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeState {
    /// A signature was successfully detected.
    Success,
    /// Probing is complete; no more signatures to find.
    Done,
    /// No signature was detected on the device.
    NothingDetected,
    /// Multiple conflicting signatures were detected (ambiguous result).
    Ambivalent,
}

impl Drop for Prober {
    fn drop(&mut self) {
        unsafe { blkid_free_probe(self.0) }
    }
}

impl Prober {
    /// Creates a newly allocated probe handle.
    pub fn new() -> BlkIdResult<Self> {
        let probe = unsafe { c_result(blkid_new_probe(), "blkid_new_probe") }?;
        Ok(Self(probe))
    }

    /// Creates a newly allocated probe handle for the given file.
    /// `filename` can be either a regular file or a block device.
    pub fn new_from_filename<P: AsRef<Path>>(filename: P) -> BlkIdResult<Self> {
        let path = path_to_cstring(filename)?;
        let probe = unsafe { c_result(blkid_new_probe_from_filename(path.as_ptr()), "blkid_new_probe_from_filename") }?;
        Ok(Self(probe))
    }

    /// Calls probing functions in all enabled chains. The superblocks chain is enabled by default.
    /// Stores the result from only one probing function.
    ///
    /// # Note
    ///
    /// It's necessary to call this routine in a loop to get results from all probing functions in
    /// all chains. The probing is reset by [`Self::reset_probe`] or by filter functions.
    ///
    /// Returns the following possible states:
    /// * [`ProbeState::Success`]
    /// * [`ProbeState::Done`]
    ///
    /// # Examples
    ///
    /// Basic case -- use the first result only:
    /// ```rust,no_run
    /// # use blkid::prober::{Prober, ProbeState};
    /// let prober = Prober::new_from_filename("/dev/sda1").unwrap();
    ///
    /// if let Ok(ProbeState::Success) = prober.do_probe() {
    ///     let value_map = prober.get_values_map().unwrap();
    ///     println!("{:#?}", value_map);
    /// }
    /// ```
    ///
    /// Advanced case -- probe for all signatures:
    /// ```rust,no_run
    /// # use blkid::prober::{Prober, ProbeState};
    /// let prober = Prober::new_from_filename("/dev/sda").unwrap();
    ///
    /// while let Ok(ProbeState::Success) = prober.do_probe() {
    ///     let value_map = prober.get_values_map().unwrap();
    ///     println!("{:#?}", value_map);
    /// }
    /// ```
    pub fn do_probe(&self) -> BlkIdResult<ProbeState> {
        let ret_code = unsafe { blkid_do_probe(self.0) };

        match ret_code {
            0 => Ok(ProbeState::Success),
            1 => Ok(ProbeState::Done),
            _ => Err(BlkIdError::FfiError {
                func: "blkid_do_probe",
                errno: std::io::Error::last_os_error(),
            }),
        }
    }

    /// Gathers probing results from all enabled chains and checks for ambivalent
    /// results (e.g. more than one filesystem on the device).
    ///
    /// This is the string-based `NAME=value` interface only.
    ///
    /// # Note
    ///
    /// Superblocks chain -- the function does not check for filesystems when a `RAID` signature is
    /// detected. The function also does not check for collisions between `RAID`s. The first detected
    /// `RAID` is returned. The function checks for collision between partition table and `RAID`
    /// signature -- it's recommended to enable the partitions chain ([`Self::enable_partitions`])
    /// together with the superblocks chain (enabled by default).
    ///
    /// Returns the following possible states:
    /// * [`ProbeState::Success`]
    /// * [`ProbeState::NothingDetected`]
    /// * [`ProbeState::Ambivalent`]
    pub fn do_safe_probe(&self) -> BlkIdResult<ProbeState> {
        let ret_code = unsafe { blkid_do_safeprobe(self.0) };

        match ret_code {
            0 => Ok(ProbeState::Success),
            1 => Ok(ProbeState::NothingDetected),
            -2 => Ok(ProbeState::Ambivalent),
            _ => Err(BlkIdError::FfiError {
                func: "blkid_do_safeprobe",
                errno: std::io::Error::last_os_error(),
            }),
        }
    }

    /// Gathers probing results from all enabled chains. Same as
    /// [`Self::do_safe_probe`] but does not check for collisions between probing results.
    ///
    /// Returns the following possible states:
    /// * [`ProbeState::Success`]
    /// * [`ProbeState::NothingDetected`]
    pub fn do_full_probe(&self) -> BlkIdResult<ProbeState> {
        let ret_code = unsafe { blkid_do_fullprobe(self.0) };

        match ret_code {
            0 => Ok(ProbeState::Success),
            1 => Ok(ProbeState::NothingDetected),
            _ => Err(BlkIdError::FfiError {
                func: "blkid_do_fullprobe",
                errno: std::io::Error::last_os_error(),
            }),
        }
    }

    /// Erases the current signature detected by the prober. The prober must be opened in `O_RDWR`
    /// mode, and `BLKID_SUBLKS_MAGIC` and/or `BLKID_PARTS_MAGIC` flags must be enabled via
    /// [`Self::set_superblocks_flags`] or [`Self::set_partitions_flags`].
    ///
    /// After successful signature removal the prober will be moved one step back and the next
    /// [`Self::do_probe`] call will again call the previously called probing function.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::os::unix::io::AsRawFd;
    /// use std::fs::OpenOptions;
    /// use blkid::prober::{Prober, ProbeState};
    ///
    /// let file = OpenOptions::new()
    ///     .read(true)
    ///     .write(true)
    ///     .open("/dev/sda")
    ///     .unwrap();
    /// let fd = file.as_raw_fd();
    ///
    /// let mut prober = Prober::new().unwrap();
    /// prober.set_device(fd, 0, None).unwrap();
    ///
    /// while let Ok(ProbeState::Success) = prober.do_probe() {
    ///     prober.do_wipe(false).unwrap();
    /// }
    /// ```
    pub fn do_wipe(&self, dry_run: bool) -> BlkIdResult<ProbeState> {
        let ret_code = unsafe { blkid_do_wipe(self.0, dry_run as i32) };

        match ret_code {
            0 => Ok(ProbeState::Success),
            1 => Ok(ProbeState::Done),
            _ => Err(BlkIdError::FfiError {
                func: "blkid_do_wipe",
                errno: std::io::Error::last_os_error(),
            }),
        }
    }

    /// Retrieves the Nth `(name, value)` pair in the probing result, where `num` is in
    /// `0..self.numof_values()`.
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
            ), "blkid_probe_get_value")
        }?;

        let name_value = unsafe { CStr::from_ptr(name_ptr).to_str()?.to_owned() };
        let data_value = unsafe { CStr::from_ptr(data_ptr).to_str()?.to_owned() };
        Ok((name_value, data_value))
    }

    /// Retrieves a [`HashMap`] of all the probed values.
    pub fn get_values_map(&self) -> BlkIdResult<HashMap<String, String>> {
        let numof_values = self.numof_values()?;
        let mut map = HashMap::with_capacity(numof_values as usize);

        for i in 0..numof_values {
            let (key, value) = self.get_value(i)?;
            map.insert(key, value);
        }

        Ok(map)
    }

    /// Checks if the device has the specified value.
    pub fn has_value(&self, name: &str) -> BlkIdResult<bool> {
        let name = CString::new(name)?;
        unsafe { c_result(blkid_probe_has_value(self.0, name.as_ptr()), "blkid_probe_has_value").map(|val| val == 1) }
    }

    /// Looks up a probing result value by `name`.
    ///
    /// # Note
    ///
    /// You must call [`Self::do_probe`] before using this.
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
            ), "blkid_probe_lookup_value")
        }?;

        let data_value = unsafe { CStr::from_ptr(data_ptr).to_str()?.to_owned() };
        Ok(data_value)
    }

    /// Returns the number of values in the probing result.
    pub fn numof_values(&self) -> BlkIdResult<i32> {
        unsafe { c_result(blkid_probe_numof_values(self.0), "blkid_probe_numof_values") }
    }

    /// Returns the block device number, or 0 for a regular file.
    pub fn get_devno(&self) -> libc::dev_t {
        unsafe { blkid_probe_get_devno(self.0) }
    }

    /// Returns the file descriptor for the assigned device/file.
    pub fn get_fd(&self) -> i32 {
        unsafe { blkid_probe_get_fd(self.0) }
    }

    /// Returns the block device logical sector size (`BLKSSZGET` ioctl, default 512).
    pub fn get_sector_size(&self) -> u32 {
        unsafe { blkid_probe_get_sectorsize(self.0) }
    }

    /// Sets the logical sector size.
    ///
    /// Note that [`Self::set_device`] resets this setting. Call this after [`Self::set_device`] and
    /// before any probing call.
    #[cfg(blkid = "2.30")]
    pub fn set_sector_size(&self, size: u32) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_set_sectorsize(self.0, size), "blkid_probe_set_sectorsize").map(|_| ()) }
    }

    /// Returns the 512-byte sector count.
    pub fn get_sectors(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_probe_get_sectors(self.0), "blkid_probe_get_sectors") }
    }

    /// Returns the size of the probing area in bytes as defined by [`Self::set_device`]. If the
    /// size of the probing area is unrestricted then this function returns the real size of the device.
    pub fn get_size(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_probe_get_size(self.0), "blkid_probe_get_size") }
    }

    /// Returns the offset of the probing area as defined by [`Self::set_device`].
    pub fn get_offset(&self) -> BlkIdResult<i64> {
        unsafe { c_result(blkid_probe_get_offset(self.0), "blkid_probe_get_offset") }
    }

    /// Returns the device number of the whole disk, or 0 for regular files.
    pub fn get_wholedisk_devno(&self) -> libc::dev_t {
        unsafe { blkid_probe_get_wholedisk_devno(self.0) }
    }

    /// Returns `true` if the device is a whole disk.
    pub fn is_wholedisk(&self) -> bool {
        unsafe { blkid_probe_is_wholedisk(self.0) == 1 }
    }

    /// Modifies in-memory cached data from the device. The specified range is zeroized.
    /// This is usable together with [`Self::step_back`]. The next [`Self::do_probe`] will not see
    /// specified area.
    ///
    /// Note that this is usable for already (by library) read data, and this function is not a way
    /// how to hide any large areas on your device.
    ///
    /// The [`Self::reset_buffers`] reverts all.
    #[cfg(blkid = "2.31")]
    pub fn hide_range(&self, offset: u64, size: u64) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_hide_range(self.0, offset, size), "blkid_probe_hide_range").map(|_| ()) }
    }

    /// Resets and frees all cached buffers from the device. The buffers may have been modified by
    /// [`Self::hide_range`]. The next [`Self::do_probe`] call will read all data from the device.
    #[cfg(blkid = "2.31")]
    pub fn reset_buffers(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_reset_buffers(self.0), "blkid_probe_reset_buffers").map(|_| ()) }
    }

    /// Moves the probing chain pointer one step back, so the previously used probing function
    /// will be called again in the next [`Self::do_probe`] call.
    ///
    /// This is necessary, for example, if you erase or modify an on-disk superblock based on
    /// the current libblkid probing result.
    ///
    /// Note that [`Self::hide_range`] changes the semantics of this function: cached buffers are
    /// not reset, and the library uses in-memory modified buffers to call the next probing function.
    #[cfg(blkid = "2.23")]
    pub fn step_back(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_step_back(self.0), "blkid_probe_step_back").map(|_| ()) }
    }

    /// Assigns the device to probe control struct, resets internal buffers and resets the current
    /// probing.
    ///
    /// `fd`: device file descriptor
    /// `offset`: begin of probing area
    /// `size`: size of probing area (`None` means whole device/file)
    pub fn set_device(&mut self, fd: i32, offset: i64, size: Option<i64>) -> BlkIdResult<()> {
        let size = size.unwrap_or(0);
        unsafe { c_result(blkid_probe_set_device(self.0, fd, offset, size), "blkid_probe_set_device").map(|_| ()) }
    }

    /// Zeroizes probing results and resets the current probing (this only affects
    /// [`Self::do_probe`]). This function does not touch probing filters and keeps the assigned device.
    pub fn reset_probe(&self) {
        unsafe { blkid_reset_probe(self.0) }
    }

    /// Enables or disables the superblocks probing for the non-binary interface.
    pub fn enable_superblocks(&self, enable: bool) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_enable_superblocks(self.0, enable as i32), "blkid_probe_enable_superblocks").map(|_| ()) }
    }

    /// Returns `true` if `fstype` is a known filesystem type.
    pub fn known_fstype(fstype: &str) -> BlkIdResult<bool> {
        let fstype = CString::new(fstype)?;
        Ok(unsafe { blkid_known_fstype(fstype.as_ptr()) == 1 })
    }

    /// Returns name and usage flags of a supported superblock (filesystem/raid).
    pub fn superblocks_get_name(idx: usize) -> BlkIdResult<(String, i32)> {
        let mut name: *const ::libc::c_char = ptr::null();
        let mut usage: i32 = 0;
        unsafe { c_result(blkid_superblocks_get_name(idx, &mut name, &mut usage), "blkid_superblocks_get_name") }?;
        let name = unsafe { CStr::from_ptr(name).to_str()?.to_owned() };
        Ok((name, usage))
    }

    /// Filters superblocks probing by filesystem type name.
    pub fn filter_superblocks_type(
        &self,
        mode: FilterMode,
        names: &[&str],
    ) -> BlkIdResult<()> {
        let c_names: Vec<CString> = names
            .iter()
            .map(|n| CString::new(*n))
            .collect::<Result<Vec<_>, _>>()?;
        let mut ptrs: Vec<*mut ::libc::c_char> =
            c_names.iter().map(|c| c.as_ptr() as *mut _).collect();
        ptrs.push(ptr::null_mut());
        unsafe {
            c_result(
                blkid_probe_filter_superblocks_type(
                    self.0,
                    mode as i32,
                    ptrs.as_mut_ptr(),
                ),
                "blkid_probe_filter_superblocks_type",
            )
            .map(|_| ())
        }
    }

    /// Filters superblocks probing by usage flags.
    pub fn filter_superblocks_usage(
        &self,
        mode: FilterMode,
        usage: SuperblocksUsage,
    ) -> BlkIdResult<()> {
        unsafe {
            c_result(blkid_probe_filter_superblocks_usage(
                self.0,
                mode as i32,
                usage.bits(),
            ), "blkid_probe_filter_superblocks_usage")
            .map(|_| ())
        }
    }

    /// Inverts the superblocks probing filter.
    pub fn invert_superblocks_filter(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_invert_superblocks_filter(self.0), "blkid_probe_invert_superblocks_filter").map(|_| ()) }
    }

    /// Resets the superblocks probing filter.
    pub fn reset_superblocks_filter(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_reset_superblocks_filter(self.0), "blkid_probe_reset_superblocks_filter").map(|_| ()) }
    }

    /// Sets probing flags for the superblocks prober. This function is optional; the default is
    /// [`SuperblocksFlags::DEFAULT`].
    pub fn set_superblocks_flags(&self, flags: SuperblocksFlags) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_set_superblocks_flags(self.0, flags.bits()), "blkid_probe_set_superblocks_flags").map(|_| ()) }
    }

    /// Enables or disables the partitions probing for the non-binary interface.
    pub fn enable_partitions(&self, enable: bool) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_enable_partitions(self.0, enable as i32), "blkid_probe_enable_partitions").map(|_| ()) }
    }

    /// Sets probing flags for the partitions prober. This function is optional.
    pub fn set_partitions_flags(&self, flags: PartitionsFlags) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_set_partitions_flags(self.0, flags.bits()), "blkid_probe_set_partitions_flags").map(|_| ()) }
    }

    /// Filters partitions probing by partition type name.
    pub fn filter_partitions_type(
        &self,
        mode: FilterMode,
        names: &[&str],
    ) -> BlkIdResult<()> {
        let c_names: Vec<CString> = names
            .iter()
            .map(|n| CString::new(*n))
            .collect::<Result<Vec<_>, _>>()?;
        let mut ptrs: Vec<*mut ::libc::c_char> =
            c_names.iter().map(|c| c.as_ptr() as *mut _).collect();
        ptrs.push(ptr::null_mut());
        unsafe {
            c_result(
                blkid_probe_filter_partitions_type(
                    self.0,
                    mode as i32,
                    ptrs.as_mut_ptr(),
                ),
                "blkid_probe_filter_partitions_type",
            )
            .map(|_| ())
        }
    }

    /// Inverts the partitions probing filter.
    pub fn invert_partitions_filter(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_invert_partitions_filter(self.0), "blkid_probe_invert_partitions_filter").map(|_| ()) }
    }

    /// Resets the partitions probing filter.
    pub fn reset_partitions_filter(&self) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_reset_partitions_filter(self.0), "blkid_probe_reset_partitions_filter").map(|_| ()) }
    }

    /// Returns `true` if `pttype` is a known partition table type.
    pub fn known_pttype(pttype: &str) -> BlkIdResult<bool> {
        let pttype = CString::new(pttype)?;
        Ok(unsafe { blkid_known_pttype(pttype.as_ptr()) == 1 })
    }

    /// Returns the name of a supported partition table type by index.
    #[cfg(blkid = "2.30")]
    pub fn partitions_get_name(idx: usize) -> BlkIdResult<String> {
        let mut name: *const ::libc::c_char = ptr::null();
        unsafe { c_result(blkid_partitions_get_name(idx, &mut name), "blkid_partitions_get_name") }?;
        let name = unsafe { CStr::from_ptr(name).to_str()?.to_owned() };
        Ok(name)
    }

    /// Returns [`PartList`] object.
    ///
    /// This is a binary interface for partitions.
    ///
    /// This is independent of [`Self::do_safe_probe`]/[`Self::do_full_probe`] and
    /// [`Self::enable_partitions`] calls.
    ///
    /// # Warning
    ///
    /// The returned object will be overwritten by the next [`Self::part_list`] call for the same
    /// prober. If you want to use multiple [`PartList`] objects at the same time you must create
    /// multiple [`Prober`] handles.
    pub fn part_list(&self) -> BlkIdResult<PartList<'_>> {
        unsafe { c_result(blkid_probe_get_partitions(self.0), "blkid_probe_get_partitions").map(PartList::new) }
    }

    /// Enables or disables the topology probing for the non-binary interface.
    pub fn enable_topology(&self, enable: bool) -> BlkIdResult<()> {
        unsafe { c_result(blkid_probe_enable_topology(self.0, enable as i32), "blkid_probe_enable_topology").map(|_| ()) }
    }

    /// Returns topology.
    ///
    /// This is a binary interface for topology values.
    ///
    /// This is independent of [`Self::do_safe_probe`]/[`Self::do_full_probe`] and
    /// [`Self::enable_partitions`] calls.
    ///
    /// # Warning
    ///
    /// The returned object will be overwritten by the next [`Self::topology`] call for the same
    /// prober. If you want to use multiple [`Topology`] objects at the same time you must create
    /// multiple [`Prober`] handles.
    pub fn topology(&self) -> BlkIdResult<Topology<'_>> {
        unsafe { c_result(blkid_probe_get_topology(self.0), "blkid_probe_get_topology").map(Topology::new) }
    }

    /// Sets an extra hint for the low-level prober. If the hint is set by `NAME=value` notation
    /// then the value is ignored. [`Self::set_device`] and [`Self::reset_probe`] reset all hints.
    ///
    /// Hints are an optional way to force libblkid probing functions to check, for example,
    /// another location.
    #[cfg(blkid = "2.37")]
    pub fn set_hint(&self, hint_name: &str, offset: u64) -> BlkIdResult<()> {
        let name = CString::new(hint_name)?;
        unsafe { c_result(blkid_probe_set_hint(self.0, name.as_ptr(), offset), "blkid_probe_set_hint").map(|_| ()) }
    }

    /// Removes all previously defined probing hints. See also [`Self::set_hint`].
    #[cfg(blkid = "2.37")]
    pub fn reset_hints(&self) {
        unsafe { blkid_probe_reset_hints(self.0) }
    }
}
