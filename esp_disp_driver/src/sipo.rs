use defmt::warn;
use esp_hal::{self as hal, gpio, peripherals::Peripherals};
use hal::gpio::{AnyPin, Io, Level, Output, OutputConfig, OutputPin};

pub struct PinCfg<'a> {
    // Pin configuration for SNx4HC595 Shift Register
    pub srclr_al: Option<AnyPin<'a>>, // Shift Register Clear (active low)
    pub rclk:     Option<AnyPin<'a>>, // Register Clock (latch)
    pub srclk:    AnyPin<'a>,         // Shift Register Clock
    pub ser:      AnyPin<'a>,         // Serial Data Input
}

// SIPO chain with const-generic width (N bytes = 8*N bits).
// N = 1 for a single 74HC595; N = 2 for two chips daisy-chained (1 -> 16), etc.
pub struct Sipo<'a, const N: usize> {
    // Optional internal control lines: if None, expect an external shared control
    srclr_al_out: Option<Output<'a>>,
    rclk_out:     Option<Output<'a>>,
    // Always-owned per-device shift clock and data
    srclk_out:    Output<'a>,
    ser_out:      Output<'a>,
}

impl<'a, const N: usize> Sipo<'a, N> {
    pub fn new(pin_cfg: PinCfg<'a>) -> Self {
        let cfg = OutputConfig::default()
            .with_drive_mode(gpio::DriveMode::OpenDrain)
            .with_pull(gpio::Pull::Up);
        let mut s = Self {
            // If present, create internal outputs; otherwise leave as None for external control
            srclr_al_out: pin_cfg.srclr_al.map(|p| Output::new(p, Level::High, cfg)),
            rclk_out:     pin_cfg.rclk.map(|p| Output::new(p, Level::Low,  cfg)),
            srclk_out:    Output::new(pin_cfg.srclk,    Level::Low,  cfg),
            ser_out:      Output::new(pin_cfg.ser,      Level::Low,  cfg),
        };
        // Perform an initial shift-register clear if an internal \SRCLR is available
        s.clear();
        s
    }

    /// Total width in bits (for info/validation).
    #[inline]
    pub const fn width_bits(&self) -> usize { 8 * N }

    #[inline]
    pub const fn width_bytes(&self) -> usize { N }

    pub fn latch(&mut self) {
        match &mut self.rclk_out {
            Some(r) => {
                assert!(r.is_set_low());
                r.set_high();
                r.set_low();
            }
            None => {
                warn!("Attempted to use internal latch, but no RCLK pin configured.")
            }
        }
    }

    /// Clear the shift register using the *internal* \SRCLR (if available).
    /// Note: This only clears the shift register; call `latch` (internal or external)
    /// to propagate zeros to the parallel outputs.
    pub fn clear(&mut self) {
        match &mut self.srclr_al_out {
            Some(c) => {
                assert!(c.is_set_high());
                c.set_low();
                c.set_high();
            }
            None => {
                warn!("Attempted to use internal SRCLR, but no SRCLR pin configured.")
            }
        }
    }

    /// Shift one byte MSB-first into the chain. For N>1, this sends only 1/ N of a full frame.
    pub fn shift_byte(&mut self, byte: u8) {
        for i in 0..8 {
            let bit = (byte >> (7 - i)) & 0x01;
            if bit == 1 {
                self.ser_out.set_high();
            } else {
                self.ser_out.set_low();
            }
            self.srclk_out.set_high();
            self.srclk_out.set_low();
        }
    }

    /// Shift a sequence of bytes (MSB-first per byte). Length can be any number;
    /// for a full update of an N-byte chain, pass exactly N bytes (far-end first).
    pub fn shift_byte_seq(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.shift_byte(b);
        }
    }

    /// Convenience: shift exactly N bytes (full chain width). By convention,
    /// send the far-end byte first, near-end last, then latch.
    pub fn shift_exact(&mut self, frame: &[u8; N]) {
        self.shift_byte_seq(frame);
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.shift_byte(byte);
        self.latch();
    }

    pub fn write_byte_seq(&mut self, bytes: &[u8]) {
        self.shift_byte_seq(bytes);
        self.latch();
    }

    /// Write exactly one full frame (N bytes) then latch (simultaneous update).
    pub fn write_exact(&mut self, frame: &[u8; N]) {
        self.shift_exact(frame);
        self.latch();
    }

    pub fn latch_with(&mut self, shared_rclk: &mut Output<'a>) {
        shared_rclk.set_high();
        shared_rclk.set_low();
    }
}



pub trait ShiftDevN<const N: usize> {
    /// Shift a sequence of bytes (MSB-first per byte), without latching.
    fn shift_bytes(&mut self, bytes: &[u8]);
    fn shift_exact(&mut self, frame: &[u8; N]);

    /// Latch the shifted bits into the parallel output register.
    fn latch(&mut self);

    /// Clear the shift register (not the outputs); caller may latch zeros afterwards.
    fn clear(&mut self);
}


impl<'a, const N: usize> ShiftDevN<N> for Sipo<'a, N> {
    #[inline] fn shift_exact(&mut self, frame: &[u8; N]) { self.shift_exact(frame); }
    #[inline] fn latch(&mut self) { self.latch(); }
    #[inline] fn clear(&mut self) { self.clear(); }
    #[inline] fn shift_bytes(&mut self, bytes: &[u8]) { self.shift_byte_seq(bytes) }
}


/* ---------- Parallel composition enforcing same N at compile time ---------- */

/// A parallel bank of ShiftDevN devices (e.g., multiple SIPO chains),
/// all with the same BYTES = N. Optionally owns a shared RCLK to latch all at once.
pub struct ParallelBank<'a, D, const LANES: usize, const N: usize>
where
    D: ShiftDevN<N>,
{
    lanes: [D; LANES],
    shared_rclk: Option<Output<'a>>, // If Some: one pulse latches all lanes simultaneously
}

impl<'a, D, const LANES: usize, const N: usize> ParallelBank<'a, D, LANES, N>
where
    D: ShiftDevN<N>,
{
    /// Create a bank. If `shared_rclk` is Some, wire all lane RCLKs to that pin
    /// and do NOT give each lane its own internal RCLK (set Sipo.rclk = None).
    pub fn new(lanes: [D; LANES], shared_rclk: Option<Output<'a>>) -> Self {
        Self { lanes, shared_rclk }
    }

    /// Shift exactly one full frame per lane (far-end first), no latch.
    pub fn shift_exact_per_lane(&mut self, frames: [[u8; N]; LANES]) {
        for (lane, frame) in self.lanes.iter_mut().zip(frames) {
            lane.shift_exact(&frame);
        }
    }

    /// Latch all lanes: if a shared RCLK exists, pulse it once; else call lane.latch().
    pub fn latch_all(&mut self) {
        if let Some(r) = &mut self.shared_rclk {
            r.set_high();
            r.set_low();
        } else {
            for lane in &mut self.lanes {
                lane.latch();
            }
        }
    }

    /// Clear all lanes (shift-registers only). Caller may latch zeros afterwards.
    pub fn clear_all(&mut self) {
        for lane in &mut self.lanes {
            lane.clear();
        }
    }

    /// Mutably access a lane if needed.
    pub fn lane_mut(&mut self, idx: usize) -> Option<&mut D> {
        self.lanes.get_mut(idx)
    }

    /// Consume and return lanes.
    pub fn into_inner(self) -> [D; LANES] { self.lanes }
}