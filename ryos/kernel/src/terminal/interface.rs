use crate::my_info;
use crate::terminal::input::buffer::BUFFER;

pub fn run()
{
    my_info!("Please type something");
    let input = BUFFER.lock().get_input();
    my_info!("You typed \"{}\"", input);
}