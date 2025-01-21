#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

bootloader_api::entry_point!(kernel_main);
mod terminal;
mod interrupts;

// ↓ this replaces the `_start` function ↓
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    init(boot_info);

    x86_64::instructions::interrupts::int3();

    loop {
        terminal::interface::run();
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    eprintln!("{}", _info);
    hlt_loop();
}


fn init(boot_info: &'static mut bootloader_api::BootInfo)
{
    let frame_buffer_optional = &mut boot_info.framebuffer;

    // free the wrapped framebuffer from the FFI-safe abstraction provided by bootloader_api
    let frame_buffer = frame_buffer_optional.take().unwrap();



    terminal::output::framebuffer::init_writer(frame_buffer);

    //interrupts::gdt::init();
    interrupts::interrupts::init_idt();
    println!("IDT initialized");

    unsafe { interrupts::interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}