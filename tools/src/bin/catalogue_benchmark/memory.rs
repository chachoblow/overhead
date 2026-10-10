//! Requested heap bytes, not allocator footprint/RSS/stack. No allocation in hooks.
//! Phase snapshots require a single-threaded process with no concurrent allocations.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use serde::Serialize;

pub struct CountingAllocator {
    live: AtomicUsize,
    peak: AtomicUsize,
    calls: AtomicUsize,
    requested: AtomicUsize,
}

impl CountingAllocator {
    pub const fn new() -> Self {
        Self {
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
            requested: AtomicUsize::new(0),
        }
    }

    fn allocated(&self, size: usize) {
        let live = self.live.fetch_add(size, Relaxed) + size;
        self.peak.fetch_max(live, Relaxed);
        self.calls.fetch_add(1, Relaxed);
        self.requested.fetch_add(size, Relaxed);
    }

    pub fn live(&self) -> usize {
        self.live.load(Relaxed)
    }

    pub fn begin(&self) -> Snapshot {
        let live = self.live();
        self.peak.store(live, Relaxed);
        Snapshot {
            live,
            calls: self.calls.load(Relaxed),
            requested: self.requested.load(Relaxed),
        }
    }

    pub fn finish(&self, start: Snapshot) -> Heap {
        Heap {
            peak_extra_bytes: self.peak.load(Relaxed) - start.live,
            retained_extra_bytes: self.live() - start.live,
            allocation_calls: self.calls.load(Relaxed) - start.calls,
            requested_bytes: self.requested.load(Relaxed) - start.requested,
        }
    }
}

pub struct Snapshot {
    live: usize,
    calls: usize,
    requested: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Heap {
    pub peak_extra_bytes: usize,
    pub retained_extra_bytes: usize,
    /// Successful alloc/alloc_zeroed/realloc calls; excludes dealloc.
    pub allocation_calls: usize,
    /// Sum of requested sizes, including full new sizes on realloc.
    pub requested_bytes: usize,
}

// SAFETY: Every operation forwards the caller's pointer/layout unchanged to
// System. Counters never dereference pointers, allocate, or unwind. Failed
// allocations leave live-byte accounting unchanged. Each successful realloc
// replaces its old logical allocation; System's internal transient storage is
// deliberately not visible here. Layouts must obey GlobalAlloc's contract.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            self.allocated(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            self.allocated(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        self.live.fetch_sub(layout.size(), Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, new_size) };
        if !result.is_null() {
            self.live.fetch_sub(layout.size(), Relaxed);
            self.allocated(new_size);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_zeroing_growth_shrink_and_release_account_exactly() {
        // Local counters isolate this test from the parallel Rust test harness.
        let allocator = CountingAllocator::new();
        let start = allocator.begin();
        unsafe {
            let layout = Layout::from_size_align(32, 16).unwrap();
            let ptr = allocator.alloc_zeroed(layout);
            assert!(!ptr.is_null());
            assert_eq!(ptr as usize % 16, 0);
            assert!((0..32).all(|i| *ptr.add(i) == 0));
            let ptr = allocator.realloc(ptr, layout, 128);
            assert!(!ptr.is_null());
            let layout = Layout::from_size_align(128, 16).unwrap();
            let ptr = allocator.realloc(ptr, layout, 16);
            assert!(!ptr.is_null());
            assert_eq!(
                allocator.finish(start),
                Heap {
                    peak_extra_bytes: 128,
                    retained_extra_bytes: 16,
                    allocation_calls: 3,
                    requested_bytes: 176,
                }
            );
            // A preexisting allocation is excluded from the next phase.
            let start = allocator.begin();
            let other_layout = Layout::from_size_align(64, 8).unwrap();
            let other = allocator.alloc(other_layout);
            assert!(!other.is_null());
            allocator.dealloc(other, other_layout);
            assert_eq!(
                allocator.finish(start),
                Heap {
                    peak_extra_bytes: 64,
                    retained_extra_bytes: 0,
                    allocation_calls: 1,
                    requested_bytes: 64,
                }
            );
            allocator.dealloc(ptr, Layout::from_size_align(16, 16).unwrap());
            assert_eq!(allocator.live(), 0);
        }
    }
}
