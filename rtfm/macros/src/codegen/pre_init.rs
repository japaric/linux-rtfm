use core::ops::Range;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::ast::App;

use crate::{analyze::Analysis, codegen::util};

pub fn codegen(app: &App, analysis: &Analysis) -> (Vec<TokenStream2>, Vec<TokenStream2>) {
    let mut const_app = vec![];
    let mut stmts = vec![];

    let signo_max = match analysis
        .signals
        .values()
        .flat_map(|signals| signals.map.values())
        .max()
    {
        Some(signo) => quote!(Some(#signo)),
        None => quote!(None),
    };
    stmts.push(quote!(rtfm::export::init_runtime(#signo_max);));

    // populate the `FreeQueue`s
    for (name, senders) in &analysis.free_queues {
        let cap = app.software_tasks[name].args.capacity;

        // NOTE all free queues share the same INPUTS / INSTANTS buffers
        stmts.push(quote!(
            let mut index = 0;
        ));
        for &sender in senders.keys() {
            let fq = util::fq_ident_(name, sender);

            stmts.push(quote!(
                for _ in 0..#cap {
                    #fq.enqueue_unchecked(index);
                    index += 1;
                }
            ));
        }
    }

    // initialize the `TGID`
    const_app.push(quote!(
        static TGID: rtfm::export::Pid = rtfm::export::Pid::uninit();
    ));
    stmts.push(quote!(
        let tgid = rtfm::export::getpid();
        TGID.init(tgid) ;
    ));

    // initialize `TIMER0`
    if let Some(tq) = analysis.timer_queues.get(&0) {
        let timer = util::timer_ident(0);
        let signo = analysis.signals[&0].map[&tq.priority];

        let tid = if app.args.cores == 1 {
            quote!(None)
        } else {
            quote!(Some(tgid))
        };
        stmts.push(quote!(
            #timer.init(rtfm::export::timer_create(#tid, #signo));
        ));
    }

    // register signal handlers
    // NOTE iterating analysis.channels instead of analysis.signals avoid referring to non-existent
    // (not codegen-ed) signal handlers
    for (&core, dispatchers) in &analysis.channels {
        let signals = &analysis.signals[&core];

        let Range { start, end } = signals.range();
        for priority in dispatchers.keys() {
            let rt = util::rt_ident(signals.map[priority]);

            stmts.push(quote!(
                rtfm::export::register(#start..#end, #priority, #rt);
            ));
        }

        // the timer handler may be its own signal handler
        if let Some(tq) = analysis.timer_queues.get(&core) {
            if !dispatchers.contains_key(&tq.priority) {
                let priority = tq.priority;
                let rt = util::rt_ident(signals.map[&priority]);

                stmts.push(quote!(
                    rtfm::export::register(#start..#end, #priority, #rt);
                ));
            }
        }
    }

    if app.args.cores > 1 {
        let tid = util::tid_ident(0);
        const_app.push(quote!(
            static #tid: rtfm::export::Pid = rtfm::export::Pid::uninit();
        ));

        stmts.push(quote!(
            #tid.init(tgid);
        ));
    }

    // initialize the other threads, their timers and the `TID`s
    for &core in &analysis.used_cores {
        if core == 0 {
            // we are core #0
            continue;
        }

        let tid = util::tid_ident(core);
        let child = util::child_ident(core);

        const_app.push(quote!(
            static #tid: rtfm::export::Pid = rtfm::export::Pid::uninit();
        ));

        stmts.push(quote!(
            let tid = rtfm::export::spawn(#child);
        ));

        // create timer
        if let Some(tq) = analysis.timer_queues.get(&core) {
            let timer = util::timer_ident(core);
            let signo = analysis.signals[&core].map[&tq.priority];
            stmts.push(quote!(
                #timer.init(rtfm::export::timer_create(Some(tid), #signo));
            ));
        }

        stmts.push(quote!(
            // migrate the thread to a different core
            rtfm::export::set_affinity(tid, #core);

            // unblock the thread
            #tid.init(tid);
        ));
    }

    (const_app, stmts)
}
