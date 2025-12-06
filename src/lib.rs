pub mod memory;
pub mod process;
pub mod runtime;

use elf_loader::Error as ElfError;
use std::io::Error as IOError;
use std::result::Result as LoaderResult;

#[derive(Debug)]
pub enum LoaderError {
    Elf(ElfError),
    Io(IOError),
    Mmap(String),
    Symbol(String),
}

impl From<ElfError> for LoaderError {
    fn from(e: ElfError) -> Self {
        LoaderError::Elf(e)
    }
}

impl From<IOError> for LoaderError {
    fn from(e: IOError) -> Self {
        LoaderError::Io(e)
    }
}
