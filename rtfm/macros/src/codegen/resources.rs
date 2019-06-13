use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rtfm_syntax::{analyze::Ownership, ast::App};

use crate::{analyze::Analysis, codegen::util};

pub fn codegen(
    app: &App,
    analysis: &Analysis,
) -> (
    // const_app
    Vec<TokenStream2>,
    // mod_resources
    TokenStream2,
) {
    let mut const_app = vec![];
    let mut mod_resources = vec![];

    for (name, res, expr, loc) in app.resources(analysis) {
        let cfgs = &res.cfgs;
        let attrs = &res.attrs;
        let ty = &res.ty;

        if let Some(expr) = expr {
            const_app.push(quote!(
                #(#attrs)*
                #(#cfgs)*
                static mut #name: #ty = #expr;
            ));
        } else {
            const_app.push(quote!(
                #(#attrs)*
                #(#cfgs)*
                static mut #name: core::mem::MaybeUninit<#ty> =
                    core::mem::MaybeUninit::uninit();
            ));
        }

        // generate a resource proxy when needed
        if res.mutability.is_some() {
            if let Some(Ownership::Shared { ceiling }) = analysis.ownerships.get(name) {
                let ptr = if expr.is_none() {
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

                let range = analysis.signals[&loc.core().unwrap()].range();
                const_app.push(util::impl_mutex(
                    cfgs,
                    true,
                    name,
                    quote!(#ty),
                    *ceiling,
                    range.clone(),
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
