use std::{ffi::NulError, io, str::Utf8Error};
use thiserror::Error;

pub type BlkIdResult<T, E = BlkIdError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum BlkIdError {
    #[error(transparent)]
    Utf8(#[from] Utf8Error),

    #[error(transparent)]
    Nul(#[from] NulError),

    #[error(transparent)]
    Io(#[from] io::Error),
}

pub(crate) trait RawResult: Copy {
    fn is_error(self) -> bool;
}

pub(crate) fn c_result<T: RawResult>(value: T) -> BlkIdResult<T> {
    if value.is_error() {
        Err(BlkIdError::Io(std::io::Error::last_os_error()))
    } else {
        Ok(value)
    }
}

impl RawResult for i32 {
    fn is_error(self) -> bool {
        self < 0
    }
}

impl RawResult for i64 {
    fn is_error(self) -> bool {
        self < 0
    }
}

impl<T> RawResult for *const T {
    fn is_error(self) -> bool {
        self.is_null()
    }
}

impl<T> RawResult for *mut T {
    fn is_error(self) -> bool {
        self.is_null()
    }
}
