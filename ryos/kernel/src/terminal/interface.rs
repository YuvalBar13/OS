use crate::terminal::input::buffer::BUFFER;
use crate::{print, println};
use heapless::String;
use x86_64::instructions::port::Port;

pub fn run() {
    print!(">>> ");
    let input = BUFFER.lock().get_input();
    println!();
    handle_input(input);
}

fn handle_input(input: String<{ crate::terminal::input::buffer::BUFFER_SIZE }>) {
    if input == "clear" {
        crate::terminal::output::framebuffer::WRITER
            .get()
            .unwrap()
            .lock()
            .clear_screen();
    } else if let Some(data) = input.strip_prefix("echo ") {
        if data.starts_with("") && data.ends_with("") && data.len() > 1 {
            let result = &data[1..data.len() - 1];
            println!("{}", result);
            return;
        }
        println!("{}", data);
    } else if input == "exit" || input == "shut down" {
        unsafe {
            use x86_64::instructions::port::Port;
            let mut port = Port::new(0x604);
            port.write(0x2000u16);
        }
    } else if input == "logo" {
        handle_input("clear".into());
        crate::print_logo();
    } else if input == "help" {
        println!("clear - clear the screen");
        println!("echo - echo a string");
        println!("logo - print the logo");
        println!("shutdown - shutdown the computer");
        println!("reboot - reboot the computer");

    } else if input == "shutdown" {
        println!("asdfas");
        unsafe {
            let mut port: Port<u16> = Port::new(0x604); // ACPI command port
            port.write(0x2000); // Command to shut down the system
        }
    } else if input == "reboot" {
        unsafe {
            let port: u16 = 0x64; // i8042 command port
            let value: u8 = 0xFE; // Reset command
            core::arch::asm!("out dx, al", in("dx") port, in("al") value);
        }
    } else {
        println!("{}: Unknown command", input);
    }
}
