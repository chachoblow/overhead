//! Test-only, thread-local requested-heap accounting. Not RSS, allocator
//! metadata, cross-thread frees, physical RAM, or a measured stack high-water.
//! Use only around the single-threaded host experiment on this test thread.
extern crate std;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct Stats {
    pub live: usize,
    pub peak: usize,
    pub transient_envelope: usize,
    pub calls: usize,
    active: bool,
}
impl Stats {
    const ZERO: Self = Self {
        live: 0,
        peak: 0,
        transient_envelope: 0,
        calls: 0,
        active: false,
    };
}
std::thread_local! { static STATS: Cell<Stats> = const { Cell::new(Stats::ZERO) }; }

fn update(f: impl FnOnce(&mut Stats)) {
    // During TLS destruction try_with fails safely. The hook must not allocate.
    let _ = STATS.try_with(|cell| {
        let mut stats = cell.get();
        if stats.active {
            f(&mut stats);
            cell.set(stats);
        }
    });
}

pub(super) fn begin() {
    STATS.with(|cell| {
        cell.set(Stats {
            active: true,
            ..Stats::ZERO
        })
    });
}
pub(super) fn snapshot() -> Stats {
    STATS.with(Cell::get)
}
pub(super) fn end() -> Stats {
    STATS.with(|cell| {
        let result = cell.get();
        cell.set(Stats::ZERO);
        result
    })
}

struct Meter;
#[global_allocator]
static ALLOCATOR: Meter = Meter;

// SAFETY: All pointer operations delegate unchanged to System. TLS updates
// copy a const-initialized Cell and cannot allocate, panic or dereference ptr.
// Realloc records a conservative old+new requested-byte envelope; System may
// resize in place, and its true transient or bookkeeping bytes are unknown.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            update(|s| {
                s.live = s.live.saturating_add(layout.size());
                s.peak = s.peak.max(s.live);
                s.calls = s.calls.saturating_add(1);
            });
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            update(|s| {
                s.live = s.live.saturating_add(layout.size());
                s.peak = s.peak.max(s.live);
                s.calls = s.calls.saturating_add(1);
            });
        }
        ptr
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(ptr, layout, new_size) };
        if !next.is_null() {
            update(|s| {
                s.transient_envelope = s.transient_envelope.max(s.live.saturating_add(new_size));
                s.live = s
                    .live
                    .saturating_sub(layout.size())
                    .saturating_add(new_size);
                s.peak = s.peak.max(s.live);
                s.calls = s.calls.saturating_add(1);
            });
        }
        next
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        update(|s| s.live = s.live.saturating_sub(layout.size()));
    }
}
