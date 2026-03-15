# auto-cpufreq-rust

A high-performance Linux CPU optimizer and power management tool, now built with **Rust** and **Tauri 2.0**.

## Overview
`auto-cpufreq-rust` is a complete overhaul of the original project, designed to provide macOS-level power management efficiency. It dynamically adjusts CPU frequency, governors, and turbo boost settings based on real-time metrics, load, and battery state.

## Key Features
- **Rust Engine**: Minimal resource footprint with direct, safe kernel interactions.
- **Tauri GUI**: A premium, modern interface for real-time monitoring and control.
- **Modular Battery Management**: High-precision charge threshold control for various hardware vendors (Asus, Lenovo, etc.).
- **Automatic Optimization**: Intelligent heuristics to balance performance and battery life.

## Getting Started

### Prerequisites
- Rust (Cargo)
- Node.js (npm)
- System libraries: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `libsoup-3.0-dev`

### Installation & Run
1. Clone the repository.
2. Install dependencies:
   ```bash
   npm install
   ```
3. Run in development mode:
   ```bash
   npm run tauri dev
   ```

### Building
To create a production build:
```bash
npm run tauri build
```

## Architecture
- **Backend**: Rust using `sysinfo` for monitoring and direct sysfs writes for control.
- **Frontend**: React-based Tauri interface with a premium dark-mode design.
- **Hardware Abstraction**: Utilizes a robust `cpufreqctl` script for cross-driver compatibility.

## License
MIT License
