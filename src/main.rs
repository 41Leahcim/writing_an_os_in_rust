#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use blog_os::{hlt_loop, memory::translate_address, print, println};
use bootloader::{entry_point, BootInfo};
use x86_64::{structures::paging::PageTable, VirtAddr};

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

fn table_printer(page_table: &PageTable, layer: usize, physical_memory_offset: VirtAddr) {
    if !(1..=4).contains(&layer) {
        return;
    }

    for (i, entry) in page_table
        .iter()
        .enumerate()
        .filter(|(_, entry)| !entry.is_unused())
    {
        // Print the entry
        println!("L{layer} Entry {i}: {entry:?}");

        if layer > 1 {
            // Get the physical address from the entry and convert it
            let physical_address = entry.frame().unwrap().start_address();
            let virtual_address = physical_address.as_u64() + physical_memory_offset.as_u64();
            let ptr = VirtAddr::new(virtual_address).as_mut_ptr();
            let next_layer_table: &PageTable = unsafe { &*ptr };
            table_printer(next_layer_table, layer - 1, physical_memory_offset);
        }
    }
}

entry_point!(kernel_main);

/// The function where the kernel starts
///
/// # Returns
/// Never
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use blog_os::memory::active_level_4_table;

    println!("Hello, World{}", "!");

    blog_os::init();

    // Get the physical memory offset and retrieve the l4 table
    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let l4_table = unsafe { active_level_4_table(physical_memory_offset) };

    // Iterate through the l4 table
    table_printer(l4_table, 1, physical_memory_offset);

    // Store some virtual addresses as u64s
    let addresses = [
        // The identity-mapped vga buffer page
        0xb8000,
        // Some code page
        0x201008,
        // Some stack page
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        boot_info.physical_memory_offset,
    ];

    for &address in &addresses {
        let virtual_address = VirtAddr::new(address);
        let physical_address =
            unsafe { translate_address(virtual_address, physical_memory_offset) };
        println!("{virtual_address:?} -> {physical_address:?}");
    }

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    hlt_loop();
}
