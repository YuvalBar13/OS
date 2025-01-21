use crate::println;
use crate::terminal::input::buffer::BUFFER;

pub fn run()
{
    println!("Please type something");
    let input = BUFFER.lock().get_input();
    println!("\nYou typed \"{}\"", input);
}