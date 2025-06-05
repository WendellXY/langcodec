use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("unknown format `{0}`")]
    UnknownFormat(String),

    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("XML parse error: {0}")]
    XmlParse(#[from] quick_xml::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid data: {0}")]
    DataMismatch(String),

    #[error("invalid resource: {0}")]
    InvalidResource(String),
}
