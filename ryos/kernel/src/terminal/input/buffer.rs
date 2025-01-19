use heapless::String;
use crate::{my_error, my_info};
use lazy_static::lazy_static;

const BUFFER_SIZE: usize = 100;

#[derive(Default)]
pub struct InputBuffer {
    buffer: String<BUFFER_SIZE>,
    is_listening: bool,
}

impl InputBuffer {
    pub const fn new() -> Self {
        InputBuffer {
            buffer: String::new(),
            is_listening: false,
        }
    }

    pub fn add_char(&mut self, character: char) -> bool {
        if !self.is_listening {
            return false;
        }

        if character == '\n' {
            self.end_listening();
            return true;
        }

        // If pressed delete
        if Some(character) == char::from_u32(127) || character == '\x08' {
            if self.buffer.is_empty() {
                return false;
            }

            self.buffer.pop();
            return true;
        }

        if self.buffer.len() < self.buffer.capacity() {
            self.buffer.push(character).ok();
            my_info!("{}", character);
            return true;
        } else {
            my_error!("Buffer is full");
        }

        return false;
    }

    fn end_listening(&mut self)
    {
        self.is_listening = false;
    }

    fn listen(&mut self)
    {
        self.buffer.clear();
        self.is_listening = true;


        unsafe { BUFFER.force_unlock() };
        while self.is_listening {
            x86_64::instructions::hlt();
        }
    }
    
    pub fn get_input(&mut self) -> String<BUFFER_SIZE> {
        self.listen();

        let input = self.buffer.clone();
        self.buffer.clear();
        input
    }
}

use spin::Mutex;

lazy_static!
{
    pub static ref BUFFER: Mutex<InputBuffer> = Mutex::new(InputBuffer::new());
}