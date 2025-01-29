use alloc::string::String;
use alloc::vec::Vec;
use crate::terminal::input::buffer::BUFFER;
use crate::{eprintln, print, print_logo, println};
use crate::file_system::fat16::FAtApi;

pub fn run(fs: &FAtApi) {
    print!(">>> ");
    let input = BUFFER.lock().get_input();
    println!();
    handle_command(input.as_str(), fs);
}


pub fn handle_command(command: &str, fs: &FAtApi) {
    let parts: Vec<&str> = command.splitn(2, ' ').collect();
    match parts[0] {
        "shutdown" => shutdown(),
        "reboot" => reboot(),
        "echo" => {
            if let Some(arg) = parts.get(1) {
                echo(arg);
            } else {
                println!("Usage: echo [text]");
            }
        }
        "clear" => clear_screen(),
        "help" => help(),
        "logo" => {
            clear_screen();
            print_logo();
        },
        "cat" => {
            if let Some(name) = parts.get(1) {
                cat(name, fs);
            }
        },  _ => eprintln!("{}: command not found", parts[0]),
    }
}

fn clear_screen() {
    crate::terminal::output::framebuffer::WRITER
        .get()
        .unwrap()
        .lock()
        .clear_screen();
}

fn echo(data: &str) {
    if data.starts_with('"') && data.ends_with('"') && data.len() > 2 {
        let result = &data[1..data.len() - 1];
        println!("{}", result);
        return;
    }
    println!("{}", data);
}
fn shutdown() {
    unsafe {
        use x86_64::instructions::port::Port;
        let mut port = Port::new(0x604);
        port.write(0x2000u16);
    }
}
fn reboot() {
    unsafe {
        let port: u16 = 0x64; // i8042 command port
        let value: u8 = 0xFE; // Reset command
        core::arch::asm!("out dx, al", in("dx") port, in("al") value);
    }
}
fn cat(name: &str, fs: &FAtApi) {
    match fs.index_by_name(name){
        Ok(index) => {
            match fs.get_data(index as usize) {
                Ok(data) => {
                    println!("{}", String::from_utf8_lossy(&data));
                }
                Err(e) => println!("Error: {:?}", e),
            }
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
fn help() {
    println!("clear - clear the screen");
    println!("echo - echo a string");
    println!("logo - print the logo");
    println!("shutdown - shutdown the computer");
    println!("reboot - reboot the computer");
}