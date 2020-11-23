//!
//! # amphi
//! **Why bother writing similar code twice for blocking and async code?**
//!
//! amphi is an English prefix meaning `both`. This crate provides macro `amphi` to get
//! blocking code aside async implementation for free.
//!
//! [![Build Status](https://github.com/fMeow/amphi/workflows/CI%20%28Linux%29/badge.svg?branch=master)](https://github.com/fMeow/amphi/actions)
//! [![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
//! [![Latest Version](https://img.shields.io/crates/v/amphi.svg)](https://crates.io/crates/amphi)
//! [![amphi](https://docs.rs/amphi/badge.svg)](https://docs.rs/amphi)
//!
//! When implementing both sync and async versions of API in a crate, most API
//! of the two version are almost the same except for some async/await keyword.
//!
//! Write async code once and get blocking code for free with `amphi`.
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

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Attribute, AttributeArgs, Ident, Item, Lit, NestedMeta, Meta};

use crate::visit::{AmphisbaenaConversion, AsyncAwaitRemoval};

mod visit;

#[derive(Copy, Clone)]
enum Version {
    Synchronous,
    Asynchronous,
}

impl Version {
    pub fn as_str(&self) -> &'static str {
        match self {
            Version::Synchronous => "sync",
            Version::Asynchronous => "asynchronous",
        }
    }
}

enum Mode {
    SyncOnly,
    AsyncOnly,
    Both,
}

fn parse_args(attr_args: AttributeArgs) -> Result<Mode, (Span, &'static str)> {
    match attr_args.len() {
        0 => Ok(Mode::Both),
        1 => {
            let attr = attr_args.get(0).unwrap();
            match attr {
                NestedMeta::Lit(lit) => {
                    Err((lit.span(), "Arguments shoule not be str", ))
                }
                NestedMeta::Meta(meta) => {
                    if let Meta::Path(path) = meta {
                        match path.segments.len() {
                            0 => Ok(Mode::Both),
                            1 => {
                                let arg = path.segments.first().unwrap().ident.to_string();
                                if &arg == "async_only" {
                                    Ok(Mode::AsyncOnly)
                                } else if &arg == "sync_only" {
                                    Ok(Mode::SyncOnly)
                                } else {
                                    Err((meta.span(), "Only accepts `async_only` or `sync_only`", ))
                                }
                            }

                            _ => Err((meta.span(), "Only accepts up to 1 argument", )),
                        }
                    } else {
                        Err((
                            meta.span(),
                            "Arguments shoule be str: `#[amphi(sync_only)]` or `#[amphi(async_only)]`",
                        ))
                    }
                }
            }
        }
        _ => {
            Err((Span::call_site(), "Only one argument is accepted", ))
        }
    }
}

// TODO
//  1. allow mod in separate file
#[proc_macro_attribute]
pub fn amphi(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let mode = match parse_args(attr_args) {
        Ok(mode) => mode,
        Err((span, message)) => {
            return syn::Error::new(span, message).to_compile_error().into();
        }
    };

    let (mod_name, sync, asynchronous) = match &mut parse_macro_input!(input as Item) {
        Item::Mod(item_mod) => {
            let mod_name = format!("{}", item_mod.ident);

            let mut sync = item_mod.clone();
            sync.ident = Ident::new("sync", sync.ident.span());

            let mut asynchronous = item_mod.clone();
            asynchronous.ident = Ident::new("asynchronous", sync.ident.span());

            match mode {
                Mode::SyncOnly => (mod_name, quote!(#sync), quote!()),
                Mode::AsyncOnly => (mod_name, quote!(), quote!(#asynchronous)),
                Mode::Both => (mod_name, quote!(#sync), quote!(#asynchronous)),
            }
        }
        _ => {
            return syn::Error::new(Span::call_site(), "Should apply on a mod")
                .to_compile_error()
                .into();
        }
    };

    let asynchronous_mod =
        AmphisbaenaConversion::new(Version::Asynchronous, mod_name.as_str()).convert(asynchronous);
    let sync_mod =
        AmphisbaenaConversion::new(Version::Synchronous, mod_name.as_str()).convert(sync);
    let sync_mod = AsyncAwaitRemoval.remove_async_await(sync_mod);

    let out = quote! {
        #asynchronous_mod
        #sync_mod
    };

    out.into()
}

fn remove_ident_from_attribute(attrs: &mut Vec<Attribute>, ident: &str) {
    let mut to_remove = vec![];
    for (ix, attr) in attrs.iter().enumerate() {
        let mut matched = false;
        'Segment: for seg in &attr.path.segments {
            if seg.ident == Ident::new(ident, seg.span()) {
                matched = true;
                break 'Segment;
            }
        }
        if matched {
            to_remove.push(ix)
        }
    }
    to_remove.into_iter().for_each(|ix| {
        attrs.remove(ix);
    });
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
                            "Arguments should be str: like `#[test(\"amphi\")]`",
                        ))
                    }
                }
                NestedMeta::Meta(meta) => {
                    Err((
                        meta.span(),
                        "Arguments should be str: like `#[test(\"amphi\")]`",
                    ))
                }
            }
        }
        _ => {
            Err((Span::call_site(), "Accept up to one argument", ))
        }
    }
}

#[proc_macro_attribute]
pub fn test(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = parse_macro_input!(args as AttributeArgs);
    let mod_name = match parse_test_args(attr_args) {
        Ok(mod_name) => mod_name,
        Err((span, message)) => {
            return syn::Error::new(span, message)
                .to_compile_error()
                .into();
        }
    };

    let input = TokenStream2::from(input);

    let sync =
        AmphisbaenaConversion::new(Version::Synchronous, mod_name.as_str()).convert(input.clone());
    let sync = AsyncAwaitRemoval.remove_async_await(sync);
    let sync_ts = sync.clone().into();
    let sync_test = match &mut parse_macro_input!(sync_ts as Item) {
        Item::Fn(item_fn) => {
            remove_ident_from_attribute(&mut item_fn.attrs, "test");
            let name = format!("{}_sync_version", item_fn.sig.ident);
            item_fn.sig.ident = Ident::new(name.as_str(), item_fn.sig.ident.span());
            quote!(#item_fn)
        }
        _ => sync,
    };

    let asynchronous_test =
        AmphisbaenaConversion::new(Version::Asynchronous, mod_name.as_str()).convert(input.clone());

    let test_code = quote! {
        #[test]
        #sync_test

        #asynchronous_test
    };
    test_code.into()
}
