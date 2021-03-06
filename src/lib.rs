// The purpose of this file is to make certain functionality available to both integration tests and source code
#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)] // enable custom test frameworks
#![feature(abi_x86_interrupt)] // enable "x86-interrupt" calling convention
#![feature(alloc_error_handler)] // enable alloc errors to be handled
#![test_runner(crate::test_runner)] // use test_runner for tests
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
extern crate rlibc;

// make modules available to crate
pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod serial;
pub mod vga_buffer;

#[cfg(test)]
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;

#[cfg(test)]
entry_point!(test_kernel_main);

pub fn init() {
  gdt::init();
  interrupts::init_idt();
  unsafe { interrupts::PICS.lock().initialize() }; // initialize the Interrupt Controller
  x86_64::instructions::interrupts::enable(); // enable interrupts for the CPU
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
  panic!("allocation error: {:?}", layout)
}

/**
 * hlt_loop uses the hlt instruction to preserve CPU resources
 */
pub fn hlt_loop() -> ! {
  loop {
    x86_64::instructions::hlt();
  }
}

pub trait Testable {
  fn run(&self) -> ();
}

// Testable trait adds a run function to all functions with Fn() trait
impl<T> Testable for T
where
  T: Fn(),
{
  fn run(&self) {
    serial_print!("{}...\t", core::any::type_name::<T>());
    self();
    serial_println!("[ok]");
  }
}

/**
 * test_runner runs all functions with the Testable trait
 */
pub fn test_runner(tests: &[&dyn Testable]) {
  serial_println!("Running {} tests", tests.len());
  for test in tests {
    test.run();
  }
  exit_qemu(QemuExitCode::Success);
}

/**
 * test_panic_handler gracefully handles panics and exits QEMU
 */
pub fn test_panic_handler(info: &PanicInfo) -> ! {
  serial_println!("[failed]\n");
  serial_println!("Error: {}\n", info);
  exit_qemu(QemuExitCode::Failed);
  hlt_loop();
}

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
  init();
  test_main();
  hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
  test_panic_handler(info)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
  Success = 0x10,
  Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
  use x86_64::instructions::port::Port;

  unsafe {
    let mut port = Port::new(0xf4);
    port.write(exit_code as u32);
  }
}
