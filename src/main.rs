use clap::Parser;
use serialport::{SerialPort, DataBits, Parity, StopBits};
use std::io::{self, BufRead, BufReader, Write, Read};
use std::time::Duration;
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

/// Name of the local configuration file
const CONFIG_FILE: &str = "config.json";

/// Command-line arguments for the scale driver
#[derive(Parser, Debug)]
#[command(author, version, about = "Precision Scale Driver with Scale Factor", long_about = None)]
struct Args {
    /// Serial port to connect to (e.g., COM3 or /dev/ttyUSB0).
    /// If omitted, the program will scan available ports.
    #[arg(short, long)]
    port: Option<String>,

    /// Optional command to send to the scale to trigger a weight reading (e.g., "W")
    #[arg(short, long)]
    command: Option<String>,

    /// Force the auto-detection process even if a config file exists
    #[arg(short, long, default_value_t = false)]
    force_detect: bool,

    /// Scale factor to multiply the weight by (e.g., 10.0 if the scale sends 0.17 but represents 1.7)
    #[arg(short, long, default_value_t = 1.0)]
    multiplier: f64,
}

/// Serial communication settings to be stored in the config file
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SerialSettings {
    port_name: String,
    baud_rate: u32,
    data_bits: DataBits,
    parity: Parity,
    stop_bits: StopBits,
    /// Default multiplier value if not present in the JSON
    #[serde(default = "default_multiplier")]
    multiplier: f64,
}

/// Helper function to provide a default value for Serde deserialization
fn default_multiplier() -> f64 { 1.0 }

impl Default for SerialSettings {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            multiplier: 1.0,
        }
    }
}

/// Core driver structure for managing serial port interaction
struct ScaleDriver {
    /// The opened serial port handle
    port: Box<dyn SerialPort>,
    /// Pre-compiled regex for weight extraction
    re: Regex,
    /// Optional command string to send before each read
    command: Option<String>,
}

impl ScaleDriver {
    /// Opens a serial port with specific settings and a custom timeout
    fn open(settings: &SerialSettings, command: Option<String>, timeout_ms: u64) -> Result<Self, Box<dyn std::error::Error>> {
        let port = serialport::new(&settings.port_name, settings.baud_rate)
            .timeout(Duration::from_millis(timeout_ms))
            .data_bits(settings.data_bits)
            .parity(settings.parity)
            .stop_bits(settings.stop_bits)
            .open()?;

        // Regex to find floating point numbers (supports leading +/- and optional decimal)
        let re = Regex::new(r"[-+]?[0-9]*\.?[0-9]+")?;

        Ok(ScaleDriver { port, re, command })
    }

    /// Performs a raw read from the serial port and attempts to extract a numeric value.
    /// Used primarily during auto-detection to avoid blocking on line delimiters.
    fn try_read_once(&mut self) -> Result<Option<f64>, Box<dyn std::error::Error>> {
        // Send polling command if specified
        if let Some(ref cmd) = self.command {
            let cmd_with_newline = format!("{}\r\n", cmd);
            let _ = self.port.write_all(cmd_with_newline.as_bytes());
        }

        let mut buffer: [u8; 256] = [0; 256];
        match self.port.read(&mut buffer) {
            Ok(bytes_read) => {
                // Convert bytes to UTF-8 (lossy to handle potential garbage data)
                let data = String::from_utf8_lossy(&buffer[..bytes_read]);
                if let Some(mat) = self.re.find(&data) {
                    // Extract and parse the first number found in the string
                    return Ok(mat.as_str().parse().ok());
                }
                Ok(None)
            }
            // Ignore timeouts during polling
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}

/// Brute-force scanning function to find the correct serial settings
fn auto_detect_settings(port_name: &str, command: &Option<String>) -> Result<SerialSettings, Box<dyn std::error::Error>> {
    let baud_rates = [9600, 4800, 2400, 1200, 19200, 115200];
    let parities = [Parity::None, Parity::Even, Parity::Odd];
    let data_bits = [DataBits::Eight, DataBits::Seven];
    let stop_bits = [StopBits::One, StopBits::Two];

    println!("Attempting to auto-detect scale settings on {} (Extended Scan)...", port_name);

    // Iterate through all possible combinations
    for &baud in &baud_rates {
        for &parity in &parities {
            for &bits in &data_bits {
                for &stop in &stop_bits {
                    let settings = SerialSettings {
                        port_name: port_name.to_string(),
                        baud_rate: baud,
                        data_bits: bits,
                        parity,
                        stop_bits: stop,
                        multiplier: 1.0,
                    };

                    print!("Testing: {} baud, {:?}, {:?}, {:?}... ", baud, bits, parity, stop);
                    io::stdout().flush()?;

                    // Use a short 500ms timeout for scanning
                    if let Ok(mut driver) = ScaleDriver::open(&settings, command.clone(), 500) {
                        for _ in 0..3 {
                            match driver.try_read_once() {
                                Ok(Some(weight)) => {
                                    println!("SUCCESS! Detected weight: {}", weight);
                                    return Ok(settings);
                                }
                                Ok(None) => {
                                    // If no weight found, log the raw data for debugging
                                    let mut buffer: [u8; 32] = [0; 32];
                                    if let Ok(n) = driver.port.read(&mut buffer) {
                                        if n > 0 {
                                            let raw = String::from_utf8_lossy(&buffer[..n]);
                                            print!("[Raw Data: {}] ", raw.trim().escape_debug());
                                            io::stdout().flush()?;
                                        }
                                    }
                                }
                                Err(_) => break,
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    }
                    println!("Failed.");
                }
            }
        }
    }
    Err("Could not auto-detect settings. Is the scale connected and sending data?".into())
}

/// Interactive helper to let the user select a serial port from a list
fn select_port() -> Result<String, Box<dyn std::error::Error>> {
    let ports = serialport::available_ports()?;
    if ports.is_empty() { return Err("No serial ports found!".into()); }
    
    println!("Available ports:");
    for (i, p) in ports.iter().enumerate() { 
        println!("{}: {}", i, p.port_name); 
    }
    
    print!("\nEnter the number of the port you want to use: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let index: usize = input.trim().parse()?;
    
    if index < ports.len() { 
        Ok(ports[index].port_name.clone()) 
    } else { 
        Err("Invalid selection".into()) 
    }
}

/// Loads configuration from the local JSON file
fn load_config() -> Option<SerialSettings> {
    if Path::new(CONFIG_FILE).exists() {
        let content = fs::read_to_string(CONFIG_FILE).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

/// Saves configuration to the local JSON file
fn save_config(settings: &SerialSettings) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(CONFIG_FILE, content)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse CLI arguments
    let args = Args::parse();

    // 2. Resolve settings: Load from config, scan, or use CLI overrides
    let mut settings = if !args.force_detect {
        if let Some(config) = load_config() {
            println!("Loaded existing configuration for {}.", config.port_name);
            config
        } else {
            let port_name = args.port.clone().unwrap_or(select_port()?);
            let mut detected = auto_detect_settings(&port_name, &args.command)?;
            detected.multiplier = args.multiplier;
            save_config(&detected)?;
            println!("Configuration saved to {}.", CONFIG_FILE);
            detected
        }
    } else {
        // Re-detect if forced
        let port_name = args.port.clone().unwrap_or(select_port()?);
        let mut detected = auto_detect_settings(&port_name, &args.command)?;
        detected.multiplier = args.multiplier;
        save_config(&detected)?;
        detected
    };

    // CLI multiplier override (if provided)
    if args.multiplier != 1.0 {
        settings.multiplier = args.multiplier;
    }

    // 3. Open the port with the final settings
    let driver = ScaleDriver::open(&settings, args.command, 1000)?;
    println!("\nMonitoring weight with multiplier: {}x (Press Ctrl+C to exit)...", settings.multiplier);

    // 4. Monitoring Loop: Uses BufReader for reliable line-by-line extraction
    let mut reader = BufReader::new(driver.port);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    // Search for numeric value in the line
                    if let Some(mat) = driver.re.find(&line) {
                        if let Ok(weight) = mat.as_str().parse::<f64>() {
                            // Apply scale factor and display
                            let adjusted_weight = weight * settings.multiplier;
                            println!("Weight: {:.3}", adjusted_weight);
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {},
            Err(e) => {
                eprintln!("Error: {}. Re-attempting...", e);
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        // Small delay to prevent high CPU usage
        std::thread::sleep(Duration::from_millis(50));
    }
}
