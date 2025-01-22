#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::fmt::Write;
use core::panic::PanicInfo;
use embedded_graphics::Drawable;
use embedded_graphics::image::Image;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::Point;
use tinytga::Tga;

bootloader_api::entry_point!(kernel_main);
mod terminal;
mod interrupts;

// ↓ this replaces the `_start` function ↓
fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    init(boot_info);


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
    let my_frame_buffer = terminal::output::framebuffer::MyFrameBuffer::new(frame_buffer);
    terminal::output::framebuffer::init_writer(my_frame_buffer.shallow_copy().get_buffer());
    let mut frame_buffer =  my_frame_buffer.get_buffer();
    let mut display = terminal::output::framebuffer::Display::new(&mut frame_buffer);
    print_logo();

    init_interrupts();
}

fn init_interrupts() {
    interrupts::gdt::init();
    interrupts::interrupts::init_idt();
    unsafe { interrupts::interrupts::PICS.lock().initialize() }
    x86_64::instructions::interrupts::enable();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

fn print_image(display: &mut terminal::output::framebuffer::Display)
{
    let data = include_bytes!("logo_type11_bl.tga");
    let tga: Tga<Rgb888> = Tga::from_slice(data).unwrap();
    let image = Image::new(&tga, Point::zero());    // at the second arg should put the x and y of the image
    image.draw(display).unwrap();

}
use terminal::output::framebuffer::Color;
fn print_logo() {
    let color1 = Color::new(255, 0, 0);    // Red
    let color2 = Color::new(0, 255, 0);    // Green
    let color3 = Color::new(0, 0, 255);    // Blue
    let color4 = Color::new(255, 255, 0);  // Yellow
    let color5 = Color::new(255, 165, 0);  // Orange
    let color6 = Color::new(128, 0, 128);  // Purple

    println!("\n\n\n\n");
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
}

fn change_writer_color(color: Color) {
    terminal::output::framebuffer::WRITER
        .get()
        .expect("Writer not initialized")
        .lock()
        .change_color(color);
}
