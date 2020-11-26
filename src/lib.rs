//!
//! # amphi
//! [![Build Status](https://github.com/fMeow/amphi/workflows/CI%20%28Linux%29/badge.svg?branch=main)](https://github.com/fMeow/amphi/actions)
//! [![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
//! [![Latest Version](https://img.shields.io/crates/v/amphi.svg)](https://crates.io/crates/amphi)
//! [![amphi](https://docs.rs/amphi/badge.svg)](https://docs.rs/amphi)
//!
//! **Why bother writing similar code twice for blocking and async code?**
//!
//! When implementing both sync and async versions of API in a crate, most API
//! of the two version are almost the same except for some async/await keyword.
//! `amphi` provides a macro to get blocking code from async implementation for free,
//! alongside with the async code.
//!
//! amphi is an English prefix meaning `both`. This crate copy the async code and strip all
//! async/await keyword to get a blocking implementation.
//!
//! # How to use
//! 1. place all your async code in a mod. By default, the mod should call `amphi`,
//! but it can be customize.
//! 2. apply `amphi` attribute macro on the mod declaration code.
//!
//! # LICENSE
//! MIT
extern crate proc_macro;

use proc_macro::TokenStream;
use std::ffi::OsStr;
use std::path::PathBuf;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, AttributeArgs, Ident, Item, Lit, Meta, NestedMeta};

use crate::parse::ItemModRestrict;
use crate::visit::{AmphiConversion, AsyncAwaitRemoval};

mod parse;
mod visit;

#[derive(Copy, Clone)]
enum Version {
    Sync,
    Async,
}

impl Version {
    pub fn as_str(&self) -> &'static str {
        match self {
            Version::Sync => "blocking",
            Version::Async => "asynchronous",
        }
    }
}

#[derive(PartialEq)]
enum Mode {
    SyncOnly,
    AsyncOnly,
    Both,
}

struct AmphiArgs {
    mode: Mode,
    path: PathBuf,
}

fn parse_args(attr_args: AttributeArgs) -> Result<AmphiArgs, (Span, &'static str)> {
    if attr_args.len() > 2 {
        return Err((Span::call_site(), "Only up to two argument is accepted"));
    }
    let mut args = AmphiArgs {
        mode: Mode::Both,
        path: PathBuf::from("src"),
    };
    for attr in &attr_args {
        match attr {
            NestedMeta::Lit(lit) => {
                return Err((lit.span(), "Arguments should not be literal"));
            }
            NestedMeta::Meta(meta) => match &meta {
                Meta::NameValue(meta_name_value) => {
                    if meta_name_value.path.is_ident("path") {
                        let path_value = if let Lit::Str(lit_str) = &meta_name_value.lit {
                            lit_str.value()
                        } else {
                            return Err((meta_name_value.lit.span(), "path should be string"));
                        };
                        args.path = PathBuf::from(path_value);
                        if args.path.is_absolute() {
                            return Err((
                                meta_name_value.lit.span(),
                                "Absolute path is not allowed. Please use relative path.",
                            ));
                        }
                        if args.path.is_file() {
                            args.path.set_extension("");
                        } else {
                            return Err((meta_name_value.lit.span(), "file not found"));
                        }
                    } else {
                        return Err((
                            meta.span(),
                            "Only allow `async_only`, `blocking_only`, or `path`",
                        ));
                    }
                }
                Meta::Path(path) => {
                    if path.is_ident("async_only") {
                        if args.mode == Mode::SyncOnly {
                            return Err((
                                meta.span(),
                                "Option `async_only`, `blocking_only` are mutually exclusive",
                            ));
                        }
                        args.mode = Mode::AsyncOnly;
                    } else if path.is_ident("blocking_only") {
                        if args.mode == Mode::AsyncOnly {
                            return Err((
                                meta.span(),
                                "Option `async_only`, `blocking_only` is mutually exclusive",
                            ));
                        }
                        args.mode = Mode::SyncOnly;
                    } else {
                        return Err((
                            meta.span(),
                            "Only allow `async_only`, `blocking_only`, or `path`",
                        ));
                    }
                }
                _ => {
                    return Err((
                        meta.span(),
                        "Only allow `async_only`, `blocking_only`, or `path`",
                    ));
                }
            },
        }
    }

    if args.path.file_name() == Some(OsStr::new("lib"))
        || args.path.file_name() == Some(OsStr::new("main"))
    {
        args.path.pop();
    }
    Ok(args)
}

#[proc_macro_attribute]
pub fn amphi(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let amphi_args = match parse_args(attr_args) {
        Ok(mode) => mode,
        Err((span, message)) => {
            return syn::Error::new(span, message).to_compile_error().into();
        }
    };

    let item_mod = parse_macro_input!(input as ItemModRestrict).0;
    let mod_name = format!("{}", item_mod.ident);

    let mut sync = item_mod.clone();
    sync.ident = Ident::new("blocking", sync.ident.span());

    let mut asynchronous = item_mod.clone();
    asynchronous.ident = Ident::new("asynchronous", sync.ident.span());

    let (sync, asynchronous) = match amphi_args.mode {
        Mode::SyncOnly => (quote!(#sync), quote!()),
        Mode::AsyncOnly => (quote!(), quote!(#asynchronous)),
        Mode::Both => (quote!(#sync), quote!(#asynchronous)),
    };

    let asynchronous_mod =
        AmphiConversion::new(Version::Async, mod_name.as_str(), amphi_args.path.clone())
            .convert(asynchronous);
    let sync_mod =
        AmphiConversion::new(Version::Sync, mod_name.as_str(), amphi_args.path).convert(sync);
    let sync_mod = AsyncAwaitRemoval.remove_async_await(sync_mod);

    (quote! {
        #asynchronous_mod
        #sync_mod
    })
    .into()
}

fn parse_test_args(attr_args: AttributeArgs) -> Result<String, (Span, &'static str)> {
    match attr_args.len() {
        0 => Ok("amphi".to_string()),
        1 => {
            let attr = attr_args.get(0).unwrap();
            match attr {
                NestedMeta::Lit(lit) => {
                    if let Lit::Str(mod_name) = lit {
                        Ok(mod_name.value())
                    } else {
                        Err((
                            lit.span(),
                            "Arguments should be str: like `#[test(\"amphi_mod_name\")]` or `#[test(name=\"mod_name\")]`",
                        ))
                    }
                }
                NestedMeta::Meta(meta) => {
                    if let Meta::NameValue(name_value) = meta {
                        if name_value.path.is_ident("name") {
                            if let Lit::Str(mod_name) = &name_value.lit {
                                Ok(mod_name.value())
                            } else {
                                Err((
                                    name_value.lit.span(),
                                    "test option mod `name` should be string",
                                ))
                            }
                        } else {
                            Err((
                                meta.span(),
                                "Arguments should be str: like `#[test(\"amphi_mod_name\")]` or `#[test(name=\"mod_name\")]`",
                            ))
                        }
                    } else {
                        Err((
                            meta.span(),
                            "Arguments should be str: like `#[test(\"amphi_mod_name\")]` or `#[test(name=\"mod_name\")]`",
                        ))
                    }
                }
            }
        }
        _ => Err((Span::call_site(), "Accept up to one argument")),
    }
}

#[proc_macro_attribute]
pub fn test(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let mod_name = match parse_test_args(attr_args) {
        Ok(mod_name) => mod_name,
        Err((span, message)) => {
            return syn::Error::new(span, message).to_compile_error().into();
        }
    };

    let input = TokenStream2::from(input);

    let sync = AmphiConversion::new(Version::Sync, mod_name.as_str(), None).convert(input.clone());
    let sync = AsyncAwaitRemoval.remove_async_await(sync);
    let sync_ts = sync.clone().into();
    let sync_test = match &mut parse_macro_input!(sync_ts as Item) {
        Item::Fn(item_fn) => {
            item_fn.attrs.retain(|attr| attr.path.is_ident("test"));
            let name = format!("{}_sync_version", item_fn.sig.ident);
            item_fn.sig.ident = Ident::new(name.as_str(), item_fn.sig.ident.span());
            quote!(#item_fn)
        }
        _ => sync,
    };

    let asynchronous_test =
        AmphiConversion::new(Version::Async, mod_name.as_str(), None).convert(input.clone());

    let test_code = quote! {
        #[test]
        #sync_test

        #asynchronous_test
    };
    test_code.into()
}
