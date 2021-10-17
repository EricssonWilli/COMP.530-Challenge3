#![no_std]
#![no_main]



use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{OriginDimensions, Point, Primitive, RgbColor, Size};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use longan_nano::hal::gpio::gpioa::{PA5, PA6, PA7};
use longan_nano::hal::gpio::gpiob::{PB0, PB1};
use longan_nano::hal::gpio::{Alternate, Floating, Input, Output, PushPull};
use longan_nano::hal::pac::{ECLIC, SPI0, TIMER0};
use longan_nano::hal::eclic::EclicExt;
use longan_nano::hal::spi::Spi;
use longan_nano::hal::timer::{Event, Timer};
use longan_nano::hal::prelude::*;
use panic_halt as _;

use embedded_graphics::mono_font::ascii::FONT_5X8;
use longan_nano::hal::{pac, rcu::RcuExt};
use longan_nano::{lcd, lcd_pins};
use riscv_rt::entry;


use longan_nano::hal::eclic;
use longan_nano::hal::pac::Interrupt;
use st7735_lcd::ST7735;

static mut timer0:Option<Timer<TIMER0>> = None;
static mut lcd:Option<ST7735<Spi<SPI0, (PA5<Alternate<PushPull>>, PA6<Input<Floating>>, PA7<Alternate<PushPull>>)>, PB0<Output<PushPull>>, PB1<Output<PushPull>>>> = None;
static mut counter:u32 = 0;

#[entry]
fn main() -> ! {
    // let('s) take the peripheral crate (pun intended)
    let dp = pac::Peripherals::take().unwrap();

    // Configure clocks
    let mut rcu = dp
        .RCU
        .configure()
        .ext_hf_clock(8.mhz())
        .sysclk(108.mhz())
        .freeze();

    // Configure timer
    unsafe {
        timer0 = core::prelude::v1::Some(Timer::timer0(dp.TIMER0, 1.hz(), &mut rcu));
        timer0.as_mut().unwrap().listen(Event::Update);
    }
    
    // reset and configure eclic

    ECLIC::reset();
    ECLIC::set_level_priority_bits(eclic::LevelPriorityBits::L2P2);
    ECLIC::set_threshold_level(eclic::Level::L0);
    ECLIC::setup(Interrupt::TIMER0_UP, eclic::TriggerType::Level, eclic::Level::L2, eclic::Priority::P2);


    // Constrain PAC I/O pins/interfaces for use in the LCD driver
    let mut afio = dp.AFIO.constrain(&mut rcu);
    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpiob = dp.GPIOB.split(&mut rcu);

    let lcd_pins = lcd_pins!(gpioa, gpiob);
    unsafe {
        // defining lcd global variable (yuck, global variables) 
        lcd = Some(lcd::configure(dp.SPI0, lcd_pins, &mut afio, &mut rcu));
    }

    // clearing the lcd
    unsafe {
        let (width, height) = (lcd.as_mut().unwrap().size().width as i32, lcd.as_mut().unwrap().size().height as i32);
        Rectangle::new(Point::new(0, 0), Size::new(width as u32, height as u32))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(lcd.as_mut().unwrap())
                .unwrap();
    }
    unsafe {
        ECLIC::unmask(Interrupt::TIMER0_UP);
        riscv::interrupt::enable();
    }
    loop {
        unsafe {
            riscv::asm::wfi();
        }
    }
}

#[no_mangle]
fn TIMER0_UP() {

    // disabling interrupts inside interrupt handler, because of logic
    unsafe {
        riscv::interrupt::disable();
        ECLIC::mask(Interrupt::TIMER0_UP);
        timer0.as_mut().unwrap().clear_update_interrupt_flag();
    }
    // style copied straight from the adc example :D
    let style = MonoTextStyleBuilder::new()
    .font(&FONT_5X8)
    .text_color(Rgb565::BLACK)
    .background_color(Rgb565::GREEN)
    .build();


    // Creating a text from the counter value
    let mut text = heapless::String::<32>::new();
    text.push_str("Counter value: ")
        .expect("failed to make string");
    unsafe {
        text.push_str(&heapless::String::<32>::from(counter))
        .expect("failed to make string");
    }
    // text -> lcd
    unsafe {
        let (width, height) = (lcd.as_mut().unwrap().size().width as i32, lcd.as_mut().unwrap().size().height as i32);
        // Clear screen
        Rectangle::new(Point::new(0, 0), Size::new(width as u32, height as u32))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(lcd.as_mut().unwrap())
            .unwrap();
        // and write text
        Text::new(text.as_str(), Point::new(40, 35), style)
            .draw(lcd.as_mut().unwrap())
            .unwrap();
        
    }
    // and almost forgot to increase the counter and allow interrupts again
    unsafe {
        counter+=1;
        ECLIC::unmask(Interrupt::TIMER0_UP);
        riscv::interrupt::enable();
    }
}
