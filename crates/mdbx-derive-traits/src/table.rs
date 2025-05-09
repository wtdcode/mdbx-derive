use std::borrow::Cow;

use crate::error::MDBXDeriveError;

pub trait TableObjectEncode {
    fn table_encode(&self) -> Result<Vec<u8>, MDBXDeriveError>;
}

pub trait TableObjectDecode: Sized {
    fn table_decode(val: &[u8]) -> Result<Self, MDBXDeriveError>;
}

impl TableObjectEncode for Vec<u8> {
    fn table_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.clone())
    }
}

impl TableObjectDecode for Vec<u8> {
    fn table_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
        Ok(val.to_vec())
    }
}

impl TableObjectEncode for Cow<'_, [u8]> {
    fn table_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.clone().into_owned())
    }
}

impl TableObjectDecode for Cow<'_, [u8]> {
    fn table_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
        Ok(Self::Owned(val.to_vec()))
    }
}
