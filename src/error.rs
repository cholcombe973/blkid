use errno::errno;
use std::{
    ffi::{IntoStringError, NulError},
    ptr,
    string::FromUtf8Error,
};

#[derive(thiserror::Error, Debug)]
pub enum BlkidError {
    #[error("blkid error, value is '{val}'")]
    LibBlkid { val: i32 },

    #[error("blkid returned NULL")]
    LibBlkidNull,

    #[error("Unknown return code from `blkid_probe_has_value`: '{ret_code}'")]
    LibBlkidHasValue { ret_code: i32 },

    #[error("Unknown return code from `blkid_known_fstype`: '{ret_code}'")]
    LibBlkidKnownFsType { ret_code: i32 },

    #[error(transparent)]
    FromUtf8(#[from] FromUtf8Error),

    #[error(transparent)]
    Nul(#[from] NulError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    IntoString(#[from] IntoStringError),

    #[error(transparent)]
    Errno(#[from] errno::Errno),

    #[error("Other")]
    Other(String),
}

impl BlkidError {
    pub fn get_error() -> Self {
        BlkidError::Errno(errno())
    }
}

pub fn result(val: ::libc::c_int) -> Result<(), BlkidError> {
    match val {
        0 => Ok(()),
        _ => Err(BlkidError::LibBlkid { val }),
    }
}

pub fn result_ptr_mut<T>(val: *mut T) -> Result<*mut T, BlkidError> {
    if ptr::eq(ptr::null(), val) {
        Err(BlkidError::LibBlkidNull)
    } else {
        Ok(val)
    }
}
