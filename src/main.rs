#![no_std]  // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

mod vga_buffer;

use core::panic::PanicInfo;

// this function is called on panic
#[panic_handler]
fn panic(info: &PanicInfo) -> !{
    println!("{}", info);
    loop{}
}

static HELLO: &[u8] = b"Hello World!";

// this function is the entry point, since the linker looks for a function
// named `_start` by default
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> !{
    // print a message to the screen
    println!("Hello World{}", "!");

    // print a panic message
    panic!("Some panic message");
}
