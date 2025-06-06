use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter},
    path::Path,
};

use crate::error::Error;

/// A trait for parsing and writing localization resources from/to one file.
pub trait Parser {
    /// Parse from any reader.
    fn from_reader<R: BufRead>(reader: R) -> Result<Self, Error>
    where
        Self: Sized;

    /// Parse from file path.
    fn read_from<P: AsRef<Path>>(path: P) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let file = File::open(path).map_err(Error::Io)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Write to any writer (file, memory, etc.).
    fn to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), Error>;

    /// Write to file path.
    fn write_to<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.to_writer(writer)
    }
}
