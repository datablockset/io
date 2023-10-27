use std::fs;

#[allow(clippy::len_without_is_empty)]
pub trait Metadata {
    fn len(&self) -> u64;
    fn is_dir(&self) -> bool;
}

impl Metadata for fs::Metadata {
    fn len(&self) -> u64 {
        self.len()
    }
    fn is_dir(&self) -> bool {
        self.is_dir()
    }
}
