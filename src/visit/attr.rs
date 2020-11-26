use crate::visit::attr;
use proc_macro2::TokenTree;
use syn::{Attribute, Item, ItemMod};

pub fn find_attribute(attrs: &Vec<Attribute>, to_match: &str) -> bool {
    attrs
        .iter()
        .take_while(|attr| {
            if let 1 = attr.path.segments.len() {
                if attr.path.is_ident("amphi") {
                    let tree: TokenTree = syn::parse(attr.tokens.clone().into()).unwrap();
                    if let TokenTree::Group(group) = tree {
                        let arg = group.stream().to_string();
                        if arg.as_str() == to_match {
                            return false;
                        }
                    }
                }
            }
            true
        })
        .count()
        < attrs.len()
}

#[allow(dead_code)]
pub fn pop_attribute(attrs: &mut Vec<Attribute>, ident: &str) -> Option<String> {
    let mut result = None;
    attrs.retain(|attr| {
        if let 1 = attr.path.segments.len() {
            if attr.path.is_ident(ident) {
                let tree: TokenTree = syn::parse(attr.tokens.clone().into()).unwrap();
                if let TokenTree::Group(group) = tree {
                    result = Some(group.stream().to_string());
                    return false;
                }
            }
        }
        true
    });
    result
}

pub fn remove_matched_attribute(
    attrs: &mut Vec<Attribute>,
    ident: &str,
    to_match: &str,
) -> Option<String> {
    let mut result = None;
    attrs.retain(|attr| {
        if let 1 = attr.path.segments.len() {
            if attr.path.is_ident(ident) {
                let tree: TokenTree = syn::parse(attr.tokens.clone().into()).unwrap();
                if let TokenTree::Group(group) = tree {
                    let arg = group.stream().to_string();
                    if arg.as_str() == to_match {
                        result = Some(arg);
                        return false;
                    }
                }
            }
        }
        true
    });
    result
}

pub fn mod_remove_items(item_mod: &mut ItemMod, remove: &str) {
    if item_mod.content.is_some() {
        // remove item that has attribute of remove
        item_mod.content.as_mut().unwrap().1.retain(|item| {
            let found = match item {
                Item::Impl(item) => attr::find_attribute(&item.attrs, remove),
                Item::Trait(item) => attr::find_attribute(&item.attrs, remove),
                Item::Struct(item) => attr::find_attribute(&item.attrs, remove),
                Item::Enum(item) => attr::find_attribute(&item.attrs, remove),
                Item::Fn(item) => attr::find_attribute(&item.attrs, remove),
                Item::Mod(item) => attr::find_attribute(&item.attrs, remove),
                _ => false,
            };
            !found
        });
        // recursively remove item under mod
        item_mod
            .content
            .as_mut()
            .unwrap()
            .1
            .iter_mut()
            .for_each(|item| {
                if let Item::Mod(child_item_mod) = item {
                    mod_remove_items(child_item_mod, remove)
                }
            })
    }
}

pub fn mod_remove_attr(item_mod: &mut ItemMod, preserve: &str) {
    // remove attr that has attribute of preserve
    if item_mod.content.is_some() {
        item_mod
            .content
            .as_mut()
            .unwrap()
            .1
            .iter_mut()
            .for_each(|item| {
                match item {
                    Item::Impl(item) => {
                        attr::remove_matched_attribute(&mut item.attrs, "amphi", preserve);
                    }
                    Item::Trait(item) => {
                        attr::remove_matched_attribute(&mut item.attrs, "amphi", preserve);
                    }
                    Item::Struct(item) => {
                        attr::remove_matched_attribute(&mut item.attrs, "amphi", preserve);
                    }
                    Item::Enum(item) => {
                        attr::remove_matched_attribute(&mut item.attrs, "amphi", preserve);
                    }
                    Item::Fn(item) => {
                        attr::remove_matched_attribute(&mut item.attrs, "amphi", preserve);
                    }
                    Item::Mod(item) => {
                        attr::remove_matched_attribute(&mut item.attrs, "amphi", preserve);
                    }
                    _ => (),
                };
            });
        // recursively remove attributes of items under mod
        item_mod
            .content
            .as_mut()
            .unwrap()
            .1
            .iter_mut()
            .for_each(|item| {
                if let Item::Mod(child_item_mod) = item {
                    mod_remove_attr(child_item_mod, preserve)
                }
            })
    }
}
