use defmt::warn;
use esp_hal::{self as hal, gpio, peripherals::Peripherals};
use hal::gpio::{AnyPin, Io, Level, Output, OutputConfig, OutputPin};

struct PinCfg<'a> {
    // Pin configuration for SNx4HC595 Shift Register
    pub srclr_al: Option<AnyPin<'a>>, // Shift Register Clear (active low)
    pub rclk:     Option<AnyPin<'a>>, // Register Clock (latch)
    pub srclk:    AnyPin<'a>,         // Shift Register Clock
    pub ser:      AnyPin<'a>,         // Serial Data Input
}

struct Sipo<'a> {
    // Optional internal control lines: if None, expect an external shared control
    srclr_al_out: Option<Output<'a>>,
    rclk_out:     Option<Output<'a>>,
    // Always-owned per-device shift clock and data
    srclk_out:    Output<'a>,
    ser_out:      Output<'a>,
}

impl<'a> Sipo<'a> {
    pub fn new(pin_cfg: PinCfg<'a>) -> Self {
        let cfg = OutputConfig::default();
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

    pub fn latch_with(&mut self, shared_rclk: &mut Output<'a>) {
        shared_rclk.set_high();
        shared_rclk.set_low();
    }
}
