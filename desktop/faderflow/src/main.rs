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
use std::io::{Read, Write, BufRead};
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use comms::protocol;
use crate::comms::protocol::{
    FaderMessage, HandshakeResponse,
    CMD_FADER_UPDATE, CMD_HANDSHAKE_REQUEST,
    DisplayUpdateAppCommand, DisplayUpdateVolumeCommand,
    CMD_DISPLAY_UPDATE_APP_NAME, CMD_DISPLAY_UPDATE_APP_VOLUME
};

pub const INIT_WAIT_TIME_MS: u64 = 100;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Scanning for FaderFlow devices...\n");

    let ports = serialport::available_ports()?;
    let mut device_port: Option<Box<dyn SerialPort>> = None;

    // Find device
    for port_info in ports {
        println!("Checking port: {}", port_info.port_name);

        match serialport::new(&port_info.port_name, 115200)
            .timeout(Duration::from_millis(INIT_WAIT_TIME_MS))
            .flow_control(serialport::FlowControl::None)
            .open()
        {
            Ok(mut port) => {
                println!("  Port opened successfully");

                // Wait for Arduino to reset after serial connection
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

    let port = device_port.ok_or("No FaderFlow device found")?;

    println!("Commands:");
    println!("  app:<channel>:<name>  - Set app name (e.g., 'app:0:Spotify')");
    println!("  vol:<channel>:<0-100> - Set volume (e.g., 'vol:0:75')");
    println!("Listening for fader updates... (Ctrl+C to exit)\n");

    // Wrap port in Arc<Mutex> so we can share it between threads
    let port = Arc::new(Mutex::new(port));
    let port_clone = Arc::clone(&port);

    // Spawn thread for reading console input
    thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            if let Ok(cmd) = line {
                let parts: Vec<&str> = cmd.trim().split(':').collect();

                if parts.len() >= 3 && parts[0] == "app" {
                    // Parse: app:0:Spotify
                    if let Ok(channel) = parts[1].parse::<u8>() {
                        let app_name = parts[2..].join(":"); // Handle colons in app name

                        if let Ok(mut port) = port_clone.lock() {
                            send_app_name(&mut **port, channel, &app_name).ok();
                        }
                    }
                } else if parts.len() == 3 && parts[0] == "vol" {
                    // Parse: vol:0:75
                    if let Ok(channel) = parts[1].parse::<u8>() {
                        if let Ok(volume) = parts[2].parse::<u8>() {
                            if let Ok(mut port) = port_clone.lock() {
                                send_volume(&mut **port, channel, volume).ok();
                            }
                        }
                    }
                }
            }
        }
    });

    // Listen for fader messages (main thread)
    loop {
        let mut buf = [0u8; std::mem::size_of::<FaderMessage>()];

        let result = {
            let mut port = port.lock().unwrap();
            port.read_exact(&mut buf)
        };

        match result {
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

fn send_app_name(port: &mut dyn SerialPort, channel: u8, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = DisplayUpdateAppCommand {
        cmd: CMD_DISPLAY_UPDATE_APP_NAME,
        channel,
        name: [0; 64],
    };

    let bytes = name.as_bytes();
    let len = bytes.len().min(63);
    cmd.name[..len].copy_from_slice(&bytes[..len]);

    let bytes = unsafe {
        std::slice::from_raw_parts(&cmd as *const _ as *const u8, std::mem::size_of::<DisplayUpdateAppCommand>())
    };

    println!("Sending {} bytes: {:?}", bytes.len(), &bytes[..10]); // Print first 10 bytes
    port.write_all(bytes)?;
    port.flush()?;

    println!("Sent app name '{}' to channel {}", name, channel);
    Ok(())
}

fn send_volume(port: &mut dyn SerialPort, channel: u8, volume: u8) -> Result<(), Box<dyn std::error::Error>> {
    let cmd = DisplayUpdateVolumeCommand {
        cmd: CMD_DISPLAY_UPDATE_APP_VOLUME,
        channel,
        volume: volume.min(100),
    };

    let bytes = unsafe {
        std::slice::from_raw_parts(&cmd as *const _ as *const u8, std::mem::size_of::<DisplayUpdateVolumeCommand>())
    };
    port.write_all(bytes)?;
    port.flush()?;

    println!("Sent volume {} to channel {}", volume, channel);
    Ok(())
}