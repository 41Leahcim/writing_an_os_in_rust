#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use alloc::boxed::Box;
use blog_os::{
    hlt_loop,
    memory::{self, BootInfoFrameAllocator},
    print, println,
};
use bootloader::{entry_point, BootInfo};
use x86_64::VirtAddr;

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

entry_point!(kernel_main);

/// The function where the kernel starts
///
/// # Returns
/// Never
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello, World{}", "!");

    blog_os::init();

    // Get the physical memory offset and retrieve the l4 table
    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let mut _mapper = unsafe { memory::init(physical_memory_offset) };
    let mut _frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    let x = Box::new(41);

    #[cfg(test)]
    test_main();

    println!("{x}");

    println!("It did not crash!");
    hlt_loop();
}
