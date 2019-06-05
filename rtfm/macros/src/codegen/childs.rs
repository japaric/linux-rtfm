use core::ops::Range;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::ast::App;

use crate::{analyze::Analysis, codegen::util};

pub fn codegen(app: &App, analysis: &Analysis) -> Vec<TokenStream2> {
    let mut const_app = vec![];

    // initialize the other threads and the `TID`s
    for &core in &analysis.used_cores {
        if core == 0 {
            // core #0 is not a child
            continue;
        }

        let tid = util::tid_ident(core);
        let child = util::child_ident(core);

        let mut stmts = vec![];

        // spin yield until this thread has been migrated to a different CPU
        stmts.push(quote!(
            #tid.wait();
        ));

        if let Some(init) = app.inits.get(&core) {
            let name = &init.name;
            stmts.push(quote!(
                let late = #name(#name::Locals::new(), #name::Context::new());
            ));
        }

        // initialize late resources
        if let Some(late_resources) = analysis.late_resources.get(&core) {
            for name in late_resources {
                stmts.push(quote!(
                    #name.as_mut_ptr().write(late.#name);
                ));
            }
        }

        // initialization barriers
        if let Some(senders) = analysis.initialization_barriers.get(&core) {
            for &sender in senders {
                let b = util::b_ident(sender);
                stmts.push(quote!(
                    #b.wait();
                ));
            }
        }

        // `interrupt::enable`
        let signals = &analysis.signals[&core];
        let max = signals.map.len() as u8;
        let Range { start, end } = signals.range();
        stmts.push(quote!(
            rtfm::export::mask(#start..#end, 0, #max, false);
        ));

        if let Some(idle) = app.idles.get(&core) {
            let name = &idle.name;
            stmts.push(quote!(
                #name(
                    #name::Locals::new(),
                    #name::Context::new(&rtfm::export::Priority::new(0)),
                )
            ));
        } else {
            stmts.push(quote!(loop {
                rtfm::export::pause()
            }));
        }

        const_app.push(quote!(
            extern "C" fn #child() -> ! {
                unsafe {
                    #(#stmts)*
                }
            }
        ));
    }

    const_app
}
