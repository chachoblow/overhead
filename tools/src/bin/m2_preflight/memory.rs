//! Host requested bytes only, not System footprint/RSS or target heap fit.
//! Deliberately use GlobalAlloc's allocate-copy-free realloc, like the existing
//! firmware meter, so the old and new allocations overlap in BOTH peak counters.
//! The CLI is single-threaded; tests use a local meter or a subprocess.
use serde::Serialize;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
    },
};

#[derive(Clone, Copy)]
struct Event {
    pointer: usize,
    size: usize,
}
const EMPTY: Event = Event {
    pointer: 0,
    size: 0,
};
struct Trace {
    events: [Event; 4096],
    count: usize,
    overflow: bool,
}

pub struct Meter {
    live: AtomicUsize,
    phase_peak: AtomicUsize,
    pipeline_peak: AtomicUsize,
    calls: AtomicUsize,
    requested: AtomicUsize,
    failures: AtomicUsize,
    active: AtomicBool,
    trace: Mutex<Trace>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    pub live: usize,
    pub calls: usize,
    pub requested: usize,
    pub failures: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Phase {
    pub before: Snapshot,
    pub after: Snapshot,
    pub peak: usize,
}

impl Meter {
    pub const fn new() -> Self {
        Self {
            live: AtomicUsize::new(0),
            phase_peak: AtomicUsize::new(0),
            pipeline_peak: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
            requested: AtomicUsize::new(0),
            failures: AtomicUsize::new(0),
            active: AtomicBool::new(false),
            trace: Mutex::new(Trace {
                events: [EMPTY; 4096],
                count: 0,
                overflow: false,
            }),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            live: self.live.load(Relaxed),
            calls: self.calls.load(Relaxed),
            requested: self.requested.load(Relaxed),
            failures: self.failures.load(Relaxed),
        }
    }
    pub fn start_pipeline(&self) -> Snapshot {
        let mut trace = self.trace.lock().unwrap();
        trace.count = 0;
        trace.overflow = false;
        self.calls.store(0, Relaxed);
        self.requested.store(0, Relaxed);
        self.failures.store(0, Relaxed);
        let start = self.snapshot();
        self.pipeline_peak.store(start.live, Relaxed);
        self.active.store(true, Relaxed);
        start
    }
    pub fn begin(&self) -> Snapshot {
        let start = self.snapshot();
        self.phase_peak.store(start.live, Relaxed);
        start
    }
    pub fn finish(&self, before: Snapshot) -> Phase {
        Phase {
            before,
            after: self.snapshot(),
            peak: self.phase_peak.load(Relaxed),
        }
    }
    pub fn stop_pipeline(&self) -> (usize, bool) {
        self.active.store(false, Relaxed);
        (
            self.pipeline_peak.load(Relaxed),
            self.trace.lock().unwrap().overflow,
        )
    }
    // Look up the live allocation backing a core-owned Vec's exposed slice.
    // No private-field casts, allocator rounding, pointer dereferences, or core
    // API changes. All queried slices are whole Vecs, never interior subslices.
    pub fn capacity<T>(&self, slice: &[T]) -> Option<usize> {
        if size_of::<T>() == 0 {
            return None;
        }
        let trace = self.trace.lock().unwrap();
        if trace.overflow {
            return None;
        }
        let size = trace.events[..trace.count]
            .iter()
            .rev()
            .find(|e| e.pointer == slice.as_ptr() as usize)
            .map(|e| e.size);
        match size {
            Some(n) if n > 0 && n % size_of::<T>() == 0 && n / size_of::<T>() >= slice.len() => {
                Some(n / size_of::<T>())
            }
            None if slice.is_empty() => Some(0),
            _ => None,
        }
    }
    fn event(&self, ptr: *mut u8, size: usize) {
        if !self.active.load(Relaxed) {
            return;
        }
        let mut trace = self.trace.lock().unwrap();
        let index = trace.count;
        if index == trace.events.len() {
            trace.overflow = true;
        } else {
            trace.events[index] = Event {
                pointer: ptr as usize,
                size,
            };
            trace.count += 1;
        }
    }
}

// SAFETY: all pointers/layouts are forwarded unchanged to System. Hooks neither
// allocate nor dereference pointers. Default realloc preserves the old allocation
// on failure and copies before deallocation, including shrink. Mutex poisoning
// cannot originate in event(): the bounded indexing is checked while locked.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if ptr.is_null() {
            self.failures.fetch_add(1, Relaxed);
        } else {
            let live = self.live.fetch_add(layout.size(), Relaxed) + layout.size();
            if self.active.load(Relaxed) {
                self.calls.fetch_add(1, Relaxed);
                self.requested.fetch_add(layout.size(), Relaxed);
                self.phase_peak.fetch_max(live, Relaxed);
                self.pipeline_peak.fetch_max(live, Relaxed);
            }
            self.event(ptr, layout.size());
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.event(ptr, 0);
        unsafe { System.dealloc(ptr, layout) };
        self.live.fetch_sub(layout.size(), Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlap_phase_reset_and_release() {
        let meter = Meter::new();
        let baseline = meter.start_pipeline();
        let phase = meter.begin();
        unsafe {
            let layout = Layout::from_size_align(32, 16).unwrap();
            let ptr = meter.alloc_zeroed(layout);
            assert!(!ptr.is_null());
            assert!((0..32).all(|i| *ptr.add(i) == 0));
            let ptr = meter.realloc(ptr, layout, 128);
            assert!(!ptr.is_null());
            assert_eq!(meter.finish(phase).peak, 160);
            let slice = std::slice::from_raw_parts(ptr.cast::<u64>(), 1);
            assert_eq!(meter.capacity(slice), Some(16));
            let phase = meter.begin();
            let ptr = meter.realloc(ptr, Layout::from_size_align(128, 16).unwrap(), 16);
            assert!(!ptr.is_null());
            assert_eq!(meter.finish(phase).peak, 144);
            meter.dealloc(ptr, Layout::from_size_align(16, 16).unwrap());
        }
        assert_eq!(meter.snapshot().live, baseline.live);
        assert_eq!(meter.stop_pipeline(), (160, false));
    }
    #[test]
    fn trace_overflow_fails_closed() {
        let meter = Meter::new();
        meter.start_pipeline();
        for _ in 0..4097 {
            meter.event(std::ptr::null_mut(), 0);
        }
        assert_eq!(meter.capacity::<u8>(&[]), None);
        assert!(meter.stop_pipeline().1);
    }
}
