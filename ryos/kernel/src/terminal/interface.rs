use heapless::String;
use crate::{print, println};
use crate::terminal::input::buffer::BUFFER;

pub fn run()
{
    print!(">>> ");
    let input = BUFFER.lock().get_input();
    println!();
    handle_input(input);
}

fn handle_input(input: String<{crate::terminal::input::buffer::BUFFER_SIZE}>)
{
    if input == "clear"
    {
        crate::terminal::output::framebuffer::WRITER.get().unwrap().lock().clear_screen();
    }
    else if let Some(data) = input.strip_prefix("echo ") {
        if data.starts_with("") && data.ends_with("") && data.len() > 1
        {
            let result = &data[1..data.len() - 1];
            println!("{}", result);
            return;
        }
        println!("{}", data);
    }
    else if input == "exit" || input == "shut down"
    {
        unsafe {
            use x86_64::instructions::port::Port;
            let mut port = Port::new(0x604);
            port.write(0x2000u16);
        }
    }
    else if input == "logo"
    {
        handle_input("clear".into());
        crate::print_logo();
    }
}