use thiserror::Error;

#[derive(Error, Debug)]
pub enum MDBXDeriveError {
    #[error("corrputed")]
    Corrupted,
    #[error("json: {0}")]
    JSON(serde_json::Error),
    #[error("zstd: {0}")]
    Zstd(std::io::Error),
    #[error("bincode encode: {0}")]
    BincodeEncode(bincode::error::EncodeError),
    #[error("bincode decode: {0}")]
    BincodeDecode(bincode::error::DecodeError),
    #[error("mdbx: {0}")]
    MDBX(libmdbx_remote::Error),
}

impl From<serde_json::Error> for MDBXDeriveError {
    fn from(value: serde_json::Error) -> Self {
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
