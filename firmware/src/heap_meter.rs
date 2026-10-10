//! Single-threaded phase accounting. No allocations in instrumentation.
//! Requested bytes and backend-reported occupancy are different quantities.
use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

pub trait Backend: GlobalAlloc {
    /// Backend's estimate, including its allocation rounding; not total SRAM.
    fn occupied(&self) -> usize;
}

#[cfg(target_arch = "xtensa")]
impl Backend for esp_alloc::EspHeap {
    fn occupied(&self) -> usize {
        self.used()
    }
}

pub struct Meter<A> {
    pub backend: A,
    live: AtomicUsize,
    peak: AtomicUsize,
    occupied_peak: AtomicUsize,
    calls: AtomicUsize,
    requested: AtomicUsize,
    failures: AtomicUsize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub requested_live: usize,
    pub occupied_live: usize,
    calls: usize,
    requested: usize,
    failures: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phase {
    /// Absolute occupancy, including any catalogue retained from initialization.
    pub requested_peak: usize,
    pub requested_live: usize,
    pub occupied_peak: usize,
    pub occupied_live: usize,
    pub calls: usize,
    pub requested: usize,
    pub failures: usize,
}

impl<A: Backend> Meter<A> {
    pub const fn new(backend: A) -> Self {
        Self {
            backend,
            live: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            occupied_peak: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
            requested: AtomicUsize::new(0),
            failures: AtomicUsize::new(0),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            requested_live: self.live.load(Relaxed),
            occupied_live: self.backend.occupied(),
            calls: self.calls.load(Relaxed),
            requested: self.requested.load(Relaxed),
            failures: self.failures.load(Relaxed),
        }
    }

    /// No concurrent allocating tasks/interrupt handlers, nor overlapping phases.
    pub fn begin(&self) -> Snapshot {
        let start = self.snapshot();
        self.peak.store(start.requested_live, Relaxed);
        self.occupied_peak.store(start.occupied_live, Relaxed);
        start
    }

    pub fn finish(&self, start: Snapshot) -> Phase {
        let end = self.snapshot();
        Phase {
            requested_peak: self.peak.load(Relaxed),
            requested_live: end.requested_live,
            occupied_peak: self.occupied_peak.load(Relaxed),
            occupied_live: end.occupied_live,
            calls: end.calls - start.calls,
            requested: end.requested - start.requested,
            failures: end.failures - start.failures,
        }
    }
}

// SAFETY: caller's layouts/pointers pass unchanged to the backend. Counters do
// not allocate, dereference, or panic. Default GlobalAlloc realloc allocates a
// new block BEFORE freeing the old one, so both requested/occupied peaks include
// that transient overlap (unlike the host System logical-realloc measurement).
unsafe impl<A: Backend> GlobalAlloc for Meter<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { self.backend.alloc(layout) };
        if ptr.is_null() {
            self.failures.fetch_add(1, Relaxed);
        } else {
            let live = self.live.fetch_add(layout.size(), Relaxed) + layout.size();
            self.peak.fetch_max(live, Relaxed);
            self.occupied_peak
                .fetch_max(self.backend.occupied(), Relaxed);
            self.calls.fetch_add(1, Relaxed);
            self.requested.fetch_add(layout.size(), Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.backend.dealloc(ptr, layout) };
        self.live.fetch_sub(layout.size(), Relaxed);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::alloc::System;

    struct TestBackend {
        used: AtomicUsize,
    }
    impl Backend for TestBackend {
        fn occupied(&self) -> usize {
            self.used.load(Relaxed)
        }
    }
    unsafe impl GlobalAlloc for TestBackend {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if layout.size() > 1024 {
                return core::ptr::null_mut();
            }
            let ptr = unsafe { System.alloc(layout) };
            if !ptr.is_null() {
                self.used
                    .fetch_add(layout.size().next_multiple_of(16), Relaxed);
            }
            ptr
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) };
            self.used
                .fetch_sub(layout.size().next_multiple_of(16), Relaxed);
        }
    }

    #[test]
    fn zero_growth_shrink_failure_and_release() {
        let meter = Meter::new(TestBackend {
            used: AtomicUsize::new(0),
        });
        let start = meter.begin();
        unsafe {
            let layout = Layout::from_size_align(17, 16).unwrap();
            let ptr = meter.alloc_zeroed(layout);
            assert!(!ptr.is_null());
            assert_eq!(ptr as usize % 16, 0);
            assert!((0..17).all(|i| *ptr.add(i) == 0));
            let ptr = meter.realloc(ptr, layout, 80);
            assert!(!ptr.is_null());
            assert_eq!(
                meter.finish(start),
                Phase {
                    requested_peak: 97,
                    requested_live: 80,
                    occupied_peak: 112,
                    occupied_live: 80,
                    calls: 2,
                    requested: 97,
                    failures: 0,
                }
            );
            let start = meter.begin();
            let layout = Layout::from_size_align(80, 16).unwrap();
            assert!(meter.realloc(ptr, layout, 2048).is_null());
            assert_eq!(meter.snapshot().requested_live, 80);
            let ptr = meter.realloc(ptr, layout, 9);
            assert!(!ptr.is_null());
            assert_eq!(
                meter.finish(start),
                Phase {
                    requested_peak: 89,
                    requested_live: 9,
                    occupied_peak: 96,
                    occupied_live: 16,
                    calls: 1,
                    requested: 9,
                    failures: 1,
                }
            );
            meter.dealloc(ptr, Layout::from_size_align(9, 16).unwrap());
            assert_eq!(meter.snapshot().requested_live, 0);
            assert_eq!(meter.snapshot().occupied_live, 0);
            let phase = meter.begin();
            assert_eq!(meter.finish(phase).requested_peak, 0);
            assert_eq!(meter.finish(phase).occupied_peak, 0);
        }
    }
}
