use log::trace;
use std::time::{Duration, Instant};
use batsat::{callbacks::{Callbacks, ProgressStatus}, lbool};
use crate::allocator::get_memory_usage;

/// An implementation of the `Callbacks` trait that tracks and limits the memory usage and processing time of the solver.
pub struct ResourceLimitingCB {
    start_time: Instant,
    start_memory: usize,
    time_limit: Duration,
    memory_limit: usize,
}

impl ResourceLimitingCB{
    pub fn new(time_limit_ms: u64, memory_limit_bytes: usize) -> Self {
        Self { start_time: Instant::now(), start_memory: get_memory_usage(), time_limit: Duration::from_millis(time_limit_ms), memory_limit: memory_limit_bytes }
    }

    fn measure_resources(&self) -> (Duration, usize) {
        let elapsed = self.start_time.elapsed();
        let memory_usage = get_memory_usage() - self.start_memory;
        (elapsed, memory_usage)
    }
}

impl Callbacks for ResourceLimitingCB {
    fn on_start(&mut self) {
        trace!( target: "SCP",
            "c ============================[ Search Statistics ]=============================="
        );
        trace!( target: "SCP",
            "c | Conflicts |          ORIGINAL         |          LEARNT          | Progress |"
        );
        trace!( target: "SCP",
            "c |           |    Vars  Clauses Literals |    Limit  Clauses Lit/Cl |          |"
        );
        trace!( target: "SCP",
            "c ==============================================================================="
        );        
    }

    fn on_result(&mut self, _: lbool) {
        trace!( target: "SCP",
            "c ==============================================================================="
        );
    }

    fn on_progress<F>(&mut self, p: F)
    where
        F: FnOnce() -> ProgressStatus,
    {
        let p = p();
        trace!( target: "SCP",
            "c | {:9} | {:7} {:8} {:8} | {:8} {:8} {:6.0} | {:6.3} % |",
            p.conflicts,
            p.dec_vars,
            p.n_clauses,
            p.n_clause_lits,
            p.max_learnt,
            p.n_learnt,
            p.n_learnt_lits,
            p.progress_estimate
        );
    }

    fn on_gc(&mut self, old: usize, new: usize) {
        trace!( target: "SCP",
            "|  Garbage collection:   {:12} bytes => {:12} bytes             |",
            old, new
        );
    }

    fn stop(&self) -> bool {
        let (elapsed, memory_usage) = self.measure_resources();
        let stop = memory_usage > self.memory_limit || elapsed > self.time_limit;
        if stop {   
            trace!( target: "SCP",
                "Stopped due to going over the resource limits -- Memory usage: {} bytes, Time elapsed: {} ms; Memory limit: {} bytes, Time limit: {} ms",
                memory_usage, elapsed.as_millis(), self.memory_limit, self.time_limit.as_millis()   
            );
        }
        stop
    }
}
