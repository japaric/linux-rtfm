use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::ast::App;

use crate::{
    analyze::Analysis,
    codegen::{schedule_body, util},
};

pub fn codegen(app: &App, analysis: &Analysis) -> Vec<TokenStream2> {
    let mut items = vec![];

    let mut seen = BTreeSet::new();
    for (scheduler, schedulees) in app.schedule_callers() {
        if schedulees.is_empty() {
            continue;
        }

        let mut methods = vec![];

        for name in schedulees {
            let schedulee = &app.software_tasks[name];

            let (args, _, untupled, ty) = util::regroup_inputs(&schedulee.inputs);

            let cfgs = &schedulee.cfgs;

            let schedule = util::schedule_ident(name);
            if scheduler.is_init() {
                let body = schedule_body::codegen(scheduler, name, app, analysis);

                let args = args.clone();
                methods.push(quote!(
                    #(#cfgs)*
                    fn #name(&self, instant: rtfm::Instant #(,#args)*) -> Result<(), #ty> {
                        #body
                    }
                ));
            } else {
                if !seen.contains(name) {
                    seen.insert(name);

                    let body = schedule_body::codegen(scheduler, name, app, analysis);
                    let args = args.clone();

                    items.push(quote!(
                        #(#cfgs)*
                        fn #schedule(
                            priority: &rtfm::export::Priority,
                            instant: rtfm::Instant
                                #(,#args)*
                        ) -> Result<(), #ty> {
                            #body
                        }
                    ));
                }

                methods.push(quote!(
                    #(#cfgs)*
                    #[inline(always)]
                    fn #name(&self, instant: rtfm::Instant #(,#args)*) -> Result<(), #ty> {
                        let priority = unsafe { self.priority() };

                        #schedule(priority, instant #(,#untupled)*)
                    }
                ));
            }
        }

        let lt = if scheduler.is_init() {
            None
        } else {
            Some(quote!('a))
        };
        let scheduler = scheduler.ident(app);
        items.push(quote!(
            impl<#lt> #scheduler::Schedule<#lt> {
                #(#methods)*
            }
        ));
    }

    items
}
