use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use rtfm_syntax::{ast::App, Context};

use crate::{
    analyze::Analysis,
    codegen::{locals, module, resources_struct},
};

pub fn codegen(
    app: &App,
    analysis: &Analysis,
) -> (
    // const_app_idle
    Vec<TokenStream2>,
    // mod_idle
    Vec<TokenStream2>,
    // idle_locals
    Vec<TokenStream2>,
    // idle_resources
    Vec<TokenStream2>,
    // user_idle
    Vec<TokenStream2>,
    // call_idle
    TokenStream2,
) {
    let mut const_app = vec![];
    let mut mod_idle = vec![];
    let mut idle_locals = vec![];
    let mut idle_resources = vec![];
    let mut user_idle = vec![];
    let mut call_idle = quote!(loop {
        rtfm::export::pause()
    });

    for (&core, idle) in &app.idles {
        let mut needs_lt = false;

        if !idle.args.resources.is_empty() {
            let (item, constructor) =
                resources_struct::codegen(Context::Idle(core), 0, &mut needs_lt, app, analysis);

            idle_resources.push(item);
            const_app.push(constructor);
        }

        let name = &idle.name;
        if core == 0 {
            call_idle = quote!(#name(
                #name::Locals::new(),
                #name::Context::new(&rtfm::export::Priority::new(0))
            ));
        }

        let attrs = &idle.attrs;
        let context = &idle.context;
        let (locals, locals_pat) = locals::codegen(Context::Idle(core), &idle.locals, app);
        idle_locals.push(locals);
        let stmts = &idle.stmts;
        user_idle.push(quote!(
            #(#attrs)*
            #[allow(non_snake_case)]
            fn #name(#locals_pat, #context: #name::Context) -> ! {
                use rtfm::Mutex as _;

                #(#stmts)*
            }
        ));

        mod_idle.push(module::codegen(
            Context::Idle(core),
            (!idle.args.resources.is_empty(), needs_lt),
            !idle.args.schedule.is_empty(),
            !idle.args.spawn.is_empty(),
            false,
            app,
        ));
    }

    (
        const_app,
        mod_idle,
        idle_locals,
        idle_resources,
        user_idle,
        call_idle,
    )
}
