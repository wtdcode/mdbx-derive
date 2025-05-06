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

impl<'a> TableObjectEncode for Cow<'a, [u8]> {
    fn table_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.clone().into_owned())
    }
}

impl<'a> TableObjectDecode for Cow<'a, [u8]> {
    fn table_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
        Ok(Self::Owned(val.to_vec()))
    }
}
