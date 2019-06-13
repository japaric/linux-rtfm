use core::ops::{self, Range};
use std::collections::{BTreeMap, BTreeSet};

use rtfm_syntax::{
    analyze::{self, Priority},
    ast::App,
    Core, P,
};

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
    pub map: BTreeMap<Priority, Signal>,
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
pub fn app(parent: P<analyze::Analysis>, app: &App) -> P<Analysis> {
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
            .collect::<BTreeMap<_, _>>();
        let len = map.len();
        signals.insert(core, Signals { map, start: rt });
        rt += len as u8;
    }

    P::new(Analysis { parent, signals })
}
