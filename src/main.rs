//! A utility for capturing serial port data and writing it to PCAP files.
//! 
//! This program allows capturing data from a serial port and saving it in the PCAP
//! file format, which is commonly used for network packet captures. The captured data
//! can be analyzed using standard network analysis tools like Wireshark.
//!
//! # Features
//! 
//! - Configurable baud rate, parity, and stop bits
//! - Adjustable inter-frame gap timing
//! - Output to either files or named pipes
//! - Automatic timestamp recording
//! - PCAP format compatibility
//!
//! # Example Usage
//!
//! ```bash
//! serialpcap /dev/ttyUSB0 -b 115200 -y n -p 1 -g 10 -o output
//! ```


use std::fs::File;
use std::io;
use std::time::Duration;
use clap::{value_parser, Arg, Command, ArgAction};
use serialport::SerialPort;
use pcap_file::{pcap::{PcapPacket, PcapWriter}, pcapng::blocks::packet, DataLink};
use chrono::prelude::*;
use crate::{datalink::parse_datalink, portinfo::{AnySerialPort, PortControlLines}};
use crate::datalink::parse_datalink;

pub mod datalink;
pub mod portinfo;
mod state;


/// Represents the encapsulation mode used for the captured data.
pub enum EncapsulationMode {
    Raw,
    DatalinkType
}
 
/// Represents a serial port capture session with configurable parameters
/// 
/// # Fields
/// 
/// * `port` - The serial port interface
/// * `baud_rate` - The communication speed in bits per second
/// * `parity` - Parity checking mode ('n' for none, 'e' for even, 'o' for odd)
/// * `stopbits` - Number of stop bits (1 or 2)
/// * `frame_gap_ms` - Time gap between frames in milliseconds
struct CaptureSerial {
   port: AnySerialPort,
   datalink: DataLink,
   baud_rate: u32,
   parity: char,
   stopbits: u8,
   frame_gap_ms: u64,
   encap_mode: EncapsulationMode,
}


const MAX_PACKET_SIZE: usize = 1024;

impl CaptureSerial {
    fn new(port_name: &str, baud_rate: u32, parity: char, stopbits: u8, frame_gap_ms: u64, datalink: DataLink, encap_mode: EncapsulationMode) -> io::Result<Self> {
        let port = AnySerialPort::Basic(serialport::new(port_name, baud_rate)
            .parity(match parity {
                'o' => serialport::Parity::Odd,
                'e' => serialport::Parity::Even,
                _ => serialport::Parity::None,
            })
            .stop_bits(match stopbits {
                1 => serialport::StopBits::One,
                2 => serialport::StopBits::Two,
                _ => serialport::StopBits::One,
            })
            .timeout(Duration::from_millis(frame_gap_ms))
            .open()?);

        Ok(CaptureSerial {
            port,
            baud_rate,
            parity,
            stopbits,
            frame_gap_ms,
            datalink,
            encap_mode
        })
    }

    /// Captures a packet from the serial port
    ///
    /// # Returns
    /// 
    /// An `Option<Vec<u8>>` containing the captured packet data. Returns `None` if no data is captured.
    fn capture_packet(&mut self) -> Result<Vec<u8>, io::Error> {
        let mut buffer: Vec<u8> = vec![0; MAX_PACKET_SIZE];
        let mut bytes_read = 0;
        while match self.port.as_serial_port().read(&mut buffer[bytes_read..]) {

            Ok(this_read_len) => {
                bytes_read += this_read_len;
                bytes_read < buffer.len()
            },
            Err(e) => {
                if (e.kind() == io::ErrorKind::TimedOut) {
                    // Timeout is expected, but
                    // indicates the end of a packet.
                    false
                } else {
                    // Handle other errors
                    return Err(e);
                    false
                }
            },
        }  {}

        if bytes_read == 0 {
            Ok(vec![])
        } else {
            Ok(buffer[..bytes_read].to_vec())
        }
    }

    /// Captures data from the serial port and writes it to a PCAP file
    /// 
    /// # Arguments
    ///     
    /// * `file` - The output file to write the captured data to
    fn capture(&mut self, file: File) -> io::Result<()> {
        // Setup PCap Header to set our datalink type.
        let pcap_header = pcap_file::pcap::PcapHeader {
            version_major: 2,
            version_minor: 4,
            snaplen: MAX_PACKET_SIZE as u32,
            datalink: self.datalink.clone(),
            ts_correction: 0,
            ts_accuracy: 0,
            ts_resolution: pcap_file::TsResolution::MicroSecond,
            endianness: pcap_file::Endianness::Big
        };
        let mut writer = PcapWriter::with_header(file, pcap_header).expect("Error writing output file");
        let zero_time = Utc::now().timestamp_micros(); // Initialize zero time
        loop {
            let packet_maybe = self.capture_packet(); 
            if let Ok(packet) = packet_maybe {
                if packet.is_empty() {
                    continue;
                }

                // Encapsulate the packet data for the datalink type/force raw
                let encap_packet = match self.encap_mode {
                    EncapsulationMode::Raw => packet.clone(),
                    EncapsulationMode::DatalinkType => {
                        // Use the datalink type to encapsulate the data
                        datalink::get_encapsulated_data(&Utc::now(), "serial", &self.datalink, &packet).unwrap()
                    }
                };
                writer.write_packet(
                    &PcapPacket {
                        timestamp: Duration::from_micros(
                            ((Utc::now().timestamp_micros() - zero_time) as i64).try_into().unwrap()
                        ),
                        orig_len: encap_packet.len() as u32,
                        data: encap_packet.into(),
                    }
                ).unwrap();
            } else {
                // Recast error to io::Error
                if let Err(e) = packet_maybe {
                    return Err(e);
                }
                return Err(io::Error::new(io::ErrorKind::Other, "Unknown error"));
            }
        }
    }
}



fn main() {
    let matches = Command::new("SerialPCAP")
        .version("1.0")
        .author("Author Name <email@example.com>")
        .about("Captures serial port data and writes to a pcap file")
        .arg(Arg::new("baud")
            .short('b')
            .long("baud")
            .value_name("BAUD")
            .default_value("9600")
            .value_parser(value_parser!(u32))
            .help("Serial port speed (default 9600)"))
        .arg(Arg::new("parity")
            .short('y')
            .long("parity")
            .value_name("PARITY")
            .default_value("n")
            .value_parser(value_parser!(char))
            .help("o (=odd) | e (=even) | n (=none) (default none)"))
        .arg(Arg::new("stopbits")
            .short('p')
            .long("stopbits")
            .value_name("STOPBITS")
            .value_parser(value_parser!(u8))
            .default_value("1")
            .help("1 | 2 (default 1)"))
        .arg(Arg::new("gap")
            .short('g')
            .long("gap")
            .value_name("GAP")
            .default_value("10")
            .value_parser(value_parser!(u64))
            .help("Inter frame gap in milliseconds (default 10)"))
        .arg(Arg::new("output")
            .short('o')
            .long("output")
            .value_name("OUTPUT")
            .help("Output file prefix or pipe (default port name)"))
        .arg(Arg::new("pipe")
            .long("pipe")
            .action(ArgAction::SetTrue)
            .help("Pipe mode: treat the output file as exact name not a prefix"))
        .arg(Arg::new("raw")
            .long("force-raw")
            .num_args(0)
            .help("Use raw encapsulation instead of datalink type"))
        .arg(Arg::new("datalinktype")
            .long("datalinktype")
            .value_parser(&parse_datalink)
            .help("Datalink type (default USER0)")
            .default_value("USER0")
        )
        .arg(Arg::new("port")
            .help("Serial port name")
            .required(true)
            .index(1))
        .get_matches();

    let baud_rate= *matches.get_one::<u32>("baud").unwrap(); 
    let parity = *matches.get_one::<char>("parity").unwrap();
    let stopbits = *matches.get_one::<u8>("stopbits").unwrap();
    let frame_gap_ms = *matches.get_one::<u64>("gap").unwrap();
    let port_name = matches.get_one::<String>("port").unwrap();
    let output_file_prefix = matches.get_one("output").unwrap_or(port_name);
    let use_pipe = matches.get_flag("pipe");
    let datalink = matches.get_one("datalinktype").unwrap_or(&pcap_file::DataLink::USER0);
    let encap_mode: EncapsulationMode = if matches.contains_id("raw") { EncapsulationMode::Raw } else { EncapsulationMode::DatalinkType };


    let output_file = if use_pipe {
        output_file_prefix.to_string()
    } else {
        format!("{}-{}.pcap", output_file_prefix, chrono::Utc::now().format("%Y%m%d-%H%M%S"))
    };

    let mut bus = CaptureSerial::new(port_name, baud_rate, parity, stopbits, frame_gap_ms, *datalink, encap_mode).expect("Failed to open serial port");

    let file = File::create(output_file).expect("Failed to create output file");

    if let Err(e) = bus.capture(file) {
        eprintln!("Error occurred: {}", e);
    }
}
