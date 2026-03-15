# Zenith-Energy

A premium, high-performance Linux power management suite, built on the foundations of **auto-cpufreq** and engineered with **Rust** and **Tauri 2.0**.

## Overview
**Zenith-Energy** represents the next generation of Linux CPU optimization. While built on the solid bedrock of `auto-cpufreq`, it introduces a significant leap in performance and features, aiming for a professional "macOS-level" efficiency experience.

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
