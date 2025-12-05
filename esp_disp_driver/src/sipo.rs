use defmt::warn;
use esp_hal::{self as hal, gpio};
use hal::gpio::{AnyPin, Level, Output, OutputConfig};

/// Common output configuration for 74HC595-style shift registers.
///
/// - Open-drain with pull-up is useful when level-shifting from 3.3 V to a higher rail.
/// - You can adjust this if your hardware uses different wiring.
fn shiftreg_output_cfg() -> OutputConfig {
    OutputConfig::default()
        .with_drive_mode(gpio::DriveMode::OpenDrain)
        .with_pull(gpio::Pull::None)
}

/* ============================== CONTROL PLANE ============================== */

/// Latch line (RCLK).
///
/// A single pulse on this line latches the contents of the internal shift
/// register to the output register of all chained 74HC595 devices.
pub struct LatchLine<'a> {
    rclk: Output<'a>,
}

impl<'a> LatchLine<'a> {
    /// Create a latch line driver from a pin.
    pub fn from_pin(rclk: AnyPin<'a>) -> Self {
        let cfg = shiftreg_output_cfg();
        Self {
            rclk: Output::new(rclk, Level::Low, cfg),
        }
    }

    pub fn from_pin_w_cfg(rclk: AnyPin<'a>, cfg: OutputConfig) -> Self {
        Self {
            rclk: Output::new(rclk, Level::Low, cfg),
        }
    }

    /// Emit a single latch pulse: low -> high -> low.
    #[inline]
    pub fn pulse(&mut self) {
        self.rclk.set_high();
        self.rclk.set_low();
    }
}

/// Clear line (SRCLR).
///
/// For 74HC595, this line is active-low: pulling it low clears the shift
/// register contents.
pub struct ClearLine<'a> {
    srclr: Output<'a>,
    active_low: bool,
}

impl<'a> ClearLine<'a> {
    /// Create a clear line driver from a pin.
    ///
    /// `active_low` should be `true` for 74HC595's \SRCLR.
    pub fn from_pin(srclr: AnyPin<'a>, active_low: bool) -> Self {
        let cfg = shiftreg_output_cfg();
        let init_level = if active_low { Level::High } else { Level::Low };
        Self {
            srclr: Output::new(srclr, init_level, cfg),
            active_low,
        }
    }

    pub fn from_pin_w_cfg(srclr: AnyPin<'a>, active_low: bool, cfg: OutputConfig) -> Self {
        let init_level = if active_low { Level::High } else { Level::Low };
        Self {
            srclr: Output::new(srclr, init_level, cfg),
            active_low,
        }
    }

    /// Emit a single clear pulse according to the configured polarity.
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

/// Shared shift clock line (SRCLK).
///
/// Every tick on this line shifts the entire daisy chain of 74HC595 devices
/// by one bit.
pub struct ShiftClockLine<'a> {
    srclk: Output<'a>,
}

impl<'a> ShiftClockLine<'a> {
    /// Create a shift clock line driver from a pin.
    pub fn from_pin(srclk: AnyPin<'a>) -> Self {
        let cfg = shiftreg_output_cfg();
        Self {
            srclk: Output::new(srclk, Level::Low, cfg),
        }
    }

    /// Emit a single shift clock: low -> high -> low.
    #[inline]
    pub fn tick(&mut self) {
        self.srclk.set_high();
        self.srclk.set_low();
    }
}

/// Pin configuration for a control group (SRCLK, optional RCLK, optional SRCLR).
///
/// This can represent either a shared control bus for multiple lanes,
/// or a private control bundle for a single lane.
pub struct ControlPinCfg<'a> {
    /// Shift clock (SRCLK), required.
    pub srclk: AnyPin<'a>,
    /// Latch clock (RCLK), optional because it can be chained with srclk
    pub rclk: Option<AnyPin<'a>>,
    /// Clear line (\SRCLR), optional, usually active-low.
    pub srclr: Option<AnyPin<'a>>,
    pub clr_active_low: bool,
}

/// Complete control group for a set of shift-register chains.
///
/// Whether this group is "shared" or "exclusive" depends on how many lanes
/// you pass it to. The type itself does not enforce sharing vs exclusivity.
pub struct ControlGroup<'a> {
    pub shift: ShiftClockLine<'a>,
    pub latch: Option<LatchLine<'a>>,
    pub clear: Option<ClearLine<'a>>,
}

impl<'a> ControlGroup<'a> {
    /// Build a control group from pins.
    ///
    /// `active_low` should be `true` for typical 74HC595 wiring where \SRCLR
    /// is active-low.
    pub fn from_cfg(pins: ControlPinCfg<'a>) -> Self {
        let shift = ShiftClockLine::from_pin(pins.srclk);
        let latch = pins.rclk.map(LatchLine::from_pin);
        let clear = pins.srclr.map(|p| ClearLine::from_pin(p, pins.clr_active_low));
        Self { shift, latch, clear }
    }

    /// Pulse the latch line for all devices controlled by this group.
    ///
    /// If no latch line is configured, emit a warning and do nothing.
    #[inline]
    pub fn latch_all(&mut self) {
        if let Some(l) = &mut self.latch {
            l.pulse();
        } else {
            warn!(
                "Attempted latch_all() but no RCLK configured; \
                 configure ControlGroup.latch or call latch externally."
            );
        }
    }

    /// Pulse the clear line for all devices controlled by this group.
    ///
    /// If no clear line is configured, emit a warning and do nothing.
    #[inline]
    pub fn clear_all(&mut self) {
        if let Some(c) = &mut self.clear {
            c.pulse();
        } else {
            warn!(
                "Attempted clear_all() but no SRCLR configured; \
                 tie SRCLR high or configure a ClearLine."
            );
        }
    }
}

/* =============================== DATA PLANE =============================== */

/// Single SIPO lane: owns only a SER (serial data) output pin.
///
/// This type does **not** know anything about clocks or latches.
/// It is intentionally minimal so that the same lane abstraction can be
/// used both in a single-chain setup and in a shared-clock multi-lane setup.
pub struct SerLane<'a> {
    ser_out: Output<'a>,
}

impl<'a> SerLane<'a> {
    /// Create a SIPO data lane from a pin.
    pub fn from_pin(ser: AnyPin<'a>) -> Self {
        let cfg = shiftreg_output_cfg();
        Self {
            ser_out: Output::new(ser, Level::Low, cfg),
        }
    }

    pub fn from_pin_w_cfg(ser: AnyPin<'a>, cfg: OutputConfig) -> Self {
        Self {
            ser_out: Output::new(ser, Level::Low, cfg),
        }
    }

    /// Drive the SER line to the given bit value.
    #[inline]
    pub fn set_bit(&mut self, bit: bool) {
        if bit {
            self.ser_out.set_high();
        } else {
            self.ser_out.set_low();
        }
    }
}


/* ======================= PARALLEL BANK (SHARED SRCLK) ======================= */

/// A parallel bank of SIPO lanes sharing a single control group.
///
/// - `LANES` is the number of independent chains (lanes).
/// - `N` is the number of bytes per lane (e.g., N=2 for two 74HC595 devices).
///
/// All lanes are shifted in lockstep using the shared `ControlGroup`:
/// - `ctrl.shift` provides the SRCLK ticks.
/// - `ctrl.latch` (optional) provides a shared latch (RCLK).
/// - `ctrl.clear` (optional) provides a shared clear (SRCLR).
pub struct ParallelBank<'a, const LANES: usize, const N: usize> {
    pub lanes: [SerLane<'a>; LANES],
    pub ctrl:  ControlGroup<'a>,
}

impl<'a, const LANES: usize, const N: usize> ParallelBank<'a, LANES, N> {
    /// Create a new parallel bank from an array of lanes and a control group.
    ///
    /// The `ControlGroup` is owned by this bank. If you need to share the same
    /// control lines across multiple banks, you will need to wrap it in some
    /// form of shared ownership (e.g., interior mutability) at a higher layer.
    pub fn new(lanes: [SerLane<'a>; LANES], ctrl: ControlGroup<'a>) -> Self {
        Self { lanes, ctrl }
    }
    pub fn shift_exact(&mut self, frames: [[u8; N]; LANES]) {
        let total_bit = 8 * N;
        for bit_idx in 0..total_bit {
            let byte_idx = bit_idx / 8;
            let bit_in_byte = 7 - (bit_idx % 8);
            for lane_idx in 0..LANES {
                let byte = frames[lane_idx][byte_idx];
                let bit = ((byte >> bit_in_byte) & 0x01) != 0;
                self.lanes[lane_idx].set_bit(bit);
            }
            self.ctrl.shift.tick();
        }
    }

    /// Shift one full frame per lane and then latch once via the control group.
    ///
    /// - Uses the bank's `ctrl.shift` as the shared SRCLK.
    /// - Uses `ctrl.latch` if available; otherwise emits a warning.
    pub fn write_exact(&mut self, frames: [[u8; N]; LANES]) {
        self.shift_exact(frames);
        self.ctrl.latch_all();
    }

    /// Clear all outputs via the control group, if a clear line is configured.
    pub fn clear_all(&mut self) {
        self.ctrl.clear_all();
    }
}


/* =========================== SINGLE-CHAIN WRAPPER =========================== */

pub struct SipoSingle<'a, const N: usize> {
    pub lane: SerLane<'a>,
    pub ctrl: ControlGroup<'a>,
}

impl<'a, const N: usize> SipoSingle<'a, N> {
    pub fn new(lane: SerLane<'a>, ctrl: ControlGroup<'a>) -> Self {
        Self { lane, ctrl }
    }


    /// Shift one full frame (N bytes) without latching.
    ///
    /// The caller may later call `self.ctrl.latch_all()` if it wants to latch
    /// separately. For convenience, `write_exact` does both.
    pub fn shift_exact(&mut self, frame: &[u8; N]) {
        // For a single lane, we treat it as LANES = 1.
        for bit in 0..(8 * N) {
            let byte_idx = bit / 8;
            let bit_in_byte = 7 - (bit % 8);
            let byte = frame[byte_idx];
            let bit_val = ((byte >> bit_in_byte) & 0x01) != 0;
            self.lane.set_bit(bit_val);
            self.ctrl.shift.tick();
        }
    }

    /// Shift one full frame and then latch once.
    pub fn write_exact(&mut self, frame: &[u8; N]) {
        self.shift_exact(frame);
        self.ctrl.latch_all();
    }

    /// Clear the chain using the control group's clear line, if present.
    pub fn clear(&mut self) {
        self.ctrl.clear_all();
    }
}
