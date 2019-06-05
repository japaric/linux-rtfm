use proc_macro2::{TokenStream as TokenStream2};
use quote::quote;
use rtfm_syntax::ast::App;

use crate::{analyze::Analysis, codegen::util};

pub fn codegen(app: &App, analysis: &Analysis) -> Vec<TokenStream2> {
    let mut items = vec![];

    for (&sender, timer_queue) in &analysis.timer_queues {
        let variants = timer_queue
            .tasks
            .iter()
            .map(|task| {
                let cfgs = &app.software_tasks[task].cfgs;

                quote!(
                    #(#cfgs)*
                    #task
                )
            })
            .collect::<Vec<_>>();

        let t = util::schedule_t_ident(sender);
        let doc = format!("Tasks `schedule`-able from core #{}", sender);
        items.push(quote!(
            #[doc = #doc]
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            enum #t {
                #(#variants,)*
            }
        ));

        let cap = util::typenum_capacity(timer_queue.capacity, false);
        let ty = quote!(rtfm::export::TimerQueue<#t, #cap>);
        let doc = format!("Core #{} timer queue", sender);
        let tq = util::tq_ident(sender);
        items.push(quote!(
            #[doc = #doc]
            static mut #tq: #ty = rtfm::export::TimerQueue(
                rtfm::export::BinaryHeap(rtfm::export::iBinaryHeap::new())
            );
        ));

        let timer = util::timer_ident(sender);
        let doc = format!("{} timer", tq.to_string());
        items.push(quote!(
            #[doc = #doc]
            static #timer: rtfm::export::Timer = rtfm::export::Timer::uninit();
        ));

        items.push(quote!(
            struct #tq<'a> {
                priority: &'a rtfm::export::Priority,
            }
        ));

        let range = analysis.signals[&sender].range();
        items.push(util::impl_mutex(
            &[],
            false,
            &tq,
            ty,
            timer_queue.ceiling,
            range,
            quote!(&mut #tq),
        ));
    }

    items
}
