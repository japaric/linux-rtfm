use std::{collections::HashSet, iter};

use proc_macro2::Span;
use syn::{parse, spanned::Spanned, Block, Expr, Stmt};

use crate::syntax::App;

pub fn app(app: &App) -> parse::Result<()> {
    // Check that all referenced resources have been declared
    for res in app
        .idle
        .as_ref()
        .map(|idle| -> Box<dyn Iterator<Item = _>> { Box::new(idle.args.resources.iter()) })
        .unwrap_or_else(|| Box::new(iter::empty()))
        .chain(&app.init.args.resources)
        // .chain(app.exceptions.values().flat_map(|e| &e.args.resources))
        // .chain(app.interrupts.values().flat_map(|i| &i.args.resources))
        .chain(app.tasks.values().flat_map(|t| &t.args.resources))
    {
        if !app.resources.contains_key(res) {
            return Err(parse::Error::new(
                res.span(),
                "this resource has NOT been declared",
            ));
        }
    }

    // Check that late resources have not been assigned to `init`
    for res in &app.init.args.resources {
        if app.resources.get(res).unwrap().expr.is_none() {
            return Err(parse::Error::new(
                res.span(),
                "late resources can NOT be assigned to `init`",
            ));
        }
    }

    // Check that `init` returns `LateResources` if there's any declared late resource
    if !app.init.returns_late_resources && app.resources.iter().any(|(_, res)| res.expr.is_none()) {
        return Err(parse::Error::new(
            app.init.span,
            "late resources have been specified so `init` must return `init::LateResources`",
        ));
    }

    // Check that all referenced tasks have been declared
    for task in app
        .idle
        .as_ref()
        .map(|idle| -> Box<dyn Iterator<Item = _>> {
            // Box::new(idle.args.schedule.iter().chain(&idle.args.spawn))
            Box::new(idle.args.spawn.iter())
        })
        .unwrap_or_else(|| Box::new(iter::empty()))
        // .chain(&app.init.args.schedule)
        .chain(&app.init.args.spawn)
        // .chain(
        //     app.exceptions
        //         .values()
        //         .flat_map(|e| e.args.schedule.iter().chain(&e.args.spawn)),
        // )
        // .chain(
        //     app.interrupts
        //         .values()
        //         .flat_map(|i| i.args.schedule.iter().chain(&i.args.spawn)),
        // )
        .chain(app.tasks.values().flat_map(|t| {
            // t.args.schedule.iter().chain(&t.args.spawn)
            t.args.spawn.iter()
        }))
    {
        if !app.tasks.contains_key(task) {
            return Err(parse::Error::new(
                task.span(),
                "this task has NOT been declared",
            ));
        }
    }

    // NOTE doesn't apply we can spawn an absurd number of processes
    // Check that there are enough free interrupts to dispatch all tasks
    // let ndispatchers = app
    //     .tasks
    //     .values()
    //     .map(|t| t.args.priority)
    //     .collect::<HashSet<_>>()
    //     .len();
    // if ndispatchers > app.free_interrupts.len() {
    //     return Err(parse::Error::new(
    //         Span::call_site(),
    //         &*format!(
    //             "{} free interrupt{} (`extern {{ .. }}`) {} required to dispatch all soft tasks",
    //             ndispatchers,
    //             if ndispatchers > 1 { "s" } else { "" },
    //             if ndispatchers > 1 { "are" } else { "is" },
    //         ),
    //     ));
    // }

    // Check that free interrupts are not being used
    // for (handler, interrupt) in &app.interrupts {
    //     let name = interrupt.args.binds(handler);

    //     if app.free_interrupts.contains_key(name) {
    //         return Err(parse::Error::new(
    //             name.span(),
    //             "free interrupts (`extern { .. }`) can't be used as interrupt handlers",
    //         ));
    //     }
    // }

    Ok(())
}
