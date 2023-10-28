use crate::async_trait;

#[cfg(target_family = "windows")]
use crate::windows::*;

#[cfg(target_family = "unix")]
use crate::unix::*;

pub type File = async_trait::File<TargetFamily>;

#[cfg(test)]
mod test {
    use std::{ffi::CString, fs::remove_file, thread::yield_now};

    use io_trait::{AsyncFile, AsyncOperation, OperationResult};

    use super::File;

    #[test]
    fn test() {
        let x: CString = CString::new("_test.txt").unwrap();
        //
        for _ in 0..1000 {
            let origin = b"Hello World!";
            {
                let mut handle = File::create(&x).unwrap();
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
            }
            {
                let mut handle = File::open(&x).unwrap();
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
                }
                assert_eq!(&buffer[..12], b"Hello World!");
            }
        }
        remove_file(x.to_str().unwrap()).unwrap();
    }

    #[test]
    fn test2() {
        let x: CString = CString::new("_test2.txt").unwrap();
        let origin = "Hello, world!";
        for _ in 0..1000 {
            {
                let mut file = File::create(&x).unwrap();
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
                let mut file = File::open(&x).unwrap();
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
        let x: CString = CString::new("_big_test.txt").unwrap();
        let origin = "Hello, world!".repeat(100);
        {
            let mut file = File::create(&x).unwrap();
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
            let mut file = File::open(&x).unwrap();
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
