/// Frame extraction from PNG sprite sheets and helper functions.

use image::{GenericImageView, Pixel};
use std::path::Path;

use crate::config::{P8_PALETTE, TRANS};

/// Find nearest PICO-8 color (excluding transparent).
pub fn nearest_p8(r: u8, g: u8, b: u8) -> u8 {
    let mut best_i = 0u8;
    let mut best_d = u32::MAX;
    for (i, &(pr, pg, pb)) in P8_PALETTE.iter().enumerate() {
        if i as u8 == TRANS {
            continue;
        }
        let dr = r as i32 - pr as i32;
        let dg = g as i32 - pg as i32;
        let db = b as i32 - pb as i32;
        let d = (dr * dr + dg * dg + db * db) as u32;
        if d < best_d {
            best_d = d;
            best_i = i as u8;
        }
    }
    best_i
}

/// Extract frames from a vertical strip PNG with given cell dimensions.
pub fn extract_frames(img_path: &Path, cell_w: u32, cell_h: u32, nframes: Option<u32>) -> Vec<Vec<u8>> {
    let img = image::open(img_path)
        .unwrap_or_else(|e| panic!("Failed to open {}: {}", img_path.display(), e));
    let (_w, h) = img.dimensions();
    let nf = nframes.unwrap_or(h / cell_h);
    let mut frames = Vec::new();
    for f in 0..nf {
        let y0 = f * cell_h;
        let mut pixels = Vec::with_capacity((cell_w * cell_h) as usize);
        for y in 0..cell_h {
            for x in 0..cell_w {
                let px = img.get_pixel(x, y0 + y);
                let channels = px.channels();
                let (r, g, b, a) = (channels[0], channels[1], channels[2], channels[3]);
                if a == 0 {
                    pixels.push(TRANS);
                } else {
                    pixels.push(nearest_p8(r, g, b));
                }
            }
        }
        frames.push(pixels);
    }
    frames
}

/// Extract frames from a horizontal strip PNG.
pub fn extract_horiz_frames(
    img_path: &Path,
    src_fw: u32,
    src_fh: u32,
    cell_w: u32,
    cell_h: u32,
    nframes: Option<u32>,
    pad_x: u32,
) -> Vec<Vec<u8>> {
    let img = image::open(img_path)
        .unwrap_or_else(|e| panic!("Failed to open {}: {}", img_path.display(), e));
    let nf = nframes.unwrap_or(img.dimensions().0 / src_fw);
    let mut frames = Vec::new();
    for f in 0..nf {
        let x0 = f * src_fw;
        let mut pixels = vec![TRANS; (cell_w * cell_h) as usize];
        for y in 0..src_fh.min(cell_h) {
            for x in 0..src_fw {
                let dx = x + pad_x;
                if dx < cell_w {
                    let px = img.get_pixel(x0 + x, y);
                    let ch = px.channels();
                    let (r, g, b, a) = (ch[0], ch[1], ch[2], ch[3]);
                    if a < 128 {
                        pixels[(y * cell_w + dx) as usize] = TRANS;
                    } else {
                        pixels[(y * cell_w + dx) as usize] = nearest_p8(r, g, b);
                    }
                }
            }
        }
        frames.push(pixels);
    }
    frames
}

/// Extract frames from a vertical strip, centering content into target cells.
/// Content is bottom-center aligned (characters stand on ground).
pub fn extract_frames_boss(
    img_path: &Path,
    src_fw: u32,
    src_fh: u32,
    target_w: u32,
    target_h: u32,
    frame_select: Option<&[usize]>,
) -> Vec<Vec<u8>> {
    let img = image::open(img_path)
        .unwrap_or_else(|e| panic!("Failed to open {}: {}", img_path.display(), e));
    let (_w, h) = img.dimensions();
    let total_frames = h / src_fh;

    let indices: Vec<usize> = frame_select
        .map(|fs| fs.to_vec())
        .unwrap_or_else(|| (0..total_frames as usize).collect());

    let mut frames = Vec::new();
    for &fi in &indices {
        if fi >= total_frames as usize {
            continue;
        }
        let y0 = fi as u32 * src_fh;

        // Find content bbox
        let mut min_x = src_fw as i32;
        let mut min_y = src_fh as i32;
        let mut max_x: i32 = -1;
        let mut max_y: i32 = -1;
        for y in 0..src_fh {
            for x in 0..src_fw {
                let px = img.get_pixel(x, y0 + y);
                if px.channels()[3] > 0 {
                    min_x = min_x.min(x as i32);
                    max_x = max_x.max(x as i32);
                    min_y = min_y.min(y as i32);
                    max_y = max_y.max(y as i32);
                }
            }
        }

        if max_x < 0 {
            // Empty frame
            frames.push(vec![TRANS; (target_w * target_h) as usize]);
            continue;
        }

        let content_w = (max_x - min_x + 1) as u32;
        let content_h = (max_y - min_y + 1) as u32;
        let crop_w = content_w.min(target_w);
        let crop_h = content_h.min(target_h);

        // Center horizontally, bottom-align vertically
        let dst_x = (target_w - crop_w) / 2;
        let dst_y = target_h - crop_h;

        let src_cx = min_x + content_w as i32 / 2;
        let src_x0 = src_cx - crop_w as i32 / 2;
        let src_y0 = min_y + content_h as i32 - crop_h as i32;

        let mut pixels = vec![TRANS; (target_w * target_h) as usize];
        for dy in 0..crop_h {
            for dx in 0..crop_w {
                let sx = src_x0 + dx as i32;
                let sy = y0 as i32 + src_y0 + dy as i32;
                if sx >= 0 && (sx as u32) < src_fw && sy >= y0 as i32 && sy < (y0 + src_fh) as i32
                {
                    let px = img.get_pixel(sx as u32, sy as u32);
                    let ch = px.channels();
                    if ch[3] > 0 {
                        pixels[((dst_y + dy) * target_w + (dst_x + dx)) as usize] =
                            nearest_p8(ch[0], ch[1], ch[2]);
                    }
                }
            }
        }
        frames.push(pixels);
    }
    frames
}

/// Extract font from PNG sprite sheet.
/// Returns (frames, cell_w, cell_h, advances).
pub fn extract_font_from_png(png_path: &Path, chars: &str, threshold: u8) -> (Vec<Vec<u8>>, u32, u32, Vec<u8>) {
    let img = image::open(png_path)
        .unwrap_or_else(|e| panic!("Failed to open font {}: {}", png_path.display(), e));
    let n = chars.chars().count() as u32;
    let cell_w = img.dimensions().0 / n;
    let cell_h = img.dimensions().1;
    let mut frames = Vec::new();
    let mut advances = Vec::new();

    for i in 0..n {
        let ox = i * cell_w;
        let mut pixels = Vec::with_capacity((cell_w * cell_h) as usize);
        let mut max_x: u32 = 0;
        for y in 0..cell_h {
            for x in 0..cell_w {
                let px = img.get_pixel(ox + x, y);
                let ch = px.channels();
                if ch[0] >= threshold {
                    pixels.push(7); // white
                    max_x = max_x.max(x);
                } else {
                    pixels.push(TRANS);
                }
            }
        }
        frames.push(pixels);
        advances.push((max_x + 2).min(255) as u8);
    }

    (frames, cell_w, cell_h, advances)
}

/// Get animation-wide bounding box (all non-transparent pixels across all frames).
pub fn get_bbox(frames: &[Vec<u8>], fw: u32, fh: u32) -> (u8, u8, u8, u8) {
    let mut min_x = fw as i32 - 1;
    let mut min_y = fh as i32 - 1;
    let mut max_x: i32 = 0;
    let mut max_y: i32 = 0;
    for f in frames {
        for (idx, &c) in f.iter().enumerate() {
            if c != TRANS {
                let x = idx as i32 % fw as i32;
                let y = idx as i32 / fw as i32;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    (
        min_x as u8,
        min_y as u8,
        (max_x - min_x + 1) as u8,
        (max_y - min_y + 1) as u8,
    )
}

/// Get per-frame bounding box.
pub fn get_frame_bbox(f: &[u8], fw: u32, fh: u32) -> (u8, u8, u8, u8) {
    let mut min_x = fw as i32 - 1;
    let mut min_y = fh as i32 - 1;
    let mut max_x: i32 = -1;
    let mut max_y: i32 = -1;
    for (idx, &c) in f.iter().enumerate() {
        if c != TRANS {
            let x = idx as i32 % fw as i32;
            let y = idx as i32 / fw as i32;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if max_x < 0 {
        return (0, 0, 0, 0);
    }
    (
        min_x as u8,
        min_y as u8,
        (max_x - min_x + 1) as u8,
        (max_y - min_y + 1) as u8,
    )
}

/// Crop pixels from frame at (bx, by) with size (bw, bh).
pub fn crop_pixels(f: &[u8], fw: u32, bx: u8, by: u8, bw: u8, bh: u8) -> Vec<u8> {
    let mut cropped = Vec::with_capacity(bw as usize * bh as usize);
    for y in by..by + bh {
        for x in bx..bx + bw {
            cropped.push(f[y as usize * fw as usize + x as usize]);
        }
    }
    cropped
}

/// Determine minimum bpp needed to represent all colors in frames.
pub fn min_bpp_for_frames(frames: &[Vec<u8>]) -> u8 {
    let mut colors = std::collections::HashSet::new();
    for f in frames {
        for &c in f {
            colors.insert(c);
        }
    }
    let n = colors.len();
    if n <= 2 {
        1
    } else if n <= 4 {
        2
    } else if n <= 8 {
        3
    } else {
        4
    }
}

/// Build minimal palette of size 2^bpp. TRANS at index 0.
pub fn build_palette(frames: &[Vec<u8>], bpp: u8) -> Vec<u8> {
    let mut colors = std::collections::HashSet::new();
    for f in frames {
        for &c in f {
            colors.insert(c);
        }
    }
    let mut pal = Vec::new();
    if colors.contains(&TRANS) {
        pal.push(TRANS);
    }
    colors.remove(&TRANS);
    let mut sorted: Vec<u8> = colors.into_iter().collect();
    sorted.sort();
    pal.extend(sorted);
    let size = 1usize << bpp;
    while pal.len() < size {
        pal.push(0);
    }
    pal.truncate(size);
    pal
}

/// Pack palette entries as nibbles into bytes.
pub fn pack_palette(palette: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    for i in (0..palette.len()).step_by(2) {
        let lo = palette[i] & 0xF;
        let hi = if i + 1 < palette.len() {
            palette[i + 1] & 0xF
        } else {
            0
        };
        data.push((lo << 4) | hi);
    }
    data
}

/// Map P8 color indices to palette indices.
pub fn quantize_pixels(pixels: &[u8], palette: &[u8]) -> Vec<u8> {
    pixels
        .iter()
        .map(|&p| {
            palette
                .iter()
                .position(|&pc| p == pc)
                .unwrap_or(0) as u8
        })
        .collect()
}

/// Count pixels that differ between two frames.
pub fn count_diffs(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

/// Compute per-frame horizontal centers of non-transparent body pixels.
pub fn compute_anchors(frames: &[Vec<u8>], fw: u32, body_color: Option<u8>) -> Vec<u8> {
    frames
        .iter()
        .map(|f| {
            let xs: Vec<u32> = f
                .iter()
                .enumerate()
                .filter(|(_, &c)| {
                    c != TRANS
                        && match body_color {
                            Some(bc) => c == bc,
                            None => true,
                        }
                })
                .map(|(idx, _)| idx as u32 % fw)
                .collect();
            if xs.is_empty() {
                (fw / 2) as u8
            } else {
                let min_x = *xs.iter().min().unwrap();
                let max_x = *xs.iter().max().unwrap();
                ((min_x + max_x) / 2) as u8
            }
        })
        .collect()
}
