use std::error::Error as StdError;
use std::fs::File;
use std::path::Path;
use std::io::Read;

#[derive(Debug)]
pub struct Fat16Fs {
    body: Vec<u8>,
}

impl Fat16Fs {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Fat16Fs, Box<dyn StdError>> {
        let mut file = File::open(path).unwrap();
        let mut body = Vec::new();
        file.read_to_end(&mut body).unwrap();
        Ok(Fat16Fs { body })
    }
}
