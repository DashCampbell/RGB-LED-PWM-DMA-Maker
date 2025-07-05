#![no_std]
#![no_main]
/// Example using a generic LedDmaBuffer configured for a ws2812b LED strip
/// ws2812b datasheet: https://cdn-shop.adafruit.com/datasheets/WS2812B.pdf
///
/// Information needed from datasheet:
/// Data Transfer Time = 1.25us
///     * Used to calculate pwm frequency, t1h, and t0h values
/// T1H = 0.8us
/// T0H = 0.4us
/// Reset Period = RES >= 50us
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::{Config, Peripheral};
use embassy_time::Timer;
use rgb_led_pwm_dma_maker::*;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Configure the system clock to be 40 MHz
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.sys = Sysclk::PLL1_R;
        config.rcc.hsi = true;
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI, // 16MHz
            prediv: PllPreDiv::DIV2,
            mul: PllMul::MUL10,
            divp: None,
            divq: None,
            divr: Some(PllRDiv::DIV2),
        });
    }
    let p = embassy_stm32::init(config);

    // Configure pin PA8 to PWM
    let pwm_pin = PwmPin::new_ch1(p.PA8, OutputType::PushPull);

    // PWM_FREQ = 1 / data_transfer_time = 1 / 1.25us = 800kHz
    const PWM_FREQ: Hertz = Hertz::khz(800);

    // Obtain a PWM handler, configure the Timer and Frequency
    // The prescaler and ARR are automatically set
    // Given this system frequency and pwm frequency the max duty cycle will be 50
    let mut pwm = SimplePwm::new(
        p.TIM1,
        Some(pwm_pin),
        None,
        None,
        None,
        PWM_FREQ,
        CountingMode::EdgeAlignedUp,
    );
    info!("Max Duty Cycle: {}", pwm.ch1().max_duty_cycle());

    // Enable channel 1
    pwm.ch1().enable();

    const RED: RGB = RGB::new(255, 0, 0);
    const GREEN: RGB = RGB::new(0, 255, 0);
    const BLUE: RGB = RGB::new(0, 0, 255);
    const MAGENTA: RGB = RGB::new(255, 0, 255);
    const CYAN: RGB = RGB::new(0, 255, 255);
    const YELLOW: RGB = RGB::new(255, 255, 0);
    const ORANGE: RGB = RGB::new(255, 20, 0);

    const RAINBOW: [RGB; 7] = [MAGENTA, BLUE, CYAN, GREEN, YELLOW, ORANGE, RED];
    const LED_COUNT: usize = RAINBOW.len();

    // RESET_LENGTH = reset_period / data_transfer_time = 50us / 1.25us = 40
    const RESET_LENGTH: usize = 40;
    // Calculate the dma buffer's length at compile time
    const DMA_BUFFER_LEN: usize = calc_dma_buffer_length(RGB::BIT_COUNT, LED_COUNT, RESET_LENGTH);
    // t1h = T1H / data_transfer_time * max_duty_cycle = 0.8us / 1.25us * 50 = 32
    let t1h: u16 = 32;
    // t1h = T0H / data_transfer_time * max_duty_cycle = 0.4us / 1.25us * 50 = 16
    let t0h: u16 = 16;

    // From datasheet, composition of 24 bit data is (green, red, blue), so use LedDataComposition::GRB
    let mut dma_buffer = LedDmaBuffer::<DMA_BUFFER_LEN>::new(t1h, t0h, LedDataComposition::GRB);

    let mut dma1_ch2 = p.DMA1_CH2.into_ref();
    let mut i = 0i32;
    loop {
        // Set the DMA buffer
        dma_buffer.set_dma_buffer(&RAINBOW, Some(i)).unwrap();
        // Create a pwm waveform using the dma buffer
        pwm.waveform_ch1(&mut dma1_ch2, dma_buffer.get_dma_buffer())
            .await;
        i += 1;
        Timer::after_millis(100).await;
    }
}
