//! HTML export: parse pico8.dat, convert .p8 to ROM, compress code, emit HTML+JS.
//! Based on shrinko8's approach (https://github.com/thisismypassport/shrinko8).

use std::io::Cursor;
use std::path::Path;

// ── ROM layout constants ──
const K_MEM_SPRITES: usize = 0x0000;
const K_MEM_MAP: usize = 0x2000;
const K_MEM_FLAG: usize = 0x3000;
const K_MEM_MUSIC: usize = 0x3100;
const K_MEM_SFX: usize = 0x3200;
const K_ROM_SIZE: usize = 0x4300;
const K_CART_SIZE: usize = 0x8000;
const K_CODE_SIZE: usize = 0x3D00;

// ── LZ4 decompression (for pico8.dat POD entries) ──
fn lz4_decompress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let header = data[pos]; pos += 1;
        // Literal length
        let mut lit_len = (header >> 4) as usize;
        if lit_len == 0xf {
            loop {
                let val = data[pos] as usize; pos += 1;
                lit_len += val;
                if val != 0xff { break; }
            }
        }
        out.extend_from_slice(&data[pos..pos + lit_len]);
        pos += lit_len;
        if pos >= data.len() { break; }
        // Match
        let offset = data[pos] as usize | ((data[pos + 1] as usize) << 8);
        pos += 2;
        let mut match_len = 4 + (header & 0xf) as usize;
        if match_len == 0x13 {
            loop {
                let val = data[pos] as usize; pos += 1;
                match_len += val;
                if val != 0xff { break; }
            }
        }
        for _ in 0..match_len {
            let b = out[out.len() - offset];
            out.push(b);
        }
    }
    out
}

// ── POD file parser ──
struct PodEntry {
    name: String,
    content: Vec<u8>,
}

fn parse_pod(data: &[u8]) -> Vec<PodEntry> {
    let mut entries = Vec::new();
    let mut pos = 0usize;

    // Header: "CPOD" + u32 header_size + u32 version + 0x20 name + u32 count + 0x1c junk
    assert!(&data[0..4] == b"CPOD", "Not a POD file");
    pos = 4 + 4 + 4 + 0x20; // skip to count
    let count = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
    pos += 4 + 0x1c; // skip count + junk

    for _ in 0..count {
        let header = &data[pos..pos + 4];
        match header {
            b"CFIL" | b"cFIL" => {
                let _zero = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap());
                let size = u32::from_le_bytes(data[pos + 8..pos + 12].try_into().unwrap()) as usize;
                let name_bytes = &data[pos + 12..pos + 12 + 0x40];
                let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(0x40);
                let name = String::from_utf8_lossy(&name_bytes[..name_end]).to_string();

                if header == b"cFIL" {
                    let comp_size = u32::from_le_bytes(data[pos + 12 + 0x40..pos + 12 + 0x40 + 4].try_into().unwrap()) as usize;
                    let comp_data = &data[pos + 12 + 0x40 + 4..pos + 12 + 0x40 + 4 + comp_size];
                    let content = lz4_decompress(comp_data);
                    entries.push(PodEntry { name, content });
                    pos = pos + 12 + 0x40 + 4 + comp_size;
                } else {
                    let content = data[pos + 12 + 0x40..pos + 12 + 0x40 + size].to_vec();
                    entries.push(PodEntry { name, content });
                    pos = pos + 12 + 0x40 + size;
                }
            }
            b"CBMP" | b"cBMP" => {
                let size = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize;
                if header == b"cBMP" {
                    let comp_size = u32::from_le_bytes(data[pos + 24..pos + 28].try_into().unwrap()) as usize;
                    entries.push(PodEntry { name: String::new(), content: vec![] });
                    pos = pos + 28 + comp_size;
                } else {
                    entries.push(PodEntry { name: String::new(), content: vec![] });
                    pos = pos + 4 + size;
                }
            }
            b"CPAL" => {
                let size = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap()) as usize;
                entries.push(PodEntry { name: String::new(), content: vec![] });
                pos = pos + 8 + size;
            }
            _ => panic!("Unknown POD entry type: {:?}", std::str::from_utf8(header)),
        }
    }
    entries
}

fn pod_find(entries: &[PodEntry], name: &str) -> Vec<u8> {
    entries.iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("POD entry '{}' not found", name))
        .content.clone()
}

// ── .p8 to ROM conversion ──

/// Parse a hex character to its 4-bit value.
fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

/// Parse a .p8 file into a 32KB ROM + Lua code string.
pub fn p8_to_rom(p8_text: &str) -> (Vec<u8>, String) {
    let mut rom = vec![0u8; K_ROM_SIZE];
    let mut code = String::new();
    let mut section = "";
    let mut y = 0usize;

    for line in p8_text.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with("__") && trimmed.ends_with("__") {
            section = &trimmed[2..trimmed.len() - 2];
            y = 0;
            continue;
        }

        match section {
            "lua" => {
                if !code.is_empty() { code.push('\n'); }
                code.push_str(line);
            }
            "gfx" => {
                if y < 128 {
                    let bytes = trimmed.as_bytes();
                    for x in 0..std::cmp::min(128, bytes.len()) {
                        let val = hex_val(bytes[x]);
                        let addr = K_MEM_SPRITES + y * 64 + (x >> 1);
                        if x & 1 == 0 {
                            rom[addr] = (rom[addr] & 0xf0) | val;
                        } else {
                            rom[addr] = (rom[addr] & 0x0f) | (val << 4);
                        }
                    }
                    y += 1;
                }
            }
            "map" => {
                if y < 32 {
                    let bytes = trimmed.as_bytes();
                    let mut x = 0;
                    let mut i = 0;
                    while i + 1 < bytes.len() && x < 128 {
                        let val = (hex_val(bytes[i]) << 4) | hex_val(bytes[i + 1]);
                        rom[K_MEM_MAP + y * 128 + x] = val;
                        x += 1;
                        i += 2;
                    }
                    y += 1;
                }
            }
            "gff" => {
                if y < 2 {
                    let bytes = trimmed.as_bytes();
                    let mut x = 0;
                    let mut i = 0;
                    while i + 1 < bytes.len() && x < 128 {
                        let val = (hex_val(bytes[i]) << 4) | hex_val(bytes[i + 1]);
                        rom[K_MEM_FLAG + y * 128 + x] = val;
                        x += 1;
                        i += 2;
                    }
                    y += 1;
                }
            }
            "music" => {
                if y < 64 {
                    let bytes = trimmed.as_bytes();
                    if bytes.len() >= 11 {
                        // First 2 hex chars = flags byte
                        let flags = (hex_val(bytes[0]) << 4) | hex_val(bytes[1]);
                        // Skip space at [2], then 4 channel bytes (2 hex each, space-separated)
                        let mut ch_i = 0;
                        let mut bi = 3;
                        while ch_i < 4 && bi + 1 < bytes.len() {
                            if bytes[bi] == b' ' { bi += 1; continue; }
                            let val = (hex_val(bytes[bi]) << 4) | hex_val(bytes[bi + 1]);
                            // Set bit 7 from flags
                            let val = val | (((flags >> ch_i) & 1) << 7);
                            rom[K_MEM_MUSIC + y * 4 + ch_i] = val;
                            ch_i += 1;
                            bi += 2;
                        }
                    }
                    y += 1;
                }
            }
            "sfx" => {
                if y < 64 {
                    let bytes = trimmed.as_bytes();
                    if bytes.len() >= 168 { // 8 + 32*5 = 168
                        // First 8 hex chars = 4 info bytes (editor mode, speed, loop start, loop end)
                        for i in 0..4 {
                            let val = (hex_val(bytes[i * 2]) << 4) | hex_val(bytes[i * 2 + 1]);
                            // SFX info is at the end of the SFX block: sfx_addr + note_count*2
                            let addr = K_MEM_SFX + y * 68 + 64 + i;
                            rom[addr] = val;
                        }
                        // 32 notes, each 5 hex nybbles
                        for note in 0..32 {
                            let base = 8 + note * 5;
                            let n0 = hex_val(bytes[base]) as u16;     // pitch high
                            let n1 = hex_val(bytes[base + 1]) as u16; // pitch low
                            let n2 = hex_val(bytes[base + 2]) as u16; // waveform
                            let n3 = hex_val(bytes[base + 3]) as u16; // volume
                            let n4 = hex_val(bytes[base + 4]) as u16; // effect
                            // Pack into 16-bit: pitch[5:0] | waveform[2:0] << 6 | volume[2:0] << 9 | effect[2:0] << 12 | waveform[3] << 15
                            let pitch = n1 | ((n0 & 0x3) << 4);
                            let value = pitch
                                | ((n2 & 0x7) << 6)
                                | ((n3 & 0x7) << 9)
                                | ((n4 & 0x7) << 12)
                                | ((n2 & 0x8) << 12); // waveform bit 3 -> bit 15
                            let addr = K_MEM_SFX + y * 68 + note * 2;
                            rom[addr] = (value & 0xff) as u8;
                            rom[addr + 1] = ((value >> 8) & 0xff) as u8;
                        }
                    }
                    y += 1;
                }
            }
            "label" => {
                // Label is stored after the main ROM sections but we skip it for ROM
                // (it's only needed for the thumbnail, which we handle separately)
            }
            _ => {}
        }
    }

    (rom, code)
}

// ── PICO-8 code compression (new format: 0pxa) ──

struct BitWriter {
    buf: Vec<u8>,
    bits: u32,
    nbits: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self { buf: Vec::new(), bits: 0, nbits: 0 }
    }

    fn bit(&mut self, v: u32) {
        self.bits |= (v & 1) << self.nbits;
        self.nbits += 1;
        if self.nbits == 8 {
            self.buf.push(self.bits as u8);
            self.bits = 0;
            self.nbits = 0;
        }
    }

    fn bits(&mut self, n: u32, v: u32) {
        for i in 0..n {
            self.bit((v >> i) & 1);
        }
    }

    fn flush(&mut self) {
        if self.nbits > 0 {
            self.buf.push(self.bits as u8);
            self.bits = 0;
            self.nbits = 0;
        }
    }
}

fn update_mtf(mtf: &mut [u8; 256], idx: usize, ch: u8) {
    for i in (1..=idx).rev() {
        mtf[i] = mtf[i - 1];
    }
    mtf[0] = ch;
}

fn mtf_index(mtf: &[u8; 256], ch: u8) -> usize {
    mtf.iter().position(|&c| c == ch).unwrap()
}

/// Find longest match at position `i` looking back up to `max_offset`.
fn find_match(code: &[u8], i: usize, min_c: usize, max_offset: usize) -> (usize, usize) {
    let mut best_c = 0usize;
    let mut best_j = 0usize;
    let start = if i > max_offset { i - max_offset } else { 0 };

    for j in start..i {
        if code[j] != code[i] { continue; }
        let mut c = 0;
        let limit = std::cmp::min(code.len() - i, i - j); // no_repeat: c <= i-j
        let limit = std::cmp::min(limit, code.len() - i);
        while c < limit && code[j + c] == code[i + c] {
            c += 1;
        }
        if c >= min_c && c > best_c {
            best_c = c;
            best_j = j;
        }
    }
    (best_c, best_j)
}

/// Compress Lua code using PICO-8's new format (0pxa header).
/// Uses a simplified fast compression (no optimal parsing).
fn compress_code(code: &str) -> Vec<u8> {
    let code_bytes = code.as_bytes();
    let min_c = 3usize;

    // If code fits uncompressed, just store it raw with null terminator
    if code_bytes.len() < K_CODE_SIZE {
        let mut out = Vec::with_capacity(code_bytes.len() + 1);
        out.extend_from_slice(code_bytes);
        out.push(0);
        return out;
    }

    // Compressed format
    let mut result = Vec::new();
    // Header: "\0pxa"
    result.extend_from_slice(b"\0pxa");
    // Uncompressed size (u16 BE)
    result.push(((code_bytes.len() >> 8) & 0xff) as u8);
    result.push((code_bytes.len() & 0xff) as u8);
    // Compressed size placeholder (u16 BE) - we'll fill this in later
    let len_pos = result.len();
    result.push(0);
    result.push(0);

    let mut bw = BitWriter::new();
    let mut mtf = [0u8; 256];
    for i in 0..256 { mtf[i] = i as u8; }

    let mut i = 0;
    while i < code_bytes.len() {
        let (best_c, best_j) = find_match(code_bytes, i, min_c, 0x7fff);

        if best_c >= min_c {
            // Write match
            bw.bit(0);
            let offset_val = (i - best_j - 1) as u32;
            let offset_bits = if offset_val < (1 << 5) {
                5
            } else if offset_val < (1 << 10) {
                10
            } else {
                15
            };
            bw.bit(if offset_bits < 15 { 1 } else { 0 });
            if offset_bits < 15 {
                bw.bit(if offset_bits < 10 { 1 } else { 0 });
            }
            bw.bits(offset_bits, offset_val);

            let mut count_val = (best_c - min_c) as u32;
            while count_val >= 7 {
                bw.bits(3, 7);
                count_val -= 7;
            }
            bw.bits(3, count_val);

            i += best_c;
        } else {
            // Write literal
            bw.bit(1);
            let ch = code_bytes[i];
            let ch_i = mtf_index(&mtf, ch);

            let mut i_val = ch_i as u32;
            let mut i_bits = 4u32;
            while i_val >= (1 << i_bits) {
                bw.bit(1);
                i_val -= 1 << i_bits;
                i_bits += 1;
            }
            bw.bit(0);
            bw.bits(i_bits, i_val);

            update_mtf(&mut mtf, ch_i, ch);
            i += 1;
        }
    }
    bw.flush();
    result.extend_from_slice(&bw.buf);

    // Fill in compressed size (BE)
    let total_size = result.len();
    result[len_pos] = ((total_size >> 8) & 0xff) as u8;
    result[len_pos + 1] = (total_size & 0xff) as u8;

    result
}

// ── Base64 ──

fn b64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 { out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(CHARS[(triple & 0x3F) as usize] as char); } else { out.push('='); }
    }
    out
}

// ── Label image ──

const P8_PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],       [29, 43, 83],    [126, 37, 83],   [0, 135, 81],
    [171, 82, 54],   [95, 87, 79],    [194, 195, 199], [255, 241, 232],
    [255, 0, 77],    [255, 163, 0],   [255, 236, 39],  [0, 228, 54],
    [41, 173, 255],  [131, 118, 156], [255, 119, 168], [255, 204, 170],
];

/// Extract __label__ from .p8 text and encode as a base64 PNG data URI.
fn label_to_data_uri(p8_text: &str) -> Option<String> {
    let start = p8_text.find("__label__\n")?;
    let after = &p8_text[start + 10..];
    let end = after.find("\n__").unwrap_or(after.len());
    let lines: Vec<&str> = after[..end].lines().filter(|l| !l.is_empty()).collect();
    if lines.len() < 128 { return None; }

    let mut img = image::RgbaImage::new(128, 128);
    for (y, line) in lines.iter().take(128).enumerate() {
        for (x, ch) in line.chars().take(128).enumerate() {
            let idx = ch.to_digit(16).unwrap_or(0) as usize;
            let [r, g, b] = P8_PALETTE[idx & 0xf];
            img.put_pixel(x as u32, y as u32, image::Rgba([r, g, b, 255]));
        }
    }

    let mut png_buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(Cursor::new(&mut png_buf));
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        128, 128,
        image::ExtendedColorType::Rgba8,
    ).ok()?;

    let b64 = b64_encode(&png_buf);
    Some(format!("data:image/png;base64,{}", b64))
}

// ── Main export function ──

/// Extract the PICO-8 web player from pico8.dat and create HTML+JS export files.
pub fn export_html(
    pico8_dat_path: &Path,
    p8_cart_path: &Path,
    output_dir: &Path,
    basename: &str,
) -> Result<(), String> {
    // 1. Read pico8.dat and extract f_html5.pod
    let dat = std::fs::read(pico8_dat_path)
        .map_err(|e| format!("Failed to read pico8.dat: {}", e))?;
    let outer_entries = parse_pod(&dat);
    let html5_pod = pod_find(&outer_entries, "pod/f_html5.pod");

    // 2. Parse inner pod to get pico8.js and shell.html
    let inner_entries = parse_pod(&html5_pod);
    let js_player = pod_find(&inner_entries, "src/pico8.js");
    let shell_html = pod_find(&inner_entries, "src/shell.html");

    let js_player_str = String::from_utf8(js_player)
        .map_err(|e| format!("pico8.js is not valid UTF-8: {}", e))?;
    let shell_html_str = String::from_utf8(shell_html)
        .map_err(|e| format!("shell.html is not valid UTF-8: {}", e))?;

    // 3. Read .p8 cart and convert to ROM
    let p8_text = std::fs::read_to_string(p8_cart_path)
        .map_err(|e| format!("Failed to read cart: {}", e))?;
    let (rom, lua_code) = p8_to_rom(&p8_text);

    // 4. Compress Lua code and build full cart binary
    let compressed_code = compress_code(&lua_code);
    let mut cart = vec![0u8; K_CART_SIZE];
    cart[..K_ROM_SIZE].copy_from_slice(&rom);
    let code_end = K_ROM_SIZE + compressed_code.len();
    if code_end > K_CART_SIZE {
        return Err(format!("Compressed code too large: {} bytes (max {})", compressed_code.len(), K_CODE_SIZE));
    }
    cart[K_ROM_SIZE..code_end].copy_from_slice(&compressed_code);

    // 5. Build JS file with embedded cart data
    let cart_name = basename;
    let cartnames_str = format!("`{}`", cart_name);
    let cartdat_str = {
        let mut chunks = Vec::new();
        for chunk in cart.chunks(256) {
            chunks.push(chunk.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(","));
        }
        format!("\n{}", chunks.join(",\n"))
    };

    let js_content = format!(
        "var _cartname=[{}];\nvar _cdpos=0; var iii=0; var ciii=0;\nvar _cartdat=[{}];\n\n{}",
        cartnames_str, cartdat_str, js_player_str
    );

    // 6. Build HTML file from shell template
    let label_uri = label_to_data_uri(&p8_text)
        .unwrap_or_else(|| "data:image/png;base64,".to_string());
    let js_filename = format!("{}.js", basename);
    let html_content = shell_html_str
        .replace("##js_file##", &js_filename)
        .replace("##label_file##", &label_uri);

    // 7. Write output files
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let js_path = output_dir.join(&js_filename);
    let html_path = output_dir.join(format!("{}.html", basename));

    std::fs::write(&js_path, &js_content)
        .map_err(|e| format!("Failed to write {}: {}", js_path.display(), e))?;
    std::fs::write(&html_path, &html_content)
        .map_err(|e| format!("Failed to write {}: {}", html_path.display(), e))?;

    eprintln!("  {} ({:.0}KB)", js_path.display(), js_content.len() as f64 / 1024.0);
    eprintln!("  {} ({:.0}KB)", html_path.display(), html_content.len() as f64 / 1024.0);

    Ok(())
}
