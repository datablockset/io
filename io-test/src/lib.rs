use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::{self, Read, Seek, Write},
    iter::once,
    ops::Add,
    rc::Rc,
    str::from_utf8,
    time::Duration,
    vec,
};

use io_trait::{File, Io};

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
    fn len(&self) -> u64 {
        self.0.borrow().len() as u64
    }
    fn metadata(&self) -> Metadata {
        Metadata {
            len: self.len(),
            is_dir: false,
        }
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
            Entity::File(x) => x.metadata(),
        }
    }
}

#[derive(Debug, Default)]
pub struct FileSystem {
    entity_map: BTreeMap<String, Entity>,
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
            stdout: Default::default(),
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

impl File for MemFile {
    type Metadata = Metadata;
    fn metadata(&self) -> io::Result<Self::Metadata> {
        Ok(self.vec_ref.metadata())
    }
}

impl Seek for MemFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.pos = match pos {
            io::SeekFrom::Start(x) => x as usize,
            io::SeekFrom::End(x) => (self.vec_ref.len() as i64 + x) as usize,
            io::SeekFrom::Current(x) => (self.pos as i64 + x) as usize,
        };
        Ok(self.pos as u64)
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
        let pos = self.pos;
        let buf_len = buf.len();
        let end = pos + buf_len;
        {
            let mut v = self.vec_ref.0.borrow_mut();
            if end > v.len() {
                v.resize(end, 0);
            }
            v[pos..end].copy_from_slice(buf);
        }
        self.pos = end;
        Ok(buf_len)
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
        .all(|c| c.is_ascii_alphanumeric() || "/_.-".contains(c))
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
        let dir_end = path.ends_with('/');
        let path = if dir_end {
            &path[..path.len() - 1]
        } else {
            path
        };
        let result = fs
            .entity_map
            .get(path)
            .map(Entity::metadata)
            .ok_or_else(not_found)?;
        if !result.is_dir && dir_end {
            return Err(not_found());
        }
        Ok(result)
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
        *d = d.add(Duration::from_millis(1));
        result
    }

    fn current_dir(&self) -> io::Result<String> {
        Ok(String::default())
    }
    fn set_current_dir(&self, path: &str) -> io::Result<()> {
        if path.is_empty() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "directory not found",
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::{self, Seek, SeekFrom, Write};

    use io_trait::{DirEntry, File, Io, Metadata};
    use wasm_bindgen_test::wasm_bindgen_test;

    use crate::VirtualIo;

    #[wasm_bindgen_test]
    #[test]
    fn test() {
        fn check_len(m: &super::Metadata, f: fn(m: &super::Metadata) -> u64, len: u64) {
            assert_eq!(f(m), len);
        }
        let io = VirtualIo::new(&[]);
        io.write("test.txt", "Hello, world!".as_bytes()).unwrap();
        let result = io.read_to_string("test.txt").unwrap();
        assert_eq!(result, "Hello, world!");
        check_len(&io.metadata("test.txt").unwrap(), Metadata::len, 13);
        // assert_eq!(io.metadata("test.txt").unwrap().len(), 13);
        assert_eq!(io.current_dir().unwrap(), "");
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
        fn flush<W: Write>(w: &mut W, f: fn(&mut W) -> io::Result<()>) {
            f(w).unwrap();
        }
        let i = VirtualIo::new(&[]);
        {
            let mut f = i.create("test.txt").unwrap();
            f.write("Hello, world!".as_bytes()).unwrap();
            f.write("?".as_bytes()).unwrap();
            flush(&mut f, Write::flush);
            let m = f.metadata().unwrap();
            assert_eq!(m.len(), 14);
            assert!(!m.is_dir());
        }
        let result = i.read("test.txt").unwrap();
        assert_eq!(result, "Hello, world!?".as_bytes());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_write_seek() {
        let io = VirtualIo::new(&[]);
        {
            let mut f = io.create("test.txt").unwrap();
            f.write("Hello, world!".as_bytes()).unwrap();
            f.seek(SeekFrom::Start(7)).unwrap();
            f.write("there!".as_bytes()).unwrap();
            f.flush().unwrap();
            let m = f.metadata().unwrap();
            assert_eq!(m.len(), 13);
            assert!(!m.is_dir());
        }
        let result = io.read("test.txt").unwrap();
        assert_eq!(result, "Hello, there!".as_bytes());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_write_seek_current() {
        let io = VirtualIo::new(&[]);
        {
            let mut f = io.create("test.txt").unwrap();
            f.write("Hello, world!".as_bytes()).unwrap();
            f.seek(SeekFrom::Current(2)).unwrap();
            f.write("there".as_bytes()).unwrap();
            f.flush().unwrap();
            let m = f.metadata().unwrap();
            assert_eq!(m.len(), 20);
            assert!(!m.is_dir());
        }
        let result = io.read("test.txt").unwrap();
        assert_eq!(result, "Hello, world!\0\0there".as_bytes());
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_write_seek_end() {
        let io = VirtualIo::new(&[]);
        {
            let mut f = io.create("test.txt").unwrap();
            f.write("Hello, world!".as_bytes()).unwrap();
            f.seek(SeekFrom::End(-2)).unwrap();
            f.write("there".as_bytes()).unwrap();
            f.flush().unwrap();
            let m = f.metadata().unwrap();
            assert_eq!(m.len(), 16);
            assert!(!m.is_dir());
        }
        let result = io.read("test.txt").unwrap();
        assert_eq!(result, "Hello, worlthere".as_bytes());
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
        assert_eq!(io.now().as_millis(), 0);
        assert_eq!(io.now().as_millis(), 1);
    }

    fn check_len(m: &super::Metadata, f: fn(m: &super::Metadata) -> u64, len: u64) {
        assert_eq!(m.len(), len);
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_metadata() {
        let io = VirtualIo::new(&[]);
        io.write("test.txt", "Hello, world!".as_bytes()).unwrap();
        io.create_dir("a").unwrap();
        {
            let m = io.metadata("test.txt").unwrap();
            // assert_eq!(m.len(), 13);
            check_len(&m, super::Metadata::len, 13);
            assert!(!m.is_dir());
        }
        {
            io.metadata("test.txt/").unwrap_err();
        }
        {
            io.metadata("b").unwrap_err();
        }
        {
            let m = io.metadata("a").unwrap();
            assert!(m.is_dir());
        }
        {
            let m = io.metadata("a/").unwrap();
            assert!(m.is_dir());
        }
    }

    #[wasm_bindgen_test]
    #[test]
    fn test_set_current_dir() {
        let io = VirtualIo::new(&[]);
        assert!(io.set_current_dir("").is_ok());
        assert!(io.set_current_dir("a").is_err());
    }
}
