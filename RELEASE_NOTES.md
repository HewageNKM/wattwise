# Release Notes: WattWise v1.0.0 (The Open Rebirth)

We are thrilled to announce the official release of **WattWise v1.0.0**. This release marks the full transition from Zenith Energy to an open-source, community-focused power management suite.

### 🌟 What's New
-   **Official Rebranding**: Transitioned entire codebase to the "WattWise" identity.
-   **Refined Visual Language**: New circular badge logo with a clean, dark navy background and external transparency.
-   **Enhanced Telemetry**: Full battery metadata support (Manufacturer, Serial, Model, Technology).
-   **Standardized Metrics**: All energy and power measurements now use Wh, Ah, and W units.
-   **Structured Data Tables**: Replaced grid layouts with high-precision tables for technical vitals.
-   **Universal Deployment**: Optimized `.deb` packaging for seamless Ubuntu 22.04/24.04 support.

### 🔧 Improvements
-   **Binary Naming**: Unified command-line interface as `wattwise-ctl`.
-   **Icon Quality**: Native 32-bit RGBA circular icons for crisp desktop integration.
-   **Build Stability**: Fixed numerous crate import issues and resource path conflicts.

### 🚀 How to Upgrade
1.  Remove old `zenith-energy` package: `sudo dpkg -r zenith-energy`
2.  Install the new WattWise package: `sudo dpkg -i WattWise_1.0.0_amd64.deb`
3.  Restart the daemon: `sudo systemctl restart wattwise.service`
