use crate::display::pix_writer::PixelWriter;
use crate::display::backend::utils::DoubleBuffer;
use crate::par_data_rw::*;
use esp_hal::{gpio::{AnyPin, InputConfig, OutputConfig, Level}, peripherals};
use defmt::info;
pub const FB_WIDTH: usize = 201;
pub const FB_HEIGHT: usize = 151;
pub type FrameBuf = [[u8; FB_WIDTH]; FB_HEIGHT];
pub type DoubleFb = DoubleBuffer<FrameBuf>;

pub struct BwPixelWriter8h8v1ch4<'a> {  
    pub haddr_reader : ParDataReader<'a, 8>,
    pub vaddr_reader : ParDataReader<'a, 8>,
    // unfortunately, the s3 dosn't have a DAC 
    pub data_writer  : ParDataWriter<'a, 4>,
    pub dbf : &'static DoubleFb,
}

pub struct VgaHwResources<'a, const HADDR_CNT : usize, const VADDR_CNT : usize, const DATA_CNT : usize> { 
    pub haddr_pins : [AnyPin<'a>; HADDR_CNT],
    pub vaddr_pins : [AnyPin<'a>; VADDR_CNT],
    pub data_pins  : [AnyPin<'a>; DATA_CNT],
}

impl <'a> BwPixelWriter8h8v1ch4<'a> {
    pub fn new(
        haddr_reader : ParDataReader<'a, 8>,
        vaddr_reader : ParDataReader<'a, 8>,
        data_writer  : ParDataWriter<'a, 4>,
        dbf : &'static DoubleFb,
    ) -> Self {
        BwPixelWriter8h8v1ch4{
            haddr_reader,
            vaddr_reader,
            data_writer,
            dbf,
        }
    }

    pub fn with_hw_resources(
        res : VgaHwResources<'a, 8, 8, 4>,
        dbf : &'static DoubleFb,
        iconf : Option<InputConfig>,
        oconf : Option<OutputConfig>,
        init_level : Option<Level>,
    ) -> Self {
        let iconf = iconf.unwrap_or(InputConfig::default());
        let oconf = oconf.unwrap_or(OutputConfig::default());
        let init_level = init_level.unwrap_or(Level::Low);
        let haddr_reader = ParDataReader::from_pins(res.haddr_pins, iconf);
        let vaddr_reader = ParDataReader::from_pins(res.vaddr_pins, iconf);
        let data_writer  = ParDataWriter::from_pins(res.data_pins, oconf, init_level);
        Self::new(haddr_reader, vaddr_reader, data_writer, dbf)
    }

    pub fn present_frame(&mut self) {
        self.dbf.swap();
    }

    pub async fn scan_loop(&mut self) {
        loop {
            let h = self.haddr_reader.read_u8() as usize;
            let v = self.vaddr_reader.read_u8() as usize;
            if h < FB_WIDTH && v < FB_HEIGHT {
                let fb = self.dbf;
                fb.with_active(|frame| {
                    let color = frame[v][h]; 
                    self.data_writer.write_u8(color);
                });
            } 
        }
    }
}

impl PixelWriter<u8, u8> for BwPixelWriter8h8v1ch4<'_> {
    fn write_pixel(&mut self, i: u8, j: u8, color: u8) {
        self.dbf.with_inactive(|frame| {
            frame[i as usize][j as usize] = color;
        });
    }

    #[inline(always)]
    fn addr_range(&self) -> ((u8, u8), (u8, u8)) {
        ((0, FB_HEIGHT as u8 - 1), (0, FB_WIDTH as u8 - 1))
    }

    #[inline(always)]
    fn color_range(&self) -> (u8, u8) {
        (0, 255)
    }
}

#[embassy_executor::task]
pub async fn bw8h8v1ch4_scan_task(mut writer: BwPixelWriter8h8v1ch4<'static>) {
    writer.scan_loop().await;
}