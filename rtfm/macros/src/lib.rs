#![deny(warnings)]
#![recursion_limit = "128"]

extern crate proc_macro;

use proc_macro::TokenStream;
use std::{fs, path::Path};

use rtfm_syntax::Settings;

mod analyze;
mod codegen;

#[proc_macro_attribute]
pub fn app(args: TokenStream, input: TokenStream) -> TokenStream {
    let (app, analysis) = match rtfm_syntax::parse(
        args,
        input,
        Settings {
            parse_cores: true,
            parse_schedule: true,
            ..Settings::default()
        },
    ) {
        Err(e) => return e.to_compile_error().into(),
        Ok(x) => x,
    };

    let analysis = match analyze::app(analysis, &app) {
        Err(e) => return e.to_compile_error().into(),
        Ok(x) => x,
    };

    // Code generation
    let ts = codegen::app(&app, &analysis);

    // Try to write the expanded code to disk
    if Path::new("target").exists() {
        fs::write("target/rtfm-expansion.rs", ts.to_string()).ok();
    }

    ts.into()
}
