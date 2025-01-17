use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

pub mod pci;
pub mod time;
pub mod qemu;

pub enum VideoMode {
    Text,
    Graphic,
}

impl VideoMode {
    pub fn is_text(&self) -> bool {
        match self {
            VideoMode::Text => true,
            VideoMode::Graphic => false,
        }
    }

    pub fn set_graphic(&mut self) {
        *self = VideoMode::Graphic;
    }
}

lazy_static! {
    pub static ref VIDEO_MODE : Mutex<VideoMode> = Mutex::new(VideoMode::Text);
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::io::qemu::_qemu_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debugln {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::debug!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    if VIDEO_MODE.lock().is_text() {
        crate::vga_buffer::_print(args);
    } else {
        crate::graphic::_print(args);
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::io::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
