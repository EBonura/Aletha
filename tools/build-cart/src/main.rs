mod animation;
mod cart;
mod config;
mod eg2;
mod frame;
mod level;
mod macrotile;
mod music;
mod rle;
mod tileset;

use image::{GenericImageView, Pixel};
use std::path::{Path, PathBuf};
use std::time::Instant;

use config::*;
use frame::*;

fn main() {
    let t0 = Instant::now();

    // Determine project directory (one arg = project dir, or default to ../../)
    let args: Vec<String> = std::env::args().collect();
    let dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        // Default: assume running from tools/build-cart/
        let exe_dir = std::env::current_exe()
            .map(|p| p.parent().unwrap().to_path_buf())
            .unwrap_or_else(|_| PathBuf::from("."));
        // Walk up to find project root (has ashen_edge.lua)
        let mut dir = exe_dir;
        for _ in 0..5 {
            if dir.join("ashen_edge.lua").exists() {
                break;
            }
            dir = dir.parent().unwrap_or(Path::new(".")).to_path_buf();
        }
        dir
    };

    let asset_dir = dir.join("assets").join("assassin");
    let output_p8 = dir.join("ashen_edge.p8");
    let level_json = dir.join("level_data.json");
    let music_p8 = dir.join("music.p8");
    let tileset_png = dir.join("assets").join("tileset").join("fg_tileset.png");
    let bg_tileset_png = dir.join("assets").join("tileset").join("bg_tileset.png");
    let door_png = dir.join("assets").join("door").join("door open 41x48.png");
    let sw_start_png = dir.join("assets").join("save").join("start up16x19.png");
    let sw_idle_png = dir.join("assets").join("save").join("idle 16x19.png");
    let sw_down_png = dir.join("assets").join("save").join("down 16x19.png");
    let title_png = dir.join("assets").join("title").join("title.png");
    let font_png = dir.join("assets").join("fonts").join("font_sheet.png");
    let spider_dir = dir.join("assets").join("spider");
    let wheelbot_dir = dir.join("assets").join("wheelbot");
    let hellbot_dir = dir.join("assets").join("hellbot");
    let boss_dir = dir.join("assets").join("boss");
    let portal_dir = dir.join("assets").join("portal");
    let torch_src = dir.join("assets").join("torch").join("Torch 16x16.png");
    let box_src = dir.join("assets").join("border_corner.png");
    let hp_src = dir.join("assets").join("hp_bar.png");

    eprintln!("Loading sprite sheets...");
    eprintln!("  Cell size: {}x{}", CELL_W, CELL_H);

    // ── Player animations ──
    eprintln!("\nExtracting frames...");
    let mut all_frames: Vec<(&str, Vec<Vec<u8>>)> = Vec::new();
    for anim in ANIMS {
        let frames = extract_frames(&asset_dir.join(anim.filename), CELL_W, CELL_H, anim.nframes);
        eprintln!("    {}: {} frames from {}", anim.name, frames.len(), anim.filename);
        all_frames.push((anim.name, frames));
    }

    eprintln!("\nCompressing animations...");
    let mut anim_blocks: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total_frames = 0usize;
    for (name, frames) in &all_frames {
        let (block, info) = animation::encode_animation(name, frames, CELL_W, CELL_H, None, None);
        anim_blocks.push((name.to_string(), block));
        total_frames += frames.len();
        eprintln!("{}", info);
    }

    // Build char chunk
    let num_anims_initial = anim_blocks.len();

    // ── Entity animations ──
    eprintln!("\nExtracting entity frames...");
    let door_frames = extract_horiz_frames(&door_png, 41, 48, 48, 48, None, 3);
    eprintln!("    door: {} frames", door_frames.len());
    let sw_start_frames = extract_horiz_frames(&sw_start_png, 16, 19, 16, 19, None, 0);
    eprintln!("    sw_start: {} frames", sw_start_frames.len());
    let sw_idle_frames = extract_horiz_frames(&sw_idle_png, 16, 19, 16, 19, Some(1), 0);
    eprintln!("    sw_idle: {} frames", sw_idle_frames.len());
    let sw_down_frames = extract_horiz_frames(&sw_down_png, 16, 19, 16, 19, None, 0);
    eprintln!("    sw_down: {} frames", sw_down_frames.len());

    let title_frames = extract_horiz_frames(&title_png, 128, 128, 128, 128, Some(1), 0);
    eprintln!("    title: 1 frame");

    eprintln!("\nExtracting font frames...");
    let (font_frames, font_cw, font_ch, font_adv) =
        extract_font_from_png(&font_png, FONT_CHARS, 128);
    eprintln!("    from PNG: {} chars, cell {}x{}", font_frames.len(), font_cw, font_ch);

    eprintln!("\nCompressing entity animations...");
    let ent_anims: Vec<(&str, &Vec<Vec<u8>>, u32, u32)> = vec![
        ("door", &door_frames, 48, 48),
        ("sw_start", &sw_start_frames, 16, 19),
        ("sw_idle", &sw_idle_frames, 16, 19),
        ("sw_down", &sw_down_frames, 16, 19),
    ];
    for (name, frames, cw, ch) in &ent_anims {
        let (block, info) = animation::encode_animation(name, frames, *cw, *ch, None, None);
        anim_blocks.push((name.to_string(), block));
        total_frames += frames.len();
        eprintln!("{}", info);
    }

    eprintln!("\nEncoding title and font...");
    let (title_block, title_info) = animation::encode_animation("title", &title_frames, 128, 128, None, None);
    eprintln!("{}", title_info);
    let (font_block, font_info) = animation::encode_animation("font", &font_frames, font_cw, font_ch, None, None);
    eprintln!("{}", font_info);
    total_frames += title_frames.len() + font_frames.len();
    let title_chunk = {
        let mut c = vec![1u8, 0, 0, 0, 0];
        c.extend_from_slice(&title_block);
        c
    };
    let font_chunk = {
        let mut c = vec![1u8, 0, 0, 0, 0];
        c.extend_from_slice(&font_block);
        c
    };
    eprintln!("  title_chunk: {}b  font_chunk: {}b", title_chunk.len(), font_chunk.len());

    // ── Spider enemy ──
    eprintln!("\nExtracting spider frames...");
    let spider_anims = config::spider_anims();
    let mut spider_anim_blocks: Vec<(String, Vec<u8>)> = Vec::new();
    let mut spider_all_frames: Vec<(&str, Vec<Vec<u8>>)> = Vec::new();
    for sp in &spider_anims {
        let mut frames = Vec::new();
        for (fi, file) in sp.files.iter().enumerate() {
            let nf = sp.nframes[fi];
            let mut f = extract_frames(&spider_dir.join(file), SPIDER_W, SPIDER_H, nf);
            frames.append(&mut f);
        }
        let (block, info) = animation::encode_animation(sp.name, &frames, SPIDER_W, SPIDER_H, None, None);
        spider_anim_blocks.push((sp.name.to_string(), block));
        total_frames += frames.len();
        eprintln!("{}", info);
        spider_all_frames.push((sp.name, frames));
    }
    let spider_chunk = cart::build_multi_anim_chunk(&spider_anim_blocks, SPIDER_W, SPIDER_H);
    eprintln!("  spider_chunk: {}b", spider_chunk.len());
    let sp_anc_parts: Vec<String> = spider_all_frames.iter().map(|(_, frames)| {
        compute_anchors(frames, SPIDER_W, None).iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
    }).collect();
    let sp_anc_str = sp_anc_parts.join("|");

    // ── Wheel Bot enemy ──
    eprintln!("\nExtracting wheel bot frames...");
    let wb_anims = config::wheelbot_anims();
    let mut wb_anim_blocks: Vec<(String, Vec<u8>)> = Vec::new();
    let mut wb_all_frames: Vec<(&str, Vec<Vec<u8>>)> = Vec::new();
    for wb in &wb_anims {
        let mut frames = extract_frames_boss(
            &wheelbot_dir.join(wb.filename),
            wb.src_fw, wb.src_fh,
            WHEELBOT_W, WHEELBOT_H,
            wb.frame_select.as_deref(),
        );
        // Merge lavender(13) -> light grey(6)
        for f in &mut frames {
            for c in f.iter_mut() {
                if *c == 13 { *c = 6; }
            }
        }
        let (block, info) = animation::encode_animation(wb.name, &frames, WHEELBOT_W, WHEELBOT_H, None, None);
        wb_anim_blocks.push((wb.name.to_string(), block));
        total_frames += frames.len();
        eprintln!("{}", info);
        wb_all_frames.push((wb.name, frames));
    }
    let wheelbot_chunk = cart::build_multi_anim_chunk(&wb_anim_blocks, WHEELBOT_W, WHEELBOT_H);
    eprintln!("  wheelbot_chunk: {}b", wheelbot_chunk.len());
    let wb_anc_parts: Vec<String> = wb_all_frames.iter().map(|(_, frames)| {
        compute_anchors(frames, WHEELBOT_W, None).iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
    }).collect();
    let wb_anc_str = wb_anc_parts.join("|");

    // ── Hell Bot enemy ──
    eprintln!("\nExtracting hell bot frames...");
    let hb_anims = config::hellbot_anims();
    let mut hb_anim_blocks: Vec<(String, Vec<u8>)> = Vec::new();
    let mut hb_all_frames: Vec<(&str, Vec<Vec<u8>>)> = Vec::new();
    for hb in &hb_anims {
        let mut frames = extract_frames(&hellbot_dir.join(hb.filename), HELLBOT_W, HELLBOT_H, hb.nframes);
        // Merge dark_blue(1) and dark_grey(5) -> black(0)
        for f in &mut frames {
            for c in f.iter_mut() {
                if *c == 1 || *c == 5 { *c = 0; }
            }
        }
        let (block, info) = animation::encode_animation(hb.name, &frames, HELLBOT_W, HELLBOT_H, Some(2), None);
        hb_anim_blocks.push((hb.name.to_string(), block));
        total_frames += frames.len();
        eprintln!("{}", info);
        hb_all_frames.push((hb.name, frames));
    }
    let hellbot_chunk = cart::build_multi_anim_chunk(&hb_anim_blocks, HELLBOT_W, HELLBOT_H);
    eprintln!("  hellbot_chunk: {}b", hellbot_chunk.len());
    let hb_anc_parts: Vec<String> = hb_all_frames.iter().map(|(_, frames)| {
        compute_anchors(frames, HELLBOT_W, None).iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
    }).collect();
    let hb_anc_str = hb_anc_parts.join("|");

    // ── Blood King boss ──
    eprintln!("\nExtracting Blood King boss frames...");
    let bk_anims = config::boss_anims();
    let mut bk_anim_blocks: Vec<(String, Vec<u8>)> = Vec::new();
    let mut bk_all_frames: Vec<(&str, Vec<Vec<u8>>)> = Vec::new();
    for bk in &bk_anims {
        let mut frames = extract_frames_boss(
            &boss_dir.join(bk.filename),
            bk.src_fw, bk.src_fh,
            BOSS_W, BOSS_H,
            bk.frame_select.as_deref(),
        );
        // Remap: dark->0, red/warm->8 (forces 1bpp)
        for f in &mut frames {
            for c in f.iter_mut() {
                if *c == TRANS {
                    // keep
                } else if *c == 4 || *c == 8 || *c == 9 || *c == 15 {
                    *c = 8;
                } else {
                    *c = 0;
                }
            }
        }
        let (block, info) = animation::encode_animation(bk.name, &frames, BOSS_W, BOSS_H, None, None);
        bk_anim_blocks.push((bk.name.to_string(), block));
        total_frames += frames.len();
        eprintln!("{}", info);
        bk_all_frames.push((bk.name, frames));
    }
    let boss_chunk = cart::build_multi_anim_chunk(&bk_anim_blocks, BOSS_W, BOSS_H);
    eprintln!("  boss_chunk: {}b", boss_chunk.len());
    let bk_anc_parts: Vec<String> = bk_all_frames.iter().map(|(_, frames)| {
        compute_anchors(frames, BOSS_W, None).iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
    }).collect();
    let bk_anc_str = bk_anc_parts.join("|");

    // ── Portal checkpoint ──
    eprintln!("\nExtracting portal frames...");
    let portal_frames = {
        let img = image::open(&portal_dir.join("idle 28x41.png")).expect("portal PNG");
        let nf = img.dimensions().0 / PORTAL_SRC_W;
        let mut frames = Vec::new();
        for f in 0..nf {
            let mut pixels = Vec::new();
            for y in PORTAL_CROP_Y..PORTAL_SRC_H {
                for x in 0..PORTAL_W {
                    let px = img.get_pixel(f * PORTAL_SRC_W + x, y);
                    let ch = px.0;
                    if ch[3] == 0 {
                        pixels.push(TRANS);
                    } else {
                        pixels.push(nearest_p8(ch[0], ch[1], ch[2]));
                    }
                }
            }
            frames.push(pixels);
        }
        frames
    };
    let (ptl_block, ptl_info) = animation::encode_animation("ptl_idle", &portal_frames, PORTAL_W, PORTAL_H, Some(2), None);
    total_frames += portal_frames.len();
    eprintln!("{}", ptl_info);
    let portal_chunk = cart::build_single_anim_chunk(&ptl_block, PORTAL_W, PORTAL_H);
    eprintln!("  portal_chunk: {}b", portal_chunk.len());

    // ── Torch ──
    eprintln!("\nExtracting torch frames...");
    let torch_frames = {
        let img = image::open(&torch_src).expect("torch PNG");
        let nf = img.dimensions().1 / TORCH_H;
        let mut frames = Vec::new();
        for f in 0..nf {
            let mut pixels = Vec::new();
            for y in 0..TORCH_H {
                for x in 0..TORCH_W {
                    let px = img.get_pixel(x, f * TORCH_H + y);
                    let ch = px.0;
                    if ch[3] == 0 {
                        pixels.push(TRANS);
                    } else {
                        let mut c = nearest_p8(ch[0], ch[1], ch[2]);
                        // Remap grays to red tones (skip last frame = unlit)
                        if f < nf - 1 {
                            c = match c {
                                1 => 2, 5 => 2, 6 => 8, 7 => 8, 13 => 8,
                                _ => c,
                            };
                        }
                        pixels.push(c);
                    }
                }
            }
            frames.push(pixels);
        }
        frames
    };
    let (torch_block, torch_info) = animation::encode_animation("torch", &torch_frames, TORCH_W, TORCH_H, Some(2), None);
    total_frames += torch_frames.len();
    eprintln!("{}", torch_info);
    let torch_chunk = cart::build_single_anim_chunk(&torch_block, TORCH_W, TORCH_H);
    eprintln!("  torch_chunk: {}b", torch_chunk.len());

    // ── Box corner ──
    eprintln!("\nExtracting box corner...");
    let box_pixels = {
        let img = image::open(&box_src).expect("box PNG");
        let mut pixels = Vec::new();
        for y in 0..BOX_S {
            for x in 0..BOX_S {
                let px = img.get_pixel(x, y);
                let ch = px.0;
                if ch[3] == 0 {
                    pixels.push(TRANS);
                } else if ch[0] > 200 {
                    pixels.push(7); // white
                } else if ch[0] > 100 {
                    pixels.push(5); // grey
                } else {
                    pixels.push(0); // dark
                }
            }
        }
        pixels
    };
    let box_pal = vec![TRANS, 0, 7, 5];
    let (box_block, box_info) = animation::encode_animation(
        "box_corner", &[box_pixels], BOX_S, BOX_S, Some(2), Some(&box_pal));
    eprintln!("{}", box_info);
    let box_chunk = cart::build_single_anim_chunk(&box_block, BOX_S, BOX_S);
    eprintln!("  box_chunk: {}b", box_chunk.len());

    // ── HP bar ──
    eprintln!("\nEncoding HP bar...");
    let (hp_pixels, hp_w, hp_h) = {
        let img = image::open(&hp_src).expect("hp_bar PNG");
        let (w, h) = img.dimensions();
        let mut pixels = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let px = img.get_pixel(x, y);
                let ch = px.0;
                if ch[3] < 128 {
                    pixels.push(TRANS);
                } else if ch[0] > 200 && ch[1] > 200 && ch[2] > 200 {
                    pixels.push(7); // white = fill
                } else {
                    pixels.push(0); // black = outline
                }
            }
        }
        (pixels, w, h)
    };
    let (hp_block, hp_info) = animation::encode_animation("hp_bar", &[hp_pixels], hp_w, hp_h, None, None);
    eprintln!("{}", hp_info);
    let hp_chunk = {
        let mut c = vec![1u8, 0, 0, 0, 0];
        c.extend_from_slice(&hp_block);
        c
    };
    eprintln!("  hp_chunk: {}b", hp_chunk.len());

    // ── Rebuild char chunk ──
    let num_anims = anim_blocks.len();
    let mut anim_offsets = Vec::new();
    let mut anim_data = Vec::new();
    for (_, block) in &anim_blocks {
        anim_offsets.push(anim_data.len());
        anim_data.extend_from_slice(block);
    }
    let mut char_chunk = Vec::new();
    char_chunk.push(num_anims as u8);
    char_chunk.push(CELL_W as u8);
    char_chunk.push(CELL_H as u8);
    for &off in &anim_offsets {
        char_chunk.push((off & 0xFF) as u8);
        char_chunk.push(((off >> 8) & 0xFF) as u8);
    }
    char_chunk.extend_from_slice(&anim_data);

    // ── Tilesets & Level data ──
    eprintln!("\nLoading tilesets...");
    let mut active_band_colors = BAND_COLORS;
    let mut bg_active_band_colors = BAND_COLORS;

    let tileset_tiles = tileset::slice_tileset(&tileset_png, &active_band_colors);
    eprintln!("  {} unique main tiles from tileset", tileset_tiles.len());
    let bg_tileset_tiles = tileset::slice_bg_tileset(&bg_tileset_png, &bg_active_band_colors);
    eprintln!("  {} unique BG tiles from bg_tileset", bg_tileset_tiles.len());

    let mut map_level_data = Vec::new();
    let mut num_spr_tiles = 0usize;
    let mut level_gen_lines: Vec<String> = Vec::new();

    if level_json.exists() {
        eprintln!("\nReading level data from {}...", level_json.display());
        if let Some(map_data) = level::read_level_json(&level_json) {
            // Update band colors from level data
            if let Some(bc) = map_data.band_colors {
                active_band_colors = bc;
            }
            if let Some(bc) = map_data.bg_band_colors {
                bg_active_band_colors = bc;
            } else {
                bg_active_band_colors = active_band_colors;
            }

            let result = level::build_level_data(
                &tileset_tiles, &bg_tileset_tiles, &map_data,
                &active_band_colors, &bg_active_band_colors,
            );
            map_level_data = result.map_section;
            num_spr_tiles = result.num_spr_tiles;
            level_gen_lines = result.gen_lines;
        }
    }

    // ── Memory allocation ──
    let data_chunks = vec![
        cart::DataChunk { name: "char".into(), data: char_chunk },
        cart::DataChunk { name: "spider".into(), data: spider_chunk },
        cart::DataChunk { name: "wheelbot".into(), data: wheelbot_chunk },
        cart::DataChunk { name: "hellbot".into(), data: hellbot_chunk },
        cart::DataChunk { name: "boss".into(), data: boss_chunk },
        cart::DataChunk { name: "portal".into(), data: portal_chunk },
        cart::DataChunk { name: "torch".into(), data: torch_chunk },
        cart::DataChunk { name: "box".into(), data: box_chunk },
        cart::DataChunk { name: "hp".into(), data: hp_chunk },
        cart::DataChunk { name: "level".into(), data: map_level_data },
        cart::DataChunk { name: "title".into(), data: title_chunk },
        cart::DataChunk { name: "font".into(), data: font_chunk },
    ];

    let layout = cart::allocate_memory(&data_chunks);
    let total_used = layout.total_used;
    let sfx_used = total_used.saturating_sub(VGAP);

    // ── Music ──
    let mut music_buf = vec![0u8; 256];
    let mut sfx_buf = layout.sfx_buf.clone();
    let mut sfx_shift = 0usize;

    if music_p8.exists() {
        if let Some((music_sfx, music_pat)) = music::load_music_cart(&music_p8) {
            let sfx_data_slots = if sfx_used > 0 { (sfx_used + 67) / 68 } else { 0 };
            let shift = sfx_data_slots;
            sfx_shift = shift;
            let mut audio_slots = 0;

            for src_slot in 0..64 {
                let slot_data = &music_sfx[src_slot * 68..(src_slot + 1) * 68];
                // Check if note data (first 64 bytes) has any content;
                // skip slots with only editor header bytes (speed/loop) but no notes
                let has_notes = slot_data[..64].iter().any(|&b| b != 0);
                if has_notes {
                    let dst_slot = src_slot + shift;
                    if dst_slot >= 64 {
                        eprintln!("  ERROR: audio SFX {} remapped to {} (out of range)!", src_slot, dst_slot);
                        continue;
                    }
                    sfx_buf[dst_slot * 68..(dst_slot + 1) * 68].copy_from_slice(slot_data);
                    audio_slots += 1;
                }
            }

            music_buf = music_pat.clone();
            for i in 0..64 {
                let raw: Vec<u8> = (0..4).map(|c| music_pat[i * 4 + c]).collect();
                if raw.iter().all(|&b| b == 0) {
                    for ch in 0..4 {
                        music_buf[i * 4 + ch] = 0x40;
                    }
                    continue;
                }
                for ch in 0..4 {
                    let b = music_buf[i * 4 + ch];
                    let idx = (b & 0x3F) as usize;
                    let flags = b & 0xC0;
                    let new_idx = idx + shift;
                    if new_idx >= 64 {
                        eprintln!("  ERROR: music pattern {} ch{} SFX {}→{} out of range!", i, ch, idx, new_idx);
                    } else {
                        music_buf[i * 4 + ch] = flags | (new_idx as u8 & 0x3F);
                    }
                }
            }

            eprintln!("\nLoaded music from {}:", music_p8.display());
            eprintln!("  {} audio SFX (remapped +{})", audio_slots, shift);
        }
    }

    // ── Generate Lua metadata ──
    let mut gen_lines: Vec<String> = Vec::new();
    gen_lines.push(format!("-- {} frames, {} anims, {}b vmem", total_frames, num_anims, total_used));

    if sfx_used > 0 {
        gen_lines.push(format!(
            "do local _p=peek peek=function(a,n) if a>=0x3100 and a<0x{:04x} then a+=0x100 end if n then return _p(a,n) else return _p(a) end end end",
            total_used
        ));
    }

    gen_lines.push("char_base=0".into());
    gen_lines.push(format!("cell_w={} cell_h={}", CELL_W, CELL_H));
    gen_lines.push(format!("trans={}", TRANS));

    // Player anim indices
    let anim_vars = ["a_idle","a_run","a_jump","a_fall","a_hit","a_land","a_atk1","a_xslice","a_sweep","a_death"];
    let lhs = anim_vars.join(",");
    let rhs: String = (1..=ANIMS.len()).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    gen_lines.push(format!("{}=unpack(split\"{}\")", lhs, rhs));

    // Entity anim indices
    let ent_vars = ["a_door","a_sst","a_sid","a_sdn"];
    let ent_lhs = ent_vars.join(",");
    let ent_rhs: String = (0..ent_anims.len()).map(|i| (ANIMS.len() + i + 1).to_string()).collect::<Vec<_>>().join(",");
    gen_lines.push(format!("{}=unpack(split\"{}\")", ent_lhs, ent_rhs));

    let num_main = ANIMS.len() + ent_anims.len();
    gen_lines.push(format!("a_title={} a_font={}", num_main + 1, num_main + 2));
    gen_lines.push(format!("title_base={} font_base={}", layout.placements["title"], layout.placements["font"]));
    gen_lines.push(format!("font_cw={} font_ch={}", font_cw, font_ch));
    let adv_str: String = font_adv.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(",");
    gen_lines.push(format!("font_adv=split\"{}\"", adv_str));

    // Spider
    let sp_vars = ["a_spi","a_spw","a_spa","a_sph","a_spd"];
    let sp_base_idx = num_main + 3;
    let sp_lhs = sp_vars.join(",");
    let sp_rhs: String = (0..spider_anims.len()).map(|i| (sp_base_idx + i).to_string()).collect::<Vec<_>>().join(",");
    gen_lines.push(format!("{}=unpack(split\"{}\")", sp_lhs, sp_rhs));
    gen_lines.push(format!("spider_base={} spider_cw={} spider_ch={}", layout.placements["spider"], SPIDER_W, SPIDER_H));
    gen_lines.push(format!("_sa=split(\"{}\",\"|\",false)", sp_anc_str));
    gen_lines.push("sp_anc={} for i=1,#_sa do sp_anc[a_spi+i-1]=split(_sa[i]) end".into());

    // Wheel bot
    let wb_vars = ["a_wbi","a_wbm","a_wbc","a_wbs","a_wbfd","a_wbwk","a_wbd","a_wbdt"];
    let wb_base_idx = sp_base_idx + spider_anims.len();
    let wb_lhs = wb_vars.join(",");
    let wb_rhs: String = (0..wb_anims.len()).map(|i| (wb_base_idx + i).to_string()).collect::<Vec<_>>().join(",");
    gen_lines.push(format!("{}=unpack(split\"{}\")", wb_lhs, wb_rhs));
    gen_lines.push(format!("wheelbot_base={} wheelbot_cw={} wheelbot_ch={}", layout.placements["wheelbot"], WHEELBOT_W, WHEELBOT_H));
    gen_lines.push(format!("_wa=split(\"{}\",\"|\",false)", wb_anc_str));
    gen_lines.push("wb_anc={} for i=1,#_wa do wb_anc[a_wbi+i-1]=split(_wa[i]) end".into());

    // Hell bot
    let hb_vars = ["a_hbi","a_hbr","a_hba","a_hbs","a_hbh","a_hbd"];
    let hb_base_idx = wb_base_idx + wb_anims.len();
    let hb_lhs = hb_vars.join(",");
    let hb_rhs: String = (0..hb_anims.len()).map(|i| (hb_base_idx + i).to_string()).collect::<Vec<_>>().join(",");
    gen_lines.push(format!("{}=unpack(split\"{}\")", hb_lhs, hb_rhs));
    gen_lines.push(format!("hellbot_base={} hellbot_cw={} hellbot_ch={}", layout.placements["hellbot"], HELLBOT_W, HELLBOT_H));
    gen_lines.push(format!("_ha=split(\"{}\",\"|\",false)", hb_anc_str));
    gen_lines.push("hb_anc={} for i=1,#_ha do hb_anc[a_hbi+i-1]=split(_ha[i]) end".into());

    // Blood King boss
    let bk_vars = ["a_bki","a_bkr","a_bka","a_bkc","a_bkh","a_bkd"];
    let bk_base_idx = hb_base_idx + hb_anims.len();
    let bk_lhs = bk_vars.join(",");
    let bk_rhs: String = (0..bk_anims.len()).map(|i| (bk_base_idx + i).to_string()).collect::<Vec<_>>().join(",");
    gen_lines.push(format!("{}=unpack(split\"{}\")", bk_lhs, bk_rhs));
    gen_lines.push(format!("boss_base={} boss_cw={} boss_ch={}", layout.placements["boss"], BOSS_W, BOSS_H));
    gen_lines.push(format!("_bka=split(\"{}\",\"|\",false)", bk_anc_str));
    gen_lines.push("bk_anc={} for i=1,#_bka do bk_anc[a_bki+i-1]=split(_bka[i]) end".into());

    // Portal
    let ptl_base_idx = bk_base_idx + bk_anims.len();
    gen_lines.push(format!("a_ptl={}", ptl_base_idx));
    gen_lines.push(format!("portal_base={} portal_cw={} portal_ch={}", layout.placements["portal"], PORTAL_W, PORTAL_H));

    // Torch
    let torch_base_idx = ptl_base_idx + 1;
    gen_lines.push(format!("a_torch={}", torch_base_idx));
    gen_lines.push(format!("torch_base={} torch_cw={} torch_ch={}", layout.placements["torch"], TORCH_W, TORCH_H));

    // Box corner
    let box_base_idx = torch_base_idx + 1;
    gen_lines.push(format!("a_box={}", box_base_idx));
    gen_lines.push(format!("box_base={} box_s={}", layout.placements["box"], BOX_S));

    // Animation speed table
    let aspd = config::aspd_table(
        ANIMS.len(), ent_anims.len(),
        spider_anims.len(), wb_anims.len(), hb_anims.len(), bk_anims.len(),
    );
    let aspd_str: String = aspd.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
    gen_lines.push(format!("aspd=split\"{}\"", aspd_str));

    // HP bar
    gen_lines.push(format!("hp_base={} hp_w={} hp_h={}", layout.placements["hp"], hp_w, hp_h));
    gen_lines.push(format!("sfx_confirm={}", 6 + sfx_shift));

    // Font lookup
    let fc_escaped: String = FONT_CHARS.chars().map(|c| {
        let o = c as u32;
        if o >= 128 {
            format!("\\{:03}", o)
        } else if c == '\'' {
            "\\'".to_string()
        } else if c == '\\' {
            "\\\\".to_string()
        } else {
            c.to_string()
        }
    }).collect();
    gen_lines.push(format!("_fc='{}'", fc_escaped));
    gen_lines.push("font_map={} for i=1,#_fc do font_map[ord(sub(_fc,i,i))]=i end".into());

    // Player anchors
    let anc_parts: Vec<String> = all_frames.iter().map(|(_, frames)| {
        compute_anchors(frames, CELL_W, Some(0)).iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
    }).collect();
    let anc_str = anc_parts.join("|");
    gen_lines.push(format!("_a=split(\"{}\",\"|\",false)", anc_str));
    gen_lines.push("anc={} for i=1,#_a do anc[i]=split(_a[i]) end".into());

    // Level gen lines
    if !level_gen_lines.is_empty() {
        let map_base = layout.placements.get("level").copied().unwrap_or(0);
        for line in &mut level_gen_lines {
            if line.starts_with("map_base=") {
                *line = format!("map_base={}", map_base);
            }
        }
    }

    // Zone texts
    if let Some(map_data) = level::read_level_json(&level_json) {
        if !map_data.zone_texts.is_empty() {
            let glyph_sub = |t: &str| -> String {
                let mut s = t.replace('\u{fe0f}', "");
                let glyphs: &[(char, u8)] = &[
                    ('\u{2b05}', 139), ('\u{2b06}', 148), ('\u{27a1}', 145), ('\u{2b07}', 131),
                    ('\u{1f17e}', 142), ('\u{274e}', 151),
                    ('\u{2039}', 139), ('\u{201c}', 148), ('\u{2018}', 145), ('\u{201d}', 148),
                    ('\u{0192}', 131), ('\u{017d}', 142), ('\u{2014}', 151),
                ];
                for &(uc, p8) in glyphs {
                    s = s.replace(uc, &String::from(char::from(p8)));
                }
                s
            };
            let zone_texts: Vec<String> = map_data.zone_texts.iter().map(|t| glyph_sub(t)).collect();
            let escaped: Vec<String> = zone_texts.iter().map(|t| {
                let mut out = String::new();
                for c in t.chars() {
                    let o = c as u32;
                    if o >= 128 {
                        out.push_str(&format!("\\{:03}", o));
                    } else if c == '\\' {
                        out.push_str("\\\\");
                    } else if c == '"' {
                        out.push_str("\\\"");
                    } else if c == '\n' {
                        out.push_str("\\n");
                    } else {
                        out.push(c);
                    }
                }
                out
            }).collect();
            let txt_entries: String = escaped.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(",");
            gen_lines.push(format!("_zt={{{}}}", txt_entries));
            eprintln!("\n  Zone texts: {} entries", zone_texts.len());
        } else {
            gen_lines.push("_zt={}".into());
        }
    } else {
        gen_lines.push("_zt={}".into());
    }

    // ── Combine and write ──
    let generated_block = gen_lines.join("\n");
    let full_generated = if level_gen_lines.is_empty() {
        generated_block
    } else {
        format!("{}\n{}", generated_block, level_gen_lines.join("\n"))
    };

    // Read game lua template
    let game_lua_path = dir.join("ashen_edge.lua");
    let game_lua = std::fs::read_to_string(&game_lua_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", game_lua_path.display(), e));

    let marker_start = "--##generated##";
    let marker_end = "--##end##";
    let i0 = game_lua.find(marker_start).expect("Missing --##generated## marker");
    let i1 = game_lua.find(marker_end).expect("Missing --##end## marker") + marker_end.len();
    let lua_code = format!(
        "{}{}\n{}\n{}{}",
        &game_lua[..i0],
        marker_start,
        full_generated,
        marker_end,
        &game_lua[i1..]
    );

    // Convert buffers to hex
    let gfx_hex = music::bytes_to_gfx(&layout.gfx_buf);
    let map_hex = music::bytes_to_map_hex(&layout.map_buf);
    let gff_hex = music::bytes_to_gff_hex(&layout.gff_buf);
    let sfx_hex = music::bytes_to_sfx_hex(&sfx_buf);

    let has_music = music_buf.iter().any(|&b| b != 0);
    let music_hex_str = if has_music {
        Some(music::music_hex(&music_buf))
    } else {
        None
    };

    cart::write_p8_cart(
        &output_p8,
        &lua_code,
        &gfx_hex,
        &map_hex,
        &gff_hex,
        &sfx_hex,
        music_hex_str.as_deref(),
    );

    let elapsed = t0.elapsed();
    eprintln!("\nBuild completed in {:.2}s", elapsed.as_secs_f64());
}
