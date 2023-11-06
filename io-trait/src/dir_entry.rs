use std::{fs, io};

use crate::Metadata;

pub trait DirEntry {
    type Metadata: Metadata;
    fn path(&self) -> String;
    fn metadata(&self) -> io::Result<Self::Metadata>;
    fn file_name(&self) -> String;
}

impl DirEntry for fs::DirEntry {
    type Metadata = fs::Metadata;
    fn path(&self) -> String {
        self.path().to_str().unwrap().to_string()
    }
    fn metadata(&self) -> io::Result<Self::Metadata> {
        self.metadata()
    }
    fn file_name(&self) -> String {
        self.file_name().into_string().unwrap()
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use crate::DirEntry;

    #[test]
    fn test() {
        let x = fs::read_dir(".").unwrap();
        for i in x {
            let i = i.unwrap();
            assert_eq!(DirEntry::path(&i), i.path().to_str().unwrap());
            assert_eq!(DirEntry::file_name(&i), i.file_name().to_str().unwrap());
            assert_eq!(
                DirEntry::metadata(&i).unwrap().is_dir(),
                i.metadata().unwrap().is_dir()
            );
        }
    }
}
