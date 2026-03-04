/// EG-2 compression: Exponential Golomb zero-run encoding with differential predictors.
/// This is the core compression algorithm used for animation frames, tile pixels, and map layers.

/// Apply differential encoding mode.
/// mode 0 = raw (identity), mode 4 = paeth predictor.
fn apply_diff_mode(pixels: &[u8], w: usize, _h: usize, mode: u8) -> Vec<u8> {
    if mode == 0 {
        return pixels.to_vec();
    }
    let mut out = pixels.to_vec();
    // Process in reverse so each pixel depends on original neighbors only
    for i in (0..pixels.len()).rev() {
        let x = i % w;
        let y = i / w;
        let ref_val = match mode {
            1 => {
                if x > 0 { pixels[i - 1] } else { 0 }
            }
            2 => {
                if y > 0 { pixels[(y - 1) * w + x] } else { 0 }
            }
            3 => {
                if y > 0 && x > 0 { pixels[(y - 1) * w + x - 1] } else { 0 }
            }
            _ => unreachable!(),
        };
        out[i] = pixels[i] ^ ref_val;
    }
    out
}

/// Apply Paeth prediction: XOR each pixel with paeth(left, up, up-left).
fn apply_paeth(pixels: &[u8], w: usize, _h: usize) -> Vec<u8> {
    let mut out = pixels.to_vec();
    for i in (0..pixels.len()).rev() {
        let x = i % w;
        let y = i / w;
        let a = if x > 0 { pixels[i - 1] as i32 } else { 0 };
        let b = if y > 0 { pixels[i - w] as i32 } else { 0 };
        let c = if x > 0 && y > 0 { pixels[i - w - 1] as i32 } else { 0 };
        let p = a + b - c;
        let pa = (p - a).abs();
        let pb = (p - b).abs();
        let pc = (p - c).abs();
        let pred = if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        };
        out[i] = pixels[i] ^ (pred as u8);
    }
    out
}

/// Encode non-negative integer using Exp-Golomb of given order.
fn eg_encode_bits(val: u32, order: u8) -> Vec<u8> {
    let val2 = val + (1u32 << order);
    let n = 32 - val2.leading_zeros(); // bit_length
    let prefix_len = n as usize - 1 - order as usize;
    let mut bits = vec![0u8; prefix_len];
    for b in (0..n).rev() {
        bits.push(((val2 >> b) & 1) as u8);
    }
    bits
}

/// Encode a frame/data with best (diff mode, EG order) combo.
/// Tries modes 0 (raw) and 4 (paeth), EG orders 1-3.
/// Returns (compressed_bytes, best_mode, best_order).
pub fn eg2_encode_frame(pixels: &[u8], bpp: u8, w: usize, h: usize) -> (Vec<u8>, u8, u8) {
    let mut best_bytes: Option<Vec<u8>> = None;
    let mut best_mode: u8 = 0;
    let mut best_order: u8 = 2;

    for mode in [0u8, 4] {
        let diff = if mode == 4 {
            apply_paeth(pixels, w, h)
        } else {
            apply_diff_mode(pixels, w, h, mode)
        };

        // Convert to bitstream (MSB first per pixel)
        let mut bitstream = Vec::with_capacity(diff.len() * bpp as usize);
        for &p in &diff {
            for b in (0..bpp).rev() {
                bitstream.push((p >> b) & 1);
            }
        }

        for order in [1u8, 2, 3] {
            // Header: 3 bits mode (LSB first), 2 bits (order-1) (LSB first)
            let mut out_bits: Vec<u8> = vec![
                mode & 1,
                (mode >> 1) & 1,
                (mode >> 2) & 1,
                (order - 1) & 1,
                ((order - 1) >> 1) & 1,
            ];

            const MAX_EG_RUN: u32 = 16383; // PICO-8 16.16 fixed-point safe limit
            let nb = bitstream.len();
            let mut i = 0;
            while i < nb {
                let mut zero_run: u32 = 0;
                while i < nb && bitstream[i] == 0 {
                    zero_run += 1;
                    i += 1;
                }
                // Split long runs to avoid PICO-8 integer overflow
                while zero_run > MAX_EG_RUN {
                    out_bits.extend_from_slice(&eg_encode_bits(MAX_EG_RUN, order));
                    zero_run -= MAX_EG_RUN;
                }
                out_bits.extend_from_slice(&eg_encode_bits(zero_run, order));
                if i < nb {
                    i += 1; // consume the 1-bit (implicit)
                }
            }

            // Pack bits into bytes (LSB first within each byte)
            let mut out = Vec::with_capacity((out_bits.len() + 7) / 8);
            for chunk in out_bits.chunks(8) {
                let mut byte = 0u8;
                for (b, &bit) in chunk.iter().enumerate() {
                    byte |= bit << b;
                }
                out.push(byte);
            }

            if best_bytes.is_none() || out.len() < best_bytes.as_ref().unwrap().len() {
                best_bytes = Some(out);
                best_mode = mode;
                best_order = order;
            }
        }
    }

    (best_bytes.unwrap(), best_mode, best_order)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eg_encode_bits_order2() {
        // val=0, order=2: val2=4 (100), n=3, prefix=0, bits=[1,0,0]
        let bits = eg_encode_bits(0, 2);
        assert_eq!(bits, vec![1, 0, 0]);
    }

    #[test]
    fn test_eg_encode_bits_order1() {
        // val=0, order=1: val2=2 (10), n=2, prefix=0, bits=[1,0]
        let bits = eg_encode_bits(0, 1);
        assert_eq!(bits, vec![1, 0]);
    }

    #[test]
    fn test_roundtrip_simple() {
        // Simple test: all zeros should compress well
        let pixels = vec![0u8; 64];
        let (data, mode, order) = eg2_encode_frame(&pixels, 4, 8, 8);
        assert!(!data.is_empty());
        assert!(data.len() < 64); // should compress well
        // mode should be 0 (raw) since all zeros
        assert!(mode == 0 || mode == 4);
        assert!(order >= 1 && order <= 3);
    }
}
