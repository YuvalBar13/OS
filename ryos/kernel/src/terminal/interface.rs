use crate::file_system::disk_driver::SECTOR_SIZE;
use crate::file_system::fat16::FAtApi;
use crate::terminal::input::buffer::BUFFER;
use crate::terminal::output::framebuffer::{Color, DEFAULT_COLOR};
use crate::{change_writer_color, eprintln, print, print_logo, println};
use alloc::string::{String};
use alloc::vec::Vec;
use spin::Mutex;
use spin::lazy::Lazy;
pub const OUTPUT_COLOR: Color = Color::new(255, 200, 35);
pub static WORKING_DIR: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::from("/")));
pub fn run(fs: &mut FAtApi) {
    print!("{}> ", WORKING_DIR.lock());
    let input = BUFFER.lock().get_input();
    println!();
    handle_command(input.as_str(), fs);
    fs.save().unwrap();
}

pub fn handle_command(command: &str, fs: &mut FAtApi) {
    let parts: Vec<&str> = command.splitn(3, ' ').filter(|s| !s.is_empty()).collect();
    change_writer_color(OUTPUT_COLOR);
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
        }
        "cat" => {
            if let Some(name) = parts.get(1) {
                cat(name, fs);
            } else {
                eprintln!("Usage: cat [name]")
            }
        }
        "write" => {
            if let Some(name) = parts.get(1) {
                if let Some(buffer) = parts.get(2) {
                    write(name, to_buffer(buffer), fs);
                } else {
                    eprintln!("Usage: write [name] [buffer]")
                }
            } else {
                eprintln!("Usage: write [name] [buffer]")
            }
        }
        "append" => {
            if let Some(name) = parts.get(1) {
                if let Some(buffer) = parts.get(2) {
                    append_data(name, to_buffer(buffer), fs);
                } else {
                    eprintln!("Usage: append [name] [buffer]")
                }
            } else {
                eprintln!("Usage: append [name] [buffer]")
            }
        }
        "ls" => {
            ls(fs);
        }
        "touch" => {
            if let Some(name) = parts.get(1) {
                touch(name, fs);
            } else {
                eprintln!("Usage: touch [name]")
            }

        }
        "mkdir" => {
            if let Some(name) = parts.get(1) {
                mkdir(name, fs);
            } else {
                eprintln!("mkdir: touch [name]")
            }
        }
        "rm" => {
            if let Some(name) = parts.get(1) {
                rm(name, fs);
            } else {
                eprintln!("Usage: rm [name]")
            }
        }
        "cd" => {
            if let Some(parm) = parts.get(1) {
                cd(parm, fs);
            } else {
                eprintln!("Usage: cd [path]")
            }
        }
        "multitasking" => {
            crate::test_multitasking();
        }
        _ => eprintln!("{}: command not found", parts[0]),
    }
    change_writer_color(DEFAULT_COLOR);
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
    let data = get_file_data(name, fs);
    if data.is_none() {
        return;
    }
    let data = data.unwrap();
    for i in 0..SECTOR_SIZE {
        if data[i] == 0 {
            if i != 0
            // in case that the file isn't empty but isn't full print a new line at the end
            {
                println!();
            }
            return;
        }
        print!("{}", data[i] as char);
    }
    println!(); // new line
}

fn get_file_data(name: &str, fs: &FAtApi) -> Option<[u8; SECTOR_SIZE]> {
    match fs.get_data(name) {
        Ok(data) => Some(data),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            None
        }
    }
}
fn write(name: &str, buffer: [u8; SECTOR_SIZE], fs: &mut FAtApi) {
    match fs.change_data(name, &buffer) {
        Ok(_) => {}
        Err(e) => eprintln!("Error {:?}", e),
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
    println!("rm - remove file");
    println!("multitasking - test multitasking");
    println!("append - add data to task");
}

fn to_buffer(str: &str) -> [u8; SECTOR_SIZE] {
    let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];
    for (index, char) in str.char_indices() {
        buffer[index] = char as u8;
    }
    buffer
}

fn ls(fs: &FAtApi) {
    fs.list_dir();
}

fn touch(name: &str, fs: &mut FAtApi) {
    match fs.add_file(name)
    {
        Ok(_) => {},
        Err(e) => eprintln!("Error adding file {:?}", e)
    }
}

fn rm(name: &str, fs: &mut FAtApi) {
    match fs.remove_entry(name)
    {
        Ok(_) => {},
        Err(e) => eprintln!("Error removing file {:?}", e)
    }
}

fn append_data(name: &str, new_data: [u8; SECTOR_SIZE], fs: &mut FAtApi) {
    let data = get_file_data(name, fs);
    if data.is_none() {
        return;
    }
    let mut data = data.unwrap();
    let mut new_data_index = 0;
    for i in 0..SECTOR_SIZE {
        if data[i] == 0 {
            data[i] = new_data[new_data_index];
            new_data_index += 1;
        }
    }
    write(name, data, fs);
}
fn mkdir(name: &str, fs: &mut FAtApi) {
    match fs.new_dir(name)
    {
        Ok(_) => {},
        Err(e) => eprintln!("Error adding dir {:?}", e)
    }
}

fn cd(parm: &str, fs: &FAtApi) {
    if parm == ".." {
       remove_last_path();
    }
    else {
        add_path(fs, parm);
    }
}
fn remove_last_path() {
    let mut dir = WORKING_DIR.lock();
    dir.pop();
    if let Some(pos) = dir.rfind('/') {
        if pos == 0 {
            // Keep at least the root `/`
            dir.truncate(1);
        } else {
            dir.truncate(pos+ 1);
        }
    }
}

fn add_path(fs: &FAtApi, dir_name: &str)
{
    match fs.search_directory(dir_name)
    {
        Err(e) => eprintln!("Error searching directory: {:?}", e),
        Ok(found) => {
            if !found
            {
                eprintln!("Error directory not found!");
                return;
            }
            *WORKING_DIR.lock() += dir_name ;
            *WORKING_DIR.lock() += "/";

        }
    }
}