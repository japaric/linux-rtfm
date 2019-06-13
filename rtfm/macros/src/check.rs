use std::collections::{BTreeSet, HashSet};

use proc_macro2::Span;
use rtfm_syntax::{analyze::Analysis, ast::App};
use syn::parse;

// Linux 5.0 only supports 32 real time signals
const NSIGNALS: usize = 32;

pub fn app(app: &App, analysis: &Analysis) -> parse::Result<()> {
    // this RTFM implementation uses the same namespace for all cores so we need to check that the
    // identifiers used for each core `#[init]` and `#[idle]` functions don't collide
    let mut seen = HashSet::new();

    for name in app
        .inits
        .values()
        .map(|init| &init.name)
        .chain(app.idles.values().map(|idle| &idle.name))
    {
        if seen.contains(name) {
            return Err(parse::Error::new(
                name.span(),
                "this identifier is already being used by another core",
            ));
        } else {
            seen.insert(name);
        }
    }

    // check that there are enough signal handlers to dispatch all tasks
    let signals = app
        .software_tasks
        .values()
        .map(|task| (task.args.core, task.args.priority))
        .chain(
            analysis
                .timer_queues
                .iter()
                .map(|(core, tq)| (*core, tq.priority)),
        )
        .collect::<BTreeSet<_>>();

    if signals.len() > NSIGNALS {
        return Err(parse::Error::new(
            Span::call_site(),
            "there are not enough real time signals to dispatch all tasks",
        ));
    }

    Ok(())
}
