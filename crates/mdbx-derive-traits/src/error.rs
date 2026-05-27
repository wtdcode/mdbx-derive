use thiserror::Error;

#[cfg(all(feature = "serde_json", not(feature = "simd-json")))]
type JSONError = serde_json::Error;
#[cfg(feature = "simd-json")]
type JSONError = simd_json::Error;

#[derive(Error, Debug)]
pub enum MDBXDeriveError {
    #[error("corrputed")]
    Corrupted,
    #[error("incorrect schema")]
    IncorrectSchema(Vec<u8>),
    #[cfg(any(feature = "serde_json", feature = "simd-json"))]
    #[error("JSON: {0}")]
    JSON(#[from] JSONError),
    #[error("zstd: {0}")]
    Zstd(#[from] std::io::Error),
    #[error("postcard: {0}")]
    Postcard(#[from] postcard::Error),
    #[cfg(feature = "mdbx")]
    #[error("mdbx: {0}")]
    MDBX(#[from] libmdbx_remote::Error),
    #[cfg(feature = "mdbx")]
    #[error("mdbx-remote: {0}")]
    Client(libmdbx_remote::ClientError),
    #[error("bcs: {0}")]
    BCS(#[from] bcs::Error),
}

#[cfg(feature = "mdbx")]
impl From<libmdbx_remote::ClientError> for MDBXDeriveError {
    fn from(value: libmdbx_remote::ClientError) -> Self {
        match value {
            libmdbx_remote::ClientError::MDBX(e) => Self::MDBX(e),
            _ => Self::Client(value),
        }
    }
}
