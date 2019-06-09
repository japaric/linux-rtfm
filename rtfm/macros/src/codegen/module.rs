use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::{ast::App, Context};

use crate::codegen::util;

pub fn codegen(
    ctxt: Context,
    resources: (/* has */ bool, /* 'a */ bool),
    schedule: bool,
    spawn: bool,
    late_resources: bool,
    app: &App,
) -> TokenStream2 {
    let mut items = vec![];
    let mut fields = vec![];
    let mut values = vec![];

    let name = ctxt.ident(app);

    let core = ctxt.core(app);
    let mut needs_instant = false;
    let mut lt = None;
    match ctxt {
        Context::Init(..) | Context::Idle(..) => {}

        Context::HardwareTask(_) => unreachable!(),

        Context::SoftwareTask(_) => {
            if app.uses_schedule(core) {
                fields.push(quote!(
                    /// The time at which this task was scheduled to run
                    pub scheduled: rtfm::Instant
                ));

                values.push(quote!(scheduled: instant));

                needs_instant = true;
            }
        }
    }

    let ident = util::locals_ident(ctxt, app);
    items.push(quote!(
        #[doc(inline)]
        pub use super::#ident as Locals;
    ));

    if resources.0 {
        let ident = util::resources_ident(ctxt, app);
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

        let priority = if ctxt.is_init() {
            None
        } else {
            Some(quote!(priority))
        };
        values.push(quote!(resources: Resources::new(#priority)));
    }

    if schedule {
        let doc = "Tasks that can be `schedule`-d from this context";
        if ctxt.is_init() {
            items.push(quote!(
                #[doc = #doc]
                #[derive(Clone, Copy)]
                pub struct Schedule {
                    _not_send: core::marker::PhantomData<*mut ()>,
                }
            ));

            fields.push(quote!(
                #[doc = #doc]
                pub schedule: Schedule
            ));

            values.push(quote!(
                schedule: Schedule { _not_send: core::marker::PhantomData }
            ));
        } else {
            lt = Some(quote!('a));

            items.push(quote!(
                #[doc = #doc]
                #[derive(Clone, Copy)]
                pub struct Schedule<'a> {
                    priority: &'a rtfm::export::Priority,
                }

                impl<'a> Schedule<'a> {
                    #[doc(hidden)]
                    #[inline(always)]
                    pub unsafe fn priority(&self) -> &rtfm::export::Priority {
                        &self.priority
                    }
                }
            ));

            fields.push(quote!(
                #[doc = #doc]
                pub schedule: Schedule<'a>
            ));

            values.push(quote!(
                schedule: Schedule { priority }
            ));
        }
    }

    if spawn {
        let doc = "Tasks that can be `spawn`-ed from this context";
        if ctxt.is_init() {
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
            if ctxt.is_idle() {
                items.push(quote!(
                    #[doc = #doc]
                    #[derive(Clone, Copy)]
                    pub struct Spawn<'a> {
                        priority: &'a rtfm::export::Priority,
                    }
                ));

                values.push(quote!(spawn: Spawn { priority }));
            } else {
                let instant_field = if app.uses_schedule(core) {
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

    let doc = match ctxt {
        Context::Idle(_) => "Idle loop",
        Context::Init(_) => "Initialization function",
        Context::HardwareTask(_) => unreachable!(),
        Context::SoftwareTask(_) => "Software task",
    };

    let priority = if ctxt.is_init() {
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
