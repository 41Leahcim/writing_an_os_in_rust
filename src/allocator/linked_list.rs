use core::{
    alloc::{GlobalAlloc, Layout},
    mem::{align_of, size_of},
};

use x86_64::align_up;

use super::Locked;

pub struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

/// More general purpose allocator than the bump allocator, but:
///  - Wastes memory by creating lots of smaller blocks without merging them
///  - By not merging smaller blocks, blocks also become smaller until large ones are impossible
///  - The linked_list_allocator crate merges the blocks for those reasons
///  - The list might have to be traversed to the end to find a suitable block, which is slow
pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// Creates an empty LinkedListAllocator
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// # Safety
    /// This function is unsafe, because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    /// Adds the given memory region to the front of the list
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // Ensure that the freed region is capable of holding ListNode
        assert_eq!(
            align_up(addr as u64, align_of::<ListNode>() as u64),
            addr as u64
        );
        assert!(size >= size_of::<ListNode>());

        // Create a new list node and append it at the start of the list
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr);
    }

    /// looks for a free region with the given size and alignment and removes it
    /// from the list.
    ///
    /// Returns a tuple of the list node and the start address of the allocation.
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        // Reference to current list node, updated for each iteration
        let mut current = &mut self.head;

        // Look for a large enough memory region in linked list
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(region, size, align) {
                // Region suitable for allocation -> remove node from list
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // Region not suitable -> continue with next region
                current = current.next.as_mut().unwrap();
            }
        }

        // No suitable region found
        None
    }

    /// Try to use the given region for an allocation with given size and
    /// alignment.
    ///
    /// Returns the allocation start address on success.
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr() as u64, align as u64) as usize;
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // Region too small
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < size_of::<ListNode>() {
            // Rest of region too small to hold a ListNode (required because the
            // allocation splits the region in a used and a free part)
            return Err(());
        }

        // Region suitable for allocation
        Ok(alloc_start)
    }

    /// Adjust the given layout so that the resulting allocated memory
    /// region is also capable of storing a `ListNode`
    ///
    /// Returns the adjusted size and alignment as a (size, align) tuple
    fn size_align(layout: Layout) -> (usize, usize) {
        // Adjust the alignment to also be aligned to the ListNode
        let layout = layout
            .align_to(align_of::<ListNode>())
            .expect("Adjustment alignment failed")
            .pad_to_align();

        // Adjust the size to be atleast large enough for a ListNode
        let size = layout.size().max(size_of::<ListNode>());

        // Return the size and alignment
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // Perform layout adjustments
        let (size, align) = LinkedListAllocator::size_align(layout);

        // Take a mutable reference to the LinkedListAllocator
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        // Perform layout adjustments
        let (size, _) = LinkedListAllocator::size_align(layout);

        // Add the region to the free regions
        self.lock().add_free_region(ptr as usize, size);
    }
}
