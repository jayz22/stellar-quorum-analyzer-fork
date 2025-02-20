use crate::FbasAnalyzer;
use batsat::callbacks::Basic;
use std::{
    alloc::System,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tracking_allocator::{AllocationGroupId, AllocationRegistry, AllocationTracker, Allocator};

// This is where we actually set the global allocator to be the shim allocator implementation from `tracking_allocator`.
// This allocator is purely a facade to the logic provided by the crate, which is controlled by setting a global tracker
// and registering allocation groups.  All of that is covered below.
//
// As well, you can see here that we're wrapping the system allocator.  If you want, you can construct `Allocator` by
// wrapping another allocator that implements `GlobalAlloc`.  Since this is a static, you need a way to construct ther
// allocator to be wrapped in a const fashion, but it _is_ possible.
#[global_allocator]
static GLOBAL: Allocator<System> = Allocator::system();

struct MemTracker {
    current: Arc<AtomicU64>,
    cumulative: Arc<AtomicU64>,
}

impl MemTracker {
    fn new() -> Self {
        Self {
            current: Arc::new(AtomicU64::new(0)),
            cumulative: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl std::fmt::Display for MemTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "current: {}, cumulative: {}",
            self.current.load(Ordering::SeqCst),
            self.cumulative.load(Ordering::SeqCst)
        )
    }
}

// This is our tracker implementation.  You will always need to create an implementation of `AllocationTracker` in order
// to actually handle allocation events.  The interface is straightforward: you're notified when an allocation occurs,
// and when a deallocation occurs.
impl AllocationTracker for MemTracker {
    fn allocated(
        &self,
        _addr: usize,
        _object_size: usize,
        wrapped_size: usize,
        _group_id: AllocationGroupId,
    ) {
        self.current
            .fetch_add(wrapped_size as u64, Ordering::SeqCst);
        self.cumulative
            .fetch_add(wrapped_size as u64, Ordering::SeqCst);
        // Allocations have all the pertinent information upfront, which you may or may not want to store for further
        // analysis. Notably, deallocations also know how large they are, and what group ID they came from, so you
        // typically don't have to store much data for correlating deallocations with their original allocation.
        println!(
            "allocation -> current={:?} cumulative={:?}",
            self.current, self.cumulative
        );
    }

    fn deallocated(
        &self,
        _addr: usize,
        _object_size: usize,
        wrapped_size: usize,
        _source_group_id: AllocationGroupId,
        _current_group_id: AllocationGroupId,
    ) {
        // When a deallocation occurs, as mentioned above, you have full access to the address, size of the allocation,
        // as well as the group ID the allocation was made under _and_ the active allocation group ID.
        //
        // This can be useful beyond just the obvious "track how many current bytes are allocated by the group", instead
        // going further to see the chain of where allocations end up, and so on.
        self.current
            .fetch_sub(wrapped_size as u64, Ordering::SeqCst);
        println!(
            "deallocation -> current={:?} cumulative={:?}",
            self.current, self.cumulative
        );
    }
}

#[test]
fn test_random_data() -> std::io::Result<()> {
    let mem_tracker = MemTracker::new();
    let _ = AllocationRegistry::set_global_tracker(mem_tracker)
        .expect("no other global tracker should be set yet");

    let test_cases: Vec<_> = std::fs::read_dir("./tests/test_data/mem/")?
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.extension()?.to_str()? == "json" {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .collect();

    AllocationRegistry::enable_tracking();

    for json_path in test_cases {
        println!("{json_path:?}");
        if let Some(path_str) = json_path.to_str() {
            if let Ok(mut solver) = FbasAnalyzer::from_json_path(path_str, Basic::default()) {
                let _res = solver.solve();
            }
        }
    }

    AllocationRegistry::disable_tracking();
    Ok(())
}
