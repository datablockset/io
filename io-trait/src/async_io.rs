use std::{ffi::CStr, io};

#[derive(Debug)]
pub enum OperationResult {
    Ok(usize),
    Pending,
    Err(io::Error),
}

pub trait AsyncOperation {
    fn get_result(&mut self) -> OperationResult;
}

pub trait AsyncFile: Sized {
    type Operation<'a>: AsyncOperation
    where
        Self: 'a;
    fn create(path: &CStr) -> io::Result<Self>;
    fn open(path: &CStr) -> io::Result<Self>;
    fn read<'a>(&'a mut self, offset: u64, buffer: &'a mut [u8])
        -> io::Result<Self::Operation<'a>>;
    fn write<'a>(&'a mut self, offset: u64, buffer: &'a [u8]) -> io::Result<Self::Operation<'a>>;
}

pub trait AsyncIo {
    type File: AsyncFile;
    fn create(&self, path: &CStr) -> io::Result<Self::File>;
    fn open(&self, path: &CStr) -> io::Result<Self::File>;
}
