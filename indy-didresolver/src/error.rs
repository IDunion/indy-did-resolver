use thiserror::Error;
use indy_vdr::common::error::VdrError;


pub type DidIndyResult<T> = std::result::Result<T, DidIndyError>;

#[derive(Debug, Error)]
pub enum DidIndyError {
    #[error("Parsing error")]
    Parsing(#[from] serde_json::Error),
    #[error("Namespace not supported")]
    NamespaceNotSupported,
    #[error("Object not found")]
    NotFound,
    #[error("VDR error")]
    VDR(#[from] VdrError ),
    #[error("Unkown error")]
    Unknown,
}

// impl fmt::Display for DidIndyError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "DID Indy Error")
//     }
// }
