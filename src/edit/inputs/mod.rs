use crate::edit::inputs::key::*;
use unicode_width::UnicodeWidthStr;

pub mod events;
pub mod key;

pub enum InputEvent {
    Input(Key),
    Tick,
}

pub struct InputBuffer {
    buffer: String,
    index: usize,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            index: 0,
        }
    }
    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    pub fn get_buffer_drain(&mut self) -> String {
        let buffer = self.buffer.clone();
        self.buffer.clear();
        return buffer;
    }

    pub fn push_char(&mut self, c: char) {
        self.buffer.push(c);
        self.index += 1;
    }

    pub fn pop(&mut self) {
        if self.buffer.pop().is_some() {
            self.index -= 1;
        };
    }
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
