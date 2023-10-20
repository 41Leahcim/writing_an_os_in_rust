use core::{
    alloc::{GlobalAlloc, Layout},
    mem::{align_of, size_of},
    ptr::NonNull,
};

use super::Locked;

pub struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// The block sizes to use.
///
/// The sizes must each be power of 2 because they are also used as
/// the block alignment (alignments must always be powers of 2)
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// An allocator just like the list allocator, but with less efficient memory usage, but better
/// performance.
///
///  - Prefilling the lists might improve performance.
///  - Storing the alignment may improve memory usage
///  - Deallocations aren't freed, freeing them would improve memory usage
///  - Using a paging allocator instead of linked_list_allocator would decrease fragmentation
///  - A paging allocator would also improve performance predictability, improving worst-case performance
pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list_allocator::Heap,
}

impl FixedSizeBlockAllocator {
    /// Creates an empty FixedSizeBlockAllocator.
    pub const fn new() -> Self {
        const EMPTY: Option<&'static mut ListNode> = None;
        FixedSizeBlockAllocator {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback_allocator: linked_list_allocator::Heap::empty(),
        }
    }

    /// Initializes the allocator with the given heap bounds.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback_allocator
            .init(heap_start as *mut u8, heap_size);
    }

    /// Allocates using the fallback allocator
    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback_allocator.allocate_first_fit(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(()) => core::ptr::null_mut(),
        }
    }
}

/// Choose an appropriate block size for the given layout.
///
/// Returns an index into the `BLOCK_SIZES` array
fn list_index(layout: &Layout) -> Option<usize> {
    // The block size should be larger than or equal to the size or alignment of the layout.
    // Based on what's larger.
    let required_block_size = layout.size().max(layout.align());

    // Find the index of the first (smallest) block size larger than or equal to the required size
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(index) => match allocator.list_heads[index].take() {
                Some(node) => {
                    allocator.list_heads[index] = node.next.take();
                    node as *mut ListNode as *mut u8
                }
                None => {
                    // No block exists in list => allocate a new block
                    let block_size = BLOCK_SIZES[index];

                    // Only works if all block sizes are a power of 2
                    let block_align = block_size;
                    let layout = Layout::from_size_align(block_size, block_align).unwrap();
                    allocator.fallback_alloc(layout)
                }
            },
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Take a mutable reference to the allocator
        let mut allocator = self.lock();

        // Choose an appropriate block size, if available
        match list_index(&layout) {
            Some(index) => {
                // Create a new list node
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };

                // Verify that block has size and alignment required for storing the node
                assert!(size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(align_of::<ListNode>() <= BLOCK_SIZES[index]);

                // Prepend the node to the correct list
                let new_node_ptr = ptr as *mut ListNode;
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            }
            None => {
                // Convert the pointer to a NonNull pointer
                let ptr = NonNull::new(ptr).unwrap();

                // Deallocate the pointer
                allocator.fallback_allocator.deallocate(ptr, layout);
            }
        }
    }
}
