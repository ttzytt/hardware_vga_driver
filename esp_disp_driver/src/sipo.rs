use defmt::warn;
use esp_hal::{self as hal, gpio, peripherals::Peripherals};
use hal::gpio::{AnyPin, Io, Level, Output, OutputConfig, OutputPin};

/// Pin configuration for SNx4HC595 Shift Register
pub struct PinCfg<'a> {
    pub srclr_al: Option<AnyPin<'a>>, // Shift Register Clear (active low)
    pub rclk:     Option<AnyPin<'a>>, // Register Clock (latch)
    pub srclk:    AnyPin<'a>,         // Shift Register Clock
    pub ser:      AnyPin<'a>,         // Serial Data Input
}

/* ------------------------- Control-plane: shared lines ------------------------- */

/// Shared latch line (RCLK). Emits a single latch pulse.
pub struct LatchLine<'a> {
    rclk: Output<'a>,
}
impl<'a> LatchLine<'a> {
    pub fn new(rclk: Output<'a>) -> Self { Self { rclk } }
    #[inline]
    pub fn pulse(&mut self) {
        self.rclk.set_high();
        self.rclk.set_low();
    }
}

/// Shared clear line (SRCLR). For 74HC595, the line is active-low.
pub struct ClearLine<'a> {
    srclr: Output<'a>,
    active_low: bool,
}
impl<'a> ClearLine<'a> {
    /// `active_low = true` for 74HC595's \SRCLR.
    pub fn new(srclr: Output<'a>, active_low: bool) -> Self { Self { srclr, active_low } }
    #[inline]
    pub fn pulse(&mut self) {
        if self.active_low {
            self.srclr.set_low();
            self.srclr.set_high();
        } else {
            self.srclr.set_high();
            self.srclr.set_low();
        }
    }
}

/// Mandatory shared-control group: has both a shared RCLK and a shared SRCLR.
pub struct LatchGroup<'a> {
    latch: LatchLine<'a>,
    clear: ClearLine<'a>,
}
impl<'a> LatchGroup<'a> {
    pub fn new(latch: LatchLine<'a>, clear: ClearLine<'a>) -> Self { Self { latch, clear } }

    /// Perform all shifts inside `do_shifts` and then emit one shared latch pulse.
    pub fn shift_then_latch<F>(&mut self, do_shifts: F)
    where
        F: FnOnce(),
    {
        do_shifts();
        self.latch.pulse();
    }

    pub fn latch_all(&mut self) { self.latch.pulse(); }

    /// Emit one shared clear pulse for the whole group.
    #[inline] pub fn clear_all(&mut self) { self.clear.pulse(); }

    /// Expose the underlying lines if you need to pass them down.
    #[inline] pub fn latch_line(&mut self) -> &mut LatchLine<'a> { &mut self.latch }
    #[inline] pub fn clear_line(&mut self) -> &mut ClearLine<'a> { &mut self.clear }
}

/* --------------------------- Data-plane: SIPO chain --------------------------- */

/// SIPO chain with const-generic width (N bytes = 8*N bits).
/// N = 1 for a single 74HC595; N = 2 for two chips daisy-chained (1 -> 16), etc.
pub struct Sipo<'a, const N: usize> {
    // Optional internal control lines: if None, expect an external shared control.
    srclr_al_out: Option<Output<'a>>,
    rclk_out:     Option<Output<'a>>,
    // Always-owned per-device shift clock and data.
    srclk_out:    Output<'a>,
    ser_out:      Output<'a>,
}

impl<'a, const N: usize> Sipo<'a, N> {
    /// Create a SIPO. If you plan to use shared control lines, pass PinCfg with rclk/srclr_al = None
    /// or call `.with_external_control()` afterward.
    pub fn new(pin_cfg: PinCfg<'a>) -> Self {
        // This is for voltage level conversion from 3.3V to 4V
        let cfg = OutputConfig::default()
            .with_drive_mode(gpio::DriveMode::OpenDrain)
            .with_pull(gpio::Pull::Up);

        let mut s = Self {
            srclr_al_out: pin_cfg.srclr_al.map(|p| Output::new(p, Level::High, cfg)),
            rclk_out:     pin_cfg.rclk.map(|p| Output::new(p, Level::Low,  cfg)),
            srclk_out:    Output::new(pin_cfg.srclk,    Level::Low,  cfg),
            ser_out:      Output::new(pin_cfg.ser,      Level::Low,  cfg),
        };
        // Initial clear if internal \SRCLR exists.
        s.clear();
        s
    }

    /// Switch to external-control mode: detach internal RCLK/SRCLR drivers.
    pub fn with_external_control(mut self) -> Self {
        self.rclk_out = None;
        self.srclr_al_out = None;
        self
    }

    /// Info helpers
    #[inline] pub const fn width_bits(&self)  -> usize { 8 * N }
    #[inline] pub const fn width_bytes(&self) -> usize { N }

    /// Internal latch: use internal RCLK if present, else warn.
    pub fn latch(&mut self) {
        match &mut self.rclk_out {
            Some(r) => {
                assert!(r.is_set_low());
                r.set_high();
                r.set_low();
            }
            None => warn!("Attempted internal latch but no RCLK configured; use an external LatchLine."),
        }
    }

    /// Internal clear: use internal \SRCLR if present, else warn.
    pub fn clear(&mut self) {
        match &mut self.srclr_al_out {
            Some(c) => {
                assert!(c.is_set_high());
                c.set_low();
                c.set_high();
            }
            None => warn!("Attempted internal SRCLR but no SRCLR configured; use an external ClearLine."),
        }
    }

    /// Latch via an external shared latch line (from LatchGroup).
    #[inline] pub fn latch_via(&mut self, ext: &mut LatchLine<'a>) { ext.pulse(); }

    /// Clear via an external shared clear line (from LatchGroup).
    #[inline] pub fn clear_via(&mut self, ext: &mut ClearLine<'a>) { ext.pulse(); }

    /// Shift one byte MSB-first into the chain. For N>1, this sends only 1/N of a full frame.
    pub fn shift_byte(&mut self, byte: u8) {
        for i in 0..8 {
            let bit = (byte >> (7 - i)) & 0x01;
            if bit == 1 { self.ser_out.set_high(); } else { self.ser_out.set_low(); }
            self.srclk_out.set_high();
            self.srclk_out.set_low();
        }
    }

    /// Shift a sequence (MSB-first per byte), no latch.
    pub fn shift_byte_seq(&mut self, bytes: &[u8]) {
        for &b in bytes { self.shift_byte(b); }
    }

    /// Shift exactly N bytes (full chain width), no latch.
    /// Convention: far-end first, near-end last.
    pub fn shift_exact(&mut self, frame: &[u8; N]) {
        self.shift_byte_seq(frame);
    }

    /// Write one byte then latch (partial update for N>1).
    pub fn write_byte(&mut self, byte: u8) {
        self.shift_byte(byte);
        self.latch();
    }

    /// Write a sequence then latch (length may be anything).
    pub fn write_byte_seq(&mut self, bytes: &[u8]) {
        self.shift_byte_seq(bytes);
        self.latch();
    }

    /// Write exactly one full frame (N bytes) then latch (simultaneous output update).
    pub fn write_exact(&mut self, frame: &[u8; N]) {
        self.shift_exact(frame);
        self.latch();
    }

    /// External-control variant: shift full frame (N bytes) *without* latching.
    pub fn write_exact_external(&mut self, frame: &[u8; N]) {
        self.shift_exact(frame);
        // no internal latch; caller should use LatchGroup::pulse_latch()
    }
}

/* ----------------------------- Capability trait ------------------------------ */

/// Minimal shift-register capability with compile-time byte width.
pub trait ShiftDevN<const N: usize> {
    fn shift_bytes(&mut self, bytes: &[u8]);      // no latch
    fn shift_exact(&mut self, frame: &[u8; N]);   // no latch
    fn latch(&mut self);                          // latch outputs
    fn clear(&mut self);                          // clear shift-register contents
}

impl<'a, const N: usize> ShiftDevN<N> for Sipo<'a, N> {
    #[inline] fn shift_exact(&mut self, frame: &[u8; N]) { Sipo::shift_exact(self, frame) }
    #[inline] fn latch(&mut self) { Sipo::latch(self) }
    #[inline] fn clear(&mut self) { Sipo::clear(self) }
    #[inline] fn shift_bytes(&mut self, bytes: &[u8]) { Sipo::shift_byte_seq(self, bytes) }
}

/* --------------------- Parallel composition (same width) --------------------- */

/// A parallel bank of ShiftDevN devices (e.g., multiple SIPO chains),
/// all lanes have the same width N. This bank does NOT own shared lines:
/// - Use `*_internal()` when each lane owns its RCLK/SRCLR.
/// - Use `*_via()` with a LatchGroup to pulse shared lines once for multiple banks.
pub struct ParallelBank<D, const LANES: usize, const N: usize>
where
    D: ShiftDevN<N>,
{
    lanes: [D; LANES],
}

impl<D, const LANES: usize, const N: usize> ParallelBank<D, LANES, N>
where
    D: ShiftDevN<N>,
{
    pub fn new(lanes: [D; LANES]) -> Self { Self { lanes } }

    /// Shift exactly one full frame per lane (far-end first), no latch.
    pub fn shift_exact_per_lane(&mut self, frames: [[u8; N]; LANES]) {
        for (lane, frame) in self.lanes.iter_mut().zip(frames) {
            lane.shift_exact(&frame);
        }
    }

    /// Internal mode: latch each lane individually (for non-shared wiring).
    pub fn latch_all_internal(&mut self) {
        for lane in &mut self.lanes { lane.latch(); }
    }

    /// Internal mode: clear each lane individually.
    pub fn clear_all_internal(&mut self) {
        for lane in &mut self.lanes { lane.clear(); }
    }

    /// External mode: use a shared LatchGroup (one pulse for all banks).
    #[inline] pub fn latch_all_via<'a>(&mut self, g: &mut LatchGroup<'a>) { g.latch_line().pulse(); }

    /// External mode: use a shared ClearLine (one pulse for all banks).
    #[inline] pub fn clear_all_via<'a>(&mut self, g: &mut LatchGroup<'a>) { g.clear_line().pulse(); }

    pub fn lane_mut(&mut self, idx: usize) -> Option<&mut D> { self.lanes.get_mut(idx) }

    pub fn into_inner(self) -> [D; LANES] { self.lanes }
}