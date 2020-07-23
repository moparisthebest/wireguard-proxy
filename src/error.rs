use core::result;

#[cfg(not(any(feature = "async")))]
pub type IoResult<T> = result::Result<T, std::io::Error>;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub fn new(msg: &str) -> Error {
        Error(msg.to_owned())
    }
    pub fn new_owned(msg: String) -> Error {
        Error(msg)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        &self.0
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::new(&format!("{}", value))
    }
}

/*
impl From<std::option::NoneError> for Error {
    fn from(value: std::option::NoneError) -> Self {
        Error::new(value.description())
    }
}
*/