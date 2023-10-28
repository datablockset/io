use std::{ffi::CStr, io};

use io_trait::OperationResult;

pub trait AsyncTrait {
    type Handle;
    type Overlapped;
    fn close(handle: Self::Handle);
    fn cancel(handle: Self::Handle, overlapped: &mut Self::Overlapped);
    fn get_result(handle: Self::Handle, overlapped: &mut Self::Overlapped) -> OperationResult;
    fn open(path: &CStr, create: bool) -> io::Result<Self::Handle>;
}
