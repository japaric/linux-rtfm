use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::analyze::Analysis;

pub fn codegen(analysis: &Analysis) -> Vec<TokenStream2> {
    let mut stmts = vec![];

    for ty in &analysis.send_types {
        stmts.push(quote!(rtfm::export::assert_send::<#ty>();));
    }

    for ty in &analysis.sync_types {
        stmts.push(quote!(rtfm::export::assert_sync::<#ty>();));
    }

    stmts
}
