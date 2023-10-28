#![cfg(target_family = "unix")]
#![cfg(not(tarpaulin_include))]

use std::{ffi::CStr, io, mem::zeroed, thread::yield_now};

use io_trait::{AsyncOperation, OperationResult};
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

fn to_operation_error(result: c_int) -> io::Result<()> {
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

impl AsyncTrait for Unix {
    type Handle = i32;
    type Overlapped = aiocb;
    fn close(handle: Self::Handle) {
        unsafe {
            close(handle);
        }
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
        match unsafe { open(path.as_ptr(), oflag, 0o644) } {
            -1 => Err(io::Error::last_os_error()),
            fd => Ok(fd),
        }
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
        to_operation_error(unsafe { aio_read(overlapped) })
    }

    fn write(
        _handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        _buffer: &[u8],
    ) -> io::Result<()> {
        to_operation_error(unsafe { aio_write(overlapped) })
    }
}

pub struct File(i32);

impl Drop for File {
    fn drop(&mut self) {
        Unix::close(self.0);
    }
}

pub struct Overlapped(aiocb);

impl Default for Overlapped {
    fn default() -> Self {
        Self(unsafe { zeroed() })
    }
}

pub struct Operation<'a> {
    file: &'a mut File,
    overlapped: &'a mut Overlapped,
}

impl Drop for Operation<'_> {
    fn drop(&mut self) {
        Unix::cancel(self.file.0, &mut self.overlapped.0)
    }
}

impl Operation<'_> {
    fn get_result(&mut self) -> OperationResult {
        Unix::get_result(self.file.0, &mut self.overlapped.0)
    }
}

impl File {
    pub fn create(path: &CStr) -> io::Result<Self> {
        Unix::open(path, true).map(Self)
    }
    pub fn open(path: &CStr) -> io::Result<Self> {
        Unix::open(path, false).map(Self)
    }
    pub fn write<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped,
        offset: u64,
        buffer: &'a [u8],
    ) -> io::Result<Operation<'a>> {
        Unix::init_overlapped(self.0, &mut overlapped.0, offset, buffer);
        Unix::write(self.0, &mut overlapped.0, buffer).map(|_| Operation {
            file: self,
            overlapped,
        })
    }
    pub fn read<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped,
        offset: u64,
        buffer: &'a mut [u8],
    ) -> io::Result<Operation<'a>> {
        Unix::init_overlapped(self.0, &mut overlapped.0, offset, buffer);
        Unix::read(self.0, &mut overlapped.0, buffer).map(|_| Operation {
            file: self,
            overlapped,
        })
    }
}

impl AsyncOperation for Operation<'_> {
    fn get_result(&mut self) -> OperationResult {
        self.get_result()
    }
}
