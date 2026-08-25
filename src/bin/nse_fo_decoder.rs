//! CLI entry point: mirrors `main()` in `nse_fo_decoder.cpp`. Reads one
//! hex-encoded datagram per line from stdin, writes one JSON array per line to
//! stdout, and logs decode failures to stderr rather than aborting.

use std::io::{self, BufRead, Write};

fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    if hex.len() % 2 != 0 {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect()
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        let packet = match hex_to_bytes(&line) {
            Some(bytes) if !bytes.is_empty() => bytes,
            _ => {
                writeln!(out, "[]")?;
                continue;
            }
        };

        let messages = match nse_fo_decoder::decode_packet(&packet) {
            Ok(msgs) => msgs,
            Err(e) => {
                eprintln!("[nse_fo_decoder] decode_packet failed: {e}");
                Vec::new()
            }
        };

        let json = serde_json::to_string(&messages).expect("Message serialization cannot fail");
        writeln!(out, "{json}")?;
    }

    Ok(())
}
