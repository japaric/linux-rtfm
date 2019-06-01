use core::ops::Range;
use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::{analyze::Analysis, codegen::util};

pub fn codegen(analysis: &Analysis) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
    let mut const_app = vec![];
    let mut stmts = vec![];

    // initialize late resources
    if let Some(late_resources) = analysis.late_resources.get(&0) {
        for name in late_resources {
            stmts.push(quote!(
                #name.as_mut_ptr().write(late.#name);
            ));
        }
    }

    // synchronization barriers
    let all_senders = analysis
        .initialization_barriers
        .values()
        .flat_map(|senders| senders.iter().cloned())
        .collect::<BTreeSet<_>>();

    for &sender in &all_senders {
        let b = util::b_ident(sender);
        const_app.push(quote!(
            static #b: rtfm::export::Barrier = rtfm::export::Barrier::new();
        ));
    }

    if all_senders.contains(&0) {
        let b = util::b_ident(0);
        stmts.push(quote!(
            #b.release();
        ));
    }

    if let Some(senders) = analysis.initialization_barriers.get(&0) {
        for &sender in senders {
            let b = util::b_ident(sender);
            stmts.push(quote!(
                #b.wait();
            ));
        }
    }

    // `interrupt::enable()`
    let signals = &analysis.signals[&0];
    let max = signals.map.len() as u8;
    let Range { start, end } = signals.range();
    stmts.push(quote!(
        rtfm::export::mask(#start..#end, 0, #max, false);
    ));

    (const_app, stmts)
}
