#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;
use esp_hal::{clock::ClockControl, gpio::IO, peripherals::Peripherals, prelude::*, delay::Delay};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut led = io.pins.gpio4.into_push_pull_output();

    led.set_high();

    let mut delay = Delay::new(&clocks);
    println!("Hello world!");
    loop {
        delay.delay_millis(1000);
        led.toggle();
    }
}
