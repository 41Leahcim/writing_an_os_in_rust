#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use blog_os::{hlt_loop, print, println};

/// This function is called on panic, only run whe not testing
///
/// # Arguments
/// ```info```: a struct containing the location where the panic was called, and the error message
///
/// # Returns
/// Never
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

/// This function is called on panic, only run whe not testing
///
/// # Arguments
/// ```info```: a struct containing the location where the panic was called, and the error message
///
/// # Returns
/// Never
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info);
}

/// The function where the application starts
///
/// # Returns
/// Never
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello, World{}", "!");

    blog_os::init();

    use x86_64::registers::control::Cr3;

    // Read Cr3, store the level 4 page table, but ignore the Cr3Flags
    let (level_4_page_table, _) = Cr3::read();
    println!(
        "Level 4 page table at: {:?}",
        level_4_page_table.start_address()
    );

    let ptr = 0x2031b2 as *mut u8;

    unsafe {
        println!("Read worked: {}", *ptr);
    }

    unsafe { *ptr = 42 };
    println!("Write worked");

    #[cfg(test)]
    test_main();

    println!("It did not crash!");

    hlt_loop();
}
