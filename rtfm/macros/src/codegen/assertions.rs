use std::collections::HashSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::analyze::Analysis;

pub fn codegen(analysis: &Analysis) -> Vec<TokenStream2> {
    let mut stmts = vec![];

    let send_types = analysis
        .send_types
        .values()
        .flat_map(|tys| tys)
        .collect::<HashSet<_>>();
    for ty in send_types {
        stmts.push(quote!(rtfm::export::assert_send::<#ty>();));
    }

    let sync_types = analysis
        .sync_types
        .values()
        .flat_map(|tys| tys)
        .collect::<HashSet<_>>();
    for ty in sync_types {
        stmts.push(quote!(rtfm::export::assert_sync::<#ty>();));
    }

    stmts
}
