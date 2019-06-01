use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::{ast::App, Context};
use syn::Ident;

use crate::{analyze::Analysis, codegen::util};

pub fn codegen(ctxt: Context, name: &Ident, app: &App, analysis: &Analysis) -> TokenStream2 {
    let sender = ctxt.core(app);
    let schedulee = &app.software_tasks[name];
    let receiver = schedulee.args.core;

    let (_, tupled, _, _) = util::regroup_inputs(&schedulee.inputs);

    let fq = util::fq_ident_(name, sender);
    let tq = util::tq_ident(sender);
    let inputs = util::inputs_ident(name);

    let signo = analysis.signals[&sender].map[&analysis.timer_queues[&sender].priority];
    let tgid_tid = if app.args.cores == 1 {
        quote!(None)
    } else {
        let tid = util::tid_ident(sender);

        quote!(Some((
            TGID.get(),
            #tid.get(),
        )))
    };
    let (dequeue, enqueue) = if ctxt.is_init() {
        // `init` has exclusive access to these queues so we can bypass the resources AND
        // the consumer / producer split
        (
            quote!(#fq.dequeue()),
            quote!(#tq.enqueue_unchecked(nr, #tgid_tid, #signo);),
        )
    } else {
        (
            quote!((#fq { priority }).lock(|fq| fq.split().1.dequeue())),
            quote!((#tq { priority }).lock(|tq| {
                tq.enqueue_unchecked(nr, #tgid_tid, #signo)
            });),
        )
    };

    let instants_write = if app.uses_schedule(receiver) {
        let instants = util::instants_ident(name);

        Some(quote!(#instants.get_unchecked_mut(usize::from(index)).as_mut_ptr().write(instant);))
    } else {
        None
    };

    let t = util::schedule_t_ident(sender);
    quote!(
        unsafe {
            use rtfm::Mutex as _;

            let input = #tupled;
            if let Some(index) = #dequeue {
                #instants_write

                #inputs.get_unchecked_mut(usize::from(index)).as_mut_ptr().write(input);

                let nr = rtfm::export::NotReady {
                    instant,
                    index,
                    task: #t::#name,
                };

                #enqueue

                Ok(())
            } else {
                Err(input)
            }
        }
    )
}
