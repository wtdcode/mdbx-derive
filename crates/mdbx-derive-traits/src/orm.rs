use std::collections::HashMap;

use libmdbx_remote::{DatabaseFlags, EnvironmentAny, RW, TransactionKind, WriteFlags};

use crate::{
    error::MDBXDeriveError,
    key::{KeyObjectDecode, KeyObjectEncode},
    table::{TableObjectDecode, TableObjectEncode},
};

pub fn type_eq<T: ?Sized, U: ?Sized>() -> bool {
    typeid::of::<T>() == typeid::of::<U>()
}

pub trait MatchName {
    fn match_name<T>(&self, name: Option<&str>) -> Option<&T>;
    fn match_name_mut<T>(&mut self, name: Option<&str>) -> Option<&mut T>;
}

pub trait MDBXTables<E> {
    fn create_all(
        tx: &libmdbx_remote::TransactionAny<RW>,
        flags: DatabaseFlags,
    ) -> impl Future<Output = Result<HashMap<String, u32>, E>> + Send;
}

impl MatchName for () {
    fn match_name<T>(&self, _name: Option<&str>) -> Option<&T> {
        None
    }
    fn match_name_mut<T>(&mut self, _name: Option<&str>) -> Option<&mut T> {
        None
    }
}

impl<Head, Tail> MatchName for (Head, Tail)
where
    Head: MDBXTable,
    Tail: MatchName,
{
    fn match_name<T>(&self, name: Option<&str>) -> Option<&T> {
        if type_eq::<Head, T>() && name == Head::NAME {
            unsafe { (&raw const self.0 as *const T).as_ref() }
        } else {
            self.1.match_name::<T>(name)
        }
    }

    fn match_name_mut<T>(&mut self, name: Option<&str>) -> Option<&mut T> {
        if type_eq::<Head, T>() && name == Head::NAME {
            unsafe { (&raw mut self.0 as *mut T).as_mut() }
        } else {
            self.1.match_name_mut::<T>(name)
        }
    }
}

impl<E> MDBXTables<E> for () {
    async fn create_all(
        _tx: &libmdbx_remote::TransactionAny<RW>,
        _flags: DatabaseFlags,
    ) -> Result<HashMap<String, u32>, E> {
        Ok(HashMap::new())
    }
}

impl<Head, Tail, E> MDBXTables<E> for (Head, Tail)
where
    E: From<Head::Error> + 'static,
    Head: MDBXTable,
    Tail: MDBXTables<E>,
{
    async fn create_all(
        tx: &libmdbx_remote::TransactionAny<RW>,
        flags: DatabaseFlags,
    ) -> Result<HashMap<String, u32>, E> {
        let mut vals = HashMap::new();
        let dbi = Head::create_table_tx(tx, flags).await?;
        vals.insert(Head::NAME.map(|s| s.to_string()).unwrap_or_default(), dbi);
        vals.extend(Tail::create_all(tx, flags).await?);
        Ok(vals)
    }
}

pub trait MDBXTable: Sized {
    type Key: KeyObjectEncode + KeyObjectDecode + Send + Sync;
    type Value: TableObjectEncode + TableObjectDecode + Send + Sync;
    type Error: From<libmdbx_remote::ClientError> + From<MDBXDeriveError> + Send + 'static;
    type Metadata: TableObjectEncode + TableObjectDecode + Send + Sync;
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
        async move {
            let db = tx.create_db(Self::NAME, flags).await?;
            Ok(db.dbi())
        }
    }

    fn create_table(
        env: &libmdbx_remote::EnvironmentAny,
        flags: libmdbx_remote::DatabaseFlags,
    ) -> impl Future<Output = Result<u32, Self::Error>> + Send {
        async move {
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

pub trait HasMDBXEnvironment {
    fn env(&self) -> &EnvironmentAny;
}

pub trait HasMDBXDBIStore {
    fn dbis(&self) -> &HashMap<String, u32>;

    fn dbi<T: MDBXTable>(&self) -> Option<u32> {
        self.dbis()
            .get(&T::NAME.map(|v| v.to_string()).unwrap_or_default())
            .copied()
    }
}

pub trait HasMDBXTables {
    type Error: From<libmdbx_remote::ClientError> + From<MDBXDeriveError> + Send + 'static;
    type Tables: MDBXTables<Self::Error>;
}

pub trait MDBXDatabase: Sized + Send + Sync + HasMDBXEnvironment + HasMDBXTables {
    type Metadata: TableObjectEncode + TableObjectDecode + Send + Sync;
    const METADATA_NAME: &'static [u8] = b"metadata";

    fn create_all(
        &self,
        flags: DatabaseFlags,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let tx = self.env().begin_rw_txn().await?;
            Self::Tables::create_all(&tx, flags).await?;
            Ok(())
        }
    }

    fn write_metadata_tx(
        &self,
        dbi: Option<u32>,
        tx: &libmdbx_remote::TransactionAny<RW>,
        meta: &Self::Metadata,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let dbi = if let Some(dbi) = dbi {
                dbi
            } else {
                tx.open_db(None).await?.dbi()
            };
            Ok(tx
                .put(
                    dbi,
                    Self::METADATA_NAME,
                    &meta.table_encode()?,
                    WriteFlags::default(),
                )
                .await?)
        }
    }

    fn write_metadata(
        &self,
        meta: &Self::Metadata,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            let tx = self.env().begin_rw_txn().await?;
            self.write_metadata_tx(None, &tx, meta).await?;
            tx.commit().await?;
            Ok(())
        }
    }

    fn metadata_tx<K: TransactionKind>(
        &self,
        dbi: Option<u32>,
        tx: &libmdbx_remote::TransactionAny<K>,
    ) -> impl Future<Output = Result<Option<Self::Metadata>, Self::Error>> + Send {
        async move {
            let dbi = if let Some(dbi) = dbi {
                dbi
            } else {
                tx.open_db(None).await?.dbi()
            };
            Ok(tx
                .get::<Vec<u8>>(dbi, Self::METADATA_NAME)
                .await?
                .map(|v| Self::Metadata::table_decode(&v))
                .transpose()?)
        }
    }

    fn metadata(&self) -> impl Future<Output = Result<Option<Self::Metadata>, Self::Error>> + Send {
        async move {
            let tx = self.env().begin_ro_txn().await?;
            self.metadata_tx(None, &tx).await
        }
    }
}

// macros to generate table/database

#[macro_export]
macro_rules! mdbx_table {
    (
        $struct_name:ident,
        $key_type:ty,
        $value_type:ty
    ) => {
        $crate::mdbx_table!($struct_name, $key_type, $value_type, mdbx_derive::Error, ());
    };
    (
        $struct_name:ident,
        $key_type:ty,
        $value_type:ty,
        $error_type:ty
    ) => {
        $crate::mdbx_table!($struct_name, $key_type, $value_type, $error_type, ());
    };
    (
        $struct_name:ident,
        $key_type:ty,
        $value_type:ty,
        $error_type:ty,
        $metadata_type:ty
    ) => {
        impl mdbx_derive::MDBXTable for $struct_name {
            type Key = $key_type;
            type Value = $value_type;
            type Error = $error_type;
            type Metadata = $metadata_type;

            const NAME: Option<&'static str> = Some(stringify!($struct_name));
        }
    };
}

#[macro_export]
macro_rules! mdbx_table_def {
    (
        $struct_name:ident,
        $key_type:ty,
        $value_type:ty
    ) => {
        $crate::mdbx_table_def!($struct_name, $key_type, $value_type, mdbx_derive::Error, ());
    };
    (
        $struct_name:ident,
        $key_type:ty,
        $value_type:ty,
        $error_type:ty
    ) => {
        $crate::mdbx_table_def!($struct_name, $key_type, $value_type, $error_type, ());
    };
    (
        $struct_name:ident,
        $key_type:ty,
        $value_type:ty,
        $error_type:ty,
        $metadata_type:ty
    ) => {
        #[derive(Clone, Debug, Copy, Default)]
        pub struct $struct_name;

        impl mdbx_derive::MDBXTable for $struct_name {
            type Key = $key_type;
            type Value = $value_type;
            type Error = $error_type;
            type Metadata = $metadata_type;

            const NAME: Option<&'static str> = Some(stringify!($struct_name));
        }
    };
}

#[macro_export]
macro_rules! mdbx_database {
    (
        $db_name:ident,
        $error_type:ty,
        $metadata_type:ty,
        $($tables:ty),+
    ) => {
        mdbx_derive::paste::paste! {
            mdbx_derive::generate_dbi_struct!([<$db_name Dbi>], $error_type, $($tables),*);

            #[derive(Debug, Clone)]
            pub struct $db_name {
                pub env: mdbx_derive::mdbx::EnvironmentAny,
                pub dbis: [<$db_name Dbi>]
            }

            impl std::ops::Deref for $db_name {
                type Target = mdbx_derive::mdbx::EnvironmentAny;
                fn deref(&self) -> &Self::Target {
                    &self.env
                }
            }

            impl $db_name {
                pub fn new(env: mdbx_derive::mdbx::EnvironmentAny, dbis: [<$db_name Dbi>]) -> Self {
                    Self {
                        env,
                        dbis
                    }
                }

                pub async fn open_create_tables_with_defaults(url: &str, defaults: mdbx_derive::mdbx::EnvironmentBuilder) -> Result<Self, $error_type> {
                    let env =  mdbx_derive::mdbx::EnvironmentAny::open_with_defaults(url, defaults).await?;
                    let dbis = [<$db_name Dbi>]::new(&env, mdbx_derive::mdbx::DatabaseFlags::default())
                            .await?;
                    Ok(Self::new(env, dbis))
                }

                pub async fn open_tables_with_defaults(url: &str, defaults: mdbx_derive::mdbx::EnvironmentBuilder) -> Result<Self, $error_type> {
                    let env =  mdbx_derive::mdbx::EnvironmentAny::open_with_defaults(url, defaults).await?;
                    let tx = env.begin_ro_txn().await?;
                    let dbis = [<$db_name Dbi>]::new_ro(&tx)
                            .await?;
                    Ok(Self::new(env, dbis))
                }
            }
        }

        impl mdbx_derive::HasMDBXEnvironment for $db_name {
            fn env(&self) -> &mdbx_derive::mdbx::EnvironmentAny {
                &self.env
            }
        }

        impl mdbx_derive::MDBXDatabase for $db_name {
            type Metadata = $metadata_type;
        }

        impl mdbx_derive::HasMDBXTables for $db_name {
            type Error = $error_type;
            type Tables = mdbx_derive::tuple_list_type!($($tables),*);
        }
    };
}
