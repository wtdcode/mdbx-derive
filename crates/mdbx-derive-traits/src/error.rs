use thiserror::Error;

#[cfg(feature = "serde_json")]
type JSONError = serde_json::Error;
#[cfg(feature = "simd-json")]
type JSONError = simd_json::Error;
#[cfg(all(not(feature = "simd-json"), not(feature = "serde_json")))]
type JSONError = String;

#[derive(Error, Debug)]
pub enum MDBXDeriveError {
    #[error("corrputed")]
    Corrupted,
    #[error("incorrect schema")]
    IncorrectSchema(Vec<u8>),
    #[cfg(any(feature = "serde_json", feature="simd-json"))]
    #[error("JSON: {0}")]
    JSON(#[from] JSONError),
    #[error("zstd: {0}")]
    Zstd(#[from] std::io::Error),
    #[error("bincode encode: {0}")]
    BincodeEncode(#[from] bincode::error::EncodeError),
    #[error("bincode decode: {0}")]
    BincodeDecode(#[from] bincode::error::DecodeError),
    #[error("mdbx: {0}")]
    MDBX(#[from] libmdbx_remote::Error),
    #[error("mdbx-remote: {0}")]
    Client(libmdbx_remote::ClientError),
    #[error("bcs: {0}")]
    BCS(#[from] bcs::Error),
}

impl From<libmdbx_remote::ClientError> for MDBXDeriveError {
    fn from(value: libmdbx_remote::ClientError) -> Self {
        match value {
            libmdbx_remote::ClientError::MDBX(e) => Self::MDBX(e),
            _ => Self::Client(value),
        }
    }
}
