use std::{ffi::CStr, io};

use io_trait::{AsyncFile, AsyncIo};

use crate::async_traits::{File, Operation, Overlapped};

#[cfg(target_family = "windows")]
use crate::windows::*;

#[cfg(target_family = "unix")]
use crate::unix::*;

struct AFile {
    file: File<Os>,
    overlapped: Overlapped<Os>,
}

impl AsyncFile for AFile {
    type Operation<'a> = Operation<'a, Os>;

    fn read<'a>(
        &'a mut self,
        offset: u64,
        buffer: &'a mut [u8],
    ) -> io::Result<Self::Operation<'a>> {
        self.file.read(&mut self.overlapped, offset, buffer)
    }

    fn write<'a>(&'a mut self, offset: u64, buffer: &'a [u8]) -> io::Result<Self::Operation<'a>> {
        self.file.write(&mut self.overlapped, offset, buffer)
    }
}

struct AIo();

impl AsyncIo for AIo {
    type File = AFile;

    fn create(&self, path: &CStr) -> io::Result<Self::File> {
        Ok(AFile {
            file: File::create(path)?,
            overlapped: Overlapped::default(),
        })
    }

    fn open(&self, path: &CStr) -> io::Result<Self::File> {
        Ok(AFile {
            file: File::open(path)?,
            overlapped: Default::default(),
        })
    }
}

#[cfg(test)]
mod test {
    use std::{ffi::CString, fs::remove_file, thread::yield_now};

    use super::AIo;
    use io_trait::{AsyncFile, AsyncIo, AsyncOperation, OperationResult};

    #[test]
    fn test() {
        let x: CString = CString::new("_test.txt").unwrap();
        let aio = AIo();
        //
        for _ in 0..1000 {
            let origin = b"Hello World!";
            {
                let mut handle = aio.create(&x).unwrap();
                let mut operation = handle.write(0, origin).unwrap();
                loop {
                    match operation.get_result() {
                        OperationResult::Ok(bytes_written) => {
                            if bytes_written != origin.len() {
                                panic!();
                            }
                            break;
                        }
                        OperationResult::Pending => {
                            yield_now();
                        }
                        OperationResult::Err(e) => {
                            panic!("e: {}", e);
                        }
                    }
                }
                // let result = operation.get_result(true).unwrap();
                // assert_eq!(result, 12);
            }
            {
                let mut handle = aio.open(&x).unwrap();
                let mut buffer = [0u8; 1024];
                {
                    let mut operation = handle.read(0, &mut buffer).unwrap();
                    loop {
                        match operation.get_result() {
                            OperationResult::Ok(bytes_written) => {
                                if bytes_written != origin.len() {
                                    panic!();
                                }
                                break;
                            }
                            OperationResult::Pending => {
                                yield_now();
                            }
                            OperationResult::Err(e) => {
                                panic!("e: {}", e);
                            }
                        }
                    }
                    // let result = operation.get_result(true).unwrap();
                    // assert_eq!(result, 12);
                }
                assert_eq!(&buffer[..12], b"Hello World!");
            }
        }
        remove_file(x.to_str().unwrap()).unwrap();
    }

    #[test]
    fn test2() {
        let aio = AIo();
        let x: CString = CString::new("_test2.txt").unwrap();
        let origin = "Hello, world!";
        for _ in 0..1000 {
            {
                let mut file = aio.create(&x).unwrap();
                let mut operation = file.write(0, origin.as_bytes()).unwrap();
                loop {
                    match operation.get_result() {
                        OperationResult::Ok(bytes_written) => {
                            if bytes_written != origin.len() {
                                panic!();
                            }
                            break;
                        }
                        OperationResult::Pending => {
                            yield_now();
                        }
                        OperationResult::Err(e) => {
                            panic!("e: {}", e);
                        }
                    }
                }
            }
        }
        for _ in 0..1000 {
            {
                let mut file = aio.open(&x).unwrap();
                let mut buffer = [0u8; 1024];
                let mut len = 0;
                {
                    let mut operation = file.read(0, &mut buffer).unwrap();
                    loop {
                        match operation.get_result() {
                            OperationResult::Ok(bytes_read) => {
                                len = bytes_read;
                                break;
                            }
                            OperationResult::Pending => {
                                yield_now();
                            }
                            OperationResult::Err(e) => {
                                panic!("e: {}", e);
                            }
                        }
                    }
                }
                assert_eq!(&buffer[..len], origin.as_bytes());
            }
        }
        remove_file(x.to_str().unwrap()).unwrap();
    }

    #[test]
    fn test3() {
        let aio = AIo();
        let x: CString = CString::new("_big_test.txt").unwrap();
        let origin = "Hello, world!".repeat(100);
        {
            let mut file = aio.create(&x).unwrap();
            let mut operation = file.write(0, origin.as_bytes()).unwrap();
            loop {
                match operation.get_result() {
                    OperationResult::Ok(bytes_written) => {
                        if bytes_written != origin.len() {
                            panic!();
                        }
                        break;
                    }
                    OperationResult::Pending => {
                        yield_now();
                    }
                    OperationResult::Err(e) => {
                        panic!("e: {}", e);
                    }
                }
            }
        }
        {
            let mut file = aio.open(&x).unwrap();
            let mut v = Vec::default();
            loop {
                let mut buffer = [0u8; 1024];
                let mut len: usize = 0;
                {
                    let mut operation = file.read(v.len() as u64, &mut buffer).unwrap();
                    loop {
                        match operation.get_result() {
                            OperationResult::Ok(bytes_read) => {
                                len = bytes_read;
                                break;
                            }
                            OperationResult::Pending => {
                                yield_now();
                            }
                            OperationResult::Err(e) => {
                                panic!("e: {}", e);
                            }
                        }
                    }
                }
                if len == 0 {
                    break;
                }
                v.extend_from_slice(&buffer[..len]);
            }
            assert_eq!(&v, origin.as_bytes());
        }
        remove_file(x.to_str().unwrap()).unwrap();
    }
}
