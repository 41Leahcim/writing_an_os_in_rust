use core::alloc::GlobalAlloc;

use x86_64::align_up;

use super::Locked;

/// The BumpAllocator is one of the simplest allocators.
/// They have super high performance, but require all memory to be deallocated
/// before reusing deallocated memory.
pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Creates a new empty bump allocator
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the bump allocator with the given heap bounds
    ///
    /// # Safety
    /// This method is unsafe because the caller must ensure that the given
    /// memory range is unused. Also, this method must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // Get a mutable reference to the BumpAllocator
        let mut bump = self.lock();

        // Calculate the start of the allocation, align upwards to prevent overlap
        let alloc_start = align_up(bump.next as u64, layout.align() as u64);

        // Add the size of the allocated memory to the start of the allocation to calculate the end
        // Return a null pointer if that overflowed
        let alloc_end = match alloc_start.checked_add(layout.size() as u64) {
            Some(end) => end,
            None => return core::ptr::null_mut(),
        };

        // Make sure the end of the allocation is before or at the end of the heap
        if alloc_end > bump.heap_end as u64 {
            // Return null otherwise
            core::ptr::null_mut()
        } else {
            // Set the start of the next allocation to the end of this one
            bump.next = alloc_end as usize;

            // Increment the number of allocations
            bump.allocations += 1;

            // Return the start address of the current allocation as a *mut u8
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Take a mutable reference to the BumpAllocator
        let mut bump = self.lock();

        // Decrement the number of allocations, reset the allocator if no allocations are left
        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}
