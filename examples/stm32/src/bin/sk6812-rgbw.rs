#![no_std]
#![no_main]
/// Example using a generic LedDmaBuffer configured for a sk6812-rgbw LED strip
/// sk6812rgbw datasheet: https://cdn-shop.adafruit.com/product-files/2757/p2757_SK6812RGBW_REV01.pdf
///
/// Information needed from datasheet:
/// Data Transfer Time = 1.25us
///     * Used to calculate pwm frequency, t1h, and t0h values
/// T1H = 0.6us
/// T0H = 0.3us
/// Reset Period = RES >= 80us
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

    const RED: RGBW = RGBW::new(255, 0, 0, 0);
    const GREEN: RGBW = RGBW::new(0, 255, 0, 0);
    const BLUE: RGBW = RGBW::new(0, 0, 255, 0);
    const MAGENTA: RGBW = RGBW::new(255, 0, 255, 0);
    const CYAN: RGBW = RGBW::new(0, 255, 255, 0);
    const YELLOW: RGBW = RGBW::new(255, 255, 0, 0);
    const ORANGE: RGBW = RGBW::new(255, 20, 0, 0);

    const RAINBOW: [RGBW; 7] = [MAGENTA, BLUE, CYAN, GREEN, YELLOW, ORANGE, RED];
    const LED_COUNT: usize = RAINBOW.len();

    // RESET_LENGTH = reset_period / data_transfer_time = 80us / 1.25us = 64
    const RESET_LENGTH: usize = 64;
    // Calculate the dma buffer's length at compile time
    const DMA_BUFFER_LEN: usize = calc_dma_buffer_length(RGBW::BIT_COUNT, LED_COUNT, RESET_LENGTH);
    // t1h = T1H / data_transfer_time * max_duty_cycle = 0.6us / 1.25us * 50 = 24
    let t1h: u16 = 24;
    // t1h = T0H / data_transfer_time * max_duty_cycle = 0.3us / 1.25us * 50 = 12
    let t0h: u16 = 12;

    // Create a DMA buffer for the led strip
    // From datasheet, data structure of 32 bit data is red -> green -> blue, so use LedDataComposition::RGB
    // If the colors are not correct, try using LedDataComposition::GRB instead.
    let mut dma_buffer = LedDmaBuffer::<DMA_BUFFER_LEN>::new(t1h, t0h, LedDataComposition::RGB);

    let mut dma1_ch2 = p.DMA1_CH2.into_ref();
    let mut brightness = 0u8;
    let mut increase = true;
    loop {
        // Set the DMA buffer
        dma_buffer
            .set_dma_buffer_with_brightness(&RAINBOW, None, brightness)
            .unwrap();

        // Create a pwm waveform usng the dma buffer
        pwm.waveform_ch1(&mut dma1_ch2, dma_buffer.get_dma_buffer())
            .await;

        // Increase or Decrease brightness accordingly
        if brightness >= 100 {
            increase = false;
        } else if brightness == 0 {
            increase = true;
        }
        if increase {
            brightness += 1;
        } else {
            brightness = brightness.saturating_sub(1);
        }
        if brightness > 100 {
            brightness = 100;
        }
        Timer::after_millis(10).await;
    }
}
