//! Traits for format-agnostic parsing and serialization in langcodec.

use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Cursor, Write},
    path::Path,
};

use crate::error::Error;

/// A trait for parsing and writing localization resources from/to one file.
///
/// # Example
///
/// ```rust,no_run
/// use langcodec::traits::Parser;
/// let format = langcodec::formats::strings::Format::read_from("en.strings")?;
/// format.write_to("en_copy.strings")?;
/// Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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
    fn to_writer<W: Write>(&self, writer: W) -> Result<(), Error>;

    /// Write to file path.
    fn write_to<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.to_writer(writer)
    }

    /// Parse from a string.
    fn from_str(s: &str) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::from_reader(Cursor::new(s))
    }

    /// Parse from bytes.
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Self::from_reader(Cursor::new(bytes))
    }
}
