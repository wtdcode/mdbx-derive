use alloy_primitives::aliases::U24;
use alloy_primitives::{Address, B256, U8, U16, U64, U128, U160, U256};

use crate::error::MDBXDeriveError;
use crate::key::{KeyObjectDecode, KeyObjectEncode};

impl KeyObjectEncode for Address {
    fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.iter().copied().collect())
    }
}

impl KeyObjectDecode for Address {
    fn key_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
        Ok(Address::new(
            val.try_into().map_err(|_| MDBXDeriveError::Corrupted)?,
        ))
    }
}

impl KeyObjectEncode for B256 {
    fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.iter().copied().collect())
    }
}

impl KeyObjectDecode for B256 {
    fn key_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
        Ok(B256::new(
            val.try_into().map_err(|_| MDBXDeriveError::Corrupted)?,
        ))
    }
}

macro_rules! impl_alloy {
    ( $( $name:ident )+ ) => {
        $(
            impl KeyObjectEncode for $name {
                fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
                    // This converts to exactly Uint::BYTES
                    Ok(self.to_be_bytes_vec())
                }
            }
            impl KeyObjectDecode for $name {
                const KEYSIZE: usize = $name::BYTES;
                fn key_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
                    Ok($name::from_be_bytes::<{$name::BYTES}>(val.try_into().map_err(|_| MDBXDeriveError::Corrupted)?))
                }
            }
        )+
    };
}

impl_alloy! { U256 U160 U128 U64 U24 U16 U8 }
