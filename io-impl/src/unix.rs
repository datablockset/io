#![cfg(target_family = "unix")]
#![cfg(not(tarpaulin_include))]

use std::{ffi::CStr, io, mem::zeroed, thread::yield_now};

use io_trait::OperationResult;
use libc::{
    aio_cancel, aio_read, aio_return, aio_write, aiocb, c_int, close, open, AIO_NOTCANCELED,
};

use crate::async_traits::AsyncTrait;

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
struct AioError(c_int);
const EINPROGRESS: AioError = AioError(libc::EINPROGRESS);

fn aio_error(overlapped: &aiocb) -> AioError {
    AioError(unsafe { libc::aio_error(overlapped) })
}

pub struct Unix();

fn to_result(result: c_int) -> io::Result<c_int> {
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(result)
}

fn to_operation_result(result: c_int) -> io::Result<()> {
    to_result(result).map(|_| ())
}

impl AsyncTrait for Unix {
    type Handle = i32;
    type Overlapped = aiocb;
    fn overlapped_default() -> Self::Overlapped {
        unsafe { zeroed() }
    }
    fn close(handle: Self::Handle) {
        unsafe { close(handle) };
    }
    fn cancel(handle: Self::Handle, overlapped: &mut Self::Overlapped) {
        if unsafe { aio_cancel(handle, overlapped) } != AIO_NOTCANCELED {
            return;
        }
        loop {
            yield_now();
            if aio_error(overlapped) != EINPROGRESS {
                return;
            }
        }
    }
    fn get_result(_handle: Self::Handle, overlapped: &mut Self::Overlapped) -> OperationResult {
        match aio_error(overlapped) {
            AioError(0) => OperationResult::Ok(unsafe { aio_return(overlapped) } as usize),
            EINPROGRESS => OperationResult::Pending,
            e => OperationResult::Err(io::Error::from_raw_os_error(e.0)),
        }
    }
    fn open(path: &CStr, create: bool) -> io::Result<Self::Handle> {
        let oflag = if create {
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC
        } else {
            libc::O_RDONLY
        };
        to_result(unsafe { open(path.as_ptr(), oflag, 0o644) })
    }
    fn init_overlapped(
        handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        offset: u64,
        buffer: &[u8],
    ) {
        *overlapped = unsafe { zeroed() };
        overlapped.aio_fildes = handle;
        overlapped.aio_buf = buffer.as_ptr() as *mut _;
        overlapped.aio_nbytes = buffer.len();
        overlapped.aio_offset = offset as i64;
    }
    fn read(
        _handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        _buffer: &mut [u8],
    ) -> io::Result<()> {
        to_operation_result(unsafe { aio_read(overlapped) })
    }

    fn write(
        _handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        _buffer: &[u8],
    ) -> io::Result<()> {
        to_operation_result(unsafe { aio_write(overlapped) })
    }
}

pub type Os = Unix;
