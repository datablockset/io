use std::{ffi::CStr, io};

use io_trait::{AsyncOperation, OperationResult};

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
pub struct File<T: AsyncTrait>(T::Handle);

impl<T: AsyncTrait> Drop for File<T> {
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

impl<T: AsyncTrait> File<T> {
    pub fn create(file_name: &CStr) -> io::Result<Self> {
        T::open(file_name, true).map(File)
    }
    pub fn open(file_name: &CStr) -> io::Result<Self> {
        T::open(file_name, false).map(File)
    }

    pub fn read<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped<T>,
        offset: u64,
        buffer: &'a mut [u8], // it's important that the buffer has the same life time as the overlapped!
    ) -> io::Result<Operation<'a, T>> {
        T::init_overlapped(self.0, &mut overlapped.0, offset, buffer);
        T::read(self.0, &mut overlapped.0, buffer).map(|_| Operation {
            handle: self,
            overlapped,
        })
    }

    pub fn write<'a>(
        &'a mut self,
        overlapped: &'a mut Overlapped<T>,
        offset: u64,
        buffer: &'a [u8], // it's important that the buffer has the same life time as the overlapped!
    ) -> io::Result<Operation<'a, T>> {
        T::init_overlapped(self.0, &mut overlapped.0, offset, buffer);
        T::write(self.0, &mut overlapped.0, buffer).map(|_| Operation {
            handle: self,
            overlapped,
        })
    }
}

pub struct Operation<'a, T: AsyncTrait> {
    handle: &'a mut File<T>,
    overlapped: &'a mut Overlapped<T>,
}

impl<T: AsyncTrait> Drop for Operation<'_, T> {
    fn drop(&mut self) {
        T::cancel(self.handle.0, &mut self.overlapped.0);
    }
}

impl<T: AsyncTrait> AsyncOperation for Operation<'_, T> {
    fn get_result(&mut self) -> OperationResult {
        T::get_result(self.handle.0, &mut self.overlapped.0)
    }
}
