extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::quote;
use syn::{
    parse, parse_macro_input, spanned::Spanned, Item, ItemFn, ItemStatic, ReturnType, Stmt, Type,
    Visibility,
};

#[proc_macro_attribute]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as ItemFn);

    if !args.is_empty() {
        return parse::Error::new(Span::call_site(), "This attribute accepts no arguments")
            .to_compile_error()
            .into();
    }

    // check the function signature
    let valid_signature = check_signature(&f)
        && f.decl.inputs.is_empty()
        && (is_bottom(&f.decl.output) || is_unit(&f.decl.output));

    if !valid_signature {
        return parse::Error::new(
            f.span(),
            "`#[entry]` function must have signature `fn() [-> !]`",
        )
        .to_compile_error()
        .into();
    }

    let (statics, stmts) = extract_statics(f.block.stmts);

    let mut args = vec![];
    let mut params = vec![];
    let mut vars = vec![];
    for static_ in statics {
        let attrs = static_.attrs;
        let ident = static_.ident;
        let ty = static_.ty;
        let expr = static_.expr;

        args.push(quote!(&mut #ident));
        params.push(quote!(#ident: &'static mut #ty));
        vars.push(quote!(
            #(#attrs)*
            static mut #ident: #ty = #expr;
        ));
    }

    let attrs = f.attrs;
    let ident = f.ident;
    quote!(
        fn #ident(#(#params),*) {
            #(#stmts)*

            #(#attrs)*
            #[export_name = "main"]
            unsafe fn #ident() {
                #(#vars)*

                crate::#ident(#(#args),*)
            }
        }
    )
    .into()
}

/// checks that a function signature
///
/// - has no bounds (like where clauses)
/// - is not `async`
/// - is not `const`
/// - is not `unsafe`
/// - is not generic (has no type parametrs)
/// - is not variadic
/// - uses the Rust ABI (and not e.g. "C")
fn check_signature(item: &ItemFn) -> bool {
    let vis_is_inherited = match item.vis {
        Visibility::Inherited => true,
        _ => false,
    };

    vis_is_inherited
        && item.constness.is_none()
        && item.asyncness.is_none()
        && item.abi.is_none()
        && item.unsafety.is_none()
        && item.decl.generics.params.is_empty()
        && item.decl.generics.where_clause.is_none()
        && item.decl.variadic.is_none()
}

fn is_bottom(ty: &ReturnType) -> bool {
    if let ReturnType::Type(_, ty) = ty {
        if let Type::Never(_) = **ty {
            true
        } else {
            false
        }
    } else {
        false
    }
}

fn is_unit(ty: &ReturnType) -> bool {
    if let ReturnType::Type(_, ty) = ty {
        if let Type::Tuple(ref tuple) = **ty {
            tuple.elems.is_empty()
        } else {
            false
        }
    } else {
        true
    }
}

/// Extracts `static mut` vars from the beginning of the given statements
fn extract_statics(stmts: Vec<Stmt>) -> (Vec<ItemStatic>, Vec<Stmt>) {
    let mut istmts = stmts.into_iter();

    let mut statics = vec![];
    let mut stmts = vec![];
    while let Some(stmt) = istmts.next() {
        match stmt {
            Stmt::Item(Item::Static(var)) => {
                if var.mutability.is_some() {
                    statics.push(var);
                } else {
                    stmts.push(Stmt::Item(Item::Static(var)));
                    break;
                }
            }
            _ => {
                stmts.push(stmt);
                break;
            }
        }
    }

    stmts.extend(istmts);

    (statics, stmts)
}
