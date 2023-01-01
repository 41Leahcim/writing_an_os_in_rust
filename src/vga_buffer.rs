use core::fmt;

use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;

/// Represents the color options for the vga buffer
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color{
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15
}

/// Represents the full color byte of a character, foreground (4-bit), background (3-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColorCode(u8);

impl ColorCode{
    /// Creates a color code
    /// 
    /// # Arguments
    /// ```foreground```: the foreground color
    /// ```background```: the background color + blink flag (most significant bit)
    /// 
    /// # Returns
    /// A color code
    fn new(foreground: Color, background: Color) -> ColorCode{
        Self((background as u8) << 4 | foreground as u8)
    }
}

/// Represents a full VGA character
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar{
    ascii_character: u8,
    color_code: ColorCode
}

/// The dimensions of the VGA buffer
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH : usize = 80;

/// The VGA buffer
#[repr(transparent)]
struct Buffer{
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT]
}

/// Writes text to the VGA buffer
pub struct Writer{
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer
}

impl fmt::Write for Writer{
    /// Writes formatted string to the screen
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

impl Writer{
    /// Writes a single character to the screen
    /// 
    /// # Arguments
    /// ```byte```: The byte to write to the screen
    pub fn write_byte(&mut self, byte: u8){
        match byte{
            // move to a new line, if a new line character is printed
            b'\n' => self.new_line(),

            // else, print the character to the screen
            byte => {
                // if we're at the end of the current line, first go to a new line
                if self.column_position >= BUFFER_WIDTH{
                    self.new_line();
                }
                
                // set the current row to the last row, and the current column to the column position
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                // get the color code for this writer
                let color_code = self.color_code;

                // create the character, and write it to the screen
                self.buffer.chars[row][col].write(ScreenChar{
                    ascii_character: byte,
                    color_code
                });

                // move to the next column position
                self.column_position += 1;
            }
        }
    }

    /// Moves the cursor to the next line
    fn new_line(&mut self){
        // shift every character 1 line up, replacing the first row
        for row in 1..BUFFER_HEIGHT{
            for col in 0..BUFFER_WIDTH{
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }

        // clear the last row, and reset the column position
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// Clears a row on the screen
    /// 
    /// # Arguments
    /// ```row```: the row index to clear
    fn clear_row(&mut self, row: usize){
        // create the blank character, to fill the row with
        let blank = ScreenChar{
            ascii_character: b' ',
            color_code: self.color_code
        };

        // fill the row with the blank character
        for col in 0..BUFFER_WIDTH{
            self.buffer.chars[row][col].write(blank);
        }
    }

    /// Writes a string to the screen
    /// 
    /// # Arguments
    /// ```s```: the string to write to the screen
    pub fn write_string(&mut self, s: &str){
        // iterate through the bytes in the string
        for byte in s.bytes(){
            match byte{
                // printable character
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe)
            }
        }
    }
}

// create a writer accessible from any module using this module
lazy_static!{
    pub static ref WRITER:Mutex<Writer> = Mutex::new(Writer{
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe{ &mut *(0xb8000 as *mut Buffer) }
    });
}

// prints formatted text to the screen
#[macro_export]
macro_rules! print {
    ($($arg: tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

// prints formatted text to the screen, ending with a new line
#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => (print!("{}\n", format_args!($($arg)*)));
}

// print formatted text to the screen
#[doc(hidden)]
pub fn _print(args: fmt::Arguments){
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
