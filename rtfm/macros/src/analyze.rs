use core::ops::{self, Range};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use proc_macro2::Span;
use rtfm_syntax::{
    analyze::{self, Priority},
    ast::App,
    Core, P,
};
use syn::parse;

// Linux 5.0 only supports 32 real time signals
const THRESHOLD: u8 = 32;

/// Signal number
pub type Signal = u8;

pub struct Analysis {
    parent: P<analyze::Analysis>,
    pub signals: BTreeMap<Core, Signals>,
}

impl ops::Deref for Analysis {
    type Target = analyze::Analysis;

    fn deref(&self) -> &Self::Target {
        &self.parent
    }
}

pub struct Signals {
    pub map: HashMap<Priority, Signal>,
    pub start: Signal,
}

impl Signals {
    pub fn range(&self) -> Range<Signal> {
        let start = self.start;
        let end = start + self.map.len() as u8;
        start..end
    }
}

// Assign a RT signal handler to each priority level
pub fn app(parent: P<analyze::Analysis>, app: &App) -> parse::Result<P<Analysis>> {
    let mut rt = 0;

    let mut signals = BTreeMap::new();
    for core in 0..app.args.cores {
        let priorities = app
            .software_tasks
            .values()
            .filter_map(|task| {
                if task.args.core == core {
                    Some(task.args.priority)
                } else {
                    None
                }
            })
            // NOTE the timer handler may be higher priority than all the other tasks
            .chain(parent.timer_queues.get(&core).map(|tq| tq.priority))
            .collect::<BTreeSet<_>>();

        let map = priorities
            .iter()
            .rev()
            .cloned()
            .zip(rt..)
            .collect::<HashMap<_, _>>();
        let len = map.len();
        signals.insert(core, Signals { map, start: rt });
        rt += len as u8;

        if rt > THRESHOLD {
            return Err(parse::Error::new(
                Span::call_site(),
                "there are not enough real time signals to dispatch all tasks",
            ));
        }
    }

    Ok(P::new(Analysis { parent, signals }))
}
