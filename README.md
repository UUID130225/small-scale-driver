# Precision Scale Driver (Rust)

A robust and efficient RS232 driver for precision scales (e.g., Hucle, XK3190, etc.) written in Rust. It features automatic parameter detection, configuration persistence, and customizable scale factors.

## Features

- **Port Discovery:** Automatically scans and lists available COM/TTY ports.
- **Smart Auto-Detection:** Brute-forces Baud Rate, Parity, Data Bits, and Stop Bits to find the correct settings for your scale.
- **Fast Scanning:** Optimized timeout (300-500ms) for quick hardware discovery.
- **Config Persistence:** Saves successful settings to `config.json` for instant connection on subsequent runs.
- **Regex Extraction:** Automatically extracts numeric weight values from complex serial strings (e.g., `+00.17150TL S`).
- **Scale Factor (Multiplier):** Adjustable multiplier to match the scale's visual display when the serial output is scaled differently.

## Installation

1.  **Install Rust:** If you don't have Rust installed, get it at [rustup.rs](https://rustup.rs/).
2.  **Clone the repository:**
    ```bash
    git clone https://github.com/UUID130225/small-scale-driver.git
    cd small-scale-driver
    ```
3.  **Build the project:**
    ```bash
    cargo build --release
    ```

## Usage

### First Run (Setup)
Run the driver without arguments. It will ask you to select a port and then attempt to auto-detect the scale settings.
```bash
cargo run
```

### With Scale Factor
If your scale displays `1.7` but the driver shows `0.17`, use the multiplier:
```bash
cargo run -- --multiplier 10
```

### Manual Configuration
You can manually edit the `config.json` file created after the first successful run:
```json
{
  "port_name": "COM3",
  "baud_rate": 1200,
  "data_bits": "Eight",
  "parity": "None",
  "stop_bits": "Two",
  "multiplier": 10.0
}
```

### Force Re-detection
If you change the scale or connection, force a new scan:
```bash
cargo run -- --force-detect
```

## Troubleshooting

- **No data found:** Check your RS232 cable. Some scales require a Null Modem cable.
- **Garbage characters:** The auto-detection will try to find the best match, but ensure your scale is set to "Continuous" or "Print" mode in its internal settings.
- **Permission Denied (Linux):** You may need to add your user to the `dialout` group: `sudo usermod -a -G dialout $USER`.

## License

MIT
