use core::fmt;
use std::{
    fs,
    io::{self, Read, Seek, Write},
};

use crate::Metadata;

pub trait File: Read + Write + Seek + fmt::Debug {
    type Metadata: Metadata;
    fn metadata(&self) -> io::Result<Self::Metadata>;
}

impl File for fs::File {
    type Metadata = fs::Metadata;
    fn metadata(&self) -> io::Result<Self::Metadata> {
        fs::File::metadata(self)
    }
}
