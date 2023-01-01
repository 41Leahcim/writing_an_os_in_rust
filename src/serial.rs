use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static!{
    pub static ref SERIAL1: Mutex<SerialPort> = {
        // create, and initialize a new default port, return it inside a mutex
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

/// Sends formatted text over the uart
/// 
/// # Arguments
/// ```args```: the arguments to parse and send
/// 
/// # Panics
/// If the serial interface can't be written to
#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments){
    // enable Write functionality
    use core::fmt::Write;

    // wait for access to the serial port, write the message over the serial interface
    // exit with a message if it fails
    SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
}

/// Prints to the host through the serial interface
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

// Prints to the host through the serial interface, appending a new line
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}
