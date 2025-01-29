use alloc::string::String;
use alloc::vec::Vec;
use crate::terminal::input::buffer::BUFFER;
use crate::{eprintln, print, print_logo, println};
use crate::file_system::disk_driver::SECTOR_SIZE;
use crate::file_system::fat16::FAtApi;

pub fn run(fs: &mut FAtApi) {
    print!(">>> ");
    let input = BUFFER.lock().get_input();
    println!();
    handle_command(input.as_str(), fs);
}


pub fn handle_command(command: &str, fs: &mut FAtApi) {
    let parts: Vec<&str> = command.splitn(3, ' ').filter(|s| !s.is_empty()).collect();

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
            else { eprintln!("Usage: cat [name]") }
        },
        "write" => {
            if let Some(name) = parts.get(1) {
                if let Some(buffer) = parts.get(2) {
                    write(name, to_buffer(buffer), fs);
                }
                else { eprintln!("Usage: write [name] [buffer]") }
            }
            else { eprintln!("Usage: write [name] [buffer]") }

        },
        "ls" => {
            ls(fs);
        },
        "touch" => {
            if let Some(name) = parts.get(1) {
                touch(name, fs);
            }
            else { eprintln!("Usage: touch [name]") }
        },
        _ => eprintln!("{}: command not found", parts[0]),
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
                    for i in 0..SECTOR_SIZE {
                        if data[i] == 0 {
                            break;
                        }
                        print!("{}", data[i] as char);
                    }
                    println!(); // new line
                    match fs.save()
                    {
                        Ok(_) => {}
                        Err(e) => eprintln!("Error saving disk: {:?}", e)
                    }
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        }
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

fn write(name: &str, buffer: [u8; SECTOR_SIZE], fs: &mut FAtApi) {

    match fs.index_by_name(name)
    {
        Ok(index) => {
            fs.change_data(index as usize, &buffer).expect("Error writing to disk");
            match fs.save()
            {
                Ok(_) => {}
                Err(e) => eprintln!("Error saving disk: {:?}", e)
            }
        }
        Err(e) => {
            eprintln!("Error adding entry to disk {:?}", e);
        }
    }
}
fn help() {
    println!("clear - clear the screen");
    println!("echo - echo a string");
    println!("logo - print the logo");
    println!("shutdown - shutdown the computer");
    println!("reboot - reboot the computer");
    println!("cat - print the contents of a file");
    println!("write - write to a file");
    println!("ls - list the contents of the disk");
    println!("touch - create a new file");
}

fn to_buffer(str: &str) -> [u8; SECTOR_SIZE] {
    let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];
    for (index, char) in str.char_indices() {
        buffer[index] = char as u8;
    }
    buffer
}

fn ls(fs: &FAtApi)
{
    fs.list_dir();
}

fn touch(name: &str, fs: &mut FAtApi)
{
    match fs.new_entry(name) {
        Ok(_) => {
            match fs.save()
            {
                Ok(_) => {}
                Err(e) => eprintln!("Error saving disk: {:?}", e)
            }
        }
        Err(e) => {
            eprintln!("Error adding entry to disk {:?}", e);
        }
    }
}