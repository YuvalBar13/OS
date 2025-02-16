#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]
extern crate alloc;

use alloc::boxed::Box;
use crate::file_system::fat16::FAtApi;
use bootloader_api::BootInfo;
use core::arch::asm;
use core::panic::PanicInfo;
use embedded_graphics::Drawable;

use embedded_graphics::prelude::PixelIteratorExt;
use terminal::output::framebuffer::Color;
use x86_64::VirtAddr;

static BOOT_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::new_default());
    config
};
bootloader_api::entry_point!(kernel_main, config = &BOOT_CONFIG);
mod file_system;
mod heap_alloc;
mod interrupts;
mod memory;
mod multitasking;
mod terminal;

extern "C" fn testa() {
    for _ in 0..5 {
        print!("a");
        x86_64::instructions::hlt();
    }
}

extern "C" fn testb() {
    for _ in 0..5 {
        print!("b");
        x86_64::instructions::hlt();
    }
}

extern "C" fn testc() {
    for _ in 0..5 {
        print!("c");
        x86_64::instructions::hlt();
    }
}
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    init(boot_info);
   multitasking::round_robin::add_task(testc);
    multitasking::round_robin::add_task(testa);
    multitasking::round_robin::add_task(testb);
    // multitasking::round_robin::add_task(testa);
    // multitasking::round_robin::add_task(testa);

    x86_64::instructions::hlt();
    x86_64::instructions::hlt();
    x86_64::instructions::hlt();
    x86_64::instructions::hlt();
    x86_64::instructions::hlt();
    println!("\n\nreal main");

   let mut fat = FAtApi::new();
    loop {
      terminal::interface::run(&mut fat);
        x86_64::instructions::hlt();
    }
}
extern "C" fn main_kernel_mode() {
       let mut fat = FAtApi::new();
    println!("\n\nmain");
    let a = Box::new(1);
    // multitasking::round_robin::add_task(testa);
    let a = 1;
    println!("{}", a +1 );
    loop {
        print!("m");
        x86_64::instructions::hlt();
    }
}
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    eprintln!("{}", _info);
    hlt_loop();
}

fn init(boot_info: &'static mut BootInfo) {
    let frame_buffer_optional = &mut boot_info.framebuffer;

    // free the wrapped framebuffer from the FFI-safe abstraction provided by bootloader_api
    let frame_buffer = frame_buffer_optional.take().unwrap();
    let my_frame_buffer = terminal::output::framebuffer::MyFrameBuffer::new(frame_buffer);
    terminal::output::framebuffer::init_writer(my_frame_buffer.shallow_copy().get_buffer());

    let mut frame_buffer = my_frame_buffer.get_buffer();
    //  let mut display = terminal::output::framebuffer::Display::new(&mut frame_buffer);
    print_logo();
    init_memory(boot_info);
    //



    init_interrupts();
}

fn init_interrupts() {
    interrupts::gdt::init();
    interrupts::interrupts::init_idt();
    unsafe { interrupts::interrupts::PICS.lock().initialize() }
    x86_64::instructions::interrupts::enable();
}

fn init_memory(boot_info: &'static mut BootInfo) {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.clone().take().unwrap());
    let mut mapper = unsafe { memory::paging::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::paging::BootInfoFrameAllocator::init(&boot_info.memory_regions) };

    heap_alloc::alloc::init_heap(&mut frame_allocator, &mut mapper)
        .expect("error initializing heap");
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

// fn print_image(display: &mut terminal::output::framebuffer::Display) {
//     let data = include_bytes!("logo_type11_bl.tga");
//     let tga: Tga<Rgb888> = Tga::from_slice(data).unwrap();
//     let mut current_y = 0;
//     let image = Image::new(&tga, Point::new(0, current_y as i32));
//     image.draw(display).unwrap();
// }
//
// fn spin_loop(iterations: u32) {
//     for _ in 0..iterations {
//         core::hint::spin_loop();
//     }
// }

fn print_logo() {
    let color1 = Color::new(255, 0, 0); // Red
    let color2 = Color::new(0, 255, 0); // Green
    let color3 = Color::new(0, 0, 255); // Blue
    let color4 = Color::new(255, 255, 0); // Yellow
    let color5 = Color::new(255, 165, 0); // Orange
    let color6 = Color::new(128, 0, 128); // Purple

    println!("\n\n");
    change_writer_color(color1);

    println!("                          /$$$$$$$  /$$     /$$ /$$$$$$   /$$$$$$ ");

    change_writer_color(color2);
    println!("                         | $$__  $$|  $$   /$$//$$__  $$ /$$__  $$");

    change_writer_color(color3);
    println!("                         | $$  \\ $$ \\  $$ /$$/| $$  \\ $$| $$  \\__/");

    change_writer_color(color4);
    println!("                         | $$$$$$$/  \\  $$$$/ | $$  | $$|  $$$$$$ ");

    change_writer_color(color5);
    println!("                         | $$__  $$   \\  $$/  | $$  | $$ \\____  $$");

    change_writer_color(color6);
    println!("                         | $$  \\ $$    | $$   | $$  | $$ /$$  \\ $$");

    change_writer_color(color1);
    println!("                         | $$  | $$    | $$   |  $$$$$$/|  $$$$$$/");

    change_writer_color(color2);
    println!("                         |__/  |__/    |__/    \\______/  \\______/ ");

    // Reset to default color
    change_writer_color(terminal::output::framebuffer::DEFAULT_COLOR);
    println!("\n\n\n\n");
    println!("Welcome to ryos os!!!\nfor help write 'help'");
}

fn change_writer_color(color: Color) {
    terminal::output::framebuffer::WRITER
        .get()
        .expect("Writer not initialized")
        .lock()
        .change_color(color);
}
