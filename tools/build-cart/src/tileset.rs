/// Tileset loading, slicing, color remapping, and deduplication.

use image::{DynamicImage, GenericImageView, Pixel};
use md5::{Digest, Md5};
use std::collections::HashMap;
use std::path::Path;

use crate::config::*;

/// Remap tile pixels (from PIL-style RGBA) to PICO-8 colors using luminance bands.
pub fn remap_tile_colors(tile_pixels: &[(u8, u8, u8, u8)], band_colors: &[u8; 5]) -> Vec<u8> {
    tile_pixels
        .iter()
        .map(|&(r, _g, _b, a)| {
            if a < 128 {
                TRANS
            } else {
                // Convert to luminance
                let lum = r; // tile is greyscale, so r == luminance
                let mut color = band_colors[4]; // default to last
                for (i, &(lo, hi)) in BAND_RANGES.iter().enumerate() {
                    if lum >= lo && lum <= hi {
                        color = band_colors[i];
                        break;
                    }
                }
                color
            }
        })
        .collect()
}

/// Remap using image luminance (converts to greyscale first).
/// If `invert_alpha` is true, transparent pixels become mid-grey and opaque pixels become transparent.
pub fn remap_tile_image(img: &DynamicImage, band_colors: &[u8; 5], invert_alpha: bool) -> Vec<u8> {
    let grey = img.to_luma_alpha8();
    let (w, h) = grey.dimensions();
    let mut result = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let px = grey.get_pixel(x, y);
            let lum = px[0];
            let alpha = px[1];
            let is_transparent = if invert_alpha { alpha >= 128 } else { alpha < 128 };
            if is_transparent {
                result.push(TRANS);
            } else {
                let actual_lum = if invert_alpha && alpha < 128 { 100 } else { lum };
                let mut color = band_colors[4];
                for (i, &(lo, hi)) in BAND_RANGES.iter().enumerate() {
                    if actual_lum >= lo && actual_lum <= hi {
                        color = band_colors[i];
                        break;
                    }
                }
                result.push(color);
            }
        }
    }
    result
}

fn md5_hash(pixels: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(pixels);
    format!("{:x}", hasher.finalize())
}

/// Apply rotation (CW) + flip to 16x16 pixel array.
/// Editor order: rotate -> vflip -> hflip.
pub fn apply_transform(pixels: &[u8], rot: u8, hflip: bool, vflip: bool) -> Vec<u8> {
    let size = TILE_SIZE as usize;
    let mut grid: Vec<Vec<u8>> = (0..size)
        .map(|y| pixels[y * size..(y + 1) * size].to_vec())
        .collect();

    // Apply rotation (CW)
    for _ in 0..(rot % 4) {
        let mut new_grid = vec![vec![0u8; size]; size];
        for x in 0..size {
            for y in (0..size).rev() {
                new_grid[x][size - 1 - y] = grid[y][x];
            }
        }
        grid = new_grid;
    }

    // Apply vflip
    if vflip {
        grid.reverse();
    }

    // Apply hflip
    if hflip {
        for row in &mut grid {
            row.reverse();
        }
    }

    grid.into_iter().flatten().collect()
}

/// Slice main tileset PNG into 16x16 tiles, deduplicate with dihedral transforms.
/// Returns list of (name, PIL Image as DynamicImage).
pub fn slice_tileset(tileset_path: &Path, band_colors: &[u8; 5]) -> Vec<(String, DynamicImage)> {
    let img = image::open(tileset_path)
        .unwrap_or_else(|e| panic!("Failed to open tileset {}: {}", tileset_path.display(), e));
    let mut tiles = Vec::new();
    let mut seen_hashes: HashMap<String, usize> = HashMap::new();

    for r in 0..TILESET_ROWS {
        for c in 0..TILESET_COLS {
            let x0 = c * TILE_SIZE;
            let y0 = r * TILE_SIZE;
            let tile_img = img.crop_imm(x0, y0, TILE_SIZE, TILE_SIZE);

            // Skip fully transparent
            let all_transparent = {
                let rgba = tile_img.to_rgba8();
                rgba.pixels().all(|p| p[3] == 0)
            };
            if all_transparent {
                continue;
            }

            let remapped = remap_tile_image(&tile_img, band_colors, false);
            let h = md5_hash(&remapped);
            if seen_hashes.contains_key(&h) {
                continue;
            }

            // Check 8 dihedral transforms for duplicates
            let mut is_dup = false;
            for rot in 0..4u32 {
                for &flip in &[false, true] {
                    use image::imageops;
                    let mut t = tile_img.clone();
                    for _ in 0..rot {
                        t = DynamicImage::ImageRgba8(imageops::rotate90(&t.to_rgba8()));
                    }
                    if flip {
                        t = DynamicImage::ImageRgba8(imageops::flip_horizontal(&t.to_rgba8()));
                    }
                    let th = md5_hash(&remap_tile_image(&t, band_colors, false));
                    if seen_hashes.contains_key(&th) {
                        is_dup = true;
                        break;
                    }
                }
                if is_dup {
                    break;
                }
            }
            if is_dup {
                continue;
            }

            let name = format!("T_{:02}_{:02}", r, c);
            seen_hashes.insert(h, tiles.len());
            tiles.push((name, tile_img));
        }
    }
    tiles
}

/// Slice BG tileset PNG into 16x16 tiles, deduplicate (no dihedral check).
pub fn slice_bg_tileset(bg_tileset_path: &Path, band_colors: &[u8; 5]) -> Vec<(String, DynamicImage)> {
    let img = image::open(bg_tileset_path)
        .unwrap_or_else(|e| panic!("Failed to open bg tileset {}: {}", bg_tileset_path.display(), e));
    let cw = (img.dimensions().0 / TILE_SIZE) * TILE_SIZE;
    let ch = (img.dimensions().1 / TILE_SIZE) * TILE_SIZE;
    let img = img.crop_imm(0, 0, cw, ch);

    let mut tiles = Vec::new();
    let mut seen_hashes: HashMap<String, usize> = HashMap::new();

    for r in 0..(ch / TILE_SIZE) {
        for c in 0..(cw / TILE_SIZE) {
            let x0 = c * TILE_SIZE;
            let y0 = r * TILE_SIZE;
            let mut tile_img = img.crop_imm(x0, y0, TILE_SIZE, TILE_SIZE);

            let all_transparent = {
                let rgba = tile_img.to_rgba8();
                rgba.pixels().all(|p| p[3] == 0)
            };
            if all_transparent {
                continue;
            }

            let remapped = remap_tile_image(&tile_img, band_colors, false);
            let h = md5_hash(&remapped);
            if seen_hashes.contains_key(&h) {
                continue;
            }

            let name = format!("BG_{:02}_{:02}", r, c);
            seen_hashes.insert(h, tiles.len());
            tiles.push((name, tile_img));
        }
    }
    tiles
}
