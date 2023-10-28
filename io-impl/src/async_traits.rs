pub trait AsyncTrait {
    type Handle;
    type Overlapped;
    fn close(handle: Self::Handle);
}