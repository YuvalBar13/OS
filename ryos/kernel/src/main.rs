#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
use core::panic::PanicInfo;
use bootloader_api::info::FrameBufferInfo;
use conquer_once::spin::OnceCell;
use bootloader_x86_64_common::logger::LockedLogger;

bootloader_api::entry_point!(kernel_main);
mod terminal;
mod interrupts;

// ↓ this replaces the `_start` function ↓
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    init(boot_info);

    //x86_64::instructions::interrupts::int3();
    unsafe {
        *(0xdeadbeef as *mut u8) = 42; //page fault that should trigger double fault
    };

    loop {
        terminal::interface::run();
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    log::error!("{}", _info);
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

    // extract the framebuffer info and, to satisfy the borrow checker, clone it
    let frame_buffer_info = frame_buffer_struct.info().clone();

    // get the framebuffer's mutable raw byte slice
    let raw_frame_buffer = frame_buffer_struct.buffer_mut();

    // finally, initialize the logger using the last two variables
    init_logger(raw_frame_buffer, frame_buffer_info);
    my_info!("Logger initialized");

    interrupts::gdt::init();
    interrupts::interrupts::init_idt();
    my_info!("IDT initialized");

    unsafe { interrupts::interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}