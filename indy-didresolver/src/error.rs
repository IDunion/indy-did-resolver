use std::fmt;

pub type DidIndyResult<T> = std::result::Result<T, DidIndyError>;

#[derive(Debug, Clone)]
pub struct DidIndyError;

impl fmt::Display for DidIndyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DID Indy Error")
    }
}
