// src/pix_writer_impl.rs

use core::marker::PhantomData;

use crate::sipo::{LatchGroup, PinCfg, Sipo};
use crate::display::pix_writer::{
    AddrShifter, ColorShifter, DefAddrShifter, BwColorShifter8Bit, PixelWriter,
};

/// Convenient alias for a default 1-channel (BW), 8-bit pixel writer:
/// Equivalent to:
/// `PixelWriter<'a, u8, u8, DefAddrShifter<'a>, BwColorShifter8Bit<'a>, 1>`
pub type BwPixelWriter<'a> =
    PixelWriter<'a, u8, u8, DefAddrShifter<'a>, BwColorShifter8Bit<'a>, 1>;

/// Construct a BW pixel writer from already-initialized SIPO chains and a latch group.
///
/// - `h_sipo`: SIPO for horizontal address (1 byte)
/// - `v_sipo`: SIPO for vertical address (1 byte)
/// - `bw_sipo`: SIPO for BW channel data (1 byte)
/// - `latch_group`: shared latch/clear control
pub fn construct_default_bw_pixel_writer_from_sipo<'a>(
    h_sipo: Sipo<'a, 1>,
    v_sipo: Sipo<'a, 1>,
    bw_sipo: Sipo<'a, 1>,
    latch_group: LatchGroup<'a>,
) -> BwPixelWriter<'a> {
    let addr_shifter = DefAddrShifter { h_sipo, v_sipo };
    let color_shifter = BwColorShifter8Bit { sipo: bw_sipo };

    BwPixelWriter::new(
        addr_shifter,
        color_shifter,
        latch_group,
    )
}

/// Input wiring/config used to build a BW pixel writer internally.
pub struct BwWriterCfg<'a> {
    /// SIPO pin config for horizontal address (1 byte)
    pub h_cfg: PinCfg<'a>,
    /// SIPO pin config for vertical address (1 byte)
    pub v_cfg: PinCfg<'a>,
    /// SIPO pin config for BW data channel (1 byte)
    pub bw_cfg: PinCfg<'a>,
    /// Shared latch/clear control
    pub latch_group: LatchGroup<'a>,
}

/// Construct a BW pixel writer from pin configs (creates the SIPOs internally).
pub fn construct_default_bw_pixel_writer<'a>(cfg: BwWriterCfg<'a>) -> BwPixelWriter<'a> {
    // Create three single-byte SIPO chains: H address, V address, BW channel.
    let h_sipo = Sipo::<1>::new(cfg.h_cfg);
    let v_sipo = Sipo::<1>::new(cfg.v_cfg);
    let bw_sipo = Sipo::<1>::new(cfg.bw_cfg);

    construct_default_bw_pixel_writer_from_sipo(h_sipo, v_sipo, bw_sipo, cfg.latch_group)
}
