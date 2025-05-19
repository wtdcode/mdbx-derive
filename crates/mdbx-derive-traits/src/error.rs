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
    #[error("JSON: {0}")]
    JSON(JSONError),
    #[error("zstd: {0}")]
    Zstd(std::io::Error),
    #[error("bincode encode: {0}")]
    BincodeEncode(bincode::error::EncodeError),
    #[error("bincode decode: {0}")]
    BincodeDecode(bincode::error::DecodeError),
    #[error("mdbx: {0}")]
    MDBX(libmdbx_remote::Error),
}

#[cfg(feature = "serde_json")]
impl From<serde_json::Error> for MDBXDeriveError {
    fn from(value: serde_json::Error) -> Self {
        Self::JSON(value)
    }
}

#[cfg(feature = "simd-json")]
impl From<simd_json::Error> for MDBXDeriveError {
    fn from(value: simd_json::Error) -> Self {
        Self::JSON(value)
    }
}

#[cfg(all(not(feature = "simd-json"), not(feature = "serde_json")))]
impl From<String> for MDBXDeriveError {
    fn from(value: String) -> Self {
        Self::JSON(value)
    }
}

impl From<bincode::error::EncodeError> for MDBXDeriveError {
    fn from(value: bincode::error::EncodeError) -> Self {
        Self::BincodeEncode(value)
    }
}

impl From<bincode::error::DecodeError> for MDBXDeriveError {
    fn from(value: bincode::error::DecodeError) -> Self {
        Self::BincodeDecode(value)
    }
}
