use lazy_static::lazy_static;
use x86_64::{
    instructions::tables::load_tss,
    registers::segmentation::{Segment, CS},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

// Use the 0th IST entry as double fault stack, an other index is also possible.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

// Lazy static as creation of the Task State Segment (TSS) can't be done during compile time.
lazy_static! {
    static ref TSS: TaskStateSegment = {
        // Create a new Task State Segment
        let mut tss = TaskStateSegment::new();

        // Assign a piece of the stack to the stack table
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            // Calculate the interrupt stack size
            const STACK_SIZE: usize = 4096 * 5;

            // Allocate the bytes on the stack
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            // Take a pointer to the allocated stack
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });

            // Return the stack end as the stack grows downwards (high to low address)
            stack_start + STACK_SIZE
        };
        tss
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        // Create the Global Descriptor Table
        let mut gdt = GlobalDescriptorTable::new();

        // Add a segment for the kernel code
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());

        // Add a segment for the TSS segment, pass it a reference to the TSS
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors{ code_selector, tss_selector })
    };
}

pub fn init() {
    GDT.0.load();

    // Use usafe as setting invalid selectors could break memory
    unsafe {
        // Reload the Code Segment register and load the Task State Segment
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
