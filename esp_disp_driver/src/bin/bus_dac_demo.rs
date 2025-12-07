#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::{info, println};
use embassy_executor::Spawner;
use esp_hal::{
    gpio::{Level, Output, OutputConfig, InputConfig, DriveMode, AnyPin},
    interrupt::software::SoftwareInterruptControl,
    system::{Cpu, Stack},
    timer::timg::TimerGroup,
    peripherals,
    clock::CpuClock,
};
use esp_rtos::embassy::Executor;
use panic_rtt_target as _;
use esp_disp_driver::{anypins_from_peri, sipo};
use esp_disp_driver::display::drawer;
use esp_disp_driver::display::backend::bus_dac::*;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
extern crate alloc;
static FB_INIT : FrameBuf = [[0; FB_WIDTH]; FB_HEIGHT];
static FRAMEBUF_CELL: StaticCell<DoubleFb> = StaticCell::new();


fn init_double_fb() -> &'static DoubleFb {
    FRAMEBUF_CELL.init(DoubleFb::new(FB_INIT))
}


pub async fn checkerboard_fade_task(fb: &'static DoubleFb) {
    const MAX_LUM4: i8 = 15;   // 4-bit peak brightness (0..=15)
    const CELL_SIZE: usize = 20; // checkerboard cell size in pixels
    
    // Current brightness for the "black" squares in 0..=15.
    let mut lum4: i8 = 0;
    // Direction of change: +1 (fade in) or -1 (fade out).
    let mut dir: i8 = 1;
    let mut offset : usize = 0;
    
    loop {
        // Clamp to the valid 4-bit range.
        let lum_black4 = lum4.clamp(0, MAX_LUM4) as u8;
        let lum_white4 = (MAX_LUM4 as u8).saturating_sub(lum_black4);
        
        // 1) Draw into inactive framebuffer.
        fb.with_inactive(|frame: &mut FrameBuf| {
            for (y, row) in frame.iter_mut().enumerate() {
                for (x, px) in row.iter_mut().enumerate() {
                    // Checkerboard pattern: (x/cell + y/cell) even/odd.
                    let is_black_square =
                    (((x + offset) / CELL_SIZE) + ((y + offset) / CELL_SIZE)) & 1 == 0;
                    
                    *px = if is_black_square {
                        lum_black4  // store directly in low 4 bits
                    } else {
                        lum_white4
                    };
                }
            }
        });
        
        // 2) Present the newly drawn frame.
        fb.swap();
        
        // 3) Control animation speed (adjust to taste).
        Timer::after_millis(200).await;
        
        // 4) Update brightness for the next frame.
        lum4 += dir;
        // offset = (offset + 1) % CELL_SIZE;
        if lum4 >= MAX_LUM4 {
            lum4 = MAX_LUM4;
            dir = -1; // start fading out
        } else if lum4 <= 0 {
            lum4 = 0;
            dir = 1; // start fading in
        }
    }
}


esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.0.1
    let fb: &'static DoubleFb = init_double_fb();

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

    let haddr_pins : [AnyPin; 8] = anypins_from_peri!(peripherals; 21, 47, 48, 45, 0, 35, 36, 37);
    let vaddr_pins : [AnyPin; 8] = anypins_from_peri!(peripherals; 14, 13, 12, 11, 10, 9, 46, 3);
    let data_pins  : [AnyPin; 4] = anypins_from_peri!(peripherals; 4, 5, 6, 7);

    let pixel_writer = BwPixelWriter8h8v1ch4::with_hw_resources(
        VgaHwResources{
            haddr_pins,
            vaddr_pins,
            data_pins,
        },
        fb,
        None,
        Some(OutputConfig::default().with_drive_mode(DriveMode::OpenDrain)),
        Some(Level::Low),
    );


    static APP_CORE_STACK: StaticCell<Stack<8192>> = StaticCell::new();
    let app_core_stack = APP_CORE_STACK.init(Stack::new());
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        sw_int.software_interrupt0,
        sw_int.software_interrupt1,
        app_core_stack,
        move || {
            static EXECUTOR: StaticCell<Executor> = StaticCell::new();
            let executor = EXECUTOR.init(Executor::new());
            executor.run(|spawner| {
                // This task (scan loop) will run on core1:
                info!("Spawning bw8h8v1ch4_scan_task on core 1");
                spawner
                    .spawn(bw8h8v1ch4_scan_task(pixel_writer))
                    .ok();
            }); 
        },
    );


    checkerboard_fade_task(fb).await;
    loop{}
}