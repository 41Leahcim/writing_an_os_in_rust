#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks, abi_x86_interrupt)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
pub mod vga_buffer;
pub mod allocator;
pub mod gdt; // Global Descriptor table
pub mod interrupts;
pub mod memory;
pub mod serial;

extern crate alloc;

use core::panic::PanicInfo;

/// This function is called on panic, when testing
///
/// # Arguments
/// ```info```: a struct containing the location where the panic was called, and the error message
///
/// # Returns
/// Never
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info);
}

/// Represents a 32-bit exit code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Exits Qemu, with an exit code
///
/// # Arguments
/// ```exit_code```: the exit code to use
pub fn exit_qemu(exit_code: QemuExitCode) {
    // make the Port struct easily accessible
    use x86_64::instructions::port::Port;

    unsafe {
        // Open a port on address 0xf4, and write the exit code to it
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

/// A trait which adds test information
pub trait Testable {
    fn run(&self);
}

/// implement the testable trait for functions
impl<T: Fn()> Testable for T {
    /// Runs the function with test information
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

/// Runs the tests
///
/// # Arguments
/// An array slice containing functions
pub fn test_runner(tests: &[&dyn Testable]) {
    // print the number of tests to run
    serial_println!("Running {} tests", tests.len());

    // run every test
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    hlt_loop();
}

#[test_case]
#[allow(clippy::eq_op)]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

pub fn init() {
    interrupts::init_idt();
    gdt::init();

    // Initialize the PICs.
    // Unsafe as it can cause undefined behavior if the PIC is misconfigured
    unsafe { interrupts::PICS.lock().initialize() };

    // Enable interrupts on the CPU
    x86_64::instructions::interrupts::enable();
}

/// Blocks for ever, while still allowing interrupts.
/// Uses less energy than `loop{}`, with the same functionality.
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
use bootloader::{entry_point, BootInfo};

#[cfg(test)]
entry_point!(test_kernel_main);

#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    hlt_loop();
}
