use crate::display::pix_writer_impl::*;
use crate::sipo::{LatchGroup, LatchLine, ClearLine, PinCfg, Sipo};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::peripherals::Peripherals;

pub fn construct_bw_pixel_writer_def_pinout<'a>(peri: &mut Peripherals) -> BwPixelWriter<'a> {
    let haddr_sipo_cfg = PinCfg {
        srclr_al: None,
        rclk: None,
        srclk: peri.GPIO21.into(),
        ser: peri.GPIO20.into(),
    };

    let vaddr_sipo_cfg = PinCfg {
        srclr_al: None,
        rclk: None,
        srclk: peri.GPIO47.into(),
        ser: peri.GPIO48.into(),
    };

    let bw_sipo_cfg = PinCfg {
        srclr_al: None,
        rclk: None,
        srclk: peri.GPIO45.into(),
        ser: peri.GPIO0.into(),
    };
    
    
    let latch_line = LatchLine::new(Output::new(
        peri.GPIO39,
        esp_hal::gpio::Level::Low,
        esp_hal::gpio::OutputConfig::default().with_drive_mode(esp_hal::gpio::DriveMode::OpenDrain),
    ));
    
    let clear_line = ClearLine::new(
        Output::new(
            peri.GPIO40,
            esp_hal::gpio::Level::High,
            esp_hal::gpio::OutputConfig::default()
            .with_drive_mode(esp_hal::gpio::DriveMode::OpenDrain),
        ),
        true,
    );
    
    let latch_group = LatchGroup {
        latch: latch_line,
        clear: clear_line,
    };
    
    let bw_writer_cfg = BwWriterCfg {
        h_cfg: haddr_sipo_cfg,
        v_cfg: vaddr_sipo_cfg,
        bw_cfg: bw_sipo_cfg,
        latch_group: latch_group,
    };

    return construct_default_bw_pixel_writer(bw_writer_cfg);
}

pub fn display_pure_color(brightness : u8, writer : &mut BwPixelWriter<'_>){

    // Example: Fill the display with a solid color (e.g., medium brightness)
    for v in 0u8..=150 {
        for h in 0u8..=200 {
            let colors = [brightness]; // Single channel (BW)
            writer.write_pixel(h, v, &colors);
        }
    }
}
