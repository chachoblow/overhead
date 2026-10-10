//! Written-stack watermark, NOT maximum reserved stack or a worst-case bound.
//! The target probe deliberately includes its own measurement/harness overhead.
const PATTERN: u32 = 0xa5c3_6996;

fn untouched_words(words: usize, mut read: impl FnMut(usize) -> u32) -> usize {
    (0..words).take_while(|&i| read(i) == PATTERN).count()
}

#[cfg(target_arch = "xtensa")]
mod target {
    use super::*;

    pub struct Painted {
        low: usize,
        high: usize,
        top: usize,
    }

    #[derive(Clone, Copy, Debug)]
    pub struct Watermark {
        pub stack_low: usize,
        pub stack_top: usize,
        pub paint_low: usize,
        pub paint_high: usize,
        pub untouched_bytes: usize,
        /// Includes unpainted live frames + 256-byte safety margin at painting.
        /// Not an upper bound on reserved stack: unwritten gaps/pattern matches
        /// can hide use. This is only the observed write-depth estimate.
        pub observed_bytes: usize,
    }

    unsafe extern "C" {
        static _stack_end_cpu0: u32;
        static _stack_start_cpu0: u32;
        static __stack_chk_guard: u32;
    }

    /// Paint free stack below SP, never a live Rust local or the HAL guard.
    ///
    /// # Safety
    /// Call only on CPU0's linker stack after esp_hal::init, with no task
    /// switching, foreign stack owner, or second-core use of this stack. Keep
    /// that invariant until scanning; only one outstanding paint is allowed.
    #[inline(never)]
    pub unsafe fn paint() -> Painted {
        let bottom = (&raw const _stack_end_cpu0) as usize;
        let top = (&raw const _stack_start_cpu0) as usize;
        let guard = (&raw const __stack_chk_guard) as usize;
        assert!(guard >= bottom && guard + 4 < top);
        let low = guard + 4;
        let high: usize;
        // SAFETY: linker-defined CPU0 stack grows down. Entire paint loop is a
        // single no-call, no-stack asm block: compiler cannot spill into the
        // range while filling it. Keep 256 bytes below current SP untouched for
        // Xtensa ABI spill slots. Interrupts may use free stack but finish before
        // this loop resumes; no asynchronous stack owner/core is started here.
        // Bounds check in asm prevents writes for an exhausted/out-of-range SP.
        unsafe {
            core::arch::asm!(
                "mov {high}, sp",
                "addi {high}, {high}, -128",
                "addi {high}, {high}, -128",
                "bgeu {cursor}, {high}, 3f",
                "bltu {top}, {high}, 3f",
                "2:",
                "s32i {pattern}, {cursor}, 0",
                "addi {cursor}, {cursor}, 4",
                "bltu {cursor}, {high}, 2b",
                "3:",
                high = out(reg) high,
                cursor = inout(reg) low => _,
                top = in(reg) top,
                pattern = in(reg) PATTERN,
                options(nostack),
            );
        }
        assert!(high > low && high < top && high.is_multiple_of(4));
        Painted { low, high, top }
    }

    #[inline(never)]
    fn exercise_stack() {
        // Black-box a reference, not the array value (which could create an
        // additional copy). The target disassembly must retain this 8 KiB local.
        let mut block = [!PATTERN; 2048];
        core::hint::black_box(&mut block);
    }

    #[inline(never)]
    unsafe fn calibration(loaded: bool) -> Watermark {
        let painted = unsafe { paint() };
        if loaded {
            exercise_stack();
        }
        unsafe { painted.scan() }
    }

    /// Check that the probe responds to a known written 8 KiB stack local.
    /// This is a smoke test, not a calibration to maximum reserved stack.
    ///
    /// # Safety
    /// Same CPU0 stack ownership requirements as `paint` and `scan`.
    pub unsafe fn self_check() -> (Watermark, Watermark) {
        let baseline = unsafe { calibration(false) };
        let loaded = unsafe { calibration(true) };
        assert!(
            loaded.observed_bytes >= baseline.observed_bytes + 4096,
            "stack probe did not observe the known stack local"
        );
        (baseline, loaded)
    }

    impl Painted {
        /// Read after the experiment returns. Volatile raw reads avoid claiming
        /// a Rust borrow of free stack; probe frames can inflate the mark.
        ///
        /// # Safety
        /// The CPU0 ownership/lifetime conditions of `paint` must still hold.
        #[inline(never)]
        pub unsafe fn scan(self) -> Watermark {
            let words = (self.high - self.low) / 4;
            let untouched = untouched_words(words, |i| unsafe {
                ((self.low + 4 * i) as *const u32).read_volatile()
            }) * 4;
            // Reaching this boundary invalidates the experiment even if the HAL
            // guard did not fire. No result is interpreted as safe capacity.
            assert!(untouched >= 256, "stack watermark exhausted its margin");
            Watermark {
                stack_low: (&raw const _stack_end_cpu0) as usize,
                stack_top: self.top,
                paint_low: self.low,
                paint_high: self.high,
                untouched_bytes: untouched,
                observed_bytes: self.top - (self.low + untouched),
            }
        }
    }
}

#[cfg(target_arch = "xtensa")]
pub use target::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_modified_word_not_total_modified_words_defines_watermark() {
        for (words, expected) in [
            ([PATTERN; 4], 4),
            ([PATTERN, PATTERN, 0, PATTERN], 2),
            ([0, PATTERN, PATTERN, PATTERN], 0),
        ] {
            assert_eq!(untouched_words(words.len(), |i| words[i]), expected);
        }
        assert_eq!(untouched_words(0, |_| panic!()), 0);
    }
}
