#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::{hint::black_box, panic::PanicInfo};

use alloc::{boxed::Box, vec::Vec};
use blog_os::{
    allocator::{self, HEAP_SIZE},
    memory::{self, BootInfoFrameAllocator},
};
use bootloader::{entry_point, BootInfo};
use x86_64::{instructions::hlt, VirtAddr};

extern crate alloc;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    blog_os::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap initialization failed");

    test_main();
    loop {
        hlt()
    }
}

/// Checks whether values can be stored on and read from the heap correctly
#[test_case]
fn simple_allocation() {
    // Store values on the heap
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);

    // Check the values
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}

/// Checks whether large amounts of allocations and allocations of
/// large pieces of memory are possible.
#[test_case]
fn large_vec() {
    // Set the target length of the vec
    let n = 1_000;

    // Create and fill the vec
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }

    // Calculate and check the sum of the values on the vec
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

/// Checks whether deallocated memory is reused.
#[test_case]
fn many_boxes() {
    // Allocate and deallocate as many boxes as the heap is large
    for i in 0..HEAP_SIZE {
        // Create a box, will fail after some iterations, if the memory isn't reused
        let x = Box::new(i);

        // Make sure the value is stored and can be read correctly
        assert_eq!(*x, i);
    }
}

/// Checks whether memory is reused when any piece of memory isn't freed
#[test_case]
fn many_boxes_long_lived() {
    // Create a box that will only be freed at the end of the function
    // Black_box it to prevent the compiler from removing the box
    let long_lived = black_box(Box::new(1));

    // Create as many extra boxes as there is heap memory
    for i in 0..HEAP_SIZE {
        // Black_box it to prevent the compiler form removing the box
        let x = black_box(Box::new(i));
        assert_eq!(*x, i);
    }

    // Check whether the long lived box is still available
    assert_eq!(*long_lived, 1);
}
