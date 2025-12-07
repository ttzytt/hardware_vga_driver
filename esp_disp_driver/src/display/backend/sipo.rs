use crate::sipo::*;
use crate::display::pix_writer::PixelWriter;
use esp_hal::{gpio::AnyPin, peripherals}; 

pub struct BwPixelWriter8h8v1ch8<'a> {
    // 8 bit for H address, 8 bit for V address
    // 1ch8: 1 channel, 8 bit color depth (BW)
    pub p_sipo_bank : ParallelBank<'a, 3, 1>,
}

pub struct VgaHwResources<'a>{ 
    pub rclk : AnyPin<'a>, 
    pub srclk : AnyPin<'a>, 
    pub srclr_al : AnyPin<'a>,
    pub data_ser : AnyPin<'a>,
    pub i_addr_ser : AnyPin<'a>,
    pub j_addr_ser : AnyPin<'a>,

}

impl BwPixelWriter8h8v1ch8<'_> {
    pub fn from_resources<'a>(res : VgaHwResources<'a>) -> BwPixelWriter8h8v1ch8<'a>{
        let control_pin_cfg = ControlPinCfg {
            srclk : res.srclk,
            rclk : Some(res.rclk),    
            srclr : Some(res.srclr_al),
            clr_active_low : true,
        };
        let control_group = ControlGroup::from_cfg(control_pin_cfg);
        let data_lane = SerLane::from_pin(res.data_ser);
        let i_addr_lane = SerLane::from_pin(res.i_addr_ser);
        let j_addr_lane = SerLane::from_pin(res.j_addr_ser);
        let p_sipo_bank = ParallelBank::new(
            [data_lane, i_addr_lane, j_addr_lane],
            control_group,
        );
        BwPixelWriter8h8v1ch8{
            p_sipo_bank
        }
    }
}

impl<'a> PixelWriter<u8, u8> for BwPixelWriter8h8v1ch8<'a> {
    fn write_pixel(&mut self, i: u8, j: u8, color: u8) {
        let frame = [
            [color], // BW channel
            [i],    // V address
            [j],    // H address
        ];
        self.p_sipo_bank.write_exact(frame);
    }

    #[inline(always)]
    fn addr_range(&self) -> ((u8, u8), (u8, u8)) {
        ((0, 150), (0, 200))
    }

    #[inline(always)]
    fn color_range(&self) -> (u8, u8) {
        (0, 255)
    }
}