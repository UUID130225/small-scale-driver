use clap::Parser;
use serialport::{SerialPort, DataBits, Parity, StopBits};
use std::io::{self, BufRead, BufReader, Write, Read};
use std::time::Duration;
use regex::Regex;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

const CONFIG_FILE: &str = "config.json";

#[derive(Parser, Debug)]
#[command(author, version, about = "Precision Scale Driver with Robust Auto-Detection", long_about = None)]
struct Args {
    /// Serial port to connect to.
    #[arg(short, long)]
    port: Option<String>,

    /// Command to send to the scale to request weight (e.g., "W")
    #[arg(short, long)]
    command: Option<String>,

    /// Force auto-detection
    #[arg(short, long, default_value_t = false)]
    force_detect: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SerialSettings {
    port_name: String,
    baud_rate: u32,
    data_bits: DataBits,
    parity: Parity,
    stop_bits: StopBits,
}

impl Default for SerialSettings {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
        }
    }
}

struct ScaleDriver {
    port: Box<dyn SerialPort>,
    re: Regex,
    command: Option<String>,
}

impl ScaleDriver {
    fn open(settings: &SerialSettings, command: Option<String>, timeout_ms: u64) -> Result<Self, Box<dyn std::error::Error>> {
        let port = serialport::new(&settings.port_name, settings.baud_rate)
            .timeout(Duration::from_millis(timeout_ms))
            .data_bits(settings.data_bits)
            .parity(settings.parity)
            .stop_bits(settings.stop_bits)
            .open()?;

        let re = Regex::new(r"[-+]?[0-9]*\.?[0-9]+")?;

        Ok(ScaleDriver { port, re, command })
    }

    fn try_read_once(&mut self) -> Result<Option<f64>, Box<dyn std::error::Error>> {
        if let Some(ref cmd) = self.command {
            let cmd_with_newline = format!("{}\r\n", cmd);
            let _ = self.port.write_all(cmd_with_newline.as_bytes());
        }

        let mut buffer: [u8; 256] = [0; 256];
        match self.port.read(&mut buffer) {
            Ok(bytes_read) => {
                let data = String::from_utf8_lossy(&buffer[..bytes_read]);
                if let Some(mat) = self.re.find(&data) {
                    return Ok(mat.as_str().parse().ok());
                }
                Ok(None)
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}

fn auto_detect_settings(port_name: &str, command: &Option<String>) -> Result<SerialSettings, Box<dyn std::error::Error>> {
    let baud_rates = [9600, 4800, 2400, 1200, 19200, 115200];
    let parities = [Parity::None, Parity::Even, Parity::Odd];
    let data_bits = [DataBits::Eight, DataBits::Seven];
    let stop_bits = [StopBits::One, StopBits::Two];

    println!("Attempting to auto-detect scale settings on {} (Extended Scan)...", port_name);

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
                    };

                    print!("Testing: {} baud, {:?}, {:?}, {:?}... ", baud, bits, parity, stop);
                    io::stdout().flush()?;

                    if let Ok(mut driver) = ScaleDriver::open(&settings, command.clone(), 500) {
                        // Try a few times because data might be interleaved
                        for _ in 0..3 {
                            match driver.try_read_once() {
                                Ok(Some(weight)) => {
                                    println!("SUCCESS! Detected weight: {}", weight);
                                    return Ok(settings);
                                }
                                Ok(None) => {
                                    // Try to see what's actually coming in
                                    let mut buffer: [u8; 32] = [0; 32];
                                    if let Ok(n) = driver.port.read(&mut buffer) {
                                        if n > 0 {
                                            let raw = String::from_utf8_lossy(&buffer[..n]);
                                            print!("[Raw: {}] ", raw.trim().escape_debug());
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

fn select_port() -> Result<String, Box<dyn std::error::Error>> {
    let ports = serialport::available_ports()?;
    if ports.is_empty() { return Err("No serial ports found!".into()); }
    println!("Available ports:");
    for (i, p) in ports.iter().enumerate() { println!("{}: {}", i, p.port_name); }
    print!("\nEnter the number of the port you want to use: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let index: usize = input.trim().parse()?;
    if index < ports.len() { Ok(ports[index].port_name.clone()) } else { Err("Invalid selection".into()) }
}

fn load_config() -> Option<SerialSettings> {
    if Path::new(CONFIG_FILE).exists() {
        let content = fs::read_to_string(CONFIG_FILE).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

fn save_config(settings: &SerialSettings) -> Result<(), Box<dyn std::error::Error>> {
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(CONFIG_FILE, content)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let settings = if !args.force_detect {
        if let Some(config) = load_config() {
            println!("Loaded existing configuration for {}.", config.port_name);
            config
        } else {
            let port_name = args.port.clone().unwrap_or(select_port()?);
            let detected = auto_detect_settings(&port_name, &args.command)?;
            save_config(&detected)?;
            println!("Configuration saved to {}.", CONFIG_FILE);
            detected
        }
    } else {
        let port_name = args.port.clone().unwrap_or(select_port()?);
        let detected = auto_detect_settings(&port_name, &args.command)?;
        save_config(&detected)?;
        detected
    };

    let mut driver = ScaleDriver::open(&settings, args.command, 1000)?;
    println!("\nMonitoring weight (Press Ctrl+C to exit)...");

    // For monitoring, we use BufReader for cleaner line-based output
    let mut reader = BufReader::new(driver.port);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {
                if let Some(mat) = driver.re.find(&line) {
                    if let Ok(weight) = mat.as_str().parse::<f64>() {
                        println!("Current Weight: {:.3}", weight);
                    }
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {},
            Err(e) => {
                eprintln!("Error: {}. Re-attempting...", e);
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
