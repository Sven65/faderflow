/**mod audio;
mod ui;
mod utils;

use ui::app::VolumeApp;

fn main() -> iced::Result {
    iced::application(VolumeApp::new, VolumeApp::update, VolumeApp::view)
        .subscription(VolumeApp::subscription)
        .run()
}
**/

mod comms;

use serialport::{self, SerialPort};
use std::io::{Read, Write};
use std::time::Duration;
use comms::protocol;
use crate::comms::protocol::{FaderMessage, HandshakeResponse, CMD_FADER_UPDATE, CMD_HANDSHAKE_REQUEST};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Scanning for FaderFlow devices...\n");

    let ports = serialport::available_ports()?;
    let mut device_port: Option<Box<dyn SerialPort>> = None;

    // Find device
    for port_info in ports {
        println!("Checking port: {}", port_info.port_name);

        match serialport::new(&port_info.port_name, 115200)
            .timeout(Duration::from_millis(1000))  // Longer timeout
            .open()
        {
            Ok(mut port) => {
                println!("  Port opened successfully");

                // Wait longer for Arduino to reset after serial connection
                std::thread::sleep(Duration::from_millis(2000));

                // Clear buffer
                let mut discard = [0u8; 256];
                match port.read(&mut discard) {
                    Ok(n) => println!("  Cleared {} bytes from buffer", n),
                    Err(_) => println!("  Buffer was empty"),
                }

                // Send handshake request
                println!("  Sending handshake request...");
                port.write_all(&[CMD_HANDSHAKE_REQUEST])?;
                port.flush()?;
                std::thread::sleep(Duration::from_millis(500));

                // Read response
                let mut buf = [0u8; std::mem::size_of::<HandshakeResponse>()];
                println!("  Waiting for {} bytes...", buf.len());

                match port.read_exact(&mut buf) {
                    Ok(_) => {
                        println!("  Received response, parsing...");
                        let response: HandshakeResponse = unsafe {
                            std::ptr::read(buf.as_ptr() as *const _)
                        };

                        println!("  Magic: {:?}", &response.magic[..9]);

                        if response.is_valid() {
                            println!("✓ Found FaderFlow device!\n");
                            device_port = Some(port);
                            break;
                        } else {
                            println!("  Invalid magic string\n");
                        }
                    }
                    Err(e) => {
                        println!("  Failed to read response: {}\n", e);
                    }
                }
            }
            Err(e) => {
                println!("  Failed to open port: {}\n", e);
            }
        }
    }

    let mut port = device_port.ok_or("No FaderFlow device found")?;

    println!("Listening for fader updates... (Ctrl+C to exit)\n");

    // Listen for fader messages
    loop {
        let mut buf = [0u8; std::mem::size_of::<FaderMessage>()];

        match port.read_exact(&mut buf) {
            Ok(_) => {
                let msg: FaderMessage = unsafe {
                    std::ptr::read(buf.as_ptr() as *const _)
                };

                if msg.cmd == CMD_FADER_UPDATE {
                    println!(
                        "Fader {} -> Position: {} ({}%)",
                        msg.channel,
                        msg.position,
                        msg.position_percent()
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => {
                eprintln!("Error reading from serial: {}", e);
                break;
            }
        }
    }

    Ok(())
}