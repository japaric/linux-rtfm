use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::{ast::App, Context};

use crate::{
    analyze::Analysis,
    codegen::{locals, module, resources_struct, util},
};

pub fn codegen(
    app: &App,
    analysis: &Analysis,
) -> (
    // const_app
    Vec<TokenStream2>,
    // mod_init
    Vec<TokenStream2>,
    // init_locals
    Vec<TokenStream2>,
    // init_resources
    Vec<TokenStream2>,
    // init_late_resources
    Vec<TokenStream2>,
    // user_init
    Vec<TokenStream2>,
    // call_init
    Option<TokenStream2>,
) {
    let mut call_init = None;
    let mut const_app = vec![];
    let mut init_late_resources = vec![];
    let mut init_resources = vec![];
    let mut locals_struct = vec![];
    let mut mod_init = vec![];
    let mut user_init = vec![];

    for (&core, init) in &app.inits {
        let mut needs_lt = false;
        let name = &init.name;

        if !init.args.resources.is_empty() {
            let (item, constructor) =
                resources_struct::codegen(Context::Init(core), 0, &mut needs_lt, app, analysis);

            init_resources.push(item);
            const_app.push(constructor);
        }

        if core == 0 {
            call_init =
                Some(quote!(let late = #name(#name::Locals::new(), #name::Context::new());));
        }

        let late_fields = analysis
            .late_resources
            .get(&core)
            .map(|resources| {
                resources
                    .iter()
                    .map(|name| {
                        let ty = &app.late_resources[name].ty;

                        quote!(pub #name: #ty)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or(vec![]);

        let attrs = &init.attrs;
        let has_late_resources = !late_fields.is_empty();
        let late_resources = util::late_resources_ident(&name);
        let ret = if has_late_resources {
            init_late_resources.push(quote!(
                /// Resources initialized at runtime
                #[allow(non_snake_case)]
                pub struct #late_resources {
                    #(#late_fields),*
                }
            ));

            Some(quote!(-> #name::LateResources))
        } else {
            None
        };

        let context = &init.context;
        let (struct_, locals_pat) = locals::codegen(Context::Init(core), &init.locals, app);
        locals_struct.push(struct_);
        let stmts = &init.stmts;
        user_init.push(quote!(
            #(#attrs)*
            #[allow(non_snake_case)]
            fn #name(#locals_pat, #context: #name::Context) #ret {
                #(#stmts)*
            }
        ));

        mod_init.push(module::codegen(
            Context::Init(core),
            (!init.args.resources.is_empty(), needs_lt),
            !init.args.schedule.is_empty(),
            !init.args.spawn.is_empty(),
            has_late_resources,
            app,
        ));
    }

    (
        const_app,
        mod_init,
        locals_struct,
        init_resources,
        init_late_resources,
        user_init,
        call_init,
    )
}
