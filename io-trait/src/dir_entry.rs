use std::io;

use crate::Metadata;

pub trait DirEntry {
    type Metadata: Metadata;
    fn path(&self) -> String;
    fn metadata(&self) -> io::Result<Self::Metadata>;
}
