use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

use self::fixed_size_block::FixedSizeBlockAllocator;

pub mod bump;
pub mod fixed_size_block;
pub mod linked_list;

/// A wrapper around spin::Mutex to permit trait implementations
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

#[global_allocator]
pub static mut ALLOCATOR: Locked<FixedSizeBlockAllocator> =
    Locked::new(FixedSizeBlockAllocator::new());

// The start address and size of the heap, can be changed if needed
pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024;

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        // Take the virtual address of the physical heap start address
        let heap_start = VirtAddr::new(HEAP_START as u64);

        // Add the heap size to the heap start and subtract 1 to get the end of the heap
        let heap_end = heap_start + HEAP_SIZE - 1u64;

        // Get the pages of the heap start and heap end
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);

        // Create a heap range from the first up to and including the last page of the heap
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // Iterate through the pages
    for page in page_range {
        // Allocate memory for each frame, return a Frame Allocation Failed error on failure
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;

        // Use the Present and Writable page table flags
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        // Create a new mapping in the page table for the current page.
        // Return the error on failure, flush on success
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    // Initialize the allocator
    unsafe { ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE) };

    Ok(())
}
