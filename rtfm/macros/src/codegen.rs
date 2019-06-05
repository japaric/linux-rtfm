use proc_macro::TokenStream;

use quote::quote;
use rtfm_syntax::ast::App;

use crate::analyze::Analysis;

mod assertions;
mod childs;
mod dispatchers;
mod idle;
mod init;
mod locals;
mod module;
mod post_init;
mod pre_init;
mod resources;
mod resources_struct;
mod schedule;
mod schedule_body;
mod spawn;
mod spawn_body;
mod tasks;
mod timer_body;
mod timer_queue;
mod util;

pub fn app(app: &App, analysis: &Analysis) -> TokenStream {
    let assertion_stmts = assertions::codegen(analysis);

    let (const_app_pre_init, pre_init_stmts) = pre_init::codegen(app, analysis);

    let const_app_childs = childs::codegen(app, analysis);

    let (
        const_app_init,
        mod_init,
        init_locals,
        init_resources,
        init_late_resources,
        user_init,
        call_init,
    ) = init::codegen(app, analysis);

    let (const_app_post_init, post_init_stmts) = post_init::codegen(analysis);

    let (const_app_idle, mod_idle, idle_locals, idle_resources, user_idle, call_idle) =
        idle::codegen(app, analysis);

    let (const_app_resources, mod_resources) = resources::codegen(app, analysis);

    let (const_app_tasks, task_mods, task_locals, task_resources, user_tasks) =
        tasks::codegen(app, analysis);

    let const_app_dispatchers = dispatchers::codegen(app, analysis);

    let const_app_spawn = spawn::codegen(app, analysis);

    let const_app_tq = timer_queue::codegen(app, analysis);

    let const_app_schedule = schedule::codegen(app, analysis);

    let name = &app.name;
    quote!(
        #(#user_init)*

        #(#user_idle)*

        #mod_resources

        #(#user_tasks)*

        #(#init_locals)*

        #(#init_resources)*

        #(#init_late_resources)*

        #(#mod_init)*

        #(#idle_locals)*

        #(#idle_resources)*

        #(#mod_idle)*

        #(#task_locals)*

        #(#task_resources)*

        #(#task_mods)*

        /// Implementation details
        const #name: () = {
            #(#const_app_pre_init)*

            #(#const_app_childs)*

            #(#const_app_init)*

            #(#const_app_post_init)*

            #(#const_app_idle)*

            #(#const_app_resources)*

            #(#const_app_tasks)*

            #(#const_app_dispatchers)*

            #(#const_app_spawn)*

            #(#const_app_tq)*

            #(#const_app_schedule)*

            #[no_mangle]
            unsafe fn main() -> ! {
                #(#assertion_stmts)*

                #(#pre_init_stmts)*

                #call_init

                #(#post_init_stmts)*

                #call_idle
            }
        };
    )
    .into()
}
