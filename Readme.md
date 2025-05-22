## MDBX Derive

Macro to define and derive mdbx types.

## Usage

There are three macros to derive on your structs:

- `KeyObject`: This will implement `key_encode/decode` for the struct, by strictly serializing the fields one by one as plain bytes, each occupying `std::size_of` bytes. For example, a struct having one u8 and one u16 will be serialized to 3 bytes. Note this encoding has ambiguity and thus can be only decoded with the scheme.
- `ZstdBincodeObject`: This will implement `table_encode/decode` for the struct, by serializing the contents to `bincode` and then compressing the `bincode` to `zstd`.
- `ZstdJSONObject`: This will `table_encode/decode` for the struct, by serializing the contents to `json` and then compressing the `json` with `zstd`.

## Features

- `serde_json`: Use `serde_json` for `ZstdJSONObject` macro.
- `simd_json`: Use `simd_json` for `ZstdJSONObject` macro.
- `alloy`: Implement `KeyObject` for alloy tyes.

## Sample

```rust
#[cfg(test)]
mod test {
    use std::io::Cursor;

    use bincode::{Decode, Encode};
    use mdbx_derive::{
        KeyObject, KeyObjectDecode, KeyObjectEncode, TableObjectDecode, TableObjectEncode,
        ZstdBincodeObject,
    };
    use serde::{Deserialize, Serialize};

    #[cfg(any(feature = "simd-json", feature = "serde_json"))]
    use mdbx_derive::ZstdJSONObject;

    #[derive(Encode, Decode, Default, Serialize, Deserialize, KeyObject, ZstdBincodeObject)]
    struct TrivialKey {
        a: u64,
        b: u64,
    }

    #[cfg(any(feature = "simd-json", feature = "serde_json"))]
    #[derive(Encode, Decode, Default, Serialize, Deserialize, ZstdJSONObject)]
    struct TrivialJSONKey {
        a: u64,
        b: u64,
    }

    #[test]
    fn trivial_key() {
        assert_eq!(TrivialKey::KEYSIZE, std::mem::size_of::<u64>() * 2);
        let k = TrivialKey { a: 42, b: 24 };
        let ky = k.key_encode().expect("fail to encode");
        assert_eq!(ky.len(), std::mem::size_of::<u64>() * 2);

        let ky = TrivialKey::key_decode(&ky).expect("fail to decode key");
        assert_eq!(ky.a, 42);
        assert_eq!(ky.b, 24);
    }

    #[test]
    fn trivial_object() {
        let k = TrivialKey { a: 42, b: 24 };
        let ky = k.table_encode().expect("fail to encode");
        let expected = mdbx_derive::zstd::encode_all(
            Cursor::new(
                mdbx_derive::bincode::encode_to_vec(&k, mdbx_derive::bincode::config::standard())
                    .expect("bincode"),
            ),
            1,
        )
        .expect("zstd");
        assert_eq!(ky, expected);

        let ky = TrivialKey::table_decode(&ky).expect("fail to decode key");
        assert_eq!(ky.a, 42);
        assert_eq!(ky.b, 24);
    }

    #[cfg(any(feature = "simd-json", feature = "serde_json"))]
    #[test]
    fn trivial_json() {
        let k = TrivialJSONKey { a: 42, b: 24 };
        let ky = k.table_encode().expect("fail to encode");
        let expected = mdbx_derive::zstd::encode_all(
            Cursor::new(mdbx_derive::json::to_vec(&k).expect("bincode")),
            1,
        )
        .expect("zstd");
        assert_eq!(ky, expected);

        let ky = TrivialJSONKey::table_decode(&ky).expect("fail to decode key");
        assert_eq!(ky.a, 42);
        assert_eq!(ky.b, 24);
    }
}
```