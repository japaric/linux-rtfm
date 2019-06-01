use std::collections::BTreeSet;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::ast::App;

use crate::{
    analyze::Analysis,
    codegen::{spawn_body, util},
};

pub fn codegen(app: &App, analysis: &Analysis) -> Vec<TokenStream2> {
    let mut items = vec![];

    let mut seen = BTreeSet::new();
    for (spawner, spawnees) in app.spawn_callers() {
        let sender = spawner.core(app);
        let mut methods = vec![];
        let spawner_ident = spawner.ident(app);

        for name in spawnees {
            let spawnee = &app.software_tasks[name];
            let receiver = spawnee.args.core;
            let cfgs = &spawnee.cfgs;
            let (args, _, untupled, ty) = util::regroup_inputs(&spawnee.inputs);

            if spawner.is_init() {
                // `init` uses a special spawn implementation; it doesn't use the `spawn_${name}`
                // functions which are shared by other contexts

                let body = spawn_body::codegen(spawner, &name, app, analysis);

                let let_instant = if app.uses_schedule(sender) {
                    Some(quote!(let instant = rtfm::Instant::now();))
                } else {
                    None
                };
                methods.push(quote!(
                    #(#cfgs)*
                    fn #name(&self #(,#args)*) -> Result<(), #ty> {
                        #let_instant
                        #body
                    }
                ));
            } else {
                let spawn = util::spawn_ident(name);

                if !seen.contains(name) {
                    // generate a `spawn_${name}` function
                    seen.insert(name);

                    let instant = if app.uses_schedule(receiver) {
                        Some(quote!(, instant: rtfm::Instant))
                    } else {
                        None
                    };
                    let body = spawn_body::codegen(spawner, &name, app, analysis);
                    let args = args.clone();
                    items.push(quote!(
                        #(#cfgs)*
                        unsafe fn #spawn(
                            priority: &rtfm::export::Priority
                            #instant
                            #(,#args)*
                        ) -> Result<(), #ty> {
                            #body
                        }
                    ));
                }

                let (let_instant, instant) = if app.uses_schedule(receiver) {
                    (
                        Some(if spawner.is_idle() {
                            quote!(let instant = rtfm::Instant::now();)
                        } else {
                            quote!(let instant = self.instant();)
                        }),
                        Some(quote!(, instant)),
                    )
                } else {
                    (None, None)
                };

                methods.push(quote!(
                    #(#cfgs)*
                    #[inline(always)]
                    fn #name(&self #(,#args)*) -> Result<(), #ty> {
                        unsafe {
                            #let_instant
                            #spawn(self.priority() #instant #(,#untupled)*)
                        }
                    }
                ));
            }
        }

        let lt = if spawner.is_init() {
            None
        } else {
            Some(quote!('a))
        };
        items.push(quote!(
            impl<#lt> #spawner_ident::Spawn<#lt> {
                #(#methods)*
            }
        ));
    }

    items
}
