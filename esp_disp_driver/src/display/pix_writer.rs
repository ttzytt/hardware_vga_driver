use crate::sipo::*;
use core::marker::PhantomData;

pub trait PixelWriter<AddrT, ColorT>{
    fn write_pixel(&mut self, x: AddrT, y: AddrT, color: ColorT);
}

pub struct BwPixelWriter8h8v8ch<'a> {
    pub p_sipo_bank : ParallelBank<'a, 3, 1>,
}

impl<'a> PixelWriter<u8, u8> for BwPixelWriter8h8v8ch<'a> {
    fn write_pixel(&mut self, x: u8, y: u8, color: u8) {
        let frame = [
            [x],    // H address
            [y],    // V address
            [color] // BW channel
        ];
        self.p_sipo_bank.write_exact(frame);
    }
}