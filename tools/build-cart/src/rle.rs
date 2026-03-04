/// RLE encoding variants for animation compression.

/// Basic nibble RLE: byte = (color << 4) | (run-1), max run 16.
pub fn nibble_rle_encode(pixels: &[u8]) -> Vec<u8> {
    if pixels.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    let mut cur_color = pixels[0];
    let mut cur_count: u32 = 1;

    let mut emit = |out: &mut Vec<u8>, color: u8, mut count: u32| {
        while count > 0 {
            let run = count.min(16);
            out.push((color << 4) | (run as u8 - 1));
            count -= run;
        }
    };

    for &p in &pixels[1..] {
        if p == cur_color && cur_count < 16 {
            cur_count += 1;
        } else {
            emit(&mut out, cur_color, cur_count);
            cur_color = p;
            cur_count = 1;
        }
    }
    emit(&mut out, cur_color, cur_count);
    out
}

/// Extended bpp-aware RLE: byte = (color << run_bits) | (run-1).
/// run_bits = 8 - bpp. Escape when run-1 == run_mask.
pub fn ext_nibble_rle_encode(pixels: &[u8], bpp: u8) -> Vec<u8> {
    if pixels.is_empty() {
        return vec![];
    }
    let run_bits = 8 - bpp;
    let run_mask = (1u32 << run_bits) - 1;
    let mut out = Vec::new();
    let mut cur_color = pixels[0];
    let mut cur_count: u32 = 1;

    let emit = |out: &mut Vec<u8>, color: u8, mut count: u32| {
        while count > 0 {
            if count <= run_mask {
                out.push((color << run_bits) | (count as u8 - 1));
                count = 0;
            } else {
                let ext = ((count - (run_mask + 1)) as u8).min(255);
                out.push((color << run_bits) | run_mask as u8);
                out.push(ext);
                count -= run_mask + 1 + ext as u32;
            }
        }
    };

    for &p in &pixels[1..] {
        if p == cur_color {
            cur_count += 1;
        } else {
            emit(&mut out, cur_color, cur_count);
            cur_color = p;
            cur_count = 1;
        }
    }
    emit(&mut out, cur_color, cur_count);
    out
}

/// Delta encoding with skip compression.
/// Encodes only changed pixels between base and frame.
pub fn delta_encode_skip(base_pixels: &[u8], frame_pixels: &[u8]) -> Vec<u8> {
    let entries: Vec<(usize, u8)> = base_pixels
        .iter()
        .zip(frame_pixels.iter())
        .enumerate()
        .filter(|(_, (b, f))| b != f)
        .map(|(i, (_, &f))| (i, f))
        .collect();

    let mut out = Vec::new();
    if entries.len() >= 255 {
        out.push(0xFF);
        let n = entries.len();
        out.push((n & 0xFF) as u8);
        out.push(((n >> 8) & 0xFF) as u8);
    } else {
        out.push(entries.len() as u8);
    }

    if entries.is_empty() {
        return out;
    }

    // First entry: absolute position (u16 LE) + color
    out.push((entries[0].0 & 0xFF) as u8);
    out.push(((entries[0].0 >> 8) & 0xFF) as u8);
    out.push(entries[0].1);

    for j in 1..entries.len() {
        let gap = entries[j].0 - entries[j - 1].0;
        let color = entries[j].1;
        let skip = gap - 1;
        if skip <= 14 {
            out.push(((skip as u8) << 4) | color);
        } else if skip <= 14 + 255 {
            out.push((15 << 4) | color);
            out.push((skip - 15) as u8);
        } else {
            out.push((15 << 4) | color);
            out.push(0xFF);
            out.push((skip & 0xFF) as u8);
            out.push(((skip >> 8) & 0xFF) as u8);
        }
    }
    out
}

/// Pack 1bpp pixels into bytes (MSB first), trim trailing zero bytes.
pub fn pack_bits(pixels: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut bits: u8 = 0;
    let mut bit_count = 0;
    for &p in pixels {
        bits = (bits << 1) | (p & 1);
        bit_count += 1;
        if bit_count == 8 {
            out.push(bits);
            bits = 0;
            bit_count = 0;
        }
    }
    if bit_count > 0 {
        out.push(bits << (8 - bit_count));
    }
    // Trim trailing zeros
    while out.last() == Some(&0) {
        out.pop();
    }
    out
}

/// XOR each row with the row above (first row unchanged).
pub fn row_delta(pixels: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = pixels.to_vec();
    for y in (1..h).rev() {
        for x in 0..w {
            out[y * w + x] ^= pixels[(y - 1) * w + x];
        }
    }
    out
}
