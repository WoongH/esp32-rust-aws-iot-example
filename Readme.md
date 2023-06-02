# Readme

This project has been established using the ESP-IDF template provided at the following link: https://github.com/esp-rs/esp-idf-template

## Prepare to build

Before you start building, you'll need to install 'espup'. Instructions for this can be found at: https://github.com/esp-rs/rust-build

## Build

`$ cargo build`

## Upload firmware and monitor

```sh
# default speed is 115200
$ espflash /dev/cu.usbserial-0001 target/xtensa-esp32-espidf/debug/template-test --monitor

# Explicitly specify speed to 921600
$ espflash /dev/cu.usbserial-0001 target/xtensa-esp32-espidf/debug/template-test --monitor --speed 921600
```
