use std::{ffi::CStr, io};

use io_trait::{AsyncFile, AsyncOperation, OperationResult};

pub trait AsyncTrait {
    type Handle: Copy;
    type Overlapped;
    fn overlapped_default() -> Self::Overlapped;
    fn close(handle: Self::Handle);
    fn cancel(handle: Self::Handle, overlapped: &mut Self::Overlapped);
    fn get_result(handle: Self::Handle, overlapped: &mut Self::Overlapped) -> OperationResult;
    fn open(path: &CStr, create: bool) -> io::Result<Self::Handle>;
    fn init_overlapped(
        handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        offset: u64,
        buffer: &[u8],
    );
    fn read(
        handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        buffer: &mut [u8],
    ) -> io::Result<()>;
    fn write(
        handle: Self::Handle,
        overlapped: &mut Self::Overlapped,
        buffer: &[u8],
    ) -> io::Result<()>;
}

//

#[repr(transparent)]
pub struct Handle<T: AsyncTrait>(T::Handle);

impl<T: AsyncTrait> Drop for Handle<T> {
    fn drop(&mut self) {
        T::close(self.0);
    }
}

#[repr(transparent)]
pub struct Overlapped<T: AsyncTrait>(T::Overlapped);

impl<T: AsyncTrait> Default for Overlapped<T> {
    fn default() -> Self {
        Self(T::overlapped_default())
    }
}

impl<T: AsyncTrait> Handle<T> {
    pub fn create(file_name: &CStr) -> io::Result<Self> {
        T::open(file_name, true).map(Handle)
    }
    pub fn open(file_name: &CStr) -> io::Result<Self> {
        T::open(file_name, false).map(Handle)
    }
    /// Note: it's important that self, overlapped and the buffer have the same life time as the returned operation!
    pub fn read<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped<T>,
        offset: u64,
        buffer: &'a mut [u8],
    ) -> io::Result<Operation<'a, T>> {
        T::init_overlapped(self.0, &mut overlapped.0, offset, buffer);
        T::read(self.0, &mut overlapped.0, buffer).map(|_| Operation {
            handle: self.0,
            overlapped,
        })
    }
    /// Note: it's important that self, overlapped and the buffer have the same life time as the returned operation!
    pub fn write<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped<T>,
        offset: u64,
        buffer: &'a [u8],
    ) -> io::Result<Operation<'a, T>> {
        T::init_overlapped(self.0, &mut overlapped.0, offset, buffer);
        T::write(self.0, &mut overlapped.0, buffer).map(|_| Operation {
            handle: self.0,
            overlapped,
        })
    }
}

pub struct File<T: AsyncTrait> {
    pub handle: Handle<T>,
    pub overlapped: Overlapped<T>,
}

impl<T: AsyncTrait> AsyncFile for File<T> {
    type Operation<'a> = Operation<'a, T> where T: 'a;
    fn create(path: &CStr) -> io::Result<Self> {
        Ok(File {
            handle: Handle::create(path)?,
            overlapped: Overlapped::default(),
        })
    }
    fn open(path: &CStr) -> io::Result<Self> {
        Ok(File {
            handle: Handle::open(path)?,
            overlapped: Default::default(),
        })
    }
    fn read<'a>(
        &'a mut self,
        offset: u64,
        buffer: &'a mut [u8],
    ) -> io::Result<Self::Operation<'a>> {
        self.handle.read(&mut self.overlapped, offset, buffer)
    }

    fn write<'a>(&'a mut self, offset: u64, buffer: &'a [u8]) -> io::Result<Self::Operation<'a>> {
        self.handle.write(&mut self.overlapped, offset, buffer)
    }
}

pub struct Operation<'a, T: AsyncTrait> {
    handle: T::Handle,
    overlapped: &'a mut Overlapped<T>,
}

impl<T: AsyncTrait> Drop for Operation<'_, T> {
    fn drop(&mut self) {
        T::cancel(self.handle, &mut self.overlapped.0);
    }
}

impl<T: AsyncTrait> AsyncOperation for Operation<'_, T> {
    fn get_result(&mut self) -> OperationResult {
        T::get_result(self.handle, &mut self.overlapped.0)
    }
}