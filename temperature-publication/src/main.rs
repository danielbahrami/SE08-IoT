use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();

    println!("Hello World!");

    let peripherals = Peripherals::take().unwrap();
    let mut led = PinDriver::output(peripherals.pins.gpio4)?;

    loop {
        let _ = led.set_high();
        FreeRtos::delay_ms(1000);
        let _ = led.set_low();
        FreeRtos::delay_ms(1000);
    }
}
