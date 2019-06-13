use core::ops::Range;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use rtfm_syntax::{ast::App, Context};
use syn::{ArgCaptured, Attribute, Ident, IntSuffix, LitInt};

pub fn impl_mutex(
    cfgs: &[Attribute],
    resources_prefix: bool,
    name: &Ident,
    ty: TokenStream2,
    ceiling: u8,
    Range { start, end }: Range<u8>,
    ptr: TokenStream2,
) -> TokenStream2 {
    let (path, priority) = if resources_prefix {
        (quote!(resources::#name), quote!(self.priority()))
    } else {
        (quote!(#name), quote!(self.priority))
    };

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
                        #start..#end,
                        f,
                    )
                }
            }
        }
    )
}

// Regroups a task inputs
//
// e.g. &[`input: Foo`], &[`mut x: i32`, `ref y: i64`]
pub fn regroup_inputs(
    inputs: &[ArgCaptured],
) -> (
    // args e.g. &[`_0`],  &[`_0: i32`, `_1: i64`]
    Vec<TokenStream2>,
    // tupled e.g. `_0`, `(_0, _1)`
    TokenStream2,
    // untupled e.g. &[`_0`], &[`_0`, `_1`]
    Vec<TokenStream2>,
    // ty e.g. `Foo`, `(i32, i64)`
    TokenStream2,
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

/// `u8` -> (unsuffixed) `LitInt`
pub fn capacity_literal(capacity: u8) -> LitInt {
    LitInt::new(u64::from(capacity), IntSuffix::None, Span::call_site())
}

/// e.g. `4u8` -> `U4`
pub fn typenum_capacity(capacity: u8, power_of_two: bool) -> TokenStream2 {
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
pub fn inputs_ident(base: &Ident) -> Ident {
    Ident::new(&format!("{}_INPUTS", base), Span::call_site())
}

/// e.g. `foo` -> `foo_INSTANTS`
pub fn instants_ident(base: &Ident) -> Ident {
    Ident::new(&format!("{}_INSTANTS", base), Span::call_site())
}

pub fn locals_ident(ctxt: Context, app: &App) -> Ident {
    let mut s = match ctxt {
        Context::Init(core) => app.inits[&core].name.to_string(),
        Context::Idle(core) => app.idles[&core].name.to_string(),
        Context::HardwareTask(ident) | Context::SoftwareTask(ident) => ident.to_string(),
    };

    s.push_str("Locals");

    Ident::new(&s, Span::call_site())
}

pub fn resources_ident(ctxt: Context, app: &App) -> Ident {
    let mut s = match ctxt {
        Context::Init(core) => app.inits[&core].name.to_string(),
        Context::Idle(core) => app.idles[&core].name.to_string(),
        Context::HardwareTask(ident) | Context::SoftwareTask(ident) => ident.to_string(),
    };

    s.push_str("Resources");

    Ident::new(&s, Span::call_site())
}

/// e.g. `3` -> `RT3`
pub fn rt_ident(i: u8) -> Ident {
    Ident::new(&format!("RT{}", i), Span::call_site())
}

pub fn schedule_ident(task: &Ident) -> Ident {
    Ident::new(&format!("schedule_{}", task), Span::call_site())
}

pub fn spawn_ident(task: &Ident) -> Ident {
    Ident::new(&format!("spawn_{}", task), Span::call_site())
}

pub fn timer_ident(sender: u8) -> Ident {
    Ident::new(&format!("TIMER{}", sender), Span::call_site())
}

pub fn fq_ident_(task: &Ident, sender: u8) -> Ident {
    Ident::new(
        &format!("{}_S{}_FQ", task.to_string(), sender),
        Span::call_site(),
    )
}

pub fn tid_ident(core: u8) -> Ident {
    Ident::new(&format!("TID{}", core), Span::call_site())
}

pub fn child_ident(core: u8) -> Ident {
    Ident::new(&format!("child{}", core), Span::call_site())
}

pub fn late_resources_ident(init: &Ident) -> Ident {
    Ident::new(
        &format!("{}LateResources", init.to_string()),
        Span::call_site(),
    )
}

pub fn b_ident(core: u8) -> Ident {
    Ident::new(&format!("B{}", core), Span::call_site())
}

pub fn spawn_t_ident(receiver: u8, priority: u8) -> Ident {
    Ident::new(&format!("R{}_T{}", receiver, priority), Span::call_site())
}

pub fn schedule_t_ident(sender: u8) -> Ident {
    Ident::new(&format!("T{}", sender), Span::call_site())
}

pub fn task_ident(task: &Ident, sender: u8) -> Ident {
    Ident::new(&format!("{}_S{}", task, sender), Span::call_site())
}

pub fn tq_ident(sender: u8) -> Ident {
    Ident::new(&format!("TQ{}", sender), Span::call_site())
}
