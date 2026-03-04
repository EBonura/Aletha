/// Music/SFX cart parsing and hex output.

use std::path::Path;

/// Parse one __sfx__ line (168 hex chars) back into 68 raw bytes.
fn parse_p8_sfx_line(line: &str) -> [u8; 68] {
    let mut slot_bytes = [0u8; 68];
    // Header: first 8 hex chars -> bytes 64-67
    let header = &line[..8];
    for i in 0..4 {
        slot_bytes[64 + i] = u8::from_str_radix(&header[i * 2..i * 2 + 2], 16).unwrap_or(0);
    }
    // Notes: 32 notes x 5 hex chars each
    for n in 0..32 {
        let s = &line[8 + n * 5..8 + (n + 1) * 5];
        let pitch = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let wf_hex = u8::from_str_radix(&s[2..3], 16).unwrap_or(0);
        let vol = u8::from_str_radix(&s[3..4], 16).unwrap_or(0);
        let eff = u8::from_str_radix(&s[4..5], 16).unwrap_or(0);
        let wf = wf_hex & 0x7;
        let custom = (wf_hex >> 3) & 0x1;
        let b0 = (pitch & 0x3F) | ((wf & 0x3) << 6);
        let b1 = ((wf >> 2) & 0x1) | ((vol & 0x7) << 1) | ((eff & 0x7) << 4) | ((custom & 0x1) << 7);
        slot_bytes[2 * n] = b0;
        slot_bytes[2 * n + 1] = b1;
    }
    slot_bytes
}

/// Load __sfx__ and __music__ sections from a .p8 cart.
pub fn load_music_cart(path: &Path) -> Option<(Vec<u8>, Vec<u8>)> {
    let text = std::fs::read_to_string(path).ok()?;

    // Parse __sfx__
    let mut sfx_buf = vec![0u8; 68 * 64];
    if let Some(start) = text.find("__sfx__\n") {
        let after = &text[start + 8..];
        let end = after.find("\n__").unwrap_or(after.len());
        for (i, line) in after[..end].trim().lines().enumerate() {
            let line = line.trim();
            if line.len() == 168 && i < 64 {
                let slot = parse_p8_sfx_line(line);
                sfx_buf[i * 68..(i + 1) * 68].copy_from_slice(&slot);
            }
        }
    }

    // Parse __music__
    let mut music_buf = vec![0u8; 256];
    if let Some(start) = text.find("__music__\n") {
        let after = &text[start + 10..];
        let end = after.find("\n__").unwrap_or(after.len());
        for (i, line) in after[..end].trim().lines().enumerate() {
            let line = line.trim();
            if line.len() >= 10 && i < 64 {
                let flag = u8::from_str_radix(&line[0..2], 16).unwrap_or(0);
                let ch0 = u8::from_str_radix(&line[3..5], 16).unwrap_or(0);
                let ch1 = u8::from_str_radix(&line[5..7], 16).unwrap_or(0);
                let ch2 = u8::from_str_radix(&line[7..9], 16).unwrap_or(0);
                let ch3 = u8::from_str_radix(&line[9..11], 16).unwrap_or(0);
                music_buf[i * 4] = ch0 | ((flag & 1) << 7);
                music_buf[i * 4 + 1] = ch1 | ((flag & 2) << 6);
                music_buf[i * 4 + 2] = ch2 | ((flag & 4) << 5);
                music_buf[i * 4 + 3] = ch3 | ((flag & 8) << 4);
            }
        }
    }

    Some((sfx_buf, music_buf))
}

/// Convert 68*64 bytes to __sfx__ section hex format.
pub fn bytes_to_sfx_hex(data: &[u8]) -> String {
    let mut padded = data.to_vec();
    padded.resize(68 * 64, 0);
    let mut lines = Vec::new();
    for slot in 0..64 {
        let d = &padded[slot * 68..(slot + 1) * 68];
        let header = format!("{:02x}{:02x}{:02x}{:02x}", d[64], d[65], d[66], d[67]);
        let mut notes = String::new();
        for n in 0..32 {
            let (b0, b1) = (d[2 * n], d[2 * n + 1]);
            let pitch = b0 & 0x3F;
            let wf = ((b0 >> 6) & 0x3) | ((b1 & 0x1) << 2);
            let custom = (b1 >> 7) & 0x1;
            let vol = (b1 >> 1) & 0x7;
            let eff = (b1 >> 4) & 0x7;
            let wf_hex = wf | (custom << 3);
            notes.push_str(&format!("{:02x}{:1x}{:1x}{:1x}", pitch, wf_hex, vol, eff));
        }
        lines.push(format!("{}{}", header, notes));
    }
    lines.join("\n")
}

/// Convert 256 bytes of music data to __music__ section hex.
pub fn music_hex(music_buf: &[u8]) -> String {
    let mut lines = Vec::new();
    for i in 0..64 {
        let b = &music_buf[i * 4..(i + 1) * 4];
        let flag = ((b[0] >> 7) & 1) | (((b[1] >> 7) & 1) << 1) | (((b[2] >> 7) & 1) << 2) | (((b[3] >> 7) & 1) << 3);
        let ch0 = b[0] & 0x7F;
        let ch1 = b[1] & 0x7F;
        let ch2 = b[2] & 0x7F;
        let ch3 = b[3] & 0x7F;
        lines.push(format!("{:02x} {:02x}{:02x}{:02x}{:02x}", flag, ch0, ch1, ch2, ch3));
    }
    lines.join("\n")
}

/// Convert bytes to __gfx__ hex format (128 rows of 128 hex chars).
pub fn bytes_to_gfx(data: &[u8]) -> String {
    let row_bytes = 64;
    let total_rows = 128;
    let mut padded = data.to_vec();
    padded.resize(total_rows * row_bytes, 0);
    let mut lines = Vec::new();
    for row in 0..total_rows {
        let row_data = &padded[row * row_bytes..(row + 1) * row_bytes];
        let mut hex_str = String::new();
        for &b in row_data {
            let lo = b & 0x0F;
            let hi = (b >> 4) & 0x0F;
            hex_str.push_str(&format!("{:x}{:x}", lo, hi));
        }
        lines.push(hex_str);
    }
    lines.join("\n")
}

/// Convert bytes to __map__ hex format (32 rows of 256 hex chars).
pub fn bytes_to_map_hex(data: &[u8]) -> String {
    let mut padded = data.to_vec();
    padded.resize(4096, 0);
    let row_bytes = 128;
    let mut lines = Vec::new();
    for row in 0..32 {
        let row_data = &padded[row * row_bytes..(row + 1) * row_bytes];
        lines.push(row_data.iter().map(|b| format!("{:02x}", b)).collect::<String>());
    }
    lines.join("\n")
}

/// Convert bytes to __gff__ hex format (2 rows of 256 hex chars).
pub fn bytes_to_gff_hex(data: &[u8]) -> String {
    let mut padded = data.to_vec();
    padded.resize(256, 0);
    let mut lines = Vec::new();
    for row in 0..2 {
        let row_data = &padded[row * 128..(row + 1) * 128];
        lines.push(row_data.iter().map(|b| format!("{:02x}", b)).collect::<String>());
    }
    lines.join("\n")
}
