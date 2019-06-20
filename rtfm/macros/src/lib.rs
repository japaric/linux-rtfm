#![deny(warnings)]
#![recursion_limit = "128"]

extern crate proc_macro;

use proc_macro::TokenStream;
use std::{fs, path::Path};

use rtfm_syntax::Settings;

mod analyze;
mod check;
mod codegen;

#[proc_macro_attribute]
pub fn app(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut settings = Settings::default();
    settings.parse_cores = true;
    settings.parse_schedule = true;

    let (app, analysis) = match rtfm_syntax::parse(args, input, settings) {
        Err(e) => return e.to_compile_error().into(),
        Ok(x) => x,
    };

    if let Err(e) = check::app(&app, &analysis) {
        return e.to_compile_error().into();
    }

    let analysis = analyze::app(analysis, &app);

    // Code generation
    let ts = codegen::app(&app, &analysis);

    // Try to write the expanded code to disk
    if Path::new("target").exists() {
        fs::write("target/rtfm-expansion.rs", ts.to_string()).ok();
    }

    ts.into()
}
