use clap::Parser;
use serialport::SerialPort;
use std::io::{self, Read, Write};
use std::time::Duration;
use regex::Regex;

#[derive(Parser, Debug)]
#[command(author, version, about = "Precision Scale Driver", long_about = None)]
struct Args {
    /// Serial port to connect to (e.g., /dev/ttyUSB0 or COM3). If omitted, the program will scan and ask.
    #[arg(short, long)]
    port: Option<String>,

    /// Baud rate for the serial connection
    #[arg(short, long, default_value_t = 9600)]
    baud_rate: u32,

    /// Timeout in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    timeout: u64,

    /// Command to send to the scale to request weight (e.g., "W")
    #[arg(short, long)]
    command: Option<String>,
}

struct ScaleDriver {
    port: Box<dyn SerialPort>,
    re: Regex,
    command: Option<String>,
}

impl ScaleDriver {
    fn new(port_name: &str, baud_rate: u32, timeout_ms: u64, command: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(timeout_ms))
            .data_bits(serialport::DataBits::Eight)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .open()?;

        // Regex to find floating point numbers
        let re = Regex::new(r"[-+]?[0-9]*\.?[0-9]+")?;

        Ok(ScaleDriver { port, re, command })
    }

    fn read_weight(&mut self) -> Result<Option<f64>, Box<dyn std::error::Error>> {
        if let Some(ref cmd) = self.command {
            let cmd_with_newline = format!("{}\r\n", cmd);
            self.port.write_all(cmd_with_newline.as_bytes())?;
        }

        let mut buffer: [u8; 128] = [0; 128];
        match self.port.read(&mut buffer) {
            Ok(bytes_read) => {
                let data = String::from_utf8_lossy(&buffer[..bytes_read]);
                Ok(parse_weight(&self.re, &data))
            }
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}

fn parse_weight(re: &Regex, data: &str) -> Option<f64> {
    if let Some(mat) = re.find(data) {
        return mat.as_str().parse().ok();
    }
    None
}

fn select_port() -> Result<String, Box<dyn std::error::Error>> {
    let ports = serialport::available_ports()?;
    if ports.is_empty() {
        return Err("No serial ports found!".into());
    }

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let port_name = match args.port {
        Some(p) => p,
        None => select_port()?,
    };

    println!("Connecting to scale on {} at {} baud...", port_name, args.baud_rate);

    let mut driver = ScaleDriver::new(&port_name, args.baud_rate, args.timeout, args.command)?;

    println!("Connection established. Monitoring weight (Press Ctrl+C to exit)...");

    loop {
        match driver.read_weight() {
            Ok(Some(weight)) => {
                println!("Current Weight: {:.3}", weight);
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("Error reading from scale: {}", e);
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_weight() {
        let re = Regex::new(r"[-+]?[0-9]*\.?[0-9]+").unwrap();
        assert_eq!(parse_weight(&re, "STX  1.25 kg ETX"), Some(1.25));
        assert_eq!(parse_weight(&re, "Weight: -0.500 g"), Some(-0.5));
        assert_eq!(parse_weight(&re, "No data"), None);
        assert_eq!(parse_weight(&re, "100"), Some(100.0));
    }
}
