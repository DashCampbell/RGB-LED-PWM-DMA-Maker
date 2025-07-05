# Examples for STM32 Family

Run individual examples with
```
cargo run --bin <module-name>
```
for example
```
cargo run --bin ws2812b
```

## Checklist before running examples
Examples are configured for a STM32L432KC board. You might need to adjust `.cargo/config.toml`, `Cargo.toml` and possibly update pin numbers or peripherals to match the specific MCU or board you are using.

* [ ] Update .cargo/config.toml with the correct probe-rs command to use your specific MCU. For example for L4R5ZI-P it should be `probe-rs run --chip STM32L4R5ZITxP`. (use `probe-rs chip list` to find your chip)
* [ ] Update Cargo.toml to have the correct `embassy-stm32` feature. For example for L4R5ZI-P it should be `stm32l4r5zi`. Look in the `Cargo.toml` file of the `embassy-stm32` project to find the correct feature flag for your chip.
* [ ] If your board has a special clock or power configuration, make sure that it is set up appropriately.
* [ ] If your board has different pin mapping, update any pin numbers or peripherals in the given example code to match your schematic

