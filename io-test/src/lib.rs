use std::{
    cell::RefCell,
    collections::HashMap,
    io::{self, Read, Write},
    iter::once,
    ops::Add,
    rc::Rc,
    time::Duration,
    vec, str::from_utf8,
};

use io_trait::Io;

#[derive(Debug, Clone)]
pub struct Metadata {
    len: u64,
    is_dir: bool,
}

impl io_trait::Metadata for Metadata {
    fn len(&self) -> u64 {
        self.len
    }
    fn is_dir(&self) -> bool {
        self.is_dir
    }
}

#[derive(Debug, Default, Clone)]
pub struct VecRef(Rc<RefCell<Vec<u8>>>);

impl VecRef {
    pub fn to_stdout(&self) -> String {
        let mut result = Vec::default();
        let mut i = 0;
        for &c in self.0.borrow().iter() {
            if c == 8 {
                i -= 1;
            } else {
                if i < result.len() {
                    result[i] = c;
                } else {
                    result.push(c);
                }
                i += 1;
            }
        }
        from_utf8(&result).unwrap().to_string()
    }
}

impl Write for VecRef {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Default)]
enum Entity {
    #[default]
    Dir,
    File(VecRef),
}

impl Entity {
    fn metadata(&self) -> Metadata {
        match self {
            Entity::Dir => Metadata {
                len: 0,
                is_dir: true,
            },
            Entity::File(x) => Metadata {
                len: x.0.borrow().len() as u64,
                is_dir: false,
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct FileSystem {
    entity_map: HashMap<String, Entity>,
}

impl FileSystem {
    pub fn check_dir(&self, path: &str) -> io::Result<()> {
        if let Some(Entity::Dir) = self.entity_map.get(path) {
            Ok(())
        } else {
            Err(not_found())
        }
    }
    pub fn check_parent(&self, path: &str) -> io::Result<()> {
        if let Some(d) = path.rfind('/').map(|i| &path[..i]) {
            self.check_dir(d)
        } else {
            Ok(())
        }
    }
}

pub struct DirEntry {
    path: String,
    metadata: Metadata,
}

impl io_trait::DirEntry for DirEntry {
    type Metadata = Metadata;
    fn path(&self) -> String {
        self.path.clone()
    }
    fn metadata(&self) -> io::Result<Self::Metadata> {
        Ok(self.metadata.clone())
    }
}

pub struct VirtualIo {
    pub args: Vec<String>,
    pub fs: RefCell<FileSystem>,
    pub stdout: VecRef,
    pub duration: RefCell<Duration>,
}

impl VirtualIo {
    pub fn new(args: &[&str]) -> Self {
        Self {
            args: once("blockset".to_string())
                .chain(args.iter().map(|v| v.to_string()))
                .collect(),
            fs: Default::default(),
            stdout: VecRef::default(),
            duration: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct MemFile {
    vec_ref: VecRef,
    pos: usize,
}

impl MemFile {
    fn new(vec_ref: VecRef) -> Self {
        Self { vec_ref, pos: 0 }
    }
}

impl Read for MemFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let source = &self.vec_ref.0.borrow()[self.pos..];
        let len = source.len().min(buf.len());
        buf[..len].copy_from_slice(&source[..len]);
        self.pos += len;
        Ok(len)
    }
}

impl Write for MemFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.vec_ref.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.vec_ref.flush()
    }
}

fn not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "file not found")
}

fn check_path(a: &str) -> io::Result<()> {
    if a.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '.')
    {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid file name",
        ))
    }
}

impl Io for VirtualIo {
    type File = MemFile;
    type Stdout = VecRef;
    type Args = vec::IntoIter<String>;
    type Metadata = Metadata;
    type DirEntry = DirEntry;
    type Instant = Duration;
    fn args(&self) -> Self::Args {
        self.args.clone().into_iter()
    }
    fn metadata(&self, path: &str) -> io::Result<Metadata> {
        let fs = self.fs.borrow();
        fs.entity_map
            .get(path)
            .map(Entity::metadata)
            .ok_or_else(not_found)
    }
    fn create(&self, path: &str) -> io::Result<Self::File> {
        let mut fs = self.fs.borrow_mut();
        fs.check_parent(path)?;
        let vec_ref = VecRef::default();
        check_path(path)?;
        fs.entity_map
            .insert(path.to_string(), Entity::File(vec_ref.clone()));
        Ok(MemFile::new(vec_ref))
    }
    fn create_dir(&self, path: &str) -> io::Result<()> {
        let mut fs = self.fs.borrow_mut();
        fs.entity_map.insert(path.to_string(), Entity::Dir);
        Ok(())
    }
    fn open(&self, path: &str) -> io::Result<Self::File> {
        let fs = self.fs.borrow();
        fs.check_parent(path)?;
        check_path(path)?;
        fs.entity_map
            .get(path)
            .and_then(|v| {
                if let Entity::File(x) = v {
                    Some(MemFile::new(x.to_owned()))
                } else {
                    None
                }
            })
            .ok_or_else(not_found)
    }
    fn stdout(&self) -> VecRef {
        self.stdout.clone()
    }

    fn read_dir(&self, path: &str) -> io::Result<Vec<DirEntry>> {
        let fs = self.fs.borrow();
        fs.check_dir(path)?;
        let i = fs.entity_map.iter().map(|(p, e)| DirEntry {
            path: p.to_owned(),
            metadata: e.metadata(),
        });
        let x = i
            .filter(|p| {
                if let Some((a, _)) = p.path.rsplit_once('/') {
                    a == path
                } else {
                    false
                }
            })
            .collect();
        Ok(x)
    }

    fn now(&self) -> Self::Instant {
        let mut d = self.duration.borrow_mut();
        let result = *d;
        *d = d.add(Duration::from_secs(1));
        result
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;

    use io_trait::{DirEntry, Io, Metadata};
    use wasm_bindgen_test::wasm_bindgen_test;

    use crate::VirtualIo;

    #[wasm_bindgen_test]
    #[test]
    fn test() {
        let io = VirtualIo::new(&[]);
        io.write("test.txt", "Hello, world!".as_bytes()).unwrap();
        let result = io.read_to_string("test.txt").unwrap();
        assert_eq!(result, "Hello, world!");
        assert_eq!(io.metadata("test.txt").unwrap().len(), 13);
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_args() {
        let mut io = VirtualIo::new(&[]);
        io.args = ["a".to_string(), "b".to_string()].to_vec();
        let x = io.args().collect::<Vec<_>>();
        assert_eq!(&x, &["a", "b"]);
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_stdout() {
        {
            let io = VirtualIo::new(&[]);
            let mut s = io.stdout();
            s.write(b"Hello, world!\x08?").unwrap();
            assert_eq!(s.to_stdout(), "Hello, world?");
        }
        {
            let io = VirtualIo::new(&[]);
            let mut s = io.stdout();
            s.write(b"Hello, world!\x08\x08?").unwrap();
            assert_eq!(s.to_stdout(), "Hello, worl?!");
        }
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_write() {
        let io = VirtualIo::new(&[]);
        io.write("test.txt", "Hello, world!".as_bytes()).unwrap();
        let result = io.read("test.txt").unwrap();
        assert_eq!(result, "Hello, world!".as_bytes());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_write_file() {
        let io = VirtualIo::new(&[]);
        {
            let mut f = io.create("test.txt").unwrap();
            f.write("Hello, world!".as_bytes()).unwrap();
            f.flush().unwrap();
        }
        let result = io.read("test.txt").unwrap();
        assert_eq!(result, "Hello, world!".as_bytes());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_dir_fail() {
        let io = VirtualIo::new(&[]);
        assert!(io.write("a/test.txt", "Hello, world!".as_bytes()).is_err());
        assert!(io.open("a").is_err());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_write_recursively() {
        let io = VirtualIo::new(&[]);
        assert!(io
            .write_recursively("a/test.txt", "Hello, world!".as_bytes())
            .is_ok());
        assert!(io
            .write_recursively("a/test2.txt", "Hello, world!".as_bytes())
            .is_ok());
        assert!(io.open("a").is_err());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_dir_rec() {
        let io = VirtualIo::new(&[]);
        assert!(io
            .write_recursively("a/b/test.txt", "Hello, world!".as_bytes())
            .is_ok());
        let x = io
            .read_dir_type("a", true)
            .unwrap()
            .iter()
            .map(|v| v.path().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(x, ["a/b"].to_vec());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_err() {
        let io = VirtualIo::new(&[]);
        assert!(io
            .write_recursively("?", "Hello, world!".as_bytes())
            .is_err());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_now() {
        let io = VirtualIo::new(&[]);
        assert_eq!(io.now().as_secs(), 0);
        assert_eq!(io.now().as_secs(), 1);
    }
}
