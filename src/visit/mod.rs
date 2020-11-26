use std::io::Read;
use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_quote,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Expr, ExprBlock, File, ImplItem, Item, ItemMod, Stmt, TraitItem, UseTree,
};

use crate::visit::attr::remove_matched_attribute;
use crate::Version;

mod attr;

const MOD_DECLARE: &'static str = "declare_mod";

macro_rules! tailor_expr {
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

        if let Err(syn_error) = self.fill_mod(&mut syntax_tree) {
            return syn_error.into();
        }
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
                if path.ident.to_string() == self.mod_name {
                    path.ident = Ident::new(self.version.as_str(), path.span());
                }
            }

            // An identifier imported by a `use` item: `HashMap`.
            UseTree::Name(name) => {
                if name.ident.to_string() == self.mod_name {
                    name.ident = Ident::new(self.version.as_str(), name.span());
                }
            }

            // An renamed identifier imported by a `use` item: `HashMap as Map`.
            UseTree::Rename(rename) => {
                if rename.ident.to_string() == self.mod_name {
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

    fn fill_mod(&mut self, file: &mut File) -> Result<(), TokenStream> {
        for item in &mut file.items {
            // iterate inside the amphi mod
            if let Item::Mod(item_mod) = item {
                if item_mod.content.is_some() {
                    for i in &mut item_mod.content.as_mut().unwrap().1 {
                        if matches!(i, Item::Macro(_)) {
                            // TODO allow non root mod
                            self.macro_mod_declaration(i, vec![format!("src/{}", &self.mod_name)])?
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn macro_mod_declaration(
        &mut self,
        item: &mut Item,
        path: Vec<String>,
    ) -> Result<(), TokenStream> {
        if let Item::Macro(item_macro) = item {
            if item_macro.mac.path.is_ident(MOD_DECLARE) {
                let token: TokenStream = item_macro.mac.tokens.clone().into();

                match syn::parse::<ItemMod>(token) {
                    Ok(mut item_mod) => {
                        if item_mod.semi.is_none() {
                            return Err(syn::Error::new(
                                item_macro.span(),
                                "Only accept mod declaration",
                            )
                            .to_compile_error()
                            .into());
                        }

                        let mod_name = format!("{}", item_mod.ident);

                        // find mod file
                        let mut file_path: PathBuf = path.iter().collect();
                        file_path.push(&mod_name);
                        file_path.set_extension("rs");
                        if !file_path.as_path().exists() {
                            file_path.set_extension("");
                            file_path.push("mod.rs");
                            if !file_path.as_path().exists() {
                                return Err(syn::Error::new(
                                    item_macro.span(),
                                    format!("File not found for mod `{}`", &mod_name),
                                )
                                .to_compile_error()
                                .into());
                            }
                        }

                        // parse file content
                        let mut file = std::fs::File::open(file_path).unwrap();

                        let mut content = String::new();
                        file.read_to_string(&mut content).unwrap();

                        let mut ast = syn::parse_file(&content).unwrap();

                        // recursively update
                        for item in &mut ast.items {
                            if matches!(item, Item::Macro(_)) {
                                let mut p = path.clone();
                                p.push(mod_name.clone());
                                self.macro_mod_declaration(item, p)?;
                            }
                        }

                        item_mod.content = Some((syn::token::Brace::default(), ast.items));
                        *item = Item::Mod(item_mod);
                    }
                    Err(_) => {
                        return Err(syn::Error::new(
                            item_macro.span(),
                            "Only accept mod declaration, ending with trailing semicolon `;`",
                        )
                        .to_compile_error()
                        .into());
                    }
                }
            }
        }
        Ok(())
    }
}

impl VisitMut for AmphiConversion {
    fn visit_item_mut(&mut self, item: &mut Item) {
        // Delegate to the default impl to visit nested expressions.
        visit_mut::visit_item_mut(self, item);

        if let Item::Use(item_use) = item {
            // leading_colon is some indicating using crates
            if item_use.leading_colon.is_none() {
                // here is when use amphi mod
                self.replace_use_tree(&mut item_use.tree);
            }
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
            Expr::Array(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // An assignment expression: `a :&str=compute()`;
            Expr::Assign(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A compound assignment expression: `counter += 1`.
            Expr::AssignOp(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // An async block: `async { ... }`.
            Expr::Async(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // An await expression: `fut.await`.
            Expr::Await(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A binary operation: `a + b`, `a * b`.
            Expr::Binary(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A blocked scope: `{ ... }`.
            Expr::Block(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A box expression: `box f`.
            Expr::Box(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A `break`, with an optional label to break and an optional expression.
            Expr::Break(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A function call expression: `invoke(a, b)`.
            Expr::Call(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A cast expression: `foo as f64`.
            Expr::Cast(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A closure expression: `|a, b| a + b`.
            Expr::Closure(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A `continue`, with an optional label.
            Expr::Continue(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // Access of a named struct field (`obj.k`) or unnamed tuple struct field (`obj.0`).
            Expr::Field(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A for loop: `for pat in expr { ... }`.
            Expr::ForLoop(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // An expression contained within invisible delimiters.
            //
            // This variant is important for faithfully representing the precedence
            // of expressions and is related to `None`-delimited spans in a
            // `TokenStream`.
            Expr::Group(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // An `if` expression with an optional `else` block: `if expr { ... }
            // else { ... }`.
            //
            // The `else` branch expression may only be an `If` or `Block`
            // expression, not any of the other types of expression.
            Expr::If(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A square bracketed indexing expression: `vector[2]`.
            Expr::Index(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A `let` guard: `let Some(x) = opt`.
            Expr::Let(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A literal in place of an expression: `1`, `"foo"`.
            Expr::Lit(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // Conditionless loop: `loop { ... }`.
            Expr::Loop(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A macro invocation expression: `format!("{}", q)`.
            Expr::Macro(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A `match` expression: `match n { Some(n) => {}, None => {} }`.
            Expr::Match(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A method call expression: `x.foo::<T>(a, b)`.
            Expr::MethodCall(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A parenthesized expression: `(a + b)`.
            Expr::Paren(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A path like `std::mem::replace` possibly containing generic
            // parameters and a qualified self-type.
            //
            // A plain identifier like `x` is a path of length 1.
            Expr::Path(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A range expression: `1..2`, `1..`, `..2`, `1..=2`, `..=2`.
            Expr::Range(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A referencing operation: `&a` or `&mut a`.
            Expr::Reference(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // An array literal constructed from one repeated element: `[0u8; N]`.
            Expr::Repeat(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A `return`, with an optional value to be returned.
            Expr::Return(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A struct literal expression: `Point { x: 1, y: 1 }`.
            //
            // The `rest` provides the value of the remaining fields as in `S { a:
            // 1, b: 1, ..rest }`.
            Expr::Struct(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A try-expression: `expr?`.
            Expr::Try(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A try block: `try { ... }`.
            Expr::TryBlock(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A tuple expression: `(a, b, c, d)`.
            Expr::Tuple(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A type ascription expression: `foo: f64`.
            Expr::Type(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A unary operation: `!x`, `*x`.
            Expr::Unary(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // An unsafe block: `unsafe { ... }`.
            Expr::Unsafe(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A while loop: `while expr { ... }`.
            Expr::While(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

            // A yield expression: `yield expr`.
            Expr::Yield(expr) => tailor_expr!(expr.attrs, preserve, remove, node),

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
