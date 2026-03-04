/// Macro-tile compression: brute-force search for optimal block size.
/// Tests all (bw, bh, ox, oy) combos with EG-2 compression.
/// Two encoding variants are tried per combo:
///   A) Raw cell values (no palette, bpp covers max cell value)
///   B) Quantized indices + stored palette (lower bpp + palette cost)

use rayon::prelude::*;
use std::collections::HashMap;

use crate::eg2::eg2_encode_frame;

/// Result of evaluating one (bw, bh, ox, oy) combo.
struct MacroResult {
    bw: usize,
    bh: usize,
    ox: usize,
    oy: usize,
    n_blocks: usize,
    total: usize,
    uses_palette: bool,
}

/// Minimum bits needed to represent values 0..n (n exclusive).
fn min_bpp(n: usize) -> u8 {
    if n <= 1 { return 1; }
    let mut bpp = 1u8;
    while (1u32 << bpp) < n as u32 { bpp += 1; }
    bpp
}

fn eval_combo(
    cells_flat: &[u16],
    map_w: usize,
    map_h: usize,
    bw: usize,
    bh: usize,
    ox: usize,
    oy: usize,
) -> Option<MacroResult> {
    let get_cell = |y: i32, x: i32| -> u16 {
        if y >= 0 && (y as usize) < map_h && x >= 0 && (x as usize) < map_w {
            cells_flat[y as usize * map_w + x as usize]
        } else {
            0
        }
    };

    let macro_w = (map_w + ox + bw - 1) / bw;
    let macro_h = (map_h + oy + bh - 1) / bh;

    let mut block_list: Vec<Vec<u16>> = Vec::new();
    let mut block_idx: HashMap<Vec<u16>, usize> = HashMap::new();
    let mut macro_flat: Vec<u16> = Vec::new();

    for my in 0..macro_h {
        for mx in 0..macro_w {
            let mut block = Vec::with_capacity(bw * bh);
            for dy in 0..bh {
                for dx in 0..bw {
                    block.push(get_cell(
                        (my * bh + dy) as i32 - oy as i32,
                        (mx * bw + dx) as i32 - ox as i32,
                    ));
                }
            }
            let idx = if let Some(&idx) = block_idx.get(&block) {
                idx
            } else {
                let idx = block_list.len();
                block_idx.insert(block.clone(), idx);
                block_list.push(block);
                idx
            };
            macro_flat.push(idx as u16);
        }
    }

    let n_blocks = block_list.len();
    if n_blocks > 255 { return None; } // limit to fit in u8

    // Reorder blocks by frequency (most common first) for better EG-2 on macro map
    let mut freq: Vec<usize> = vec![0; n_blocks];
    for &idx in &macro_flat { freq[idx as usize] += 1; }
    let mut order: Vec<usize> = (0..n_blocks).collect();
    order.sort_by(|&a, &b| freq[b].cmp(&freq[a]));
    let mut remap = vec![0usize; n_blocks];
    for (new_idx, &old_idx) in order.iter().enumerate() {
        remap[old_idx] = new_idx;
    }
    let macro_flat_reordered: Vec<u16> = macro_flat.iter().map(|&v| remap[v as usize] as u16).collect();
    let reordered_blocks: Vec<&Vec<u16>> = order.iter().map(|&old| &block_list[old]).collect();

    // EG-2 compress macro map
    let macro_bpp = min_bpp(n_blocks);
    let macro_map_u8: Vec<u8> = macro_flat_reordered.iter().map(|&v| v as u8).collect();
    let (macro_map_eg2, _, _) = eg2_encode_frame(&macro_map_u8, macro_bpp, macro_w, macro_h);
    let macro_map_size = macro_map_eg2.len() + 1; // +1 for bpp byte

    // Collect unique cell values across all blocks
    let mut cell_set_sorted: Vec<u16> = {
        let mut s = std::collections::HashSet::new();
        for bl in &block_list { for &c in bl { s.insert(c); } }
        let mut v: Vec<u16> = s.into_iter().collect();
        v.sort();
        v
    };

    let max_cell = cell_set_sorted.last().copied().unwrap_or(0) as usize;
    let n_bvals = cell_set_sorted.len();

    // === Variant A: Raw cell values (no palette) ===
    let raw_bpp = min_bpp(max_cell + 1);
    let mut block_flat_raw: Vec<u8> = Vec::new();
    for bl in &reordered_blocks {
        for &c in *bl { block_flat_raw.push(c as u8); }
    }
    let (block_defs_eg2_raw, _, _) = eg2_encode_frame(&block_flat_raw, raw_bpp, bw, bh * n_blocks);
    let block_defs_size_raw = block_defs_eg2_raw.len() + 1; // +1 bpp byte
    // Header: 0xFF + bw + bh + ox + oy + n_blocks(u16) + block_defs_size(u16) + n_palette(u8=0) = 10 bytes
    let total_raw = 10 + block_defs_size_raw + macro_map_size;

    // === Variant B: Quantized + palette ===
    let quant_bpp = min_bpp(n_bvals);
    let bval_map: HashMap<u16, u8> = cell_set_sorted.iter().enumerate().map(|(i, &v)| (v, i as u8)).collect();
    let mut block_flat_quant: Vec<u8> = Vec::new();
    for bl in &reordered_blocks {
        for &c in *bl { block_flat_quant.push(bval_map[&c]); }
    }
    let (block_defs_eg2_quant, _, _) = eg2_encode_frame(&block_flat_quant, quant_bpp, bw, bh * n_blocks);
    let block_defs_size_quant = block_defs_eg2_quant.len() + 1;
    // Palette cost: n_bvals bytes (each cell value as u8)
    let palette_cost = n_bvals;
    // Header: 10 + palette
    let total_quant = 10 + palette_cost + block_defs_size_quant + macro_map_size;

    let (total, uses_palette) = if total_quant < total_raw {
        (total_quant, true)
    } else {
        (total_raw, false)
    };

    Some(MacroResult { bw, bh, ox, oy, n_blocks, total, uses_palette })
}

/// Try macro-tile encoding for a layer. Returns (encoded_bytes, description).
/// Tests all block sizes from 1x2 to MAX_BLOCK x MAX_BLOCK with all offsets.
pub fn encode_layer_macro(
    cell_grid: &[Vec<u16>],
    map_w: usize,
    map_h: usize,
    label: &str,
) -> (Vec<u8>, String) {
    let cells_flat: Vec<u16> = cell_grid
        .iter()
        .flat_map(|row| row.iter().copied())
        .collect();

    if cells_flat.iter().all(|&c| c == 0) {
        return (vec![0u8; 99999], "MACRO: empty layer".to_string());
    }

    const MAX_DIM: usize = 16;
    const MAX_AREA: usize = 48; // limit offset explosion for large blocks

    let mut jobs: Vec<(usize, usize, usize, usize)> = Vec::new();
    for bh in 1..=MAX_DIM {
        for bw in 1..=MAX_DIM {
            if bw == 1 && bh == 1 { continue; }
            if bw > map_w || bh > map_h { continue; }
            if bw * bh > MAX_AREA { continue; }
            for oy in 0..bh {
                for ox in 0..bw {
                    jobs.push((bw, bh, ox, oy));
                }
            }
        }
    }

    let results: Vec<Option<MacroResult>> = jobs
        .par_iter()
        .map(|&(bw, bh, ox, oy)| eval_combo(&cells_flat, map_w, map_h, bw, bh, ox, oy))
        .collect();

    let best = results
        .into_iter()
        .filter_map(|r| r)
        .min_by_key(|r| r.total);

    let best = match best {
        Some(b) => b,
        None => return (vec![0u8; 99999], "MACRO: no valid result".to_string()),
    };

    // Re-encode the best result to produce output bytes
    let bw = best.bw;
    let bh = best.bh;
    let ox = best.ox;
    let oy = best.oy;

    let get_cell = |y: i32, x: i32| -> u16 {
        if y >= 0 && (y as usize) < map_h && x >= 0 && (x as usize) < map_w {
            cells_flat[y as usize * map_w + x as usize]
        } else {
            0
        }
    };

    let macro_w = (map_w + ox + bw - 1) / bw;
    let macro_h = (map_h + oy + bh - 1) / bh;

    let mut block_list: Vec<Vec<u16>> = Vec::new();
    let mut block_idx: HashMap<Vec<u16>, usize> = HashMap::new();
    let mut macro_flat: Vec<u16> = Vec::new();

    for my in 0..macro_h {
        for mx in 0..macro_w {
            let mut block = Vec::with_capacity(bw * bh);
            for dy in 0..bh {
                for dx in 0..bw {
                    block.push(get_cell(
                        (my * bh + dy) as i32 - oy as i32,
                        (mx * bw + dx) as i32 - ox as i32,
                    ));
                }
            }
            let idx = if let Some(&idx) = block_idx.get(&block) {
                idx
            } else {
                let idx = block_list.len();
                block_idx.insert(block.clone(), idx);
                block_list.push(block);
                idx
            };
            macro_flat.push(idx as u16);
        }
    }

    let n_blocks = block_list.len();

    // Reorder blocks by frequency
    let mut freq: Vec<usize> = vec![0; n_blocks];
    for &idx in &macro_flat { freq[idx as usize] += 1; }
    let mut order: Vec<usize> = (0..n_blocks).collect();
    order.sort_by(|&a, &b| freq[b].cmp(&freq[a]));
    let mut remap_table = vec![0usize; n_blocks];
    for (new_idx, &old_idx) in order.iter().enumerate() {
        remap_table[old_idx] = new_idx;
    }
    let macro_flat_reordered: Vec<u16> = macro_flat.iter().map(|&v| remap_table[v as usize] as u16).collect();
    let reordered_blocks: Vec<&Vec<u16>> = order.iter().map(|&old| &block_list[old]).collect();

    // EG-2 compress macro map
    let macro_bpp = min_bpp(n_blocks);
    let macro_map_u8: Vec<u8> = macro_flat_reordered.iter().map(|&v| v as u8).collect();
    let (macro_map_eg2, _, _) = eg2_encode_frame(&macro_map_u8, macro_bpp, macro_w, macro_h);

    // Collect unique cell values
    let cell_set_sorted: Vec<u16> = {
        let mut s = std::collections::HashSet::new();
        for bl in &block_list { for &c in bl { s.insert(c); } }
        let mut v: Vec<u16> = s.into_iter().collect();
        v.sort();
        v
    };
    let max_cell = cell_set_sorted.last().copied().unwrap_or(0) as usize;
    let n_bvals = cell_set_sorted.len();

    let mut out = Vec::new();

    if best.uses_palette {
        // Variant B: quantized + palette
        let quant_bpp = min_bpp(n_bvals);
        let bval_map: HashMap<u16, u8> = cell_set_sorted.iter().enumerate().map(|(i, &v)| (v, i as u8)).collect();
        let mut block_flat: Vec<u8> = Vec::new();
        for bl in &reordered_blocks {
            for &c in *bl { block_flat.push(bval_map[&c]); }
        }
        let (block_defs_eg2, _, _) = eg2_encode_frame(&block_flat, quant_bpp, bw, bh * n_blocks);
        let block_defs_total = 1 + block_defs_eg2.len();

        // Header: [0xFF][bw][bh][ox][oy][n_blocks:u16][block_defs_size:u16][n_palette]
        out.push(0xFF);
        out.push(bw as u8);
        out.push(bh as u8);
        out.push(ox as u8);
        out.push(oy as u8);
        out.push((n_blocks & 0xFF) as u8);
        out.push(((n_blocks >> 8) & 0xFF) as u8);
        out.push((block_defs_total & 0xFF) as u8);
        out.push(((block_defs_total >> 8) & 0xFF) as u8);
        // Palette
        out.push(n_bvals as u8);
        for &v in &cell_set_sorted { out.push(v as u8); }
        // Block definitions
        out.push(quant_bpp);
        out.extend_from_slice(&block_defs_eg2);
        // Macro map
        out.push(macro_bpp);
        out.extend_from_slice(&macro_map_eg2);
    } else {
        // Variant A: raw cell values (no palette)
        let raw_bpp = min_bpp(max_cell + 1);
        let mut block_flat: Vec<u8> = Vec::new();
        for bl in &reordered_blocks {
            for &c in *bl { block_flat.push(c as u8); }
        }
        let (block_defs_eg2, _, _) = eg2_encode_frame(&block_flat, raw_bpp, bw, bh * n_blocks);
        let block_defs_total = 1 + block_defs_eg2.len();

        // Header: [0xFF][bw][bh][ox][oy][n_blocks:u16][block_defs_size:u16][n_palette=0]
        out.push(0xFF);
        out.push(bw as u8);
        out.push(bh as u8);
        out.push(ox as u8);
        out.push(oy as u8);
        out.push((n_blocks & 0xFF) as u8);
        out.push(((n_blocks >> 8) & 0xFF) as u8);
        out.push((block_defs_total & 0xFF) as u8);
        out.push(((block_defs_total >> 8) & 0xFF) as u8);
        // No palette
        out.push(0);
        // Block definitions
        out.push(raw_bpp);
        out.extend_from_slice(&block_defs_eg2);
        // Macro map
        out.push(macro_bpp);
        out.extend_from_slice(&macro_map_eg2);
    }

    let desc = format!(
        "MACRO {}x{} off=({},{}) {} blks {}b (map:{}b defs:{}b{})",
        bw, bh, ox, oy, n_blocks, out.len(),
        macro_map_eg2.len() + 1,
        out.len() - (macro_map_eg2.len() + 1) - 8 - if best.uses_palette { n_bvals } else { 0 },
        if best.uses_palette { format!(" pal:{}b", n_bvals + 1) } else { String::new() }
    );

    (out, desc)
}
