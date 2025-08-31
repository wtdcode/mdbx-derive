use heck::ToSnakeCase;
use itertools::Itertools;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    Data, DeriveInput, Fields, Ident, Index, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
};

#[proc_macro_derive(KeyObject)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let decode = decode_impl(&input);
    // Encode implementation
    let ident = input.ident;
    let ts = match &input.data {
        Data::Struct(st) => match &st.fields {
            Fields::Named(fields) => {
                let recur = fields.named.iter().map(|t| {
                    let name = &t.ident;
                    quote_spanned! {t.span()=>
                        self.#name.key_encode()?.into_iter()
                    }
                });
                quote! {
                    [#(#recur),*].into_iter().flatten().collect()
                }
            }
            Fields::Unnamed(fields) => {
                let recur = fields.unnamed.iter().enumerate().map(|(idx, t)| {
                    let index = Index::from(idx);
                    quote_spanned! {t.span()=>
                        self.#index.key_encode()?.into_iter()
                    }
                });
                quote! {
                    [#(#recur),*].into_iter().flatten().collect()
                }
            }
            _ => quote! {
                compile_error!("Not supported")
            },
        },
        _ => quote! {
            compile_error!("Not supported struct")
        },
    };
    let output = quote! {
        impl mdbx_derive::KeyObjectEncode for #ident {
            fn key_encode(&self) -> Result<Vec<u8>, mdbx_derive::Error> {
                Ok(#ts)
            }
        }

        impl mdbx_derive::mdbx::TableObject for #ident {
            fn decode(data_val: &[u8]) -> Result<Self, mdbx_derive::mdbx::Error> {
                <Self as mdbx_derive::KeyObjectDecode>::key_decode(data_val).map_err(|_| mdbx_derive::mdbx::Error::Corrupted)
            }
        }

        #decode
    };
    output.into()
}

fn decode_impl(input: &DeriveInput) -> proc_macro2::TokenStream {
    let ident = &input.ident;
    let body = match &input.data {
        Data::Struct(st) => {
            let mut named = false;
            let fs = match &st.fields {
                Fields::Named(fields) => {
                    named = true;
                    Some(fields.named.iter())
                }
                Fields::Unnamed(fields) => Some(fields.unnamed.iter()),
                _ => None,
            };

            if let Some(fs) = fs {
                let ranges = fs
                    .clone()
                    .scan(quote! {0}, |acc, x| {
                        let ty = &x.ty;
                        let ret = Some(quote_spanned! {x.span()=>
                            (#acc)..(#acc + <#ty>::KEYSIZE)
                        });

                        *acc = quote! { #acc + <#ty>::KEYSIZE };
                        ret
                    })
                    .collect_vec();
                let recur = fs.clone().map(|t| {
                    let ty = &t.ty;
                    quote_spanned! {t.span()=>
                        <#ty>::KEYSIZE
                    }
                });
                let tyts = quote! {
                    0 #(+ #recur)*
                };

                if named {
                    let names = fs.clone().map(|t| {
                        let name = &t.ident;
                        quote_spanned! {t.span()=>
                            #name
                        }
                    });
                    let recur = fs.clone().zip(ranges).map(|(t, idx)| {
                        let name = &t.ident;
                        let ty = &t.ty;
                        quote_spanned! {t.span()=>
                            let #name = <#ty>::key_decode(bs[#idx].try_into().unwrap())?;
                        }
                    });
                    quote! {
                        let bs: [u8; #tyts] = val.try_into().map_err(|_| mdbx_derive::Error::IncorrectSchema(val.to_vec()))?;
                        #(#recur)*
                        Ok(Self {
                            #(#names),*
                        })
                    }
                } else {
                    let recur = fs.zip(ranges).map(|(t, idx)| {
                        let ty = &t.ty;
                        quote_spanned! {t.span()=>
                            <#ty>::key_decode(bs[#idx].try_into().unwrap())?
                        }
                    });

                    quote! {
                        let bs: [u8; #tyts] = val.try_into().map_err(|_| mdbx_derive::Error::IncorrectSchema(val.to_vec()))?;
                        Ok(Self(#(#recur),*))
                    }
                }
            } else {
                quote! {
                    compile_error("Not supported field")
                }
            }
        }
        _ => quote! {
            compile_error!("Not supported struct")
        },
    };

    let key_sz = match &input.data {
        Data::Struct(st) => {
            let ks = st.fields.iter().map(|f| {
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    <#ty>::KEYSIZE
                }
            });

            quote! {
                0 #(+ #ks)*
            }
        }
        _ => quote! { 0 },
    };

    let output = quote! {
        impl mdbx_derive::KeyObjectDecode for #ident {
            const KEYSIZE: usize = #key_sz ;
            fn key_decode(val: &[u8]) -> Result<Self, mdbx_derive::Error> {
                #body
            }
        }
    };
    output
}

#[proc_macro_derive(BcsObject)]
pub fn derive_bcs_object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let output = quote! {
        impl mdbx_derive::TableObjectDecode for #ident {
            fn table_decode(data_val: &[u8]) -> Result<Self, mdbx_derive::Error> {
                Ok(mdbx_derive::bcs::from_bytes(&data_val)?)
            }
        }

        impl mdbx_derive::mdbx::TableObject for #ident {
            fn decode(data_val: &[u8]) -> Result<Self, mdbx_derive::mdbx::Error> {
                mdbx_derive::bcs::from_bytes(&data_val).map_err(|_| mdbx_derive::mdbx::Error::Corrupted)
            }
        }

        impl mdbx_derive::TableObjectEncode for #ident {
            fn table_encode(&self) -> Result<Vec<u8>, mdbx_derive::Error> {
                Ok(mdbx_derive::bcs::to_bytes(&self)?)
            }
        }
    };
    output.into()
}

#[proc_macro_derive(ZstdBcsObject)]
pub fn derive_zstd_bcs_object(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let output = quote! {
        impl mdbx_derive::TableObjectDecode for #ident {
            fn table_decode(data_val: &[u8]) -> Result<Self, mdbx_derive::Error> {
                let decompressed = mdbx_derive::zstd::decode_all(data_val).map_err(|e| {
                    mdbx_derive::Error::Zstd(e)
                })?;
                Ok(mdbx_derive::bcs::from_bytes(&decompressed)?)
            }
        }

        impl mdbx_derive::mdbx::TableObject for #ident {
            fn decode(data_val: &[u8]) -> Result<Self, mdbx_derive::mdbx::Error> {
                let decompressed = mdbx_derive::zstd::decode_all(data_val).map_err(|_| {
                    mdbx_derive::mdbx::Error::Corrupted
                })?;
                mdbx_derive::bcs::from_bytes(&decompressed).map_err(|_| mdbx_derive::mdbx::Error::Corrupted)
            }
        }

        impl mdbx_derive::TableObjectEncode for #ident {
            fn table_encode(&self) -> Result<Vec<u8>, mdbx_derive::Error> {
                let bs = mdbx_derive::bcs::to_bytes(&self)?;
                let compressed = mdbx_derive::zstd::encode_all(std::io::Cursor::new(bs), 1).map_err(|e| {
                    mdbx_derive::Error::Zstd(e)
                })?;
                Ok(compressed)
            }
        }
    };
    output.into()
}

#[proc_macro_derive(ZstdBincodeObject)]
pub fn derive_zstd_bindcode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let output = quote! {
        impl mdbx_derive::TableObjectDecode for #ident {
            fn table_decode(data_val: &[u8]) -> Result<Self, mdbx_derive::Error> {
                let config = mdbx_derive::bincode::config::standard();
                let decompressed = mdbx_derive::zstd::decode_all(data_val).map_err(|e| {
                    mdbx_derive::Error::Zstd(e)
                })?;
                Ok(mdbx_derive::bincode::decode_from_slice(&decompressed, config)?.0)
            }
        }

        impl mdbx_derive::mdbx::TableObject for #ident {
            fn decode(data_val: &[u8]) -> Result<Self, mdbx_derive::mdbx::Error> {
                let config = mdbx_derive::bincode::config::standard();
                let decompressed = mdbx_derive::zstd::decode_all(data_val).map_err(|_| {
                    mdbx_derive::mdbx::Error::Corrupted
                })?;
                Ok(mdbx_derive::bincode::decode_from_slice(&decompressed, config).map_err(|_| mdbx_derive::mdbx::Error::Corrupted)?.0)
            }
        }

        impl mdbx_derive::TableObjectEncode for #ident {
            fn table_encode(&self) -> Result<Vec<u8>, mdbx_derive::Error> {
                let config = mdbx_derive::bincode::config::standard();
                let bs = mdbx_derive::bincode::encode_to_vec(&self, config)?;
                let compressed = mdbx_derive::zstd::encode_all(std::io::Cursor::new(bs), 1).map_err(|e| {
                    mdbx_derive::Error::Zstd(e)
                })?;
                Ok(compressed)
            }
        }
    };
    output.into()
}

#[cfg(feature = "json")]
#[proc_macro_derive(ZstdJSONObject)]
pub fn derive_zstd_json(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;
    let output = quote! {
        impl mdbx_derive::TableObjectDecode for #ident {
            fn table_decode(data_val: &[u8]) -> Result<Self, mdbx_derive::Error> {
                let mut decompressed = mdbx_derive::zstd::decode_all(data_val).map_err(|e| {
                    mdbx_derive::Error::Zstd(e)
                })?;
                Ok(mdbx_derive::json::from_slice(&mut decompressed)?)
            }
        }

        impl mdbx_derive::mdbx::TableObject for #ident {
            fn decode(data_val: &[u8]) -> Result<Self, mdbx_derive::mdbx::Error> {
                let mut decompressed = mdbx_derive::zstd::decode_all(data_val).map_err(|_| {
                    mdbx_derive::mdbx::Error::Corrupted
                })?;
                mdbx_derive::json::from_slice(&mut decompressed).map_err(|_| mdbx_derive::mdbx::Error::Corrupted)
            }
        }

        impl mdbx_derive::TableObjectEncode for #ident {
            fn table_encode(&self) -> Result<Vec<u8>, mdbx_derive::Error> {
                let bs = mdbx_derive::json::to_vec(&self)?;
                let compressed = mdbx_derive::zstd::encode_all(std::io::Cursor::new(bs), 1).map_err(|e| {
                    mdbx_derive::Error::Zstd(e)
                })?;
                Ok(compressed)
            }
        }
    };
    output.into()
}

// Helper struct to parse the macro's input
struct MacroInput {
    struct_name: Ident,
    error_type: Type,
    tables: Punctuated<Type, Token![,]>,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let error_type: Type = input.parse()?;
        input.parse::<Token![,]>()?;
        let tables = input.parse_terminated(Type::parse, Token![,])?;
        Ok(MacroInput {
            struct_name,
            error_type,
            tables,
        })
    }
}

#[proc_macro]
pub fn generate_dbi_struct(input: TokenStream) -> TokenStream {
    let MacroInput {
        struct_name,
        error_type,
        tables,
    } = syn::parse_macro_input!(input as MacroInput);

    let field_names: Vec<_> = tables
        .iter()
        .map(|table_type| {
            let type_path = if let Type::Path(tp) = table_type {
                tp
            } else {
                panic!("Expected a type path")
            };
            let type_ident_str = type_path.path.segments.last().unwrap().ident.to_string();
            let field_name_str = type_ident_str.to_snake_case();
            Ident::new(&field_name_str, proc_macro2::Span::call_site())
        })
        .collect();

    let field_statemens: Vec<_> = tables
        .iter()
        .map(|table_type| {
            let type_path = if let Type::Path(tp) = table_type {
                tp
            } else {
                panic!("Expected a type path")
            };
            let ty = type_path.path.segments.last().unwrap().ident.clone();
            let field_name_str = ty.to_string().to_snake_case();
            let ident = Ident::new(&field_name_str, proc_macro2::Span::call_site());

            quote! {
                let flags = if <#ty as mdbx_derive::MDBXTable>::DUPSORT {
                    mdbx_derive::mdbx::DatabaseFlags::DUP_SORT
                } else {
                    mdbx_derive::mdbx::DatabaseFlags::default()
                };
                let #ident = <#ty as mdbx_derive::MDBXTable>::create_table_tx(&tx, flags).await?;

            }
        })
        .collect();

    let ro_field_statemens: Vec<_> = tables
        .iter()
        .map(|table_type| {
            let type_path = if let Type::Path(tp) = table_type {
                tp
            } else {
                panic!("Expected a type path")
            };
            let ty = type_path.path.segments.last().unwrap().ident.clone();
            let field_name_str = ty.to_string().to_snake_case();
            let ident = Ident::new(&field_name_str, proc_macro2::Span::call_site());

            quote! {
                let #ident = <#ty as mdbx_derive::MDBXTable>::open_table_tx(&tx).await?;

            }
        })
        .collect();

    let fields = tables
        .iter()
        .zip(field_names.iter())
        .map(|(table_type, field_name)| {
            let type_path = if let Type::Path(tp) = table_type {
                tp
            } else {
                panic!()
            };
            let type_ident_str = type_path.path.segments.last().unwrap().ident.to_string();
            let doc_string = format!("DBI handle for the `{}` table.", type_ident_str);

            quote! {
                #[doc = #doc_string]
                pub #field_name: u32,
            }
        });

    let original_type_names: Vec<_> = tables
        .iter()
        .map(|table_type| {
            let type_path = if let Type::Path(tp) = table_type {
                tp
            } else {
                panic!("Expected a type path")
            };
            let type_ident_str = type_path.path.segments.last().unwrap().ident.to_string();
            Ident::new(&type_ident_str, proc_macro2::Span::call_site())
        })
        .collect(); // The crucial change is here!

    let rw_tables: Vec<_> = tables
        .iter()
        .map(|table_type| {
            let type_path = if let Type::Path(tp) = table_type {
                tp
            } else {
                panic!("Expected a type path")
            };
            let ty = type_path.path.segments.last().unwrap().ident.clone();
            let field_name_str = ty.to_string().to_snake_case();
            let ident = Ident::new(&field_name_str, proc_macro2::Span::call_site());
            let wfname_tx = Ident::new(format!("write_{}_tx", &field_name_str).as_str(), proc_macro2::Span::call_site());
            let rfname_tx = Ident::new(format!("read_{}_tx", &field_name_str).as_str(), proc_macro2::Span::call_site());
            quote! {
                async fn #wfname_tx
                (
                    &self,
                    tx: &mdbx_derive::mdbx::TransactionAny<mdbx_derive::mdbx::RW>,
                    key: &<#ty as mdbx_derive::MDBXTable>::Key,
                    value: &<#ty as mdbx_derive::MDBXTable>::Value,
                    flags: mdbx_derive::mdbx::WriteFlags
                ) -> Result<(), mdbx_derive::Error> {
                    tx.put(
                        self.#ident,
                        &<<#ty as mdbx_derive::MDBXTable>::Key as mdbx_derive::KeyObjectEncode>::key_encode(key)?,
                        &<<#ty as mdbx_derive::MDBXTable>::Value as mdbx_derive::TableObjectEncode>::table_encode(value)?,
                        flags
                    ).await?;
                    Ok(())
                }

                async fn #rfname_tx <K: mdbx_derive::mdbx::TransactionKind>
                (
                    &self,
                    tx: &mdbx_derive::mdbx::TransactionAny<K>,
                    key: &<#ty as mdbx_derive::MDBXTable>::Key
                ) -> Result<Option< <#ty as mdbx_derive::MDBXTable>::Value >, mdbx_derive::Error> {
                    let v = tx.get::<Vec<u8>>(
                        self.#ident,
                        &<<#ty as mdbx_derive::MDBXTable>::Key as mdbx_derive::KeyObjectEncode>::key_encode(key)?,
                    ).await?;
                    if let Some(v) = v {
                        Ok(Some(<<#ty as mdbx_derive::MDBXTable>::Value as mdbx_derive::TableObjectDecode>::table_decode(&v)?))
                    } else {
                        Ok(None)
                    }
                }
            }
        })
        .collect();

    let output = quote! {
        #[derive(Debug, Clone, Copy)]
        pub struct #struct_name {
            #( #fields )*
        }

        impl #struct_name {
            pub async fn new(
                env: &mdbx_derive::mdbx::EnvironmentAny,
            ) -> Result<Self, #error_type> {
                let tx = env.begin_rw_txn().await?;

                #(
                    #field_statemens
                )*

                tx.commit().await?;

                Ok(Self {
                    #( #field_names, )*
                })
            }

            pub async fn new_ro<K: mdbx_derive::mdbx::TransactionKind>(
                tx: &mdbx_derive::mdbx::TransactionAny<K>
            ) -> Result<Self, #error_type> {

                #(
                    #ro_field_statemens
                )*

                Ok(Self {
                    #( #field_names, )*
                })
            }

            #(
                #rw_tables
            )*
        }

        impl mdbx_derive::HasMDBXTables for #struct_name {
            type Error = #error_type;
            type Tables = mdbx_derive::tuple_list_type!(#( #original_type_names),*);
        }
    };

    output.into()
}
