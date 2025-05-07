pub use mdbx_derive_macros::*;
pub use mdbx_derive_traits::error::MDBXDeriveError as Error;
pub use mdbx_derive_traits::key::{KeyObjectDecode, KeyObjectEncode};
pub use mdbx_derive_traits::table::{TableObjectDecode, TableObjectEncode};

pub mod zstd {
    pub use zstd::{decode_all, encode_all};
}

pub mod serde_json {
    pub use serde_json::{from_slice, to_vec};
}

pub mod bincode {
    pub use bincode::{config, decode_from_slice, encode_to_vec};
}

pub mod mdbx {
    pub use libmdbx_remote::*;
}
