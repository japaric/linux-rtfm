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
    // task_mods
    Vec<TokenStream2>,
    // task_locals
    Vec<TokenStream2>,
    // task_resources
    Vec<TokenStream2>,
    // user_tasks
    Vec<TokenStream2>,
) {
    let mut const_app = vec![];
    let mut mods = vec![];
    let mut locals_structs = vec![];
    let mut resources_structs = vec![];
    let mut user_code = vec![];

    for (name, task) in &app.software_tasks {
        let core = task.args.core;
        let inputs = &task.inputs;

        if let Some(free_queues) = analysis.free_queues.get(name) {
            let (_, _, _, ty) = util::regroup_inputs(inputs);

            let cap = task.args.capacity * free_queues.len() as u8;
            let cap_lit = util::capacity_literal(cap);

            let elems = (0..cap)
                .map(|_| quote!(core::mem::MaybeUninit::uninit()))
                .collect::<Vec<_>>();

            if app.uses_schedule(core) {
                let task_instants = util::instants_ident(name);

                let elems = elems.clone();
                const_app.push(quote!(
                    /// Buffer that holds the instants associated to the inputs of a task
                    static mut #task_instants: [core::mem::MaybeUninit<rtfm::Instant>; #cap_lit] =
                        [#(#elems,)*];
                ));
            }

            let task_inputs = util::inputs_ident(name);
            const_app.push(quote!(
                /// Buffer that holds the inputs of a task
                static mut #task_inputs: [core::mem::MaybeUninit<#ty>; #cap_lit] =
                    [#(#elems,)*];
            ));

            let cap = task.args.capacity;
            let cap_ty = util::typenum_capacity(cap, true);
            for (&sender, ceiling) in free_queues {
                let task_fq = util::fq_ident_(name, sender);

                let doc = "Queue version of a free-list that keeps track of empty slots in the previous buffer(s)";
                let fq_ty = quote!(rtfm::export::FreeQueue<#cap_ty>);
                const_app.push(quote!(
                    #[doc = #doc]
                    static mut #task_fq: #fq_ty = unsafe {
                        rtfm::export::Queue(rtfm::export::iQueue::u8_sc())
                    };
                ));
                let ptr = quote!(&mut #task_fq);

                if let Some(ceil) = ceiling {
                    const_app.push(quote!(struct #task_fq<'a> {
                        priority: &'a rtfm::export::Priority,
                    }));

                    let range = analysis.signals[&core].range();
                    const_app.push(util::impl_mutex(
                        &[],
                        false,
                        &task_fq,
                        fq_ty,
                        *ceil,
                        range.clone(),
                        ptr,
                    ));
                }
            }
        } else {
            // this task is never spawned / scheduled so about generating buffers
        }

        let mut needs_lt = false;
        if !task.args.resources.is_empty() {
            let (item, constructor) = resources_struct::codegen(
                Context::SoftwareTask(name),
                task.args.priority,
                &mut needs_lt,
                app,
                analysis,
            );

            resources_structs.push(item);

            const_app.push(constructor);
        }
        mods.push(module::codegen(
            Context::SoftwareTask(name),
            (!task.args.resources.is_empty(), needs_lt),
            !task.args.schedule.is_empty(),
            !task.args.spawn.is_empty(),
            false,
            app,
        ));

        let attrs = &task.attrs;
        let context = &task.context;
        let stmts = &task.stmts;
        let (locals_struct, locals_pat) =
            locals::codegen(Context::SoftwareTask(name), &task.locals, app);
        locals_structs.push(locals_struct);
        user_code.push(quote!(
            #(#attrs)*
            #[allow(non_snake_case)]
            fn #name(#locals_pat, #context: #name::Context #(,#inputs)*) {
                use rtfm::Mutex as _;

                #(#stmts)*
            }
        ));
    }

    (
        const_app,
        mods,
        locals_structs,
        resources_structs,
        user_code,
    )
}
