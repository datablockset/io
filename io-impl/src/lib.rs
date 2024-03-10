#[cfg(any(target_family = "windows", target_family = "unix"))]
mod async_io;
mod async_traits;
mod unix;
mod windows;
mod windows_api;

use std::{
    env::{args, current_dir, Args},
    fs::{self, create_dir, File},
    io::{self, Stdout},
    time::Instant,
};

use io_trait::Io;

#[derive(Default)]
pub struct RealIo();

impl Io for RealIo {
    type Args = Args;

    type Stdout = Stdout;
    type File = File;
    type Metadata = fs::Metadata;
    type DirEntry = fs::DirEntry;
    type Instant = Instant;

    fn args(&self) -> Self::Args {
        args()
    }

    fn create(&self, path: &str) -> io::Result<Self::File> {
        File::create(path)
    }

    fn open(&self, path: &str) -> io::Result<Self::File> {
        File::open(path)
    }

    fn metadata(&self, path: &str) -> io::Result<fs::Metadata> {
        fs::metadata(path)
    }

    fn read_dir(&self, path: &str) -> io::Result<Vec<Self::DirEntry>> {
        fs::read_dir(path)?.collect()
    }

    fn create_dir(&self, path: &str) -> io::Result<()> {
        create_dir(path)
    }

    fn stdout(&self) -> Self::Stdout {
        io::stdout()
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn current_dir(&self) -> io::Result<String> {
        current_dir().map(|x| x.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod test {
    use std::{
        fs,
        io::{Read, Write},
    };

    use io_trait::{File, Io};

    #[test]
    fn test_arg() {
        let io = super::RealIo::default();
        let a = io.args().collect::<Vec<_>>();
        assert!(a.len() > 0);
    }

    #[test]
    fn test_file() {
        let io = super::RealIo::default();
        {
            let mut file = io.create("_test_file").unwrap();
            file.write_all(b"test").unwrap();
        }
        {
            let mut file = io.open("_test_file").unwrap();
            let mut buf = Vec::default();
            file.read_to_end(&mut buf).unwrap();
            let x = File::metadata(&file).unwrap();
            assert_eq!(x.len(), 4);
            assert!(x.is_file());
            assert_eq!(buf, b"test");
        }
        io.metadata("_test_file").unwrap();
        io.read_dir(".").unwrap();
        fs::remove_file("_test_file").unwrap();
        io.create_dir("_test_dir").unwrap();
        fs::remove_dir("_test_dir").unwrap();
        let _ = io.stdout();
    }

    #[test]
    fn test_now() {
        let io = super::RealIo::default();
        let _ = io.now();
    }
}
