use std::io::Read;
use std::path::PathBuf;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_quote,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Expr, ExprBlock, File, ImplItem, Item, TraitItem, UseTree,
};

use crate::visit::attr::pop_attribute;
use crate::Version;

mod attr;

pub(crate) struct AmphisbaenaConversion {
    version: Version,
    mod_name: String,
}

impl AmphisbaenaConversion {
    pub fn new<T: Into<String>>(version: Version, mod_name: T) -> Self {
        Self {
            version,
            mod_name: mod_name.into(),
        }
    }
    pub fn convert(&mut self, item: TokenStream2) -> TokenStream2 {
        let preserve = self.version.as_str();
        let remove = match self.version {
            Version::Async => Version::Sync,
            Version::Sync => Version::Async,
        }
        .as_str();

        let mut syntax_tree: File = syn::parse(item.into()).unwrap();
        self.visit_file_mut(&mut syntax_tree);

        // remove item that violate current version
        // preserve item that conform to current version, and remove tagging attribute
        syntax_tree.items.iter_mut().for_each(|item| {
            if let Item::Mod(item_mod) = item {
                attr::mod_remove_items(item_mod, remove);
                attr::mod_remove_attr(item_mod, preserve);
            }
        });

        quote!(#syntax_tree)
    }

    fn replace_use_tree(&self, item: &mut UseTree) {
        match item {
            // A path prefix of imports in a `use` item: `std::...`.
            UseTree::Path(path) => {
                self.replace_use_tree(&mut *path.tree);
                if path.ident == Ident::new(self.mod_name.as_str(), path.span()) {
                    path.ident = Ident::new(self.version.as_str(), path.span());
                }
            }

            // An identifier imported by a `use` item: `HashMap`.
            UseTree::Name(name) => {
                if name.ident == Ident::new(self.mod_name.as_str(), name.span()) {
                    name.ident = Ident::new(self.version.as_str(), name.span());
                }
            }

            // An renamed identifier imported by a `use` item: `HashMap as Map`.
            UseTree::Rename(rename) => {
                if rename.ident == Ident::new(self.mod_name.as_str(), rename.span()) {
                    rename.ident = Ident::new(self.version.as_str(), rename.span());
                }
            }

            // A braced group of imports in a `use` item: `{A, B, C}`.
            UseTree::Group(group) => {
                for item in &mut group.items {
                    self.replace_use_tree(item)
                }
            }
            _ => {}
        }
    }
}

impl VisitMut for AmphisbaenaConversion {
    fn visit_item_mut(&mut self, item: &mut Item) {
        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_item_mut(self, item);

        match item {
            Item::Use(item_use) => {
                if item_use.leading_colon.is_none() {
                    // leading_colon is some indicating using crates
                    // here is when use amphi mod
                    self.replace_use_tree(&mut item_use.tree);
                }
            }
            Item::Mod(item) => {
                let empty_mod = if let Some(true) = item.content.as_ref().map(|x| x.1.is_empty()) {
                    true
                } else {
                    false
                };
                if item.content.is_none() || empty_mod {
                    item.semi = None;

                    let filename = pop_attribute(&mut item.attrs, "non_inline_module");
                    if filename.is_none() {
                        // early stop here
                        return;
                    }

                    let mut path = filename.unwrap();
                    if path.starts_with("\"") && path.ends_with("\"") {
                        path.remove(0);
                        path.remove(path.len() - 1);
                    } else {
                        // TODO error, path should be string
                    }
                    let mut file = std::fs::File::open(PathBuf::from(path.as_str()))
                        .expect(&format!("Fail to find mod {}", path));
                    let mut content = String::new();
                    file.read_to_string(&mut content).unwrap();

                    let mut ast = syn::parse_file(&content).unwrap();

                    self.visit_file_mut(&mut ast);
                    item.attrs.extend(ast.attrs);
                    item.content = Some((syn::token::Brace::default(), ast.items));
                }
            }

            _ => {}
        }
    }
}

pub struct AsyncAwaitRemoval;

impl AsyncAwaitRemoval {
    pub fn remove_async_await(&mut self, item: TokenStream2) -> TokenStream2 {
        let mut syntax_tree: File = syn::parse(item.into()).unwrap();
        self.visit_file_mut(&mut syntax_tree);
        quote!(#syntax_tree)
    }
}

impl VisitMut for AsyncAwaitRemoval {
    fn visit_item_mut(&mut self, item: &mut Item) {
        // Delegate to the default impl to recursively visit items
        visit_mut::visit_item_mut(self, item);

        match item {
            Item::Impl(item) => {
                for inner in &mut item.items {
                    if let ImplItem::Method(ref mut method) = inner {
                        if method.sig.asyncness.is_some() {
                            method.sig.asyncness = None;
                        }
                    }
                }
            }
            Item::Trait(item) => {
                for inner in &mut item.items {
                    if let TraitItem::Method(ref mut method) = inner {
                        if method.sig.asyncness.is_some() {
                            method.sig.asyncness = None;
                        }
                    }
                }
            }
            Item::Fn(item) => {
                if item.sig.asyncness.is_some() {
                    item.sig.asyncness = None;
                }
            }
            _ => {}
        }
    }
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_expr_mut(self, node);

        match node {
            Expr::Await(expr) => *node = (*expr.base).clone(),

            Expr::Async(expr) => {
                let inner = &expr.block;
                let sync_expr = if inner.stmts.len() == 1 {
                    // remove useless braces when there is only one statement
                    let stmt = &inner.stmts.get(0).unwrap();
                    // convert statement to Expr
                    parse_quote!(#stmt)
                } else {
                    Expr::Block(ExprBlock {
                        attrs: expr.attrs.clone(),
                        block: inner.clone(),
                        label: None,
                    })
                };
                *node = sync_expr;
            }
            _ => {}
        }
    }
}
