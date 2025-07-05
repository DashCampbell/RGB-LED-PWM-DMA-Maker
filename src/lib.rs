#![no_std]
//! # Led Dma Buffers
//! Used for creating a PWM waveform using the DMA to control RGB LEDs such as WS2812, WS2812B, SK6812, SK6812-RGBW.
//! Based on Phil's Lab's RGB Led [video](https://www.youtube.com/watch?v=MqbJTj0Cw6o)
//! # Example
//! ```
//! const led_array: [RGB; 3] = [RGB::new(255,0,0), RGB::new(0,0,255), RGB::new(0,0,255)];
//! // Calculate the dma buffer's length at compile time
//! const DMA_BUFFER_LEN: usize = calc_dma_buffer_length(RGB::BIT_COUNT, led_array.len(), RESET_LENGTH);
//! // Initialize a new DMA buffer
//! let mut dma_buffer = LedDmaBuffer::<DMA_BUFFER_LEN>::new(t1h, t0h, LedDataComposition::GRB);
//! // Set the DMA buffer
//! dma_buffer.set_dma_buffer(&led_array, None).unwrap();
//! // Create a pwm waveform using the dma buffer
//! let mut dma1_ch2 = p.DMA1_CH2.into_ref();
//! pwm.waveform_ch1(&mut dma1_ch2, dma_buffer.get_dma_buffer()).await;
//! ```
//! Full examples can be found in the [examples]() folder.

use core::fmt::Debug;

#[cfg(feature = "defmt")]
use defmt::error;

/// Implemented by [RGB] & [RGBW]
pub trait RgbLedColor: Copy + Clone {
    /// The number of bits representing the color
    const BIT_COUNT: usize;
    fn set_color<const DMA_BUFFER_LEN: usize>(
        &self,
        led_dma_buffer: &mut LedDmaBuffer<DMA_BUFFER_LEN>,
        led_index: usize,
    );
}

/// The order of colors of the data sent to the LED
/// ## Example
/// If the data structure is R\[7:0] | G\[7:0] | B\[7:0] | W\[7:0] as seen
/// [here](https://cdn-shop.adafruit.com/product-files/2757/p2757_SK6812RGBW_REV01.pdf#page=6) in the sk6812rgbw datasheet
/// then use [LedDataComposition::RGB]
pub enum LedDataComposition {
    RGB,
    GRB,
}

/// Error Types
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub enum LedDmaError {
    BrightnessOver100,
    LedArrayLongerThanDmaBuffer,
}

/// Represents a RGB LED
#[derive(Clone, Copy)]
pub struct RGB {
    r: u8,
    g: u8,
    b: u8,
}
impl RGB {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}
impl RgbLedColor for RGB {
    const BIT_COUNT: usize = 8 * 3;

    fn set_color<const DMA_BUFFER_LEN: usize>(
        &self,
        led_dma_buffer: &mut LedDmaBuffer<DMA_BUFFER_LEN>,
        led_index: usize,
    ) {
        match led_dma_buffer.data_composition {
            LedDataComposition::GRB => {
                led_dma_buffer.set_byte(self.g, led_index);
                led_dma_buffer.set_byte(self.r, led_index + 8);
            }
            LedDataComposition::RGB => {
                led_dma_buffer.set_byte(self.r, led_index);
                led_dma_buffer.set_byte(self.g, led_index + 8);
            }
        }
        led_dma_buffer.set_byte(self.b, led_index + 16);
    }
}
/// Represents a RGBW LED
#[derive(Clone, Copy)]
pub struct RGBW {
    r: u8,
    g: u8,
    b: u8,
    w: u8,
}
impl RGBW {
    pub const fn new(r: u8, g: u8, b: u8, w: u8) -> Self {
        Self { r, g, b, w }
    }
}
impl RgbLedColor for RGBW {
    const BIT_COUNT: usize = 8 * 4;

    fn set_color<const DMA_BUFFER_LEN: usize>(
        &self,
        led_dma_buffer: &mut LedDmaBuffer<DMA_BUFFER_LEN>,
        led_index: usize,
    ) {
        match led_dma_buffer.data_composition {
            LedDataComposition::GRB => {
                led_dma_buffer.set_byte(self.g, led_index);
                led_dma_buffer.set_byte(self.r, led_index + 8);
            }
            LedDataComposition::RGB => {
                led_dma_buffer.set_byte(self.r, led_index);
                led_dma_buffer.set_byte(self.g, led_index + 8);
            }
        }
        led_dma_buffer.set_byte(self.b, led_index + 16);
        led_dma_buffer.set_byte(self.w, led_index + 24);
    }
}

/// Calculates the required DMA buffer length for a [LedDmaBuffer] at compile time.
/// * `bits_per_led` - Number of bits per LED. For RGB Leds use `RGB::[BIT_COUNT]`. For RGBW Leds use `RGBW::BIT_COUNT`.
/// * `led_length` - Number of LEDs
/// * `reset_length` - Represents the low voltage time for the reset code. `reset_length` = `reset_period` / `data_transfer_time`
pub const fn calc_dma_buffer_length(
    bits_per_led: usize,
    led_length: usize,
    reset_length: usize,
) -> usize {
    (bits_per_led * led_length) + reset_length
}

/// A generic DMA Buffer
pub struct LedDmaBuffer<const DMA_BUFFER_LEN: usize> {
    dma_buffer: [u16; DMA_BUFFER_LEN],
    t1h: u16,
    t0h: u16,
    data_composition: LedDataComposition,
    brightness: u8,
}

impl<const DMA_BUFFER_LEN: usize> LedDmaBuffer<DMA_BUFFER_LEN> {
    /// Creates a new DMA Buffer
    /// * `t1h` - 1 code, high voltage time value. `t1h` = `1_code_high_voltage_time / data_transfer_time * max_duty_value`
    /// * `t0h` - 0 code, high voltage time value. `t0h` = `0_code_high_voltage_time / data_transfer_time * max_duty_value`
    /// * `data_composition` - The data composition/structure of the led data, found in the LED datasheet.
    pub fn new(t1h: u16, t0h: u16, data_composition: LedDataComposition) -> Self {
        Self {
            dma_buffer: [0u16; DMA_BUFFER_LEN],
            t1h,
            t0h,
            data_composition,
            brightness: 100,
        }
    }
    /// Set the DMA buffer
    /// * `led_array` - Array of LEDs
    /// * `rotate` - Rotate LED array
    ///     * If `rotate` > 0, rotate right.
    ///     * If `rotate` < 0, rotate left.
    pub fn set_dma_buffer<T: RgbLedColor>(
        &mut self,
        led_array: &[T],
        rotate: Option<i32>,
    ) -> Result<(), LedDmaError> {
        if led_array.len() * T::BIT_COUNT > self.dma_buffer.len() {
            #[cfg(feature = "defmt")]
            error!(
                "Led length {} with {} bits per led cannot fit into the DMA buffer of size {}",
                led_array.len(),
                T::BIT_COUNT,
                self.dma_buffer.len()
            );
            return Err(LedDmaError::LedArrayLongerThanDmaBuffer);
        }
        for (mut led_index, led) in led_array.iter().enumerate() {
            if let Some(rotate) = rotate {
                led_index = (led_index as i32 + rotate) as usize % led_array.len();
            }
            led_index *= T::BIT_COUNT;
            led.set_color(self, led_index);
        }
        Ok(())
    }
    /// Set the DMA buffer
    /// * `led_array` - Array of LEDs
    /// * `rotate` - Rotate LED array
    ///     * If `rotate` > 0, rotate right.
    ///     * If `rotate` < 0, rotate left.
    /// * `brightness` - Brightness level, `0%` - `100%`
    pub fn set_dma_buffer_with_brightness<T: RgbLedColor>(
        &mut self,
        led_array: &[T],
        rotate: Option<i32>,
        brightness: u8,
    ) -> Result<(), LedDmaError> {
        if brightness > 100 {
            #[cfg(feature = "defmt")]
            error!("Brightness is greater than 100%, it is {}%.", brightness);
            return Err(LedDmaError::BrightnessOver100);
        }
        self.brightness = brightness;
        self.set_dma_buffer(led_array, rotate)?;
        // Reset brightness
        self.brightness = 100;
        Ok(())
    }
    pub fn get_dma_buffer(&self) -> &[u16] {
        &self.dma_buffer
    }
    /// Set a byte in the DMA buffer
    fn set_byte(&mut self, byte: u8, byte_index: usize) {
        // Adjust byte (r,g,b,w) to correct brightness level
        let adjusted_byte = (f32::from(byte) * f32::from(self.brightness) / 100f32) as u8;

        for i in 0..8 {
            self.dma_buffer[i + byte_index] = if (adjusted_byte & (1 << (7 - i))) > 0 {
                self.t1h
            } else {
                self.t0h
            };
        }
    }
}
