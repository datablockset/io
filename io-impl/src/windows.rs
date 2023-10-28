#![cfg(target_family = "windows")]
#![cfg(not(tarpaulin_include))]
use std::{ffi::CStr, io, os::windows::raw::HANDLE, ptr::null_mut};

use io_trait::{AsyncOperation, OperationResult};

use crate::{
    async_traits::AsyncTrait,
    windows_api::{
        self, CancelIoEx, CloseHandle, CreateFileA, Error, GetLastError, GetOverlappedResult,
        ReadFile, WriteFile, BOOL, CREATE_ALWAYS, DWORD, ERROR_SUCCESS, FILE_FLAG_OVERLAPPED,
        GENERIC_READ, GENERIC_WRITE, LPCVOID, LPVOID, OPEN_ALWAYS, OVERLAPPED,
    },
};

struct Windows();

fn get_overlapped_result(handle: HANDLE, overlapped: &mut OVERLAPPED, wait: bool) -> (BOOL, DWORD) {
    let mut size: DWORD = 0;
    let result = unsafe { GetOverlappedResult(handle, overlapped, &mut size, wait.into()) };
    (result, size)
}

fn get_last_error(v: BOOL) -> Error {
    if v.to_bool() {
        return ERROR_SUCCESS;
    }
    unsafe { GetLastError() }
}

fn to_operation_result((v, size): (BOOL, DWORD)) -> OperationResult {
    match get_last_error(v) {
        windows_api::ERROR_SUCCESS => OperationResult::Ok(size as usize),
        windows_api::ERROR_IO_PENDING | windows_api::ERROR_IO_INCOMPLETE => {
            OperationResult::Pending
        }
        windows_api::ERROR_HANDLE_EOF => OperationResult::Ok(0),
        e => OperationResult::Err(e.to_error()),
    }
}

impl AsyncTrait for Windows {
    type Handle = HANDLE;
    type Overlapped = OVERLAPPED;
    fn close(handle: Self::Handle) {
        unsafe { CloseHandle(handle) };
    }
    fn cancel(handle: Self::Handle, overlapped: &mut Self::Overlapped) {
        if unsafe { CancelIoEx(handle, overlapped) }.to_bool() {
            return;
        }
        let _ = get_overlapped_result(handle, overlapped, true);
    }
    fn get_result(handle: Self::Handle, overlapped: &mut Self::Overlapped) -> OperationResult {
        to_operation_result(get_overlapped_result(handle, overlapped, false))
    }
    fn open(path: &CStr, create: bool) -> io::Result<Self::Handle> {
        let (da, cp) = if create {
            (GENERIC_WRITE, CREATE_ALWAYS)
        } else {
            (GENERIC_READ, OPEN_ALWAYS)
        };
        match unsafe {
            CreateFileA(
                path.as_ptr(),
                da,
                0,
                null_mut(),
                cp,
                FILE_FLAG_OVERLAPPED,
                null_mut(),
            )
        } {
            windows_api::INVALID_HANDLE_VALUE => Err(io::Error::last_os_error()),
            h => Ok(h),
        }
    }
    fn init_overlapped(
        _handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        offset: u64,
        _buffer: &[u8],
    ) {
        *overlapped = OVERLAPPED::new(offset);
    }
}

#[repr(transparent)]
pub struct File(HANDLE);

impl Drop for File {
    fn drop(&mut self) {
        Windows::close(self.0);
    }
}

impl File {
    pub fn create(file_name: &CStr) -> io::Result<Self> {
        Windows::open(file_name, true).map(File)
    }
    pub fn open(file_name: &CStr) -> io::Result<Self> {
        Windows::open(file_name, false).map(File)
    }

    fn create_operation<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped,
        result: BOOL,
    ) -> io::Result<Operation<'a>> {
        if let OperationResult::Err(e) = to_operation_result((result, 0)) {
            return Err(e);
        }
        Ok(Operation {
            handle: self,
            overlapped,
        })
    }

    pub fn read<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped,
        offset: u64,
        buffer: &'a mut [u8], // it's important that the buffer has the same life time as the overlapped!
    ) -> io::Result<Operation<'a>> {
        Windows::init_overlapped(self.0, &mut overlapped.0, offset, buffer);
        let result = unsafe {
            ReadFile(
                self.0,
                buffer.as_mut_ptr() as LPVOID,
                buffer.len() as DWORD,
                null_mut(),
                &mut overlapped.0,
            )
        };
        self.create_operation(overlapped, result)
    }

    pub fn write<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped,
        offset: u64,
        buffer: &'a [u8], // it's important that the buffer has the same life time as the overlapped!
    ) -> io::Result<Operation<'a>> {
        Windows::init_overlapped(self.0, &mut overlapped.0, offset, buffer);
        let result = unsafe {
            WriteFile(
                self.0,
                buffer.as_ptr() as LPCVOID,
                buffer.len() as DWORD,
                null_mut(),
                &mut overlapped.0,
            )
        };
        self.create_operation(overlapped, result)
    }
}

#[derive(Default)]
pub struct Overlapped(OVERLAPPED);

pub struct Operation<'a> {
    handle: &'a mut File,
    overlapped: &'a mut Overlapped,
}

impl Drop for Operation<'_> {
    fn drop(&mut self) {
        Windows::cancel(self.handle.0, &mut self.overlapped.0);
    }
}

impl AsyncOperation for Operation<'_> {
    fn get_result(&mut self) -> OperationResult {
        Windows::get_result(self.handle.0, &mut self.overlapped.0)
    }
}
