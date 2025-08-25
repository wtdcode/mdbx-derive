pub use mdbx_derive_macros::*;
pub use mdbx_derive_traits::error::MDBXDeriveError as Error;
pub use mdbx_derive_traits::key::{KeyObjectDecode, KeyObjectEncode};
pub use mdbx_derive_traits::{
    orm::{HasMDBXEnvironment, MDBXDatabase, MDBXTable},
    table::{TableObjectDecode, TableObjectEncode},
    {mdbx_database, mdbx_table},
};

pub use tuple_list::{tuple_list, tuple_list_type};

pub mod zstd {
    pub use zstd::{decode_all, encode_all};
}

#[cfg(feature = "serde_json")]
pub mod json {
    pub use serde_json::to_vec;
    pub fn from_slice<'a, T>(v: &'a mut [u8]) -> serde_json::Result<T>
    where
        T: serde::de::Deserialize<'a>,
    {
        serde_json::from_slice(&*v)
    }
}

#[cfg(feature = "simd-json")]
pub mod json {
    pub use simd_json::{from_slice, to_vec};
}

pub mod bincode {
    pub use bincode::{config, decode_from_slice, encode_to_vec};
}

pub mod mdbx {
    pub use libmdbx_remote::*;
}

#[cfg(feature = "bcs")]
pub mod bcs {
    pub use bcs::{from_bytes, to_bytes};
}
