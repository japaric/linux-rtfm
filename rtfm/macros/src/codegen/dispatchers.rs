use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::ast::App;

use crate::{
    analyze::Analysis,
    codegen::{timer_body, util},
};

pub fn codegen(app: &App, analysis: &Analysis) -> Vec<TokenStream2> {
    let mut items = vec![];

    for (&receiver, dispatchers) in &analysis.channels {
        let signals = &analysis.signals[&receiver];

        for (&level, channels) in dispatchers {
            let variants = channels
                .iter()
                .flat_map(|(&sender, channel)| {
                    channel.tasks.iter().map(move |name| {
                        let cfgs = &app.software_tasks[name].cfgs;
                        let task = util::task_ident(name, sender);

                        quote!(
                            #(#cfgs)*
                            #task
                        )
                    })
                })
                .collect::<Vec<_>>();

            let t = util::spawn_t_ident(receiver, level);
            let doc = format!(
                "Software tasks to be dispatched by core #{} at priority level {}",
                receiver, level
            );
            items.push(quote!(
                #[allow(non_camel_case_types)]
                #[derive(Clone, Copy)]
                #[doc = #doc]
                #[repr(u8)]
                enum #t {
                    #(#variants,)*
                }
            ));

            let arms = channels
                .iter()
                .flat_map(|(&sender, channel)| {
                    let t = &t;
                    channel.tasks.iter().map(move |name| {
                        let task = &app.software_tasks[name];
                        let cfgs = &task.cfgs;
                        let (_, tupled, pats, _) = util::regroup_inputs(&task.inputs);

                        let inputs = util::inputs_ident(name);
                        let fq = util::fq_ident_(name, sender);

                        let input =
                            quote!(#inputs.get_unchecked(usize::from(index)).as_ptr().read());

                        let (let_instant, instant) = if app.uses_schedule(receiver) {
                            let instants = util::instants_ident(name);
                            let instant =
                                quote!(#instants.get_unchecked(usize::from(index)).as_ptr().read());

                            (
                                Some(quote!(let instant = #instant;)),
                                Some(quote!(, instant)),
                            )
                        } else {
                            (None, None)
                        };

                        let call = {
                            let pats = pats.clone();

                            quote!(
                                #name(
                                    #name::Locals::new(),
                                    #name::Context::new(priority #instant)
                                    #(,#pats)*
                                )
                            )
                        };

                        let t = t.clone();
                        let variant = util::task_ident(name, sender);
                        quote!(
                            #(#cfgs)*
                            #t::#variant => {
                                let #tupled = #input;
                                #let_instant
                                #fq.split().0.enqueue_unchecked(index);
                                let priority = &rtfm::export::Priority::new(PRIORITY);
                                #call
                            }
                        )
                    })
                })
                .collect::<Vec<_>>();

            let handler = util::rt_ident(signals.map[&level]);
            if analysis
                .timer_queues
                .get(&receiver)
                .map(|tq| tq.priority == level)
                .unwrap_or(false)
            {
                let doc = format!("Priority {} task dispatcher & timer queue handler", level);

                let tq =
                    timer_body::codegen(receiver, &analysis.timer_queues[&receiver], app, analysis);
                items.push(quote!(
                    #[allow(non_snake_case)]
                    #[doc = #doc]
                    extern "C" fn #handler(
                        _: i32,
                        si: &mut rtfm::export::siginfo_t,
                        _: *mut rtfm::export::c_void,
                    ) {
                        unsafe {
                            use rtfm::Mutex as _;

                            /// The priority of this interrupt handler
                            const PRIORITY: u8 = #level;

                            if si.si_code == rtfm::export::SI_QUEUE {
                                let task: #t = core::mem::transmute((si.si_value >> 8) as u8);
                                let index = (si.si_value & 0xff) as u8;
                                match task {
                                    #(#arms)*
                                }
                            } else {
                                #tq
                            }
                        }
                    }
                ));
            } else {
                let doc = format!("Priority {} task dispatcher", level);
                items.push(quote!(
                    #[allow(non_snake_case)]
                    #[doc = #doc]
                    extern "C" fn #handler(
                        _: i32,
                        si: &mut rtfm::export::siginfo_t,
                        _: *mut rtfm::export::c_void,
                    ) {
                        unsafe {
                            /// The priority of this interrupt handler
                            const PRIORITY: u8 = #level;

                            let task: #t = core::mem::transmute((si.si_value >> 8) as u8);
                            let index = (si.si_value & 0xff) as u8;
                            match task {
                                #(#arms)*
                            }
                        }
                    }
                ));
            }
        }

        // the timer queue handler may be a separate signal handler
        if let Some(timer_queue) = analysis.timer_queues.get(&receiver) {
            let priority = timer_queue.priority;

            if !dispatchers.contains_key(&priority) {
                let handler = util::rt_ident(signals.map[&priority]);
                let tqh =
                    timer_body::codegen(receiver, &analysis.timer_queues[&receiver], app, analysis);
                items.push(quote!(
                    /// Timer queue handler
                    #[allow(non_snake_case)]
                    extern "C" fn #handler(
                        _: i32,
                        si: &mut rtfm::export::siginfo_t,
                        _: *mut rtfm::export::c_void,
                    ) {
                        unsafe {
                            use rtfm::Mutex as _;

                            /// The priority of this interrupt handler
                            const PRIORITY: u8 = #priority;

                            #tqh
                        }
                    }
                ));
            }
        }
    }

    items
}
