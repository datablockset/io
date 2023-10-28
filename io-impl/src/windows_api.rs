#![cfg(target_os = "windows")]
#![cfg(not(tarpaulin_include))]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]

use std::{
    io,
    os::{raw::c_void, windows::raw::HANDLE},
    ptr::null_mut,
};

pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/3f6cc0e2-1303-4088-a26b-fb9582f29197
type LPCSTR = *const i8;

// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/262627d8-3418-4627-9218-4ffe110850b2
pub type DWORD = u32;
pub type LPDWORD = *mut DWORD;

// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/c0b7741b-f577-4eed-aff3-2e909df10a4d
pub type LPVOID = *mut c_void;
pub type LPCVOID = *const c_void;
type PVOID = *mut c_void;

// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/21eec394-630d-49ed-8b4a-ab74a1614611
type ULONG_PTR = usize;

// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dtyp/9d81be47-232e-42cf-8f0d-7a3b29bf2eb2
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct BOOL(i32);
pub const FALSE: BOOL = BOOL(0);
pub const TRUE: BOOL = BOOL(1);
impl BOOL {
    pub fn to_bool(self) -> bool {
        self.0 != FALSE.0
    }
}
impl From<bool> for BOOL {
    fn from(x: bool) -> Self {
        if x {
            TRUE
        } else {
            FALSE
        }
    }
}

// https://learn.microsoft.com/en-us/windows/win32/api/minwinbase/ns-minwinbase-overlapped

#[repr(C)]
#[derive(Copy, Clone)]
pub struct OVERLAPPED_Offset {
    pub Offset: DWORD,
    pub OffsetHigh: DWORD,
}

#[repr(C)]
pub union OVERLAPPED_OffsetOrPointer {
    pub Offset: OVERLAPPED_Offset,
    Pointer: PVOID,
}

#[repr(C)]
pub struct OVERLAPPED {
    Internal: ULONG_PTR,
    InternalHigh: ULONG_PTR,
    pub OffsetOrPointer: OVERLAPPED_OffsetOrPointer,
    hEvent: HANDLE,
}

impl Default for OVERLAPPED {
    fn default() -> Self {
        Self::new(0)
    }
}

impl OVERLAPPED {
    pub fn new(offset: u64) -> Self {
        Self {
            Internal: 0,
            InternalHigh: 0,
            OffsetOrPointer: OVERLAPPED_OffsetOrPointer {
                Offset: OVERLAPPED_Offset {
                    Offset: offset as DWORD,
                    OffsetHigh: (offset >> 32) as DWORD,
                },
            },
            hEvent: null_mut(),
        }
    }
}

pub type LPOVERLAPPED = *mut OVERLAPPED;

// https://learn.microsoft.com/en-us/windows/win32/secauthz/access-mask
#[repr(transparent)]
pub struct ACCESS_MASK(DWORD);
pub const GENERIC_READ: ACCESS_MASK = ACCESS_MASK(0x80000000);
pub const GENERIC_WRITE: ACCESS_MASK = ACCESS_MASK(0x40000000);

// https://learn.microsoft.com/en-us/windows/win32/api/wtypesbase/ns-wtypesbase-security_attributes
#[repr(C)]
pub struct SECURITY_ATTRIBUTES {
    nLength: DWORD,
    lpSecurityDescriptor: LPVOID,
    bInheritHandle: BOOL,
}
type LPSECURITY_ATTRIBUTES = *mut SECURITY_ATTRIBUTES;

#[repr(transparent)]
pub struct CreationDisposition(DWORD);
pub const CREATE_ALWAYS: CreationDisposition = CreationDisposition(2);
pub const OPEN_ALWAYS: CreationDisposition = CreationDisposition(4);

#[repr(transparent)]
pub struct FlagsAndAttributes(DWORD);
pub const FILE_FLAG_OVERLAPPED: FlagsAndAttributes = FlagsAndAttributes(0x40000000);

// https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--500-999-
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Error(DWORD);
pub const ERROR_HANDLE_EOF: Error = Error(38);
pub const ERROR_IO_INCOMPLETE: Error = Error(996);
pub const ERROR_IO_PENDING: Error = Error(997);
impl Error {
    pub fn to_error(self) -> io::Error {
        io::Error::from_raw_os_error(self.0 as i32)
    }
}

// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea
#[link(name = "kernel32")]
extern "system" {
    pub fn CreateFileA(
        lpFileName: LPCSTR,                          // [in]
        dwDesiredAccess: ACCESS_MASK,                // [in]
        dwShareMode: DWORD,                          // [in]
        lpSecurityAttributes: LPSECURITY_ATTRIBUTES, // [in, optional]
        dwCreationDisposition: CreationDisposition,  // [in]
        dwFlagsAndAttributes: FlagsAndAttributes,    // [in]
        hTemplateFile: HANDLE,                       // [in, optional]
    ) -> HANDLE;
}

// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile
#[link(name = "kernel32")]
extern "system" {
    pub fn ReadFile(
        hFile: HANDLE,                // [in]
        lpBuffer: LPVOID,             // [out]
        nNumberOfBytesToRead: DWORD,  // [in]
        lpNumberOfBytesRead: LPDWORD, // [out, optional]
        lpOverlapped: LPOVERLAPPED,   // [in, out, optional]
    ) -> BOOL;
}

// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile
#[link(name = "kernel32")]
extern "system" {
    pub fn WriteFile(
        hFile: HANDLE,                   // [in]
        lpBuffer: LPCVOID,               // [in]
        nNumberOfBytesToWrite: DWORD,    // [in]
        lpNumberOfBytesWritten: LPDWORD, // [out, optional]
        lpOverlapped: LPOVERLAPPED,      // [in, out, optional]
    ) -> BOOL;
}

// https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle
#[link(name = "kernel32")]
extern "system" {
    pub fn CloseHandle(hObject: HANDLE, // [in]
    ) -> BOOL;
}

// https://learn.microsoft.com/en-us/windows/win32/fileio/cancelioex-func
#[link(name = "kernel32")]
extern "system" {
    pub fn CancelIoEx(
        hFile: HANDLE,              // [in]
        lpOverlapped: LPOVERLAPPED, // [in, optional]
    ) -> BOOL;
}

// https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-getoverlappedresult
#[link(name = "kernel32")]
extern "system" {
    pub fn GetOverlappedResult(
        hFile: HANDLE,                       // [in]
        lpOverlapped: LPOVERLAPPED,          // [in]
        lpNumberOfBytesTransferred: LPDWORD, // [out, optional]
        bWait: BOOL,                         // [in]
    ) -> BOOL;
}

// https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror
#[link(name = "kernel32")]
extern "system" {
    pub fn GetLastError() -> Error;
}
