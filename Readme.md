## MDBX Derive

Derive macros and a lightweight ORM for [libmdbx](https://gitflic.ru/project/erthink/libmdbx), built on top of [libmdbx-remote](https://crates.io/crates/libmdbx-remote).

## Usage

### Derive Macros

#### Key encoding

- `KeyObject` — Implements `KeyObjectEncode` / `KeyObjectDecode` by serializing each field as raw big-endian bytes, concatenated in declaration order. A struct with one `u8` and one `u16` produces exactly 3 bytes. The encoding is unambiguous only when decoded with the same schema.
- `KeyAsTableObject` — Reuses the `KeyObject` encoding as a `TableObjectEncode` / `TableObjectDecode` implementation.

#### Value (table object) encoding

- `ZstdPostcardObject` — Serializes with [postcard](https://crates.io/crates/postcard), then compresses with zstd.
- `ZstdJSONObject` — Serializes to JSON (`serde_json` or `simd-json`), then compresses with zstd. Requires `serde_json` or `simd-json` feature.
- `ZstdBcsObject` — Serializes with [BCS](https://crates.io/crates/bcs), then compresses with zstd. Requires `bcs` feature.
- `BcsObject` — Serializes with BCS (no compression). Requires `bcs` feature.

#### ORM macros (require `mdbx` feature)

- `mdbx_table!` / `mdbx_table_def!` — Define a table with key/value types.
- `mdbx_dupsort_table!` / `mdbx_dupsort_table_def!` — Define a DUPSORT table.
- `mdbx_database!` — Define a database struct that groups multiple tables, with auto-generated DBI handles and helper methods.

### Features

| Feature | Default | Description |
|---|---|---|
| `mdbx` | no | Enable `libmdbx-remote` dependency and the ORM layer (`mdbx_table!`, `mdbx_database!`, etc.). Without this, the crate provides only the derive macros and encode/decode traits. |
| `serde_json` | yes | Use `serde_json` for `ZstdJSONObject`. |
| `simd-json` | no | Use `simd-json` for `ZstdJSONObject` (takes precedence over `serde_json` when both are enabled). |
| `bcs` | yes | Support BCS encoding (`BcsObject`, `ZstdBcsObject`). |
| `alloy` | yes (in `mdbx-derive-traits`) | Implement `KeyObjectEncode` / `KeyObjectDecode` for alloy types (`Address`, `B256`, `U256`, etc.). |

## Examples

### Minimal — encode/decode only

`mdbx-derive` can be used purely for serialization without any MDBX dependency:

```rust
use std::io::Cursor;

use mdbx_derive::{
    KeyObject, KeyObjectDecode, KeyObjectEncode,
    TableObjectDecode, TableObjectEncode,
    ZstdPostcardObject,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, KeyObject)]
struct TrivialKey {
    a: u64,
    b: u64,
}

#[derive(Default, Serialize, Deserialize, ZstdPostcardObject)]
struct TrivialObject {
    a: u64,
    b: u64,
}

// Key encode/decode
let k = TrivialKey { a: 42, b: 24 };
let encoded = k.key_encode().unwrap();
assert_eq!(encoded.len(), std::mem::size_of::<u64>() * 2);

let decoded = TrivialKey::key_decode(&encoded).unwrap();
assert_eq!(decoded.a, 42);
assert_eq!(decoded.b, 24);

// Table object encode/decode (zstd + postcard)
let obj = TrivialObject { a: 42, b: 24 };
let encoded = obj.table_encode().unwrap();
let decoded = TrivialObject::table_decode(&encoded).unwrap();
assert_eq!(decoded.a, 42);
assert_eq!(decoded.b, 24);
```

### ORM — tables and databases

Enable the `mdbx` feature to use the full ORM layer:

```toml
[dependencies]
mdbx-derive = { version = "0.7", features = ["mdbx"] }
```

```rust
use mdbx_derive::*;

pub struct TrivialTable;
pub struct TrivialTable2;

// Define tables with Key and Value types
mdbx_table!(TrivialTable, TrivialKey, TrivialObject);
mdbx_table!(TrivialTable2, TrivialKey, TrivialObject, YourCustomError, MetadataType);

// Query a table directly
let out: Option<TrivialObject> = TrivialTable::get_item(&env, &TrivialKey { a: 1, b: 2 }).await?;

// Group tables into a database
mdbx_database!(
    TrivialDatabase,
    mdbx_derive::Error,
    MetadataType,
    TrivialTable,
    TrivialTable2
);

// Open and auto-create tables
let db = TrivialDatabase::open_create_tables_with_defaults(
    url,      // URL that libmdbx-remote accepts
    defaults, // EnvironmentBuilder defaults
).await?;

// DBI handles are available on the generated struct
let dbi: u32 = db.dbis.trivial_table;

// Or open existing tables without creating
let db = TrivialDatabase::open_tables_with_defaults(url, defaults).await?;

// Read/write metadata
let meta: Option<MetadataType> = db.metadata().await?;
db.write_metadata(&new_meta).await?;
```
