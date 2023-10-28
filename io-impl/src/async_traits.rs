pub trait AsyncTrait {
    type Handle;
    type Overlapped;
    fn close(handle: Self::Handle);
    fn cancel(handle: Self::Handle, overlapped: &mut Self::Overlapped);
}
