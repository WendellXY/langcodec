use crate::{error::Error, types::Resource};

pub trait ResourceConvertible {
    fn to_resource(&self) -> Result<Resource, Error>;

    fn from_resource(resource: &Resource) -> Result<Self, Error>
    where
        Self: Sized;
}

pub trait MultiResourceConvertible {
    fn to_resources(&self) -> Result<Vec<Resource>, Error>;
    fn from_resources(resources: &[Resource]) -> Result<Self, Error>
    where
        Self: Sized;
}

pub trait Parser {
    /// Parse from any reader.
    fn from_reader<R: std::io::BufRead>(reader: R) -> Result<Self, Error>
    where
        Self: Sized;

    /// Parse from file path.
    fn read_from<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let file = std::fs::File::open(path).map_err(Error::Io)?;
        let reader = std::io::BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Write to any writer (file, memory, etc.).
    fn to_writer<W: std::io::Write>(&self, writer: W) -> Result<(), Error>;

    /// Write to file path.
    fn write_to<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), Error> {
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        self.to_writer(writer)
    }
}
