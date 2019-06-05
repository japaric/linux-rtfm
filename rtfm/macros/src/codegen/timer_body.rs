use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::{analyze::TimerQueue, ast::App, Core};

use crate::{analyze::Analysis, codegen::util};

pub fn codegen(sender: Core, timer_queue: &TimerQueue, app: &App, analysis: &Analysis) -> TokenStream2 {
    let timer = util::timer_ident(sender);
    let tq = util::tq_ident(sender);
    let arms = timer_queue
        .tasks
        .iter()
        .map(|name| {
            let task = &app.software_tasks[name];
            let receiver = task.args.core;
            let cfgs = &task.cfgs;
            let signo = analysis.signals[&receiver].map[&task.args.priority];
            let ct = util::schedule_t_ident(sender);
            let pt = util::spawn_t_ident(receiver, task.args.priority);
            let pname = util::task_ident(name, sender);

            let tid = if app.args.cores == 1 {
                quote!(None)
            } else {
                let tid = util::tid_ident(receiver);
                quote!(Some(#tid.get()))
            };

            quote!(
                #(#cfgs)*
                #ct::#name => {
                    rtfm::export::enqueue(tgid, #tid, #signo, #pt::#pname as u8, index);
                }
            )
        })
        .collect::<Vec<_>>();

    quote!(
        let tgid = TGID.get();
        let timer = #timer.get();

        while let Some((task, index)) = (#tq {
            priority: &rtfm::export::Priority::new(PRIORITY),
        }).lock(|tq| tq.dequeue(timer)) {
            match task {
                #(#arms)*
            }
        }
    )
}
