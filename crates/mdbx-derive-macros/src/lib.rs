use itertools::Itertools;
use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Fields, Index, parse_macro_input, spanned::Spanned};

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
                        let bs: [u8; #tyts] = val.try_into().map_err(|_| mdbx_derive::Error::Corrupted)?;
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
                        let bs: [u8; #tyts] = val.try_into().map_err(|_| mdbx_derive::Error::Corrupted)?;
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
                Ok(mdbx_derive::bcs::from_bytes(&data_val).map_err(|_| mdbx_derive::mdbx::Error::Corrupted)?)
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
                Ok(mdbx_derive::bcs::from_bytes(&decompressed).map_err(|_| mdbx_derive::mdbx::Error::Corrupted)?)
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
                Ok(mdbx_derive::json::from_slice(&mut decompressed).map_err(|_| mdbx_derive::mdbx::Error::Corrupted)?)
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
