use alloc::string::String;
use alloc::vec::Vec;
use crate::{print};
use lazy_static::lazy_static;
use crate::terminal::output::framebuffer::WRITER;

#[derive(Default)]
pub struct InputBuffer {
    buffer: String,
    is_listening: bool,
    pub history: Vec<String>,
}

impl InputBuffer {
    pub const fn new() -> Self {
        InputBuffer {
            buffer: String::new(),
            is_listening: false,
            history: Vec::new(),
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
            WRITER.get().expect("Writer not initialized").lock().backspace();
            return true;
        }
        self.buffer.push(character);
        print!("{}", character);
        true

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
    
    pub fn get_input(&mut self) -> String {
        self.listen();

        let input = self.buffer.clone();
        self.buffer.clear();
        self.history.push(input.clone());
        input
    }
    pub fn arrow_up(&mut self)
    {
        if self.history.is_empty() {
            return;
        }
        if !self.buffer.is_empty() {
            for _ in 0..self.buffer.len() - 1 {
                WRITER.get().expect("Writer not initialized").lock().backspace();
            }
        }

        self.buffer = self.history.pop().unwrap();
        print!("{}", self.buffer);
    }
}

use spin::Mutex;

lazy_static!
{
    pub static ref BUFFER: Mutex<InputBuffer> = Mutex::new(InputBuffer::new());
}