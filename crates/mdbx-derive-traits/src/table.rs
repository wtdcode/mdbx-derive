use std::borrow::Cow;

use libmdbx_remote::TableObject;

use crate::error::MDBXDeriveError;

pub trait TableObjectEncode {
    fn table_encode(&self) -> Result<Vec<u8>, MDBXDeriveError>;
}

pub trait TableObjectDecode: Sized {
    fn table_decode(val: &[u8]) -> Result<Self, MDBXDeriveError>;
}

impl<T> TableObjectDecode for T
where
    T: TableObject,
{
    fn table_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
        Self::decode(val).map_err(|e| MDBXDeriveError::MDBX(e))
    }
}

impl TableObjectEncode for Vec<u8> {
    fn table_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.clone())
    }
}

impl TableObjectEncode for Cow<'_, [u8]> {
    fn table_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.clone().into_owned())
    }
}
