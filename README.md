# faderflow

> Fluid control over your desktop audio

A hardware audio mixer with motorized faders, full-color displays, and automatic application detection.

![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)

## Features

- 🎛️ 5 motorized faders with position sensing
- 🖥️ 5x 1.54" color TFT displays (240x240)
- 🔄 Bidirectional sync between hardware and Windows
- 🎯 Automatic app detection
- 📄 Multi-page navigation
- 💾 SD card icon storage

## Hardware

- Arduino Mega 2560
- 5x Behringer X32 Motorized Faders
- 3x DRV8833 Motor Driver Boards
- 6x ST7789 TFT Displays (1.54", 240x240)
- 6x EC11 Rotary Encoders
- SD Card Module
- USB-C PD Power (5V 3A)


## Software Stack

- **Desktop:** Rust + egui
- **Firmware:** Arduino C++
- **Platform:** Windows (Linux planned)

## Quick Start
```bash
# Clone repo
git clone https://github.com/Sven65/faderflow.git

# Build desktop app
cd desktop
cargo build --release

# Flash firmware
# Open firmware/faderflow/faderflow.ino in Arduino IDE
```

## Status

🚧 **Work in Progress** - Hardware picked out, software in development

## License

MIT License - see [LICENSE](LICENSE)

## Acknowledgments

Inspired by [deej](https://github.com/omriharel/deej)
