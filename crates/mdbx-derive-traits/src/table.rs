use std::borrow::Cow;

use crate::{
    error::MDBXDeriveError,
    key::{KeyObjectDecode, KeyObjectEncode},
};

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

pub trait MDBXTable: Sized {
    type Key: KeyObjectEncode + KeyObjectDecode + Send + Sync;
    type Value: TableObjectEncode + TableObjectDecode + Send + Sync;
    type Error: From<libmdbx_remote::ClientError> + From<MDBXDeriveError> + Send + 'static;
    const NAME: Option<&'static str>;

    fn open_table_tx<T: libmdbx_remote::TransactionKind>(
        tx: &libmdbx_remote::TransactionAny<T>,
    ) -> impl Future<Output = Result<u32, Self::Error>> + Send {
        async {
            let db = tx.open_db(Self::NAME).await?;
            Ok(db.dbi())
        }
    }

    fn open_table(
        env: &libmdbx_remote::EnvironmentAny,
    ) -> impl Future<Output = Result<u32, Self::Error>> + Send {
        async {
            let tx = env.begin_ro_txn().await?;
            Self::open_table_tx(&tx).await
        }
    }

    fn create_table_tx(
        tx: &libmdbx_remote::TransactionAny<libmdbx_remote::RW>,
        flags: libmdbx_remote::DatabaseFlags,
    ) -> impl Future<Output = Result<u32, Self::Error>> + Send {
        async {
            let db = tx.create_db(Self::NAME, flags).await?;
            Ok(db.dbi())
        }
    }

    fn create_table(
        env: &libmdbx_remote::EnvironmentAny,
        flags: libmdbx_remote::DatabaseFlags,
    ) -> impl Future<Output = Result<u32, Self::Error>> + Send {
        async {
            let tx = env.begin_rw_txn().await?;
            Self::create_table_tx(&tx, flags).await
        }
    }

    fn get_item(
        env: &libmdbx_remote::EnvironmentAny,
        key: &Self::Key,
    ) -> impl Future<Output = Result<Option<Self::Value>, Self::Error>> + Send {
        async move {
            let tx = env.begin_ro_txn().await?;
            Self::get_item_tx(&tx, None, key).await
        }
    }

    fn get_item_tx<T: libmdbx_remote::TransactionKind>(
        tx: &libmdbx_remote::TransactionAny<T>,
        dbi: Option<u32>,
        key: &Self::Key,
    ) -> impl Future<Output = Result<Option<Self::Value>, Self::Error>> + Send {
        async move {
            let dbi = if let Some(dbi) = dbi {
                dbi
            } else {
                Self::open_table_tx(tx).await?
            };
            let v = tx
                .get::<Vec<u8>>(dbi, &key.key_encode()?)
                .await?
                .map(|v| Self::Value::table_decode(&v))
                .transpose()?;

            Ok(v)
        }
    }

    fn put_item(
        env: &libmdbx_remote::EnvironmentAny,
        key: &Self::Key,
        value: &Self::Value,
        flags: libmdbx_remote::WriteFlags,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let tx = env.begin_rw_txn().await?;
            Self::put_item_tx(&tx, None, key, value, flags).await
        }
    }

    fn put_item_tx(
        tx: &libmdbx_remote::TransactionAny<libmdbx_remote::RW>,
        dbi: Option<u32>,
        key: &Self::Key,
        value: &Self::Value,
        flags: libmdbx_remote::WriteFlags,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let dbi = if let Some(dbi) = dbi {
                dbi
            } else {
                Self::create_table_tx(tx, libmdbx_remote::DatabaseFlags::default()).await?
            };
            tx.put(dbi, &key.key_encode()?, &value.table_encode()?, flags)
                .await?;
            Ok(())
        }
    }
}

#[macro_export]
macro_rules! mdbx_table {
    (
        $struct_name:ident,
        $key_type:ty,
        $value_type:ty
    ) => {
        $crate::mdbx_table!($struct_name, $key_type, $value_type, mdbx_derive::Error);
    };
    (
        $struct_name:ident,
        $key_type:ty,
        $value_type:ty,
        $error_type:ty
    ) => {
        impl mdbx_derive::MDBXTable for $struct_name {
            type Key = $key_type;
            type Value = $value_type;
            type Error = $error_type;

            const NAME: Option<&'static str> = Some(stringify!($struct_name));
        }
    };
}
