use std::fmt;

#[derive(Debug)]
pub enum Error {
    Pixiv(String),
    NoData,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Pixiv(message) => write!(f, "{}", message),
            Error::NoData => write!(f, "response has no data"),
        }
    }
}
