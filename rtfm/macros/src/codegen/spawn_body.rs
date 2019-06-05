use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::{ast::App, Context};
use syn::Ident;

use crate::{analyze::Analysis, codegen::util};

/// Creates the body of `spawn_${name}`
pub fn codegen<'a>(
    context: Context,
    name: &Ident,
    app: &'a App,
    analysis: &Analysis,
) -> TokenStream2 {
    let sender = context.core(app);
    let spawnee = &app.software_tasks[name];
    let receiver = spawnee.args.core;
    let priority = spawnee.args.priority;

    let (_, tupled, _, _) = util::regroup_inputs(&spawnee.inputs);

    let inputs = util::inputs_ident(name);
    let fq = util::fq_ident_(name, sender);

    let t = util::spawn_t_ident(receiver, priority);

    let write_instant = if app.uses_schedule(receiver) {
        let instants = util::instants_ident(name);

        Some(quote!(
            #instants.get_unchecked_mut(usize::from(index)).as_mut_ptr().write(instant);
        ))
    } else {
        None
    };

    let variant = util::task_ident(name, sender);
    let signo = analysis.signals[&receiver].map[&priority];
    let enqueue = if app.args.cores == 1 {
        quote!(
            rtfm::export::enqueue(
                TGID.get(),
                None,
                #signo,
                #t::#variant as u8,
                index,
            );
        )
    } else {
        let tid = util::tid_ident(receiver);

        quote!(
            rtfm::export::enqueue(
                TGID.get(),
                Some(#tid.get()),
                #signo,
                #t::#variant as u8,
                index,
            );
        )
    };

    let dequeue = if context.is_init() {
        // `init` has exclusive access to these queues so we can bypass the resources AND
        // the consumer / producer split
        quote!(#fq.dequeue())
    } else {
        quote!((#fq { priority }).lock(|fq| fq.split().1.dequeue()))
    };

    quote!(
        unsafe {
            use rtfm::Mutex as _;

            let input = #tupled;
            if let Some(index) = #dequeue {
                #inputs.get_unchecked_mut(usize::from(index)).as_mut_ptr().write(input);

                #write_instant

                #enqueue

                Ok(())
            } else {
                Err(input)
            }
        }
    )
}
