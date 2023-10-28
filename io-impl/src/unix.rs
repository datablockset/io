#![cfg(target_family = "unix")]
#![cfg(not(tarpaulin_include))]

use std::{ffi::CStr, io, mem::zeroed, thread::yield_now};

use io_trait::{AsyncOperation, OperationResult};
use libc::{
    aio_cancel, aio_error, aio_read, aio_return, aio_write, aiocb, close, open, AIO_NOTCANCELED,
    ECANCELED, EINPROGRESS,
};

use crate::async_traits::AsyncTrait;

pub struct Unix();

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
            if unsafe { aio_error(overlapped) } != EINPROGRESS {
                return;
            }
        }
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
        match unsafe { aio_error(&self.overlapped.0) } {
            0 => OperationResult::Ok(unsafe { aio_return(&mut self.overlapped.0) } as usize),
            libc::EINPROGRESS => OperationResult::Pending,
            e => OperationResult::Err(io::Error::from_raw_os_error(e)),
        }
    }
}

impl File {
    fn internal_open(path: &CStr, oflag: i32) -> io::Result<Self> {
        let fd = unsafe { open(path.as_ptr(), oflag, 0o644) };
        if fd == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(fd))
        }
    }
    pub fn create(path: &CStr) -> io::Result<Self> {
        File::internal_open(path, libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC)
    }
    pub fn open(path: &CStr) -> io::Result<Self> {
        File::internal_open(path, libc::O_RDONLY)
    }
    fn create_operation<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped,
        offset: u64,
        buffer: &'a [u8],
        f: unsafe extern "C" fn(*mut aiocb) -> i32,
    ) -> io::Result<Operation<'a>> {
        *overlapped = Default::default();
        overlapped.0.aio_fildes = self.0;
        overlapped.0.aio_buf = buffer.as_ptr() as *mut _;
        overlapped.0.aio_nbytes = buffer.len();
        overlapped.0.aio_offset = offset as i64;
        if unsafe { f(&mut overlapped.0) } == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(Operation {
                file: self,
                overlapped,
            })
        }
    }
    pub fn write<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped,
        offset: u64,
        buffer: &'a [u8],
    ) -> io::Result<Operation<'a>> {
        self.create_operation(overlapped, offset, buffer, aio_write)
    }
    pub fn read<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped,
        offset: u64,
        buffer: &'a mut [u8],
    ) -> io::Result<Operation<'a>> {
        self.create_operation(overlapped, offset, buffer, aio_read)
    }
}

impl AsyncOperation for Operation<'_> {
    fn get_result(&mut self) -> OperationResult {
        self.get_result()
    }
}
