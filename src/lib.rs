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
use blkid_sys::*;
use std::{
    ffi::{CStr, CString},
    path::Path,
    ptr,
};

pub use error::{BlkIdError, BlkIdResult};
use error::c_result;

/// Filter mode for probing chain filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    /// Exclude listed types.
    Notin = 1,
    /// Include only listed types.
    Onlyin = 2,
}

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
        #[cfg(blkid = "2.24")]
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

bitflags! {
    /// Usage flags for superblocks filtering.
    pub struct SuperblocksUsage: i32 {
        const FILESYSTEM = 1;
        const RAID = 2;
        const CRYPTO = 4;
        const OTHER = 8;
    }
}

/// Returns the device name for the given device number. Returns `None` if not found.
pub fn devno_to_devname(devno: u64) -> Option<String> {
    let ptr = unsafe { blkid_devno_to_devname(devno) };
    if ptr.is_null() {
        None
    } else {
        let s = unsafe { CStr::from_ptr(ptr).to_str().ok()?.to_owned() };
        unsafe { libc::free(ptr as *mut _) };
        Some(s)
    }
}

/// Converts a device number to its whole-disk device name and device number.
/// Returns `(disk_name, disk_devno)`.
pub fn devno_to_wholedisk(devno: u64) -> BlkIdResult<(String, u64)> {
    let mut buf = vec![0u8; 128];
    let mut diskdevno: u64 = 0;
    unsafe {
        c_result(blkid_devno_to_wholedisk(
            devno,
            buf.as_mut_ptr() as *mut _,
            buf.len(),
            &mut diskdevno,
        ))?;
    }
    let name = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) }
        .to_str()?
        .to_owned();
    Ok((name, diskdevno))
}

/// Encode potentially unsafe characters in a string for use in udev-style paths.
pub fn encode_string(input: &str) -> BlkIdResult<String> {
    let c_input = CString::new(input)?;
    let mut buf = vec![0u8; input.len() * 4 + 1];
    unsafe {
        c_result(blkid_encode_string(
            c_input.as_ptr(),
            buf.as_mut_ptr() as *mut _,
            buf.len(),
        ))?;
    }
    let s = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) }
        .to_str()?
        .to_owned();
    Ok(s)
}

/// Replace all unsafe chars with the replacement char (underscore by default in libblkid).
pub fn safe_string(input: &str) -> BlkIdResult<String> {
    let c_input = CString::new(input)?;
    let mut buf = vec![0u8; input.len() + 1];
    unsafe {
        c_result(blkid_safe_string(
            c_input.as_ptr(),
            buf.as_mut_ptr() as *mut _,
            buf.len(),
        ))?;
    }
    let s = unsafe { CStr::from_ptr(buf.as_ptr() as *const _) }
        .to_str()?
        .to_owned();
    Ok(s)
}

/// Returns the library version as `(version_code, version_string, date_string)`.
pub fn get_library_version() -> (i32, String, String) {
    let mut ver_str: *const libc::c_char = ptr::null();
    let mut date_str: *const libc::c_char = ptr::null();
    let code = unsafe { blkid_get_library_version(&mut ver_str, &mut date_str) };
    let ver = unsafe { CStr::from_ptr(ver_str) }
        .to_str()
        .unwrap_or("")
        .to_owned();
    let date = unsafe { CStr::from_ptr(date_str) }
        .to_str()
        .unwrap_or("")
        .to_owned();
    (code, ver, date)
}

/// Parse a version string (e.g. "2.17.0") and return a version code.
pub fn parse_version_string(ver: &str) -> BlkIdResult<i32> {
    let c_ver = CString::new(ver)?;
    let code = unsafe { blkid_parse_version_string(c_ver.as_ptr()) };
    // blkid_parse_version_string returns -1 on error
    if code < 0 {
        Err(BlkIdError::Io(std::io::Error::last_os_error()))
    } else {
        Ok(code)
    }
}

/// Send a uevent for the specified device.
pub fn send_uevent(devname: &str, action: &str) -> BlkIdResult<()> {
    let c_devname = CString::new(devname)?;
    let c_action = CString::new(action)?;
    unsafe { c_result(blkid_send_uevent(c_devname.as_ptr(), c_action.as_ptr())).map(|_| ()) }
}

/// Parse a "NAME=value" tag string. Returns `(name, value)`.
pub fn parse_tag_string(tag: &str) -> BlkIdResult<(String, String)> {
    let c_tag = CString::new(tag)?;
    let mut ret_type: *mut libc::c_char = ptr::null_mut();
    let mut ret_val: *mut libc::c_char = ptr::null_mut();
    unsafe {
        c_result(blkid_parse_tag_string(
            c_tag.as_ptr(),
            &mut ret_type,
            &mut ret_val,
        ))?;
    }
    let name = unsafe { CStr::from_ptr(ret_type).to_str()?.to_owned() };
    let value = unsafe { CStr::from_ptr(ret_val).to_str()?.to_owned() };
    unsafe {
        libc::free(ret_type as *mut _);
        libc::free(ret_val as *mut _);
    }
    Ok((name, value))
}

/// Initialize blkid debug output.
pub fn init_debug(mask: i32) {
    unsafe { blkid_init_debug(mask) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_library_version() {
        let (code, ver, date) = get_library_version();
        assert!(code > 0, "version code should be positive");
        assert!(!ver.is_empty(), "version string should not be empty");
        assert!(!date.is_empty(), "date string should not be empty");
        // Version string should look like "2.X.Y" or "2.X"
        assert!(ver.starts_with("2."), "version should start with '2.': {}", ver);
    }

    #[test]
    fn test_parse_version_string() {
        let code = parse_version_string("2.17.0").unwrap();
        assert!(code > 0);

        let code2 = parse_version_string("2.40.0").unwrap();
        assert!(code2 > code, "newer version should have higher code");
    }

    #[test]
    fn test_parse_version_string_matches_library() {
        let (lib_code, lib_ver, _) = get_library_version();
        let parsed = parse_version_string(&lib_ver).unwrap();
        assert_eq!(lib_code, parsed);
    }

    #[test]
    fn test_encode_string_plain() {
        // Plain ASCII should pass through unchanged
        let result = encode_string("hello").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_encode_string_with_space() {
        // Spaces get hex-encoded
        let result = encode_string("hello world").unwrap();
        assert_eq!(result, "hello\\x20world");
    }

    #[test]
    fn test_safe_string_plain() {
        let result = safe_string("hello").unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_safe_string_with_unsafe_chars() {
        // Unsafe characters should be replaced with underscores
        let result = safe_string("hello\tworld").unwrap();
        assert!(
            !result.contains('\t'),
            "tab should be replaced: {}",
            result
        );
    }

    #[test]
    fn test_parse_tag_string_valid() {
        let (name, value) = parse_tag_string("LABEL=root").unwrap();
        assert_eq!(name, "LABEL");
        assert_eq!(value, "root");
    }

    #[test]
    fn test_parse_tag_string_uuid() {
        let (name, value) =
            parse_tag_string("UUID=01234567-89ab-cdef-0123-456789abcdef").unwrap();
        assert_eq!(name, "UUID");
        assert_eq!(value, "01234567-89ab-cdef-0123-456789abcdef");
    }

    #[test]
    fn test_parse_tag_string_invalid() {
        // No '=' sign, should fail
        assert!(parse_tag_string("notavalidtag").is_err());
    }

    #[test]
    fn test_devno_to_devname_invalid() {
        // devno 0 should not resolve to a real device
        assert!(devno_to_devname(0).is_none());
    }

    #[test]
    fn test_known_fstype() {
        assert!(prober::Prober::known_fstype("ext4").unwrap());
        assert!(prober::Prober::known_fstype("vfat").unwrap());
        assert!(!prober::Prober::known_fstype("not_a_real_fs").unwrap());
    }

    #[test]
    fn test_known_pttype() {
        assert!(prober::Prober::known_pttype("gpt").unwrap());
        assert!(prober::Prober::known_pttype("dos").unwrap());
        assert!(!prober::Prober::known_pttype("not_a_real_pt").unwrap());
    }

    #[test]
    fn test_superblocks_get_name() {
        // Index 0 should return a valid superblock name
        let (name, usage) = prober::Prober::superblocks_get_name(0).unwrap();
        assert!(!name.is_empty());
        assert!(usage > 0);
    }

    #[test]
    fn test_superblocks_get_name_multiple() {
        // Iterate a few known-good indices
        let mut names = Vec::new();
        for i in 0..5 {
            if let Ok((name, _)) = prober::Prober::superblocks_get_name(i) {
                names.push(name);
            }
        }
        assert!(!names.is_empty(), "should find at least one superblock name");
    }

}
