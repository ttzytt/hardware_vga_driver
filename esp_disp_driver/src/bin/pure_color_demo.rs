#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::{info, println};
use embassy_executor::Spawner;
use esp_hal::{clock::CpuClock, gpio::Output, gpio};
use esp_hal::timer::timg::TimerGroup;
use panic_rtt_target as _;
use esp_disp_driver::sipo;
use esp_disp_driver::display::{drawer, pix_writer};
use embassy_time::{Duration, Timer};
extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.0.1

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let mut peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    info!("Embassy initialized!");
    // TODO: Spawn some tasks
    let _ = spawner;


    let vga_res = pix_writer::VgaResources{
        rclk : peripherals.GPIO20.into(),
        srclk : peripherals.GPIO21.into(),
        srclr_al : peripherals.GPIO47.into(),
        data_ser : peripherals.GPIO48.into(),
        i_addr_ser : peripherals.GPIO45.into(),
        j_addr_ser : peripherals.GPIO0.into(),
    };

    let mut pixel_writer = pix_writer::BwPixelWriter8h8v1ch8::from_resources(vga_res);
    let mut drawer = drawer::Drawer::new(&mut pixel_writer);

    let mut cur_brightness = 0;
    loop {
        drawer.fill_screen(cur_brightness);
        cur_brightness += 1;
        if cur_brightness == 255 {
            cur_brightness = 0;
        }
        Timer::after(Duration::from_millis(100)).await;

    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples/src/bin
}
