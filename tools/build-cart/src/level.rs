/// Level data processing: JSON reading, tile pool building, map encoding, entity packing.

use image::DynamicImage;
use md5::{Digest, Md5};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::config::*;
use crate::eg2::eg2_encode_frame;
use crate::frame::{pack_palette, quantize_pixels};
use crate::tileset::{apply_transform, remap_tile_image};

#[derive(Deserialize)]
struct LevelJson {
    width: Option<usize>,
    height: Option<usize>,
    layers: Option<Vec<LayerJson>>,
    map: Option<Vec<Vec<i32>>>,
    #[serde(rename = "mapXform")]
    map_xform: Option<Vec<Vec<i32>>>,
    #[serde(rename = "spawnX")]
    spawn_x: Option<i32>,
    #[serde(rename = "spawnY")]
    spawn_y: Option<i32>,
    tiles: Option<Vec<serde_json::Value>>,
    #[serde(rename = "bgTiles")]
    bg_tiles: Option<Vec<serde_json::Value>>,
    flags: Option<Vec<i32>>,
    #[serde(rename = "bandColors")]
    band_colors: Option<Vec<i32>>,
    #[serde(rename = "bgBandColors")]
    bg_band_colors: Option<Vec<i32>>,
    parallax: Option<Vec<f64>>,
    entities: Option<Vec<EntityJson>>,
    version: Option<u32>,
    #[serde(rename = "mapBgColor")]
    map_bg_color: Option<u8>,
    texts: Option<Vec<String>>,
    #[serde(rename = "tileEdits")]
    tile_edits: Option<HashMap<String, TileEditJson>>,
    #[serde(rename = "bgTileEdits")]
    bg_tile_edits: Option<HashMap<String, TileEditJson>>,
}

#[derive(Deserialize)]
struct TileEditLum {
    lum: u8,
    a: u8,
}

#[derive(Deserialize)]
struct TileEditJson {
    lum: Vec<TileEditLum>,
}

#[derive(Deserialize)]
struct LayerJson {
    map: Vec<Vec<i32>>,
    xform: Vec<Vec<i32>>,
}

#[derive(Deserialize, Clone)]
pub struct EntityJson {
    #[serde(rename = "type")]
    pub etype: Option<u8>,
    pub x: Option<u8>,
    pub y: Option<u8>,
    pub group: Option<u8>,
    pub cost: Option<u8>,
    pub ew: Option<u8>,
    pub eh: Option<u8>,
}

pub struct MapData {
    pub map_w: usize,
    pub map_h: usize,
    pub map_grids: Vec<Vec<Vec<i32>>>,   // [layer][y][x]
    pub xform_grids: Vec<Vec<Vec<i32>>>, // [layer][y][x]
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub flags: Vec<u8>,
    pub band_colors: Option<[u8; 5]>,
    pub bg_band_colors: Option<[u8; 5]>,
    pub parallax: [f64; 3],
    pub entities: Vec<EntityJson>,
    pub map_bg_color: u8,
    pub zone_texts: Vec<String>,
    pub tile_edits: HashMap<String, Vec<(u8, u8)>>,  // name -> [(lum, alpha)]
}

pub fn read_level_json(json_path: &Path) -> Option<MapData> {
    let text = std::fs::read_to_string(json_path).ok()?;
    let data: LevelJson = serde_json::from_str(&text).ok()?;

    let w = data.width?;
    let h = data.height?;
    let sx = data.spawn_x.unwrap_or(-1);
    let sy = data.spawn_y.unwrap_or(-1);
    let main_count = data.tiles.as_ref().map(|t| t.len()).unwrap_or(0);

    let empty_grid = || vec![vec![255i32; w]; h];
    let zero_grid = || vec![vec![0i32; w]; h];

    let (mut map_grids, mut xform_grids) = if let Some(layers) = &data.layers {
        let mg: Vec<Vec<Vec<i32>>> = layers.iter().map(|l| l.map.clone()).collect();
        let xg: Vec<Vec<Vec<i32>>> = layers.iter().map(|l| l.xform.clone()).collect();
        (mg, xg)
    } else if let Some(map) = &data.map {
        let xf = data.map_xform.clone().unwrap_or_else(|| zero_grid());
        (
            vec![empty_grid(), map.clone(), empty_grid()],
            vec![zero_grid(), xf, zero_grid()],
        )
    } else {
        return None;
    };

    // Handle version-specific index offsets
    let version = data.version.unwrap_or(2);
    if let Some(_layers) = &data.layers {
        if version >= 4 {
            // v4: already unified indices
        } else if version >= 3 || data.bg_tiles.is_some() {
            // v3: layer 0 uses local BG indices -> offset
            for y in 0..map_grids[0].len() {
                for x in 0..map_grids[0][y].len() {
                    if map_grids[0][y][x] != 255 {
                        map_grids[0][y][x] += main_count as i32;
                    }
                }
            }
        } else {
            // v2: BG layer had main tileset indices -> clear
            map_grids[0] = empty_grid();
            xform_grids[0] = zero_grid();
        }
    }

    // Pad to 3 layers
    while map_grids.len() < 3 {
        map_grids.push(empty_grid());
    }
    while xform_grids.len() < 3 {
        xform_grids.push(zero_grid());
    }
    map_grids.truncate(3);
    xform_grids.truncate(3);

    let mut flags = vec![0u8; 256];
    if let Some(f) = &data.flags {
        for (i, &v) in f.iter().enumerate() {
            if i < 256 {
                flags[i] = v as u8;
            }
        }
    }

    let band_colors = data.band_colors.as_ref().and_then(|bc| {
        if bc.len() == 5 {
            Some([bc[0] as u8, bc[1] as u8, bc[2] as u8, bc[3] as u8, bc[4] as u8])
        } else {
            None
        }
    });

    let bg_band_colors = data.bg_band_colors.as_ref().and_then(|bc| {
        if bc.len() == 5 {
            Some([bc[0] as u8, bc[1] as u8, bc[2] as u8, bc[3] as u8, bc[4] as u8])
        } else {
            None
        }
    });

    let mut parallax = [0.5, 1.0, 1.0];
    if let Some(p) = &data.parallax {
        for (i, &v) in p.iter().take(3).enumerate() {
            parallax[i] = v;
        }
    }

    let entities = data.entities.unwrap_or_default();
    let map_bg_color = data.map_bg_color.unwrap_or(1);
    let zone_texts = data.texts.unwrap_or_default();

    // Merge tile edits from both FG and BG
    let mut tile_edits: HashMap<String, Vec<(u8, u8)>> = HashMap::new();
    for edits in [&data.tile_edits, &data.bg_tile_edits] {
        if let Some(edits) = edits {
            for (name, edit) in edits {
                let pixels: Vec<(u8, u8)> = edit.lum.iter().map(|p| (p.lum, p.a)).collect();
                tile_edits.insert(name.clone(), pixels);
            }
        }
    }

    Some(MapData {
        map_w: w,
        map_h: h,
        map_grids,
        xform_grids,
        spawn_x: sx,
        spawn_y: sy,
        flags,
        band_colors,
        bg_band_colors,
        parallax,
        entities,
        map_bg_color,
        zone_texts,
        tile_edits,
    })
}

/// Encode a map layer using EG-2. Returns (data_bytes, description).
fn encode_layer(cell_grid: &[Vec<u16>], map_w: usize, map_h: usize, label: &str) -> (Vec<u8>, String) {
    let flat: Vec<u16> = cell_grid
        .iter()
        .flat_map(|row| row.iter().copied())
        .collect();
    let max_val = flat.iter().copied().max().unwrap_or(0);
    let mut bpp = 1u8;
    for try_bpp in 1..=16 {
        if (1u32 << try_bpp) > max_val as u32 {
            bpp = try_bpp;
            break;
        }
    }
    let (eg2_data, eg2_mode, eg2_order) = crate::eg2::eg2_encode_frame_u16(&flat, bpp, map_w, map_h);
    let mut out = vec![bpp];
    out.extend_from_slice(&eg2_data);
    let desc = format!("EG2 {}b {}bpp (mode={} order={})", out.len(), bpp, eg2_mode, eg2_order);
    (out, desc)
}

pub struct LevelBuildResult {
    pub map_section: Vec<u8>,
    pub num_rt: usize,
    pub tile_flags: HashMap<usize, u8>,
    pub gen_lines: Vec<String>,
    pub num_spr_tiles: usize,
}

pub fn build_level_data(
    tileset: &[(String, DynamicImage)],
    bg_tileset: &[(String, DynamicImage)],
    map_data: &MapData,
    active_band_colors: &[u8; 5],
    bg_active_band_colors: &[u8; 5],
) -> LevelBuildResult {
    let map_w = map_data.map_w;
    let map_h = map_data.map_h;
    let num_layers = map_data.map_grids.len();
    let main_count = tileset.len();

    let band_colors_for_tile = |uni: usize| -> &[u8; 5] {
        if uni >= main_count {
            bg_active_band_colors
        } else {
            active_band_colors
        }
    };

    eprintln!("\n=== LEVEL DATA ===");
    eprintln!("  Map size: {}x{} ({} cells), {} layers", map_w, map_h, map_w * map_h, num_layers);
    eprintln!("  Spawn: ({}, {})", map_data.spawn_x, map_data.spawn_y);
    eprintln!("  Unified tileset: {} main + {} BG tiles", main_count, bg_tileset.len());

    // Step 1: Collect used tiles per layer (base + rot90 for all layers)
    let mut used_base: Vec<HashSet<usize>> = vec![HashSet::new(); num_layers];
    let mut used_rot90: Vec<HashSet<usize>> = vec![HashSet::new(); num_layers];

    for l in 0..num_layers {
        for y in 0..map_h {
            for x in 0..map_w {
                let ti = map_data.map_grids[l][y][x];
                if ti == 255 {
                    continue;
                }
                let xf = map_data.xform_grids[l][y][x] as u8;
                let rot = xf & 3;
                if rot == 0 || rot == 2 {
                    used_base[l].insert(ti as usize);
                } else {
                    used_rot90[l].insert(ti as usize);
                }
            }
        }
    }

    // Step 2: Build runtime tile pool
    let mut rt_tiles: Vec<Vec<u8>> = Vec::new();
    let mut rt_tile_flags: HashMap<usize, u8> = HashMap::new();
    let mut rt_hashes: Vec<String> = Vec::new();
    let mut base_rt: HashMap<usize, usize> = HashMap::new();
    let mut rot90_rt: HashMap<usize, usize> = HashMap::new();

    let md5_hash = |pixels: &[u8]| -> String {
        let mut hasher = Md5::new();
        hasher.update(pixels);
        format!("{:x}", hasher.finalize())
    };

    let mut add_rt_tile = |pixels: Vec<u8>, rt_tiles: &mut Vec<Vec<u8>>, rt_hashes: &mut Vec<String>| -> usize {
        let h = md5_hash(&pixels);
        for (rt_id, rh) in rt_hashes.iter().enumerate() {
            if *rh == h {
                return rt_id + 1;
            }
        }
        let rt_id = rt_tiles.len();
        rt_tiles.push(pixels);
        rt_hashes.push(h);
        rt_id + 1
    };

    // Helper to look up tile image by unified index
    let get_tile = |uni: usize| -> Option<&DynamicImage> {
        if uni < main_count {
            Some(&tileset[uni].1)
        } else if uni - main_count < bg_tileset.len() {
            Some(&bg_tileset[uni - main_count].1)
        } else {
            None
        }
    };

    // Helper to get tile name by unified index
    let get_tile_name = |uni: usize| -> Option<&str> {
        if uni < main_count {
            Some(&tileset[uni].0)
        } else if uni - main_count < bg_tileset.len() {
            Some(&bg_tileset[uni - main_count].0)
        } else {
            None
        }
    };

    // Remap tile pixels, using editor pixel edits if available
    let remap_tile = |uni: usize| -> Option<Vec<u8>> {
        let name = get_tile_name(uni)?;
        let bc = band_colors_for_tile(uni);
        if let Some(lum_data) = map_data.tile_edits.get(name) {
            let pixels: Vec<(u8, u8, u8, u8)> = lum_data.iter()
                .map(|&(lum, a)| (lum, lum, lum, a))
                .collect();
            Some(crate::tileset::remap_tile_colors(&pixels, bc))
        } else {
            let img = get_tile(uni)?;
            Some(remap_tile_image(img, bc))
        }
    };

    // Collect which tiles need base and/or rot90 across all layers
    let needs_base: HashSet<usize> = {
        let mut s = HashSet::new();
        for l in 0..num_layers { s.extend(&used_base[l]); }
        s
    };
    let needs_rot90: HashSet<usize> = {
        let mut s = HashSet::new();
        for l in 0..num_layers { s.extend(&used_rot90[l]); }
        s
    };

    // Process main-layer tiles first (these become "spr" tiles)
    let main_used: Vec<usize> = {
        let mut s: HashSet<usize> = HashSet::new();
        s.extend(&used_base[1]);
        s.extend(&used_rot90[1]);
        let mut v: Vec<usize> = s.into_iter().collect();
        v.sort();
        v
    };
    for &ti in &main_used {
        let base_pixels = match remap_tile(ti) {
            Some(p) => p,
            None => {
                eprintln!("  WARNING: tile index {} out of range, skipping", ti);
                continue;
            }
        };
        let flag = if ti < map_data.flags.len() { map_data.flags[ti] } else { 0 };
        if needs_base.contains(&ti) {
            let rt_id = add_rt_tile(base_pixels.clone(), &mut rt_tiles, &mut rt_hashes);
            base_rt.insert(ti, rt_id);
            rt_tile_flags.insert(rt_id, flag);
        }
        if needs_rot90.contains(&ti) {
            let rot90_pixels = apply_transform(&base_pixels, 1, false, false);
            let rt_id = add_rt_tile(rot90_pixels, &mut rt_tiles, &mut rt_hashes);
            rot90_rt.insert(ti, rt_id);
            rt_tile_flags.insert(rt_id, flag);
        }
    }
    let num_spr_tiles = rt_tiles.len();

    // Process BG-only tiles (not already added by main layer)
    let bg_only: Vec<usize> = {
        let mut s: HashSet<usize> = HashSet::new();
        for l in [0, 2] {
            s.extend(&used_base[l]);
            s.extend(&used_rot90[l]);
        }
        for &ti in &main_used { s.remove(&ti); }
        let mut v: Vec<usize> = s.into_iter().collect();
        v.sort();
        v
    };
    for &ti in &bg_only {
        let base_pixels = match remap_tile(ti) {
            Some(p) => p,
            None => {
                eprintln!("  WARNING: BG tile index {} out of range, skipping", ti);
                continue;
            }
        };
        if needs_base.contains(&ti) && !base_rt.contains_key(&ti) {
            let rt_id = add_rt_tile(base_pixels.clone(), &mut rt_tiles, &mut rt_hashes);
            base_rt.insert(ti, rt_id);
        }
        if needs_rot90.contains(&ti) && !rot90_rt.contains_key(&ti) {
            let rot90_pixels = apply_transform(&base_pixels, 1, false, false);
            let rt_id = add_rt_tile(rot90_pixels, &mut rt_tiles, &mut rt_hashes);
            rot90_rt.insert(ti, rt_id);
        }
    }
    let num_rt = rt_tiles.len();

    eprintln!("  Unique tiles: {} base + {} rot90 = {} runtime ({} spr + {} bg)",
        base_rt.len(), rot90_rt.len(), num_rt, num_spr_tiles, num_rt - num_spr_tiles
    );

    // Convert editor (tile_id, xform) to cell value — unified for all layers
    let editor_to_cell = |_l: usize, ti: i32, xf: i32| -> u16 {
        if ti == 255 {
            return 0;
        }
        let rot = (xf & 3) as u8;
        let rt_id = if rot == 0 || rot == 2 {
            *base_rt.get(&(ti as usize)).unwrap_or(&0)
        } else {
            *rot90_rt.get(&(ti as usize)).unwrap_or(&0)
        };
        if rt_id == 0 {
            return 0;
        }
        let hflip = (xf & 4) != 0;
        let vflip = (xf & 8) != 0;
        let (fx, fy) = if rot >= 2 {
            (!hflip as u16, !vflip as u16)
        } else {
            (hflip as u16, vflip as u16)
        };
        ((rt_id as u16) << 2) | (fx << 1) | fy
    };

    // Step 3: EG-2 compress tile pixels as TWO blobs
    let mut all_pixels: Vec<u8> = Vec::new();
    for pixels in &rt_tiles {
        all_pixels.extend_from_slice(pixels);
    }

    let tile_colors: Vec<u8> = {
        let mut s: HashSet<u8> = HashSet::new();
        for &c in &all_pixels {
            s.insert(c);
        }
        let mut v: Vec<u8> = s.into_iter().collect();
        v.sort();
        v
    };

    let tile_bpp = {
        let mut bpp = 4u8;
        for try_bpp in [1, 2, 3, 4] {
            if (1u16 << try_bpp) >= tile_colors.len() as u16 {
                bpp = try_bpp;
                break;
            }
        }
        bpp
    };

    let tile_pal: Vec<u8> = {
        let mut pal = tile_colors.clone();
        while pal.len() < (1 << tile_bpp) {
            pal.push(0);
        }
        pal
    };

    let tile_pal_map: HashMap<u8, u8> = tile_colors
        .iter()
        .enumerate()
        .map(|(i, &c)| (c, i as u8))
        .collect();
    let quantized: Vec<u8> = all_pixels.iter().map(|&c| tile_pal_map[&c]).collect();

    let spr_npix = num_spr_tiles * 256;
    let spr_q = &quantized[..spr_npix];
    let bg_q = &quantized[spr_npix..];

    let spr_eg2 = if !spr_q.is_empty() {
        eg2_encode_frame(spr_q, tile_bpp, 16, 16 * num_spr_tiles).0
    } else {
        vec![]
    };
    let num_bg_rt = num_rt - num_spr_tiles;
    let bg_eg2 = if !bg_q.is_empty() {
        eg2_encode_frame(bg_q, tile_bpp, 16, 16 * num_bg_rt).0
    } else {
        vec![]
    };

    let mut tile_blob = Vec::new();
    tile_blob.push(tile_bpp);
    tile_blob.extend_from_slice(&crate::frame::pack_palette(&tile_pal));
    tile_blob.push((spr_eg2.len() & 0xFF) as u8);
    tile_blob.push(((spr_eg2.len() >> 8) & 0xFF) as u8);
    tile_blob.extend_from_slice(&spr_eg2);
    tile_blob.extend_from_slice(&bg_eg2);

    eprintln!(
        "  Tile pixels: {}b EG-2 {}bpp (spr:{}b + bg:{}b, from {}b) pal={:?}",
        tile_blob.len(),
        tile_bpp,
        spr_eg2.len(),
        bg_eg2.len(),
        num_rt * 128,
        tile_pal
    );

    // Step 4: Build cell grids and encode layers
    let mut layer_data = Vec::new();
    let layer_names = ["BG1", "Main", "BG2"];
    for l in 0..num_layers {
        let cell_grid: Vec<Vec<u16>> = (0..map_h)
            .map(|y| {
                (0..map_w)
                    .map(|x| editor_to_cell(l, map_data.map_grids[l][y][x], map_data.xform_grids[l][y][x]))
                    .collect()
            })
            .collect();

        let label = layer_names[l];

        let (normal_data, normal_desc) = encode_layer(&cell_grid, map_w, map_h, label);
        eprintln!("  Layer {} ({}): {}", l, label, normal_desc);
        layer_data.push(normal_data);
    }

    // Step 5: Pack into binary format
    let mut header = Vec::new();
    header.push(num_rt as u8);
    header.push(num_layers as u8);
    header.push((map_w & 0xFF) as u8);
    header.push(((map_w >> 8) & 0xFF) as u8);
    header.push((map_h & 0xFF) as u8);
    header.push(((map_h >> 8) & 0xFF) as u8);
    let sx = if map_data.spawn_x >= 0 { map_data.spawn_x as u16 } else { 0xFFFF };
    let sy = if map_data.spawn_y >= 0 { map_data.spawn_y as u16 } else { 0xFFFF };
    header.push((sx & 0xFF) as u8);
    header.push(((sx >> 8) & 0xFF) as u8);
    header.push((sy & 0xFF) as u8);
    header.push(((sy >> 8) & 0xFF) as u8);
    header.push((tile_blob.len() & 0xFF) as u8);
    header.push(((tile_blob.len() >> 8) & 0xFF) as u8);
    for ld in &layer_data {
        header.push((ld.len() & 0xFF) as u8);
        header.push(((ld.len() >> 8) & 0xFF) as u8);
    }

    let mut map_section = Vec::new();
    map_section.extend_from_slice(&header);
    map_section.extend_from_slice(&tile_blob);
    for ld in &layer_data {
        map_section.extend_from_slice(ld);
    }

    // Step 5b: Entity data
    map_section.push(map_data.entities.len() as u8);
    let mut ent_bytes = 0;
    for ent in &map_data.entities {
        map_section.push(ent.etype.unwrap_or(1));
        map_section.push(ent.x.unwrap_or(0));
        map_section.push(ent.y.unwrap_or(0));
        map_section.push(ent.group.unwrap_or(1));
        ent_bytes += 4;
        if ent.etype == Some(2) || ent.etype == Some(3) {
            map_section.push(ent.cost.unwrap_or(0));
            ent_bytes += 1;
        }
        if ent.etype == Some(5) {
            map_section.push(ent.ew.unwrap_or(1));
            map_section.push(ent.eh.unwrap_or(1));
            ent_bytes += 2;
        }
    }
    if !map_data.entities.is_empty() {
        eprintln!("  Entities: {} ({}b)", map_data.entities.len(), 1 + ent_bytes);
    }

    let total_bytes = map_section.len();
    eprintln!("  Total __map__: {}/4096 bytes ({}%)", total_bytes, total_bytes * 100 / 4096);

    // Step 6: Generate Lua metadata
    let mut gen = Vec::new();
    gen.push(format!("-- level: {}x{}, {} tiles, {} layers, {}b", map_w, map_h, num_rt, num_layers, total_bytes));
    gen.push("map_base=0".to_string());
    gen.push(format!("lvl_w={} lvl_h={}", map_w, map_h));
    gen.push(format!("lvl_nt={} lvl_nl={} lvl_nst={} lvl_bg={}", num_rt, num_layers, num_spr_tiles, map_data.map_bg_color));
    if map_data.spawn_x >= 0 {
        gen.push(format!("spn_x={} spn_y={}", map_data.spawn_x, map_data.spawn_y));
    } else {
        gen.push("spn_x=0 spn_y=0".to_string());
    }
    let px_vals: String = map_data.parallax.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
    gen.push(format!("lplx={{{}}}", px_vals));
    let flag_vals: String = (1..=num_rt)
        .map(|rt_id| rt_tile_flags.get(&rt_id).unwrap_or(&0).to_string())
        .collect::<Vec<_>>()
        .join(",");
    gen.push(format!("tflg=split\"{}\"", flag_vals));

    LevelBuildResult {
        map_section,
        num_rt,
        tile_flags: rt_tile_flags,
        gen_lines: gen,
        num_spr_tiles,
    }
}
