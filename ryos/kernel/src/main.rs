#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::fmt::Write;
use core::panic::PanicInfo;
use bootloader_api::info::FrameBufferInfo;
use conquer_once::spin::OnceCell;
use bootloader_x86_64_common::logger::LockedLogger;
use noto_sans_mono_bitmap::{FontWeight, RasterHeight};
use crate::terminal::output::framebuffer::Writer;

bootloader_api::entry_point!(kernel_main);
mod terminal;
mod interrupts;

// ↓ this replaces the `_start` function ↓
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    init(boot_info);

    ///x86_64::instructions::interrupts::int3();

    loop {
        //terminal::interface::run();
       // x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    hlt_loop();
}

pub(crate) static LOGGER: OnceCell<LockedLogger> = OnceCell::uninit();
pub(crate) fn init_logger(buffer: &'static mut [u8], info: FrameBufferInfo) {
    let logger = LOGGER.get_or_init(move || LockedLogger::new(buffer, info, true, false));
    log::set_logger(logger).expect("Logger already set");
    log::set_max_level(log::LevelFilter::Trace);
    log::info!("Hello, Kernel Mode!");
}

fn init(boot_info: &'static mut bootloader_api::BootInfo)
{
    let frame_buffer_optional = &mut boot_info.framebuffer;

    // free the wrapped framebuffer from the FFI-safe abstraction provided by bootloader_api
    let frame_buffer_option = frame_buffer_optional.as_mut();

    // unwrap the framebuffer
    let frame_buffer_struct = frame_buffer_option.unwrap();

    use core::fmt::Write;

    // Create a writer with white text
    let mut writer = Writer::new(
        frame_buffer_struct,
        terminal::output::framebuffer::Color { red: 255, green: 255, blue: 255 },
        RasterHeight::Size32,
        FontWeight::Regular,
    );
    writer.clear_screen_with_color(terminal::output::framebuffer::Color { red: 0, green: 0, blue: 0 });
    // Write some text
    writer.write_str("Hello, world!\n").unwrap();
    // Or use the write! macro
    write!(writer, "Current value: {}\n", 42).unwrap();
    // finally, initialize the logger using the last two variables
    // init_logger(raw_frame_buffer, frame_buffer_info);
    // my_info!("Logger initialized");
    //
    // interrupts::interrupts::init_idt();
    // my_info!("IDT initialized");
    //
    // unsafe { interrupts::interrupts::PICS.lock().initialize() };
    // x86_64::instructions::interrupts::enable();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}