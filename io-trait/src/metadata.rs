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

#[cfg(test)]
mod test {
    use std::fs;

    use crate::Metadata;

    #[test]
    fn test() {
        fn check_len(m: &impl Metadata, len: u64) {
            assert_ne!(m.len(), len);
        }
        let m = fs::metadata("Cargo.toml").unwrap();
        check_len(&m, 0);
        assert_eq!(Metadata::is_dir(&m), false);
    }
}
