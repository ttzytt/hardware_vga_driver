//! Parallel data reader over a group of GPIO input pins.
//!
//! The pins are treated as a parallel N-bit bus:
//! - `pins[0]` is the LSB,
//! - `pins[N - 1]` is the MSB.
//!
//! You can sample the current logic levels and return them as `u8`, `u16`, or `u32`.

use esp_hal::{self as hal, gpio::{InputConfig}};
use hal::gpio::{AnyPin, Input};

/// Parallel data reader over a group of GPIO input pins.
///
/// The generic parameter `N` is the number of pins (bus width).
///
/// Bit mapping:
/// - `pins[0]`   → bit 0 (LSB)
/// - `pins[N-1]` → bit N-1 (MSB)
pub struct ParDataReader<'a, const N: usize> {
    /// Underlying input pins.
    ///
    /// Each element is an `Input<'a>` wrapped around an `AnyPin<'a>`.
    /// (The concrete pin type is type-erased by `AnyPin`.)
    pins: [Input<'a>; N],
}

impl<'a, const N: usize> ParDataReader<'a, N> {
    /// Construct a new parallel data reader from an array of GPIO pins.
    ///
    /// Each pin is configured as an input with the given pull configuration.
    ///
    /// # Parameters
    ///
    /// - `pins`: array of `AnyPin<'a>`; the index determines the bit position
    ///   in the resulting value:
    ///   - `pins[0]` is the LSB (bit 0),
    ///   - `pins[N-1]` is the MSB (bit N-1).
    /// - `input_cfg`: input configuration (pull-up, pull-down, floating).
    pub fn from_pins(pins: [AnyPin<'a>; N], input_cfg : InputConfig) -> Self {
        // `Input<'a>` is generic over the concrete pin type, which here is `AnyPin<'a>`.
        // Type inference will set `T = AnyPin<'a>` for each `Input::new`.
        let pins: [Input<'a>; N] = pins.map(|p: AnyPin<'a>| Input::new(p, input_cfg));
        Self { pins }
    }

    /// Read the raw bit values as an array of booleans.
    ///
    /// - `bits[0]`   corresponds to LSB (pins[0]),
    /// - `bits[N-1]` corresponds to MSB (pins[N-1]).
    pub fn read_bits(&self) -> [bool; N] {
        let mut out: [bool; N] = [false; N];
        let mut i: usize = 0;
        while i < N {
            // `Input<'a>` in esp-hal provides `is_high()` / `is_low()` that return bool.
            out[i] = self.pins[i].is_high();
            i += 1;
        }
        out
    }

    /// Read the current bus value as `u8`.
    ///
    /// Mapping:
    /// - bit 0   ← `pins[0]`  (LSB)
    /// - bit N-1 ← `pins[N-1]` (MSB)
    ///
    /// # Panics
    ///
    /// Panics if `N > 8`, since not all bits can fit into `u8`.
    pub fn read_u8(&self) -> u8 {
        assert!(
            N <= 8,
            "ParDataReader::read_u8() called but N > 8; value would not fit into u8"
        );

        let mut value: u8 = 0;
        let mut i: usize = 0;
        while i < N {
            if self.pins[i].is_high() {
                // pins[i] is bit i, with pins[0] as LSB
                value |= 1u8 << (i as u8);
            }
            i += 1;
        }
        value
    }

    /// Read the current bus value as `u16`.
    ///
    /// Mapping:
    /// - bit 0   ← `pins[0]`  (LSB)
    /// - bit N-1 ← `pins[N-1]` (MSB)
    ///
    /// # Panics
    ///
    /// Panics if `N > 16`, since not all bits can fit into `u16`.
    pub fn read_u16(&self) -> u16 {
        assert!(
            N <= 16,
            "ParDataReader::read_u16() called but N > 16; value would not fit into u16"
        );

        let mut value: u16 = 0;
        let mut i: usize = 0;
        while i < N {
            if self.pins[i].is_high() {
                value |= 1u16 << (i as u16);
            }
            i += 1;
        }
        value
    }

    /// Read the current bus value as `u32`.
    ///
    /// Mapping:
    /// - bit 0   ← `pins[0]`  (LSB)
    /// - bit N-1 ← `pins[N-1]` (MSB)
    ///
    /// For `N > 32`, higher bits conceptually do not fit into `u32`; this
    /// function does **not** assert on `N` but bits beyond 31 are ignored
    /// at the type level (since you cannot shift into them).
    pub fn read_u32(&self) -> u32 {
        let mut value: u32 = 0;
        let mut i: usize = 0;
        while i < N && i < 32 {
            if self.pins[i].is_high() {
                value |= 1u32 << (i as u32);
            }
            i += 1;
        }
        value
    }

    /// Convenience alias: read the bus as a `u32`.
    #[inline]
    pub fn read(&self) -> u32 {
        self.read_u32()
    }

    /// Get a reference to the underlying input pins, e.g., for manual access.
    pub fn pins(&self) -> &[Input<'a>; N] {
        &self.pins
    }
}
