use std::io::Read;
use std::path::PathBuf;

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_quote,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Expr, ExprBlock, File, ImplItem, Item, ItemMod, Stmt, TraitItem, UseTree,
};

use crate::visit::attr::{pop_attribute, remove_matched_attribute};
use crate::Version;

mod attr;

macro_rules! clean_expr {
    ($attrs:expr, $preserve:expr, $remove: expr, $node:expr) => {{
        remove_matched_attribute(&mut $attrs, "amphi", $preserve);
        if remove_matched_attribute(&mut $attrs, "amphi", $remove).is_some() {
            // remove expression
            *$node = Expr::Verbatim(quote! {});
        }
    }};
}

/// replace use tree, fill mod declaration with implementation,
pub(crate) struct AmphiConversion {
    /// async or sync
    version: Version,
    /// root module name
    mod_name: String,
}

impl AmphiConversion {
    pub fn new<T: Into<String>>(version: Version, mod_name: T) -> Self {
        Self {
            version,
            mod_name: mod_name.into(),
        }
    }
    pub fn convert(&mut self, item: TokenStream2) -> TokenStream2 {
        let mut syntax_tree: File = syn::parse(item.into()).unwrap();
        self.visit_file_mut(&mut syntax_tree);
        self.tailor_version(&mut syntax_tree);
        quote!(#syntax_tree)
    }

    // keep only code that conform to current version (async or sync)
    fn tailor_version(&self, file: &mut File) {
        let preserve = self.version.as_str();
        let remove = match self.version {
            Version::Async => Version::Sync,
            Version::Sync => Version::Async,
        }
        .as_str();
        // remove item that violate current version
        // preserve item that conform to current version, and remove tagging attribute
        file.items.iter_mut().for_each(|item| {
            if let Item::Mod(item_mod) = item {
                attr::mod_remove_items(item_mod, remove);
                attr::mod_remove_attr(item_mod, preserve);
            }
        });
    }

    /// remove all ident to sync or asynchronous according to self.version
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

    fn fill_mod_declaration(&mut self, item: &mut ItemMod) {
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
}

impl VisitMut for AmphisbaenaConversion {
    fn visit_item_mut(&mut self, item: &mut Item) {
        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_item_mut(self, item);

        match item {
            Item::Use(item_use) => {
                // leading_colon is some indicating using crates
                if item_use.leading_colon.is_none() {
                    // here is when use amphi mod
                    self.replace_use_tree(&mut item_use.tree);
                }
            }
            Item::Mod(item) => {
                self.fill_mod_declaration(item);
            }
            _ => {}
        }
    }

    fn visit_expr_mut(&mut self, node: &mut Expr) {
        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_expr_mut(self, node);

        let preserve = self.version.as_str();
        let remove = match self.version {
            Version::Async => Version::Sync,
            Version::Sync => Version::Async,
        }
        .as_str();

        match node {
            // A slice literal expression: `[a, b, c, d]`.
            Expr::Array(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // An assignment expression: `a :&str=compute()`;
            Expr::Assign(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A compound assignment expression: `counter += 1`.
            Expr::AssignOp(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // An async block: `async { ... }`.
            Expr::Async(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // An await expression: `fut.await`.
            Expr::Await(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A binary operation: `a + b`, `a * b`.
            Expr::Binary(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A blocked scope: `{ ... }`.
            Expr::Block(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A box expression: `box f`.
            Expr::Box(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A `break`, with an optional label to break and an optional expression.
            Expr::Break(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A function call expression: `invoke(a, b)`.
            Expr::Call(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A cast expression: `foo as f64`.
            Expr::Cast(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A closure expression: `|a, b| a + b`.
            Expr::Closure(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A `continue`, with an optional label.
            Expr::Continue(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // Access of a named struct field (`obj.k`) or unnamed tuple struct field (`obj.0`).
            Expr::Field(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A for loop: `for pat in expr { ... }`.
            Expr::ForLoop(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // An expression contained within invisible delimiters.
            //
            // This variant is important for faithfully representing the precedence
            // of expressions and is related to `None`-delimited spans in a
            // `TokenStream`.
            Expr::Group(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // An `if` expression with an optional `else` block: `if expr { ... }
            // else { ... }`.
            //
            // The `else` branch expression may only be an `If` or `Block`
            // expression, not any of the other types of expression.
            Expr::If(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A square bracketed indexing expression: `vector[2]`.
            Expr::Index(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A `let` guard: `let Some(x) = opt`.
            Expr::Let(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A literal in place of an expression: `1`, `"foo"`.
            Expr::Lit(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // Conditionless loop: `loop { ... }`.
            Expr::Loop(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A macro invocation expression: `format!("{}", q)`.
            Expr::Macro(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A `match` expression: `match n { Some(n) => {}, None => {} }`.
            Expr::Match(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A method call expression: `x.foo::<T>(a, b)`.
            Expr::MethodCall(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A parenthesized expression: `(a + b)`.
            Expr::Paren(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A path like `std::mem::replace` possibly containing generic
            // parameters and a qualified self-type.
            //
            // A plain identifier like `x` is a path of length 1.
            Expr::Path(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A range expression: `1..2`, `1..`, `..2`, `1..=2`, `..=2`.
            Expr::Range(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A referencing operation: `&a` or `&mut a`.
            Expr::Reference(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // An array literal constructed from one repeated element: `[0u8; N]`.
            Expr::Repeat(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A `return`, with an optional value to be returned.
            Expr::Return(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A struct literal expression: `Point { x: 1, y: 1 }`.
            //
            // The `rest` provides the value of the remaining fields as in `S { a:
            // 1, b: 1, ..rest }`.
            Expr::Struct(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A try-expression: `expr?`.
            Expr::Try(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A try block: `try { ... }`.
            Expr::TryBlock(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A tuple expression: `(a, b, c, d)`.
            Expr::Tuple(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A type ascription expression: `foo: f64`.
            Expr::Type(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A unary operation: `!x`, `*x`.
            Expr::Unary(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // An unsafe block: `unsafe { ... }`.
            Expr::Unsafe(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A while loop: `while expr { ... }`.
            Expr::While(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            // A yield expression: `yield expr`.
            Expr::Yield(expr) => clean_expr!(expr.attrs, preserve, remove, node),

            _ => {}
        }
    }

    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_stmt_mut(self, stmt);

        let preserve = self.version.as_str();
        let remove = match self.version {
            Version::Async => Version::Sync,
            Version::Sync => Version::Async,
        }
        .as_str();
        match stmt {
            Stmt::Local(local) => {
                remove_matched_attribute(&mut local.attrs, "amphi", preserve);
                if remove_matched_attribute(&mut local.attrs, "amphi", remove).is_some() {
                    *stmt = Stmt::Expr(Expr::Verbatim(quote! {}));
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
