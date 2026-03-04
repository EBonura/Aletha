/// Cart assembly: memory allocation, hex conversion, .p8 output.

use crate::config::*;

pub struct DataChunk {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct MemoryLayout {
    pub placements: std::collections::HashMap<String, usize>,
    pub total_used: usize,
    pub gfx_buf: Vec<u8>,
    pub map_buf: Vec<u8>,
    pub gff_buf: Vec<u8>,
    pub sfx_buf: Vec<u8>,
}

/// Pack all chunks into virtual address space, returning physical memory buffers.
pub fn allocate_memory(chunks: &[DataChunk]) -> MemoryLayout {
    let mut vptr = 0usize;
    let mut placements = std::collections::HashMap::new();

    eprintln!("\n=== MEMORY ALLOCATION ===");
    for chunk in chunks {
        let sz = chunk.data.len();
        if sz == 0 {
            placements.insert(chunk.name.clone(), vptr);
            continue;
        }
        if vptr + sz > TOTAL_VIRT {
            eprintln!("  ERROR: {} ({}b) exceeds capacity!", chunk.name, sz);
            break;
        }
        let pa_start = vaddr_to_phys(vptr);
        let pa_end = vaddr_to_phys(vptr + sz - 1);
        let mut regions = Vec::new();
        if pa_start < 0x2000 { regions.push("gfx"); }
        if pa_start < 0x3000 && pa_end >= 0x2000 { regions.push("map"); }
        if pa_start < 0x3100 && pa_end >= 0x3000 { regions.push("gff"); }
        if pa_end >= 0x3200 { regions.push("sfx"); }
        let straddle = if pa_start < 0x3100 && pa_end >= 0x3200 { " *SPANS GAP*" } else { "" };
        eprintln!("  {:12}: 0x{:04x}-0x{:04x}  {:5}b  [{}]{}",
            chunk.name, pa_start, pa_end, sz, regions.join("+"), straddle);
        placements.insert(chunk.name.clone(), vptr);
        vptr += sz;
    }

    let total_used = vptr;
    let gfx_used = total_used.min(0x2000);
    let map_used = total_used.saturating_sub(0x2000).min(0x1000);
    let gff_used = total_used.saturating_sub(0x3000).min(VGAP - 0x3000);
    let sfx_used = total_used.saturating_sub(VGAP);
    let free = TOTAL_VIRT - total_used;
    eprintln!("  -- total: {}/{} ({}%)", total_used, TOTAL_VIRT, total_used * 100 / TOTAL_VIRT);
    eprintln!("     gfx:{}/8192 map:{}/4096 gff:{}/256 sfx:{}/4352", gfx_used, map_used, gff_used, sfx_used);
    eprintln!("     free: {}b", free);

    // Build physical buffers
    let mut gfx_buf = vec![0u8; 8192];
    let mut map_buf = vec![0u8; 4096];
    let mut gff_buf = vec![0u8; 256];
    let mut sfx_buf = vec![0u8; 68 * 64];

    for chunk in chunks {
        if chunk.data.is_empty() { continue; }
        let va = placements[&chunk.name];
        for (i, &b) in chunk.data.iter().enumerate() {
            let pa = vaddr_to_phys(va + i);
            if pa < 0x2000 {
                gfx_buf[pa] = b;
            } else if pa < 0x3000 {
                map_buf[pa - 0x2000] = b;
            } else if pa < 0x3100 {
                gff_buf[pa - 0x3000] = b;
            } else if pa < 0x3200 {
                panic!("BUG: {} byte at phys 0x{:04x} in music gap!", chunk.name, pa);
            } else {
                sfx_buf[pa - 0x3200] = b;
            }
        }
    }

    MemoryLayout {
        placements,
        total_used,
        gfx_buf,
        map_buf,
        gff_buf,
        sfx_buf,
    }
}

/// Build multi-anim chunk: [na][cw][ch][offsets...][data...]
pub fn build_multi_anim_chunk(
    blocks: &[(String, Vec<u8>)],
    cell_w: u32,
    cell_h: u32,
) -> Vec<u8> {
    let na = blocks.len();
    let mut offsets = Vec::new();
    let mut data = Vec::new();
    for (_, blk) in blocks {
        offsets.push(data.len());
        data.extend_from_slice(blk);
    }
    let mut chunk = Vec::new();
    chunk.push(na as u8);
    chunk.push(cell_w as u8);
    chunk.push(cell_h as u8);
    for &off in &offsets {
        chunk.push((off & 0xFF) as u8);
        chunk.push(((off >> 8) & 0xFF) as u8);
    }
    chunk.extend_from_slice(&data);
    chunk
}

/// Build single-anim chunk: [1][cw][ch][0][0][block...]
pub fn build_single_anim_chunk(block: &[u8], cell_w: u32, cell_h: u32) -> Vec<u8> {
    let mut chunk = Vec::new();
    chunk.push(1);
    chunk.push(cell_w as u8);
    chunk.push(cell_h as u8);
    chunk.push(0);
    chunk.push(0);
    chunk.extend_from_slice(block);
    chunk
}

/// Write the final .p8 cart.
pub fn write_p8_cart(
    output_path: &std::path::Path,
    lua_code: &str,
    gfx_hex: &str,
    map_hex: &str,
    gff_hex: &str,
    sfx_hex: &str,
    music_hex: Option<&str>,
) {
    let map_section = format!("\n__map__\n{}", map_hex);
    let gff_section = format!("\n__gff__\n{}", gff_hex);
    let sfx_section = format!("\n__sfx__\n{}", sfx_hex);
    let music_section = music_hex
        .map(|mh| format!("\n__music__\n{}", mh))
        .unwrap_or_default();

    let p8 = format!(
        "pico-8 cartridge // http://www.pico-8.com\nversion 42\n__lua__\n{}\n__gfx__\n{}{}{}{}{}\n",
        lua_code, gfx_hex, map_section, gff_section, sfx_section, music_section
    );

    std::fs::write(output_path, &p8)
        .unwrap_or_else(|e| panic!("Failed to write {}: {}", output_path.display(), e));
    eprintln!("\nWrote cart: {}", output_path.display());
}
