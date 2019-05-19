use proc_macro::TokenStream;
use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Span;
use quote::quote;
use syn::{ArgCaptured, Attribute, Ident, IntSuffix, LitInt};

use crate::{
    analyze::{Analysis, Ownership},
    syntax::{App, Static},
};

pub fn app(name: &Ident, app: &App, analysis: &Analysis) -> TokenStream {
    let (const_app_resources, mod_resources) = resources(app, analysis);

    let (const_app_tasks, task_mods, task_locals, task_resources, user_tasks) =
        tasks(app, analysis);

    let const_app_dispatchers = dispatchers(&app, analysis);

    let const_app_spawn = spawn(app, analysis);

    let assertion_stmts = assertions(app, analysis);

    let pre_init_stmts = pre_init(&app, analysis);

    let (
        const_app_init,
        mod_init,
        init_locals,
        init_resources,
        init_late_resources,
        user_init,
        call_init,
    ) = init(app, analysis);

    let post_init_stmts = post_init(&app, analysis);

    quote!(
        #user_init

        #(#user_tasks)*

        #mod_resources

        #init_locals

        #init_resources

        #init_late_resources

        #mod_init

        #(#task_locals)*

        #(#task_resources)*

        #(#task_mods)*

        /// Implementation details
        const #name: () = {
            #(#const_app_resources)*

            #const_app_init

            #(#const_app_dispatchers)*

            #(#const_app_tasks)*

            #(#const_app_spawn)*

            static mut PID: i32 = 0;

            #[no_mangle]
            unsafe fn main() -> ! {
                #(#assertion_stmts)*

                #(#pre_init_stmts)*

                #call_init

                #(#post_init_stmts)*
            }
        };
    )
    .into()
}

/* Main functions */
/// In this pass we generate a static variable and a resource proxy for each resource
///
/// If the user specified a resource like this:
///
/// ```
/// #[rtfm::app(device = ..)]
/// const APP: () = {
///     static mut X: UserDefinedStruct = ();
///     static mut Y: u64 = 0;
///     static mut Z: u32 = 0;
/// }
/// ```
///
/// We'll generate code like this:
///
/// - `const_app`
///
/// ```
/// const APP: () = {
///     static mut X: MaybeUninit<UserDefinedStruct> = MaybeUninit::uninit();
///     static mut Y: u64 = 0;
///     static mut Z: u32 = 0;
///
///     impl<'a> Mutex for resources::X<'a> { .. }
///
///     impl<'a> Mutex for resources::Y<'a> { .. }
///
///     // but not for `Z` because it's not shared and thus requires no proxy
/// };
/// ```
///
/// - `mod_resources`
///
/// ```
/// mod resources {
///     pub struct X<'a> {
///         priority: &'a Priority,
///     }
///
///     impl<'a> X<'a> {
///         pub unsafe fn new(priority: &'a Priority) -> Self {
///             X { priority }
///         }
///
///         pub unsafe fn priority(&self) -> &Priority {
///             self.priority
///         }
///     }
///
///     // same thing for `Y`
///
///     // but not for `Z`
/// }
/// ```
fn resources(
    app: &App,
    analysis: &Analysis,
) -> (
    // const_app
    Vec<proc_macro2::TokenStream>,
    // mod_resources
    proc_macro2::TokenStream,
) {
    let mut const_app = vec![];
    let mut mod_resources = vec![];

    for (name, res) in &app.resources {
        let cfgs = &res.cfgs;
        let attrs = &res.attrs;
        let ty = &res.ty;

        if let Some(expr) = res.expr.as_ref() {
            const_app.push(quote!(
                #(#attrs)*
                #(#cfgs)*
                static mut #name: #ty = #expr;
            ));
        } else {
            const_app.push(quote!(
                #(#attrs)*
                #(#cfgs)*
                static mut #name: rtfm::export::MaybeUninit<#ty> =
                    rtfm::export::MaybeUninit::uninit();
            ));
        }

        // generate a resource proxy when needed
        if res.mutability.is_some() {
            if let Some(Ownership::Shared { ceiling }) = analysis.ownerships.get(name) {
                let ptr = if res.expr.is_none() {
                    quote!(#name.as_mut_ptr())
                } else {
                    quote!(&mut #name)
                };

                mod_resources.push(quote!(
                    pub struct #name<'a> {
                        priority: &'a Priority,
                    }

                    impl<'a> #name<'a> {
                        #[inline(always)]
                        pub unsafe fn new(priority: &'a Priority) -> Self {
                            #name { priority }
                        }

                        #[inline(always)]
                        pub unsafe fn priority(&self) -> &Priority {
                            self.priority
                        }
                    }
                ));

                const_app.push(impl_mutex(
                    app,
                    cfgs,
                    true,
                    name,
                    quote!(#ty),
                    *ceiling,
                    ptr,
                ));
            }
        }
    }

    let mod_resources = if mod_resources.is_empty() {
        quote!()
    } else {
        quote!(mod resources {
            use rtfm::export::Priority;

            #(#mod_resources)*
        })
    };

    (const_app, mod_resources)
}

// For each task we'll generate:
//
// - at the root of the crate:
//   - a ${name}Resources struct (maybe)
//   - a ${name}Locals struct
//
// - a module named after the task, see the `module` function for more details
//
// - hidden in `const APP`
//   - the ${name}Resources constructor
//   - an INPUTS buffer
//   - a free queue and a corresponding resource
//   - an INSTANTS buffer (if `timer-queue` is enabled)
//
// - the task handler specified by the user
fn tasks(
    app: &App,
    analysis: &Analysis,
) -> (
    // const_app
    Vec<proc_macro2::TokenStream>,
    // task_mods
    Vec<proc_macro2::TokenStream>,
    // task_locals
    Vec<proc_macro2::TokenStream>,
    // task_resources
    Vec<proc_macro2::TokenStream>,
    // user_tasks
    Vec<proc_macro2::TokenStream>,
) {
    let mut const_app = vec![];
    let mut mods = vec![];
    let mut locals_structs = vec![];
    let mut resources_structs = vec![];
    let mut user_code = vec![];

    for (name, task) in &app.tasks {
        let inputs = &task.inputs;
        let (_, _, _, ty) = regroup_inputs(inputs);

        let cap = analysis.capacities[name];
        let cap_lit = mk_capacity_literal(cap);
        let cap_ty = mk_typenum_capacity(cap, true);

        let task_inputs = mk_inputs_ident(name);
        let task_instants = mk_instants_ident(name);
        let task_fq = mk_fq_ident(name);

        let elems = (0..cap)
            .map(|_| quote!(rtfm::export::MaybeUninit::uninit()))
            .collect::<Vec<_>>();

        if cfg!(feature = "timer-queue") {
            let elems = elems.clone();
            const_app.push(quote!(
                /// Buffer that holds the instants associated to the inputs of a task
                static mut #task_instants: [rtfm::export::MaybeUninit<rtfm::Instant>; #cap_lit] =
                    [#(#elems,)*];
            ));
        }

        const_app.push(quote!(
            /// Buffer that holds the inputs of a task
            static mut #task_inputs: [rtfm::export::MaybeUninit<#ty>; #cap_lit] =
                [#(#elems,)*];
        ));

        let doc = "Queue version of a free-list that keeps track of empty slots in the previous buffer(s)";
        let fq_ty = quote!(rtfm::export::FreeQueue<#cap_ty>);
        const_app.push(quote!(
            #[doc = #doc]
            static mut #task_fq: #fq_ty = unsafe {
                rtfm::export::Queue(rtfm::export::iQueue::u8_sc())
            };
        ));
        let ptr = quote!(&mut #task_fq);

        if let Some(ceiling) = analysis.free_queues.get(name) {
            const_app.push(quote!(struct #task_fq<'a> {
                priority: &'a rtfm::export::Priority,
            }));

            const_app.push(impl_mutex(app, &[], false, &task_fq, fq_ty, *ceiling, ptr));
        }

        let mut needs_lt = false;
        if !task.args.resources.is_empty() {
            let (item, constructor) = resources_struct(
                Kind::Task(name.clone()),
                task.args.priority,
                &mut needs_lt,
                app,
                analysis,
            );

            resources_structs.push(item);

            const_app.push(constructor);
        }

        mods.push(module(
            Kind::Task(name.clone()),
            (!task.args.resources.is_empty(), needs_lt),
            !task.args.spawn.is_empty(),
            false,
            app,
        ));

        let attrs = &task.attrs;
        let use_u32ext = if cfg!(feature = "timer-queue") {
            Some(quote!(
                use rtfm::U32Ext as _;
            ))
        } else {
            None
        };
        let context = &task.context;
        let stmts = &task.stmts;
        let (locals_struct, lets) = locals(Kind::Task(name.clone()), &task.statics);
        locals_structs.push(locals_struct);
        user_code.push(quote!(
            #(#attrs)*
            #[allow(non_snake_case)]
            fn #name(__locals: #name::Locals, #context: #name::Context #(,#inputs)*) {
                use rtfm::Mutex as _;
                #use_u32ext

                #(#lets;)*

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

/// For each task dispatcher we'll generate
///
/// - A static variable that hold the ready queue (`RQ${priority}`) and a resource proxy for it
/// - An enumeration of all the tasks dispatched by this dispatcher `T${priority}`
/// - An interrupt handler that dispatches the tasks
fn dispatchers(app: &App, analysis: &Analysis) -> Vec<proc_macro2::TokenStream> {
    let mut items = vec![];

    // let device = &app.args.device;
    for (level, dispatcher) in &analysis.dispatchers {
        // let rq = mk_rq_ident(*level);
        let t = mk_t_ident(*level);
        let cap = mk_typenum_capacity(dispatcher.capacity, true);

        let doc = format!(
            "Queue of tasks ready to be dispatched at priority level {}",
            level
        );

        let variants = dispatcher
            .tasks
            .iter()
            .map(|task| {
                let cfgs = &app.tasks[task].cfgs;

                quote!(
                    #(#cfgs)*
                    #task
                )
            })
            .collect::<Vec<_>>();

        let doc = format!(
            "Software tasks to be dispatched at priority level {}",
            level
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

        let arms = dispatcher
            .tasks
            .iter()
            .map(|name| {
                let task = &app.tasks[name];
                let cfgs = &task.cfgs;
                let (_, tupled, pats, _) = regroup_inputs(&task.inputs);

                let inputs = mk_inputs_ident(name);
                let fq = mk_fq_ident(name);

                let input = quote!(#inputs.get_unchecked(usize::from(index)).read());
                let fq = quote!(#fq);

                let (let_instant, _instant) = if cfg!(feature = "timer-queue") {
                    let instants = mk_instants_ident(name);
                    let instant = quote!(#instants.get_unchecked(usize::from(index)).read());

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
                            #name::Context::new(priority #_instant)
                            #(,#pats)*
                        )
                    )
                };

                quote!(
                    #(#cfgs)*
                    #t::#name => {
                        let #tupled = #input;
                        #let_instant
                        #fq.split().0.enqueue_unchecked(index);
                        let priority = &rtfm::export::Priority::new(PRIORITY);
                        #call
                    }
                )
            })
            .collect::<Vec<_>>();

        let doc = format!("Priority {} task dispatcher", level);
        // let attrs = &dispatcher.attrs;
        let handler = mk_rt_ident(*level);
        // let interrupt = &dispatcher.interrupt;
        // let rq = quote!((&mut #rq));
        items.push(quote!(
            #[doc = #doc]
            // #(#attrs)*
            // #[no_mangle]
            // #[allow(non_snake_case)]
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

    items
}

/// Generates all the `Spawn.$task` related code
fn spawn(app: &App, analysis: &Analysis) -> Vec<proc_macro2::TokenStream> {
    let mut items = vec![];

    let mut seen = BTreeSet::new();
    for (spawner, spawnees) in app.spawn_callers() {
        if spawnees.is_empty() {
            continue;
        }

        let mut methods = vec![];

        let spawner_is_init = spawner == "init";
        let spawner_is_idle = spawner == "idle";
        for name in spawnees {
            let spawnee = &app.tasks[name];
            let cfgs = &spawnee.cfgs;
            let (args, _, untupled, ty) = regroup_inputs(&spawnee.inputs);

            if spawner_is_init {
                // `init` uses a special spawn implementation; it doesn't use the `spawn_${name}`
                // functions which are shared by other contexts

                let body = mk_spawn_body(&spawner, &name, app, analysis);

                let let_instant = if cfg!(feature = "timer-queue") {
                    Some(quote!(let instant = unsafe { rtfm::Instant::artificial(0) };))
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
                let spawn = mk_spawn_ident(name);

                if !seen.contains(name) {
                    // generate a `spawn_${name}` function
                    seen.insert(name);

                    let instant = if cfg!(feature = "timer-queue") {
                        Some(quote!(, instant: rtfm::Instant))
                    } else {
                        None
                    };
                    let body = mk_spawn_body(&spawner, &name, app, analysis);
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

                let (let_instant, instant) = if cfg!(feature = "timer-queue") {
                    (
                        Some(if spawner_is_idle {
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

        let lt = if spawner_is_init {
            None
        } else {
            Some(quote!('a))
        };
        items.push(quote!(
            impl<#lt> #spawner::Spawn<#lt> {
                #(#methods)*
            }
        ));
    }

    items
}

/// Generates `Send` / `Sync` compile time checks
fn assertions(app: &App, analysis: &Analysis) -> Vec<proc_macro2::TokenStream> {
    let mut stmts = vec![];

    for ty in &analysis.assert_sync {
        stmts.push(quote!(rtfm::export::assert_sync::<#ty>();));
    }

    for task in &analysis.tasks_assert_send {
        let (_, _, _, ty) = regroup_inputs(&app.tasks[task].inputs);
        stmts.push(quote!(rtfm::export::assert_send::<#ty>();));
    }

    // all late resources need to be `Send`
    for ty in &analysis.resources_assert_send {
        stmts.push(quote!(rtfm::export::assert_send::<#ty>();));
    }

    stmts
}

/// Generates code that we must run before `init` runs. See comments inside
fn pre_init(app: &App, analysis: &Analysis) -> Vec<proc_macro2::TokenStream> {
    let mut stmts = vec![];

    // initialize the `PID`
    stmts.push(quote!(
        PID = rtfm::export::getpid();
    ));

    // populate the `FreeQueue`s
    for name in app.tasks.keys() {
        let fq = mk_fq_ident(name);
        let cap = analysis.capacities[name];

        stmts.push(quote!(
            for i in 0..#cap {
                #fq.enqueue_unchecked(i);
            }
        ));
    }

    // register signal handlers
    for priority in analysis.dispatchers.keys() {
        let dispatcher = mk_rt_ident(*priority);

        stmts.push(quote!(
            rtfm::export::register(#priority, #dispatcher);
        ));
    }

    stmts.push(quote!(rtfm::export::init_scheduler();));

    stmts
}

// This generates
//
// - at the root of the crate
//  - a initResources struct (maybe)
//  - a initLateResources struct (maybe)
//  - a initLocals struct
//
// - an `init` module that contains
//   - the `Context` struct
//   - a re-export of the initResources struct
//   - a re-export of the initLateResources struct
//   - a re-export of the initLocals struct
//   - the Spawn struct (maybe)
//   - the Schedule struct (maybe, if `timer-queue` is enabled)
//
// - hidden in `const APP`
//   - the initResources constructor
//
// - the user specified `init` function
//
// - a call to the user specified `init` function
fn init(
    app: &App,
    analysis: &Analysis,
) -> (
    // const_app
    Option<proc_macro2::TokenStream>,
    // mod_init
    proc_macro2::TokenStream,
    // init_locals
    proc_macro2::TokenStream,
    // init_resources
    Option<proc_macro2::TokenStream>,
    // init_late_resources
    Option<proc_macro2::TokenStream>,
    // user_init
    proc_macro2::TokenStream,
    // call_init
    proc_macro2::TokenStream,
) {
    let mut needs_lt = false;
    let mut const_app = None;
    let mut init_resources = None;
    if !app.init.args.resources.is_empty() {
        let (item, constructor) = resources_struct(Kind::Init, 0, &mut needs_lt, app, analysis);

        init_resources = Some(item);
        const_app = Some(constructor);
    }

    let call_init = quote!(let late = init(init::Locals::new(), init::Context::new()););

    let late_fields = app
        .resources
        .iter()
        .filter_map(|(name, res)| {
            if res.expr.is_none() {
                let ty = &res.ty;

                Some(quote!(pub #name: #ty))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let attrs = &app.init.attrs;
    let has_late_resources = !late_fields.is_empty();
    let (ret, init_late_resources) = if has_late_resources {
        (
            Some(quote!(-> init::LateResources)),
            Some(quote!(
                /// Resources initialized at runtime
                #[allow(non_snake_case)]
                pub struct initLateResources {
                    #(#late_fields),*
                }
            )),
        )
    } else {
        (None, None)
    };
    let context = &app.init.context;
    let use_u32ext = if cfg!(feature = "timer-queue") {
        Some(quote!(
            use rtfm::U32Ext as _;
        ))
    } else {
        None
    };
    let (locals_struct, lets) = locals(Kind::Init, &app.init.statics);
    let stmts = &app.init.stmts;
    let user_init = quote!(
        #(#attrs)*
        #[allow(non_snake_case)]
        fn init(__locals: init::Locals, #context: init::Context) #ret {
            #use_u32ext

            #(#lets;)*

            #(#stmts)*
        }
    );

    let mod_init = module(
        Kind::Init,
        (!app.init.args.resources.is_empty(), needs_lt),
        !app.init.args.spawn.is_empty(),
        has_late_resources,
        app,
    );

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

/// Generates code that we must run after `init` returns. See comments inside
fn post_init(app: &App, analysis: &Analysis) -> Vec<proc_macro2::TokenStream> {
    let mut stmts = vec![];

    // initialize late resources
    for (name, res) in &app.resources {
        if res.expr.is_some() {
            continue;
        }

        stmts.push(quote!(#name.write(late.#name);));
    }

    stmts.push(quote!(
        // `interrupt::enable()`
        rtfm::export::set_priority(0);

        rtfm::export::exit(0);
        // we'll use this when we have a timer queue
        // loop {
        //     rtfm::export::pause();
        // }
    ));

    stmts
}

// If the user specified `idle` this generates
//
// - at the root of the crate
//  - an idleResources struct (maybe)
//  - an idleLocals struct
//
// - an `init` module that contains
//   - the `Context` struct
//   - a re-export of the idleResources struct
//   - a re-export of the idleLocals struct
//   - the Spawn struct (maybe)
//
// - hidden in `const APP`
//   - the idleResources constructor
//
// - the user specified `idle` function
//
// - a call to the user specified `idle` function
//
// Otherwise it uses `loop { WFI }` as `idle`
fn idle(
    app: &App,
    analysis: &Analysis,
) -> (
    // const_app_idle
    Option<proc_macro2::TokenStream>,
    // mod_idle
    Option<proc_macro2::TokenStream>,
    // idle_locals
    Option<proc_macro2::TokenStream>,
    // idle_resources
    Option<proc_macro2::TokenStream>,
    // user_idle
    Option<proc_macro2::TokenStream>,
    // call_idle
    proc_macro2::TokenStream,
) {
    if let Some(idle) = app.idle.as_ref() {
        let mut needs_lt = false;
        let mut const_app = None;
        let mut idle_resources = None;

        if !idle.args.resources.is_empty() {
            let (item, constructor) = resources_struct(Kind::Idle, 0, &mut needs_lt, app, analysis);

            idle_resources = Some(item);
            const_app = Some(constructor);
        }

        let call_idle = quote!(idle(
            idle::Locals::new(),
            idle::Context::new(&rtfm::export::Priority::new(0))
        ));

        let attrs = &idle.attrs;
        let context = &idle.context;
        let use_u32ext = if cfg!(feature = "timer-queue") {
            Some(quote!(
                use rtfm::U32Ext as _;
            ))
        } else {
            None
        };
        let (idle_locals, lets) = locals(Kind::Idle, &idle.statics);
        let stmts = &idle.stmts;
        let user_idle = quote!(
            #(#attrs)*
            #[allow(non_snake_case)]
            fn idle(__locals: idle::Locals, #context: idle::Context) {
                #use_u32ext
                use rtfm::Mutex as _;

                #(#lets;)*

                #(#stmts)*
            }
        );

        let mod_idle = module(
            Kind::Idle,
            (!idle.args.resources.is_empty(), needs_lt),
            !idle.args.spawn.is_empty(),
            false,
            app,
        );

        (
            const_app,
            Some(mod_idle),
            Some(idle_locals),
            idle_resources,
            Some(user_idle),
            call_idle,
        )
    } else {
        (None, None, None, None, None, quote!())
    }
}

/* Support functions */
/// This function creates the `Resources` struct
///
/// It's a bit unfortunate but this struct has to be created in the root because it refers to types
/// which may have been imported into the root.
fn resources_struct(
    kind: Kind,
    priority: u8,
    needs_lt: &mut bool,
    app: &App,
    analysis: &Analysis,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let mut lt = None;

    let resources = match &kind {
        Kind::Init => &app.init.args.resources,
        Kind::Idle => &app.idle.as_ref().expect("UNREACHABLE").args.resources,
        // Kind::Interrupt(name) => &app.interrupts[name].args.resources,
        // Kind::Exception(name) => &app.exceptions[name].args.resources,
        Kind::Task(name) => &app.tasks[name].args.resources,
    };

    let mut fields = vec![];
    let mut values = vec![];
    for name in resources {
        let res = &app.resources[name];

        let cfgs = &res.cfgs;
        let mut_ = res.mutability;
        let ty = &res.ty;

        if kind.is_init() {
            if !analysis.ownerships.contains_key(name) {
                // owned by `init`
                fields.push(quote!(
                    #(#cfgs)*
                    pub #name: &'static #mut_ #ty
                ));

                values.push(quote!(
                    #(#cfgs)*
                    #name: &#mut_ #name
                ));
            } else {
                // owned by someone else
                lt = Some(quote!('a));

                fields.push(quote!(
                    #(#cfgs)*
                    pub #name: &'a mut #ty
                ));

                values.push(quote!(
                    #(#cfgs)*
                    #name: &mut #name
                ));
            }
        } else {
            let ownership = &analysis.ownerships[name];

            if ownership.needs_lock(priority) {
                if mut_.is_none() {
                    lt = Some(quote!('a));

                    fields.push(quote!(
                        #(#cfgs)*
                        pub #name: &'a #ty
                    ));
                } else {
                    // resource proxy
                    lt = Some(quote!('a));

                    fields.push(quote!(
                        #(#cfgs)*
                        pub #name: resources::#name<'a>
                    ));

                    values.push(quote!(
                        #(#cfgs)*
                        #name: resources::#name::new(priority)
                    ));

                    continue;
                }
            } else {
                let lt = if kind.runs_once() {
                    quote!('static)
                } else {
                    lt = Some(quote!('a));
                    quote!('a)
                };

                if ownership.is_owned() || mut_.is_none() {
                    fields.push(quote!(
                        #(#cfgs)*
                        pub #name: &#lt #mut_ #ty
                    ));
                } else {
                    fields.push(quote!(
                        #(#cfgs)*
                        pub #name: &#lt mut #ty
                    ));
                }
            }

            let is_late = res.expr.is_none();
            if is_late {
                let expr = if mut_.is_some() {
                    quote!(&mut *#name.as_mut_ptr())
                } else {
                    quote!(&*#name.as_ptr())
                };

                values.push(quote!(
                    #(#cfgs)*
                    #name: #expr
                ));
            } else {
                values.push(quote!(
                    #(#cfgs)*
                    #name: &#mut_ #name
                ));
            }
        }
    }

    if lt.is_some() {
        *needs_lt = true;

        // the struct could end up empty due to `cfg` leading to an error due to `'a` being unused
        fields.push(quote!(
            #[doc(hidden)]
            pub __marker__: core::marker::PhantomData<&'a ()>
        ));

        values.push(quote!(__marker__: core::marker::PhantomData))
    }

    let ident = kind.resources_ident();
    let doc = format!("Resources {} has access to", ident);
    let item = quote!(
        #[allow(non_snake_case)]
        #[doc = #doc]
        pub struct #ident<#lt> {
            #(#fields,)*
        }
    );
    let arg = if kind.is_init() {
        None
    } else {
        Some(quote!(priority: &#lt rtfm::export::Priority))
    };
    let constructor = quote!(
        impl<#lt> #ident<#lt> {
            #[inline(always)]
            unsafe fn new(#arg) -> Self {
                #ident {
                    #(#values,)*
                }
            }
        }
    );
    (item, constructor)
}

/// Creates a `Mutex` implementation
fn impl_mutex(
    app: &App,
    cfgs: &[Attribute],
    resources_prefix: bool,
    name: &Ident,
    ty: proc_macro2::TokenStream,
    ceiling: u8,
    ptr: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let path = if resources_prefix {
        quote!(resources::#name)
    } else {
        quote!(#name)
    };

    let priority = if resources_prefix {
        quote!(self.priority())
    } else {
        quote!(self.priority)
    };

    // let device = &app.args.device;
    quote!(
        #(#cfgs)*
        impl<'a> rtfm::Mutex for #path<'a> {
            type T = #ty;

            #[inline(always)]
            fn lock<R>(&mut self, f: impl FnOnce(&mut #ty) -> R) -> R {
                /// Priority ceiling
                const CEILING: u8 = #ceiling;

                unsafe {
                    rtfm::export::lock(
                        #ptr,
                        #priority,
                        CEILING,
                        // #device::NVIC_PRIO_BITS,
                        f,
                    )
                }
            }
        }
    )
}

/// Creates a `Locals` struct and related code. This returns
///
/// - `locals`
///
/// ```
/// pub struct Locals<'a> {
///     #[cfg(never)]
///     pub X: &'a mut X,
///     __marker__: PhantomData<&'a mut ()>,
/// }
/// ```
///
/// - `lt`
///
/// ```
/// 'a
/// ```
///
/// - `lets`
///
/// ```
/// #[cfg(never)]
/// let X = __locals.X
/// ```
fn locals(
    kind: Kind,
    statics: &BTreeMap<Ident, Static>,
) -> (
    // locals
    proc_macro2::TokenStream,
    // lets
    Vec<proc_macro2::TokenStream>,
) {
    let runs_once = kind.runs_once();
    let ident = kind.locals_ident();

    let mut lt = None;
    let mut fields = vec![];
    let mut lets = vec![];
    let mut items = vec![];
    let mut values = vec![];
    for (name, static_) in statics {
        let lt = if runs_once {
            quote!('static)
        } else {
            lt = Some(quote!('a));
            quote!('a)
        };

        let cfgs = &static_.cfgs;
        let expr = &static_.expr;
        let ty = &static_.ty;
        fields.push(quote!(
            #(#cfgs)*
            #name: &#lt mut #ty
        ));
        items.push(quote!(
            #(#cfgs)*
            static mut #name: #ty = #expr
        ));
        values.push(quote!(
            #(#cfgs)*
            #name: &mut #name
        ));
        lets.push(quote!(
            #(#cfgs)*
            let #name = __locals.#name
        ));
    }

    if lt.is_some() {
        fields.push(quote!(__marker__: core::marker::PhantomData<&'a mut ()>));
        values.push(quote!(__marker__: core::marker::PhantomData));
    }

    let locals = quote!(
        #[allow(non_snake_case)]
        #[doc(hidden)]
        pub struct #ident<#lt> {
            #(#fields),*
        }

        impl<#lt> #ident<#lt> {
            #[inline(always)]
            unsafe fn new() -> Self {
                #(#items;)*

                #ident {
                    #(#values),*
                }
            }
        }
    );

    (locals, lets)
}

/// This function creates a module that contains
//
// - the Context struct
// - a re-export of the ${name}Resources struct (maybe)
// - a re-export of the ${name}LateResources struct (maybe)
// - a re-export of the ${name}Locals struct
// - the Spawn struct (maybe)
// - the Schedule struct (maybe, if `timer-queue` is enabled)
fn module(
    kind: Kind,
    resources: (/* has */ bool, /* 'a */ bool),
    spawn: bool,
    late_resources: bool,
    app: &App,
) -> proc_macro2::TokenStream {
    let mut items = vec![];
    let mut fields = vec![];
    let mut values = vec![];

    let name = kind.ident();

    let mut needs_instant = false;
    let mut lt = None;
    match kind {
        Kind::Init => {
            if cfg!(feature = "timer-queue") {
                fields.push(quote!(
                    /// System start time = `Instant(0 /* cycles */)`
                    pub start: rtfm::Instant
                ));

                values.push(quote!(start: rtfm::Instant::artificial(0)));
            }
        }

        Kind::Idle => {}

        Kind::Task(_) => {
            if cfg!(feature = "timer-queue") {
                fields.push(quote!(
                    /// The time at which this task was scheduled to run
                    pub scheduled: rtfm::Instant
                ));

                values.push(quote!(scheduled: instant));

                needs_instant = true;
            }
        }
    }

    let ident = kind.locals_ident();
    items.push(quote!(
        #[doc(inline)]
        pub use super::#ident as Locals;
    ));

    if resources.0 {
        let ident = kind.resources_ident();
        let lt = if resources.1 {
            lt = Some(quote!('a));
            Some(quote!('a))
        } else {
            None
        };

        items.push(quote!(
            #[doc(inline)]
            pub use super::#ident as Resources;
        ));

        fields.push(quote!(
            /// Resources this task has access to
            pub resources: Resources<#lt>
        ));

        let priority = if kind.is_init() {
            None
        } else {
            Some(quote!(priority))
        };
        values.push(quote!(resources: Resources::new(#priority)));
    }

    if spawn {
        let doc = "Tasks that can be `spawn`-ed from this context";
        if kind.is_init() {
            fields.push(quote!(
                #[doc = #doc]
                pub spawn: Spawn
            ));

            items.push(quote!(
                #[doc = #doc]
                #[derive(Clone, Copy)]
                pub struct Spawn {
                    _not_send: core::marker::PhantomData<*mut ()>,
                }
            ));

            values.push(quote!(spawn: Spawn { _not_send: core::marker::PhantomData }));
        } else {
            lt = Some(quote!('a));

            fields.push(quote!(
                #[doc = #doc]
                pub spawn: Spawn<'a>
            ));

            let mut instant_method = None;
            if kind.is_idle() {
                items.push(quote!(
                    #[doc = #doc]
                    #[derive(Clone, Copy)]
                    pub struct Spawn<'a> {
                        priority: &'a rtfm::export::Priority,
                    }
                ));

                values.push(quote!(spawn: Spawn { priority }));
            } else {
                let instant_field = if cfg!(feature = "timer-queue") {
                    needs_instant = true;
                    instant_method = Some(quote!(
                        pub unsafe fn instant(&self) -> rtfm::Instant {
                            self.instant
                        }
                    ));
                    Some(quote!(instant: rtfm::Instant,))
                } else {
                    None
                };

                items.push(quote!(
                    /// Tasks that can be spawned from this context
                    #[derive(Clone, Copy)]
                    pub struct Spawn<'a> {
                        #instant_field
                        priority: &'a rtfm::export::Priority,
                    }
                ));

                let _instant = if needs_instant {
                    Some(quote!(, instant))
                } else {
                    None
                };
                values.push(quote!(
                    spawn: Spawn { priority #_instant }
                ));
            }

            items.push(quote!(
                impl<'a> Spawn<'a> {
                    #[doc(hidden)]
                    #[inline(always)]
                    pub unsafe fn priority(&self) -> &rtfm::export::Priority {
                        self.priority
                    }

                    #instant_method
                }
            ));
        }
    }

    if late_resources {
        items.push(quote!(
            #[doc(inline)]
            pub use super::initLateResources as LateResources;
        ));
    }

    let doc = match kind {
        // Kind::Exception(_) => "Hardware task (exception)",
        Kind::Idle => "Idle loop",
        Kind::Init => "Initialization function",
        // Kind::Interrupt(_) => "Hardware task (interrupt)",
        Kind::Task(_) => "Software task",
    };

    let priority = if kind.is_init() {
        None
    } else {
        Some(quote!(priority: &#lt rtfm::export::Priority))
    };

    let instant = if needs_instant {
        Some(quote!(, instant: rtfm::Instant))
    } else {
        None
    };
    items.push(quote!(
        /// Execution context
        pub struct Context<#lt> {
            #(#fields,)*
        }

        impl<#lt> Context<#lt> {
            #[inline(always)]
            pub unsafe fn new(#priority #instant) -> Self {
                Context {
                    #(#values,)*
                }
            }
        }
    ));

    if !items.is_empty() {
        quote!(
            #[allow(non_snake_case)]
            #[doc = #doc]
            pub mod #name {
                #(#items)*
            }
        )
    } else {
        quote!()
    }
}

/// Creates the body of `spawn_${name}`
fn mk_spawn_body<'a>(
    spawner: &Ident,
    name: &Ident,
    app: &'a App,
    analysis: &Analysis,
) -> proc_macro2::TokenStream {
    let spawner_is_init = spawner == "init";
    // let device = &app.args.device;

    let spawnee = &app.tasks[name];
    let priority = spawnee.args.priority;
    let dispatcher = &analysis.dispatchers[&priority];

    let (_, tupled, _, _) = regroup_inputs(&spawnee.inputs);

    let inputs = mk_inputs_ident(name);
    let fq = mk_fq_ident(name);

    let rq = mk_rq_ident(priority);
    let t = mk_t_ident(priority);

    let write_instant = if cfg!(feature = "timer-queue") {
        let instants = mk_instants_ident(name);

        Some(quote!(
            #instants.get_unchecked_mut(usize::from(index)).write(instant);
        ))
    } else {
        None
    };

    let dispatcher = Ident::new(&format!("dispatcher{}", priority), Span::call_site());
    let enqueue = quote!(
        rtfm::export::enqueue(
            PID,
            #priority,
            #t::#name as u8,
            index,
        );
    );
    let dequeue = if spawner_is_init {
        // `init` has exclusive access to these queues so we can bypass the resources AND
        // the consumer / producer split
        quote!(#fq.dequeue())
    } else {
        let ceiling = analysis.ready_queues[&priority];
        quote!((#fq { priority }).lock(|fq| fq.split().1.dequeue()))
    };

    quote!(
        unsafe {
            use rtfm::Mutex as _;

            let input = #tupled;
            if let Some(index) = #dequeue {
                #inputs.get_unchecked_mut(usize::from(index)).write(input);

                #write_instant

                #enqueue

                Ok(())
            } else {
                Err(input)
            }
        }
    )
}

/// Creates the body of `schedule_${name}`
fn mk_schedule_body<'a>(scheduler: &Ident, name: &Ident, app: &'a App) -> proc_macro2::TokenStream {
    let scheduler_is_init = scheduler == "init";

    let schedulee = &app.tasks[name];

    let (_, tupled, _, _) = regroup_inputs(&schedulee.inputs);

    let fq = mk_fq_ident(name);
    let inputs = mk_inputs_ident(name);
    let instants = mk_instants_ident(name);

    let (dequeue, enqueue) = if scheduler_is_init {
        // `init` has exclusive access to these queues so we can bypass the resources AND
        // the consumer / producer split
        let dequeue = quote!(#fq.dequeue());

        (dequeue, quote!((*TQ.as_mut_ptr()).enqueue_unchecked(nr);))
    } else {
        (
            quote!((#fq { priority }).lock(|fq| fq.split().1.dequeue())),
            quote!((TQ { priority }).lock(|tq| tq.enqueue_unchecked(nr));),
        )
    };

    quote!(
        unsafe {
            use rtfm::Mutex as _;

            let input = #tupled;
            if let Some(index) = #dequeue {
                #instants.get_unchecked_mut(usize::from(index)).write(instant);

                #inputs.get_unchecked_mut(usize::from(index)).write(input);

                let nr = rtfm::export::NotReady {
                    instant,
                    index,
                    task: T::#name,
                };

                #enqueue

                Ok(())
            } else {
                Err(input)
            }
        }
    )
}

/// `u8` -> (unsuffixed) `LitInt`
fn mk_capacity_literal(capacity: u8) -> LitInt {
    LitInt::new(u64::from(capacity), IntSuffix::None, Span::call_site())
}

/// e.g. `4u8` -> `U4`
fn mk_typenum_capacity(capacity: u8, power_of_two: bool) -> proc_macro2::TokenStream {
    let capacity = if power_of_two {
        capacity
            .checked_next_power_of_two()
            .expect("capacity.next_power_of_two()")
    } else {
        capacity
    };

    let ident = Ident::new(&format!("U{}", capacity), Span::call_site());

    quote!(rtfm::export::consts::#ident)
}

/// e.g. `foo` -> `foo_INPUTS`
fn mk_inputs_ident(base: &Ident) -> Ident {
    Ident::new(&format!("{}_INPUTS", base), Span::call_site())
}

/// e.g. `foo` -> `foo_INSTANTS`
fn mk_instants_ident(base: &Ident) -> Ident {
    Ident::new(&format!("{}_INSTANTS", base), Span::call_site())
}

/// e.g. `foo` -> `foo_FQ`
fn mk_fq_ident(base: &Ident) -> Ident {
    Ident::new(&format!("{}_FQ", base), Span::call_site())
}

/// e.g. `3` -> `RQ3`
fn mk_rq_ident(level: u8) -> Ident {
    Ident::new(&format!("RQ{}", level), Span::call_site())
}

/// e.g. `3` -> `T3`
fn mk_t_ident(level: u8) -> Ident {
    Ident::new(&format!("T{}", level), Span::call_site())
}

fn mk_spawn_ident(task: &Ident) -> Ident {
    Ident::new(&format!("spawn_{}", task), Span::call_site())
}

fn mk_schedule_ident(task: &Ident) -> Ident {
    Ident::new(&format!("schedule_{}", task), Span::call_site())
}

fn mk_rt_ident(level: u8) -> Ident {
    // NOTE keep this in sync with `rtfm::export::PRIORITY_MAX`
    const PRIORITY_MAX: u8 = 16;

    Ident::new(&format!("RT{}", PRIORITY_MAX - level), Span::call_site())
}

// Regroups a task inputs
//
// e.g. &[`input: Foo`], &[`mut x: i32`, `ref y: i64`]
fn regroup_inputs(
    inputs: &[ArgCaptured],
) -> (
    // args e.g. &[`_0`],  &[`_0: i32`, `_1: i64`]
    Vec<proc_macro2::TokenStream>,
    // tupled e.g. `_0`, `(_0, _1)`
    proc_macro2::TokenStream,
    // untupled e.g. &[`_0`], &[`_0`, `_1`]
    Vec<proc_macro2::TokenStream>,
    // ty e.g. `Foo`, `(i32, i64)`
    proc_macro2::TokenStream,
) {
    if inputs.len() == 1 {
        let ty = &inputs[0].ty;

        (
            vec![quote!(_0: #ty)],
            quote!(_0),
            vec![quote!(_0)],
            quote!(#ty),
        )
    } else {
        let mut args = vec![];
        let mut pats = vec![];
        let mut tys = vec![];

        for (i, input) in inputs.iter().enumerate() {
            let i = Ident::new(&format!("_{}", i), Span::call_site());
            let ty = &input.ty;

            args.push(quote!(#i: #ty));

            pats.push(quote!(#i));

            tys.push(quote!(#ty));
        }

        let tupled = {
            let pats = pats.clone();
            quote!((#(#pats,)*))
        };
        let ty = quote!((#(#tys,)*));
        (args, tupled, pats, ty)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Kind {
    Idle,
    Init,
    Task(Ident),
}

impl Kind {
    fn ident(&self) -> Ident {
        let span = Span::call_site();
        match self {
            Kind::Init => Ident::new("init", span),
            Kind::Idle => Ident::new("idle", span),
            Kind::Task(name) => name.clone(),
        }
    }

    fn locals_ident(&self) -> Ident {
        Ident::new(&format!("{}Locals", self.ident()), Span::call_site())
    }

    fn resources_ident(&self) -> Ident {
        Ident::new(&format!("{}Resources", self.ident()), Span::call_site())
    }

    fn is_idle(&self) -> bool {
        *self == Kind::Idle
    }

    fn is_init(&self) -> bool {
        *self == Kind::Init
    }

    fn runs_once(&self) -> bool {
        match *self {
            Kind::Init | Kind::Idle => true,
            _ => false,
        }
    }
}
