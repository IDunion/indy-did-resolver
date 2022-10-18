use indy_vdr::common::error::VdrError;
use thiserror::Error;

pub type DidIndyResult<T> = std::result::Result<T, DidIndyError>;

#[derive(Debug, Error)]
pub enum DidIndyError {
    #[error("Parsing error")]
    ParsingError(#[from] serde_json::Error),
    #[error("Could not parse datetime")]
    DateTimeError(#[from] chrono::ParseError),
    #[error("Namespace not supported")]
    NamespaceNotSupported,
    #[error("Query parameter not supported")]
    QueryParameterNotSupported,
    #[error("Empty data")]
    EmptyData,
    #[error("Invalid DID URL")]
    InvalidDidUrl,
    #[error("Invalid DID Document")]
    InvalidDidDoc,
    #[error("Object family not supported")]
    ObjectFamilyNotSupported,
    #[error("Object family version not supported")]
    VersionNotSupported,
    #[error("Object type not supported")]
    ObjectTypeNotSuported,
    #[error("Object not found")]
    NotFound,
    #[error("Function not implemented")]
    NotImplemented,
    #[error("VDR error")]
    VdrError(#[from] VdrError),
    #[error("Base58 Parsing error")]
    FromBase58Error(#[from] bs58::decode::Error),
    #[error("Unexpected Key Format")]
    UnexpectedKeyFormat
}

// impl fmt::Display for DidIndyError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "DID Indy Error")
//     }
// }
