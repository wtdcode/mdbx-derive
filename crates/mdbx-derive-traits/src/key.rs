use crate::error::MDBXDeriveError;

pub trait KeyObjectEncode {
    fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError>;
}

// https://github.com/rust-lang/rust/issues/60551
// Not stablized yet, so we can't have key_decode(val: [u8; Self::KEYSIZE])
pub trait KeyObjectDecode: Sized {
    const KEYSIZE: usize = size_of::<Self>();
    fn key_decode(val: &[u8]) -> Result<Self, MDBXDeriveError>;
}

impl KeyObjectEncode for &str {
    fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.as_bytes().to_vec())
    }
}

impl KeyObjectEncode for &[u8] {
    fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.to_vec())
    }
}

impl KeyObjectEncode for Vec<u8> {
    fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.clone())
    }
}

impl<const N: usize> KeyObjectEncode for [u8; N] {
    fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
        Ok(self.to_vec())
    }
}

impl<const N: usize> KeyObjectDecode for [u8; N] {
    const KEYSIZE: usize = N;
    fn key_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
        val.try_into().map_err(|_| MDBXDeriveError::Corrupted)
    }
}

macro_rules! impl_ints {
    ( $( $name:ident )+ ) => {
        $(
            impl KeyObjectEncode for $name {
                fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
                    Ok(self.to_be_bytes().into_iter().collect())
                }
            }

            impl KeyObjectDecode for $name {
                fn key_decode(val: &[u8]) -> Result<Self, MDBXDeriveError> {
                    Ok($name::from_be_bytes(val.try_into().map_err(|_| MDBXDeriveError::Corrupted)?))
                }
            }
        )+
    };
}

impl_ints! { u8 u16 u32 u64 i8 i16 i32 i64 }

macro_rules! tuple_impls {
    ( $( $name:ident )+ ) => {
        impl<$($name: KeyObjectEncode),+> KeyObjectEncode for ($(&$name,)+)
        {
            fn key_encode(&self) -> Result<Vec<u8>, MDBXDeriveError> {
                let ($($name,)+) = self;
                Ok( std::iter::empty()$(.chain($name.key_encode()?.into_iter()))*.collect() )
            }
        }
    };
}

tuple_impls! { A }
tuple_impls! { A B }
tuple_impls! { A B C }
tuple_impls! { A B C D }
tuple_impls! { A B C D E }
tuple_impls! { A B C D E F }
tuple_impls! { A B C D E F G }
tuple_impls! { A B C D E F G H }
tuple_impls! { A B C D E F G H I }
tuple_impls! { A B C D E F G H I J }
tuple_impls! { A B C D E F G H I J K }
tuple_impls! { A B C D E F G H I J K L }
