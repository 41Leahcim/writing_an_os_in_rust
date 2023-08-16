use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{gdt, hlt_loop, println};

// The offsets at which to receive interrupts from the Programmable Interrupt Controllers.
// The usual range is 32 - 47 as 0 - 31 are used for exceptions.
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 1;

// Create a new interface for the PICs, unsafe as wrong offsets could cause undefined behavior.
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        // Use unsafe as the index shouldn't be used for another exception
        unsafe {
            // Set the double fault handler on its own piece of the stack
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        // Set an interrupt for the timer.
        // Removing this interrupt while the interrupts are enabled, will result in a double fault.
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);

        // Set an interrupt for the keyboard
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);

        // Set a page fault handler
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// This handler never returns, as a double fault can't be resolved on x86_64.
// It can only be stopped from causing a triple fault which would reset CPU
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode, // Provides more information about the type of memory access
) {
    // CR2 is set by the CPU on a page fault and contains the accessed virtual address that caused
    // the page fault.
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    // Use CR2::read to read the accessed virtual address
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {error_code:?}");
    println!("{stack_frame:#?}");

    // Halt execution as execution can't continue before the page fault is handled
    hlt_loop();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");

    // Notify the PIC that a interrupt has been handled, to receive the next interrupt.
    // Unsafe as sending the wrong interrupt vector number, could delete an important unsent
    // interrupt or cause the system to hang.
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    // Create a mutex reference to the keyboard
    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
            Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
        );
    }

    // Create a port with code 0x60 (6 * 16 = 3 * 32 = 96)
    let mut port = Port::new(0x60);

    // Read the scancode
    let scancode: u8 = unsafe { port.read() };

    // Lock the keyboard
    let mut keyboard = KEYBOARD.lock();

    // Add the received byte to the current key event
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        // Process the key
        if let Some(key) = keyboard.process_keyevent(key_event) {
            // Print the character if the keyevent is unicode, otherwise print the raw key code
            match key {
                DecodedKey::Unicode(character) => print!("{character}"),
                DecodedKey::RawKey(key) => print!("{key:?}"),
            }
        }
    }

    // Notify the PIC that a interrupt has been handled, to receive the next interrupt.
    // Unsafe as sending the wrong interrupt vector number, could delete an important unsent
    // interrupt or cause the system to hang.
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
