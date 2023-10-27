mod dir_entry;
mod metadata;
mod async_io;

pub use async_io::*;
pub use dir_entry::DirEntry;
pub use metadata::Metadata;

use std::{
    fmt, fs,
    io::{self, Read, Write},
};

pub trait Io {
    type Args: Iterator<Item = String>;
    type File: Read + Write + fmt::Debug;
    type Stdout: Write;
    type Metadata: Metadata;
    type DirEntry: DirEntry;
    fn args(&self) -> Self::Args;
    fn stdout(&self) -> Self::Stdout;
    fn metadata(&self, path: &str) -> io::Result<Self::Metadata>;
    fn create_dir(&self, path: &str) -> io::Result<()>;
    fn create(&self, path: &str) -> io::Result<Self::File>;
    fn open(&self, path: &str) -> io::Result<Self::File>;
    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        let mut file = self.open(path)?;
        let mut result = Vec::default();
        file.read_to_end(&mut result)?;
        Ok(result)
    }
    fn read_dir(&self, path: &str) -> io::Result<Vec<Self::DirEntry>>;
    fn read_to_string(&self, path: &str) -> io::Result<String> {
        let mut file = self.open(path)?;
        let mut result = String::default();
        file.read_to_string(&mut result)?;
        Ok(result)
    }
    fn write(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let mut file = self.create(path)?;
        file.write_all(data)?;
        Ok(())
    }
    fn create_dir_recursively(&self, path: &str) -> io::Result<()> {
        let mut x = String::default();
        let mut e = Ok(());
        for i in path.split('/') {
            x += i;
            e = self.create_dir(&x);
            x += "/";
        }
        e
    }
    fn write_recursively(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let e = self.write(path, data);
        if let Err(er) = e {
            return if let Some((p, _)) = path.rsplit_once('/') {
                self.create_dir_recursively(p)?;
                self.write(path, data)
            } else {
                Err(er)
            };
        }
        Ok(())
    }
    fn read_dir_type(&self, path: &str, is_dir: bool) -> io::Result<Vec<Self::DirEntry>> {
        let mut result = Vec::default();
        for i in self.read_dir(path)? {
            if i.metadata()?.is_dir() == is_dir {
                result.push(i);
            }
        }
        Ok(result)
    }
}

impl DirEntry for fs::DirEntry {
    type Metadata = fs::Metadata;
    fn path(&self) -> String {
        self.path().to_str().unwrap().to_string()
    }
    fn metadata(&self) -> io::Result<Self::Metadata> {
        self.metadata()
    }
}
