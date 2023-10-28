pub trait FileHandle {
    type Overlapped;
    fn close(&mut self);
}