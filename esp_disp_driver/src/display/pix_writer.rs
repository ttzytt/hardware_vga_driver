use crate::sipo::*;
use core::marker::PhantomData;

pub trait AddrShifter<H, V> {
    fn shift_addr(&mut self, h_addr: H, v_addr: V);
}

pub struct DefAddrShifter<'a> {
    pub h_sipo: Sipo<'a, 1>,
    pub v_sipo: Sipo<'a, 1>,
}

impl AddrShifter<u8, u8> for DefAddrShifter<'_> {
    fn shift_addr(&mut self, h_addr: u8, v_addr: u8) {
        self.h_sipo.shift_byte(h_addr);
        self.v_sipo.shift_byte(v_addr);
    }
}

/// Trait for shifting color data with given number of channels and bit depth per channel.
/// The brightness of each channel will always be sent through 1 byte (so that the maximum possible color depth is 8 bits).
pub trait ColorShifter<const CHS: usize> {
    fn shift_color(&mut self, colors: &[u8; CHS]);
}

pub struct BwColorShifter8Bit<'a> {
    pub sipo: Sipo<'a, 1>,
}

impl ColorShifter<1> for BwColorShifter8Bit<'_> {
    fn shift_color(&mut self, colors: &[u8; 1]) {
        self.sipo.shift_byte(colors[0]);
    }
}

pub struct RgbColorShifter8Bit<'a> {
    pub r_sipo: Sipo<'a, 1>,
    pub g_sipo: Sipo<'a, 1>,
    pub b_sipo: Sipo<'a, 1>,
}

pub struct PixelWriter<'a, H, V, AS, CS, const CHS: usize>
where
    AS: AddrShifter<H, V>,
    CS: ColorShifter<CHS>,
{
    pub addr_shifter: AS,
    pub color_shifter: CS,
    pub latch_group: LatchGroup<'a>,
    // TODO: add optional flag on if the SRAM have finished writing before latching
    _marker: PhantomData<(H, V)>,
}

impl<'a, H, V, AS, CS, const CHS: usize> PixelWriter<'a, H, V, AS, CS, CHS>
where
    AS: AddrShifter<H, V>,
    CS: ColorShifter<CHS>,
{
    pub fn write_pixel(&mut self, h_addr: H, v_addr: V, colors: &[u8; CHS]) {
        self.addr_shifter.shift_addr(h_addr, v_addr);
        self.color_shifter.shift_color(colors);
        self.latch_group.latch_all();
    }

    pub fn new(
        addr_shifter: AS,
        color_shifter: CS,
        latch_group: LatchGroup<'a>,
    ) -> Self {
        PixelWriter {
            addr_shifter,
            color_shifter,
            latch_group,
            _marker: PhantomData::<(H, V)>,
        }
    }
}

