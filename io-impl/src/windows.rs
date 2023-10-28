#![cfg(target_family = "windows")]
#![cfg(not(tarpaulin_include))]
use std::{ffi::CStr, io, os::windows::raw::HANDLE, ptr::null_mut};

use io_trait::OperationResult;

use crate::{
    async_traits::AsyncTrait,
    windows_api::{
        self, CancelIoEx, CloseHandle, CreateFileA, Error, GetLastError, GetOverlappedResult,
        ReadFile, WriteFile, BOOL, CREATE_ALWAYS, DWORD, ERROR_SUCCESS, FILE_FLAG_OVERLAPPED,
        GENERIC_READ, GENERIC_WRITE, LPCVOID, LPVOID, OPEN_ALWAYS, OVERLAPPED,
    },
};

pub struct Windows();

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

fn is_pending(e: Error) -> bool {
    e == windows_api::ERROR_IO_PENDING || e == windows_api::ERROR_IO_INCOMPLETE
}

fn to_operation_result((v, size): (BOOL, DWORD)) -> OperationResult {
    match get_last_error(v) {
        windows_api::ERROR_SUCCESS => OperationResult::Ok(size as usize),
        windows_api::ERROR_HANDLE_EOF => OperationResult::Ok(0),
        e => {
            if is_pending(e) {
                OperationResult::Pending
            } else {
                OperationResult::Err(e.to_error())
            }
        }
    }
}

fn to_result(result: BOOL) -> io::Result<()> {
    let e = get_last_error(result);
    if e == windows_api::ERROR_SUCCESS || is_pending(e) {
        return Ok(());
    }
    Err(e.to_error())
}

impl AsyncTrait for Windows {
    type Handle = HANDLE;
    type Overlapped = OVERLAPPED;
    fn overlapped_default() -> Self::Overlapped {
        OVERLAPPED::default()
    }
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

    fn read(
        handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        buffer: &mut [u8],
    ) -> io::Result<()> {
        to_result(unsafe {
            ReadFile(
                handle,
                buffer.as_mut_ptr() as LPVOID,
                buffer.len() as DWORD,
                null_mut(),
                overlapped,
            )
        })
    }
    fn write(
        handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        buffer: &[u8],
    ) -> io::Result<()> {
        to_result(unsafe {
            WriteFile(
                handle,
                buffer.as_ptr() as LPCVOID,
                buffer.len() as DWORD,
                null_mut(),
                overlapped,
            )
        })
    }
}

pub type Os = Windows;
