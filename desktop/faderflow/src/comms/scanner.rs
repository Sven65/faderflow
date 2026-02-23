use serialport::{self, SerialPort};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::sync::mpsc;

use crate::comms::protocol::{HandshakeResponse, CMD_HANDSHAKE_ACK, CMD_HANDSHAKE_REQUEST, CMD_HANDSHAKE_RESPONSE};

pub const RESCAN_DELAY_SECS: u64 = 3;

const ARDUINO_BOOT_WAIT_MS: u64 = 500;   // just enough to let DTR reset start
const HANDSHAKE_TIMEOUT_SECS: u64 = 8;  // beacon fires every 500ms, give plenty of room
const WATCHDOG_INTERVAL_MS: u64 = 500;
const WATCHDOG_FAIL_THRESHOLD: u32 = 3;

// ── Shared port handle ───────────────────────────────────────────────────────

pub type SharedPort = Arc<Mutex<Box<dyn SerialPort + Send>>>;

// ── Events flowing from scanner → app ───────────────────────────────────────

pub enum ScanEvent {
    Started { total_ports: usize },
    CheckingPort { name: String, index: usize, total: usize },
    PortFailed { name: String, reason: String },
    /// Sent once per found device — includes the ready-to-use port + handshake info
    DeviceFound {
        port_name: String,
        port: SharedPort,
        uuid: [u8; 16],
        version: (u8, u8),
    },
    ScanComplete { found: usize },
    DeviceLost { port_name: String },
    ScanFailed(String),
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Spawns the scan on a background thread.
/// Returns a Receiver the app should poll via ScanTick.
pub fn start_scan() -> mpsc::Receiver<ScanEvent> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || run_scan(tx));
    rx
}

/// Spawns a watchdog for a connected device.
/// Sends `DeviceLost` through `tx` if the port goes silent.
pub fn start_watchdog(
    port_name: String,
    port: SharedPort,
    tx: mpsc::Sender<ScanEvent>,
) -> Arc<std::sync::atomic::AtomicBool> {
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cancel_clone = Arc::clone(&cancel);

    thread::spawn(move || {
        let mut fails = 0u32;
        loop {
            thread::sleep(Duration::from_millis(WATCHDOG_INTERVAL_MS));

            if cancel_clone.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let alive = port.lock()
                .map(|mut p| p.flush().is_ok())
                .unwrap_or(false);

            if alive {
                fails = 0;
            } else {
                fails += 1;
                if fails >= WATCHDOG_FAIL_THRESHOLD {
                    let _ = tx.send(ScanEvent::DeviceLost { port_name });
                    break;
                }
            }
        }
    });

    cancel
}

// ── Internal scan logic ──────────────────────────────────────────────────────

fn run_scan(tx: mpsc::Sender<ScanEvent>) {
    let ports = match serialport::available_ports() {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(ScanEvent::ScanFailed(format!("Cannot list ports: {e}")));
            return;
        }
    };

    let total = ports.len();
    let _ = tx.send(ScanEvent::Started { total_ports: total });

    if total == 0 {
        let _ = tx.send(ScanEvent::ScanComplete { found: 0 });
        return;
    }

    let mut found = 0usize;

    for (idx, info) in ports.iter().enumerate() {
        let name = info.port_name.clone();
        let _ = tx.send(ScanEvent::CheckingPort {
            name: name.clone(),
            index: idx + 1,
            total,
        });

        match probe_port(&name) {
            Ok((port, uuid, version)) => {
                found += 1;
                let shared = Arc::new(Mutex::new(port));
                let _ = tx.send(ScanEvent::DeviceFound {
                    port_name: name,
                    port: shared,
                    uuid,
                    version,
                });
            }
            Err(reason) => {
                let _ = tx.send(ScanEvent::PortFailed { name, reason });
            }
        }
    }

    let _ = tx.send(ScanEvent::ScanComplete { found });
}

fn probe_port(port_name: &str) -> Result<(Box<dyn SerialPort + Send>, [u8; 16], (u8, u8)), String> {
    let mut port = serialport::new(port_name, 115200)
        .timeout(Duration::from_millis(20))
        .flow_control(serialport::FlowControl::None)
        .open()
        .map_err(|e| format!("Cannot open: {e}"))?;

    // Short wait for DTR reset to kick off, then start reading immediately
    thread::sleep(Duration::from_millis(ARDUINO_BOOT_WAIT_MS));

    // Actively request a handshake — handles two cases:
    //   1. Fresh boot: Arduino ignores this and beacons anyway
    //   2. Already running: Arduino won't beacon, so we must ask
    port.write_all(&[CMD_HANDSHAKE_REQUEST])
        .map_err(|e| format!("Failed to send handshake request: {e}"))?;
    port.flush().ok();

    let deadline = Instant::now() + Duration::from_secs(HANDSHAKE_TIMEOUT_SECS);

    loop {
        if Instant::now() > deadline {
            return Err("Handshake timeout".into());
        }

        // Read one byte at a time — sync to the beacon start byte
        let mut byte = [0u8; 1];
        match port.read(&mut byte) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => return Err(format!("Read error: {e}")),
        }

        if byte[0] != CMD_HANDSHAKE_RESPONSE {
            // Not the start of a handshake, keep scanning
            continue;
        }

        // Found the command byte — read the rest of the struct
        let remaining = std::mem::size_of::<HandshakeResponse>() - 1;
        let mut rest = vec![0u8; remaining];
        match port.read_exact(&mut rest) {
            Ok(_) => {}
            Err(_) => {
                // Partial read — the beacon will fire again in 500ms, keep waiting
                continue;
            }
        }

        let mut full = vec![0u8; std::mem::size_of::<HandshakeResponse>()];
        full[0] = CMD_HANDSHAKE_RESPONSE;
        full[1..].copy_from_slice(&rest);

        let response: HandshakeResponse = unsafe {
            std::ptr::read(full.as_ptr() as *const _)
        };

        if !response.is_valid() {
            // Bad magic — might have caught a partial beacon, wait for next one
            continue;
        }

        // Valid — ACK to stop beaconing
        port.write_all(&[CMD_HANDSHAKE_ACK])
            .map_err(|e| format!("ACK failed: {e}"))?;
        port.flush().ok();

        return Ok((port, response.uuid, (response.version_major, response.version_minor)));
    }
}

pub fn start_scan_delayed(delay_ms: u64) -> mpsc::Receiver<ScanEvent> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(delay_ms));
        run_scan(tx);
    });
    rx
}