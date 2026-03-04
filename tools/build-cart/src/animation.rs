/// Animation encoding types 0-6 and the best-pick selector.

use crate::eg2::eg2_encode_frame;
use crate::frame::*;
use crate::rle::*;

/// Type 0: Keyframe + Delta (animation-wide bbox, combinatorial keyframe search).
fn encode_type0(
    frames_pixels: &[Vec<u8>],
    fw: u32,
    fh: u32,
    bpp: u8,
    palette: Option<&[u8]>,
) -> (Vec<u8>, String) {
    let n = frames_pixels.len();
    let (bx, by, bw, bh) = get_bbox(frames_pixels, fw, fh);
    let mut cropped: Vec<Vec<u8>> = frames_pixels
        .iter()
        .map(|f| crop_pixels(f, fw, bx, by, bw, bh))
        .collect();
    if bpp < 4 {
        if let Some(pal) = palette {
            cropped = cropped.iter().map(|f| quantize_pixels(f, pal)).collect();
        }
    }

    // Generate keyframe candidates
    let candidates = pick_keyframe_candidates(n);

    let mut best_block: Option<Vec<u8>> = None;
    let mut best_info = String::new();

    for key_indices in &candidates {
        let nkeys = key_indices.len();
        // Assign each frame to closest keyframe
        let assignments: Vec<usize> = (0..n)
            .map(|i| {
                (0..nkeys)
                    .min_by_key(|&ki| count_diffs(&cropped[key_indices[ki]], &cropped[i]))
                    .unwrap()
            })
            .collect();

        let key_rles: Vec<Vec<u8>> = key_indices
            .iter()
            .map(|&ki| ext_nibble_rle_encode(&cropped[ki], bpp))
            .collect();

        let deltas: Vec<Vec<u8>> = (0..n)
            .map(|i| {
                let base_idx = key_indices[assignments[i]];
                delta_encode_skip(&cropped[base_idx], &cropped[i])
            })
            .collect();

        // Build data blob
        let mut data = Vec::new();
        for kr in &key_rles {
            data.extend_from_slice(kr);
        }
        let mut delta_offsets = Vec::new();
        for d in &deltas {
            delta_offsets.push(data.len());
            data.extend_from_slice(d);
        }

        // Build block
        let mut block = Vec::new();
        block.push(n as u8);
        block.push(0); // type 0
        block.push(bpp);
        if bpp < 4 {
            if let Some(pal) = palette {
                block.extend_from_slice(&pack_palette(pal));
            }
        }
        block.push(nkeys as u8);
        block.push(bx);
        block.push(by);
        block.push(bw);
        block.push(bh);
        for &ki in key_indices {
            block.push(ki as u8);
        }
        for kr in &key_rles {
            block.push((kr.len() & 0xFF) as u8);
            block.push(((kr.len() >> 8) & 0xFF) as u8);
        }
        for &a in &assignments {
            block.push(a as u8);
        }
        for &off in &delta_offsets {
            block.push((off & 0xFF) as u8);
            block.push(((off >> 8) & 0xFF) as u8);
        }
        block.extend_from_slice(&data);

        if best_block.is_none() || block.len() < best_block.as_ref().unwrap().len() {
            let total_keys: usize = key_rles.iter().map(|kr| kr.len()).sum();
            best_info = format!("KD {}k {}x{} keys={}b", nkeys, bw, bh, total_keys);
            best_block = Some(block);
        }
    }

    (best_block.unwrap(), best_info)
}

fn pick_keyframe_candidates(n: usize) -> Vec<Vec<usize>> {
    let mut candidates = Vec::new();
    let max_keys = if n > 16 {
        n.min(2)
    } else if n <= 8 {
        n.min(4)
    } else {
        n.min(3)
    };

    for k in 1..=max_keys {
        if n > 12 && k >= 3 {
            let step = n / k;
            let base: Vec<usize> = (0..k).map(|i| i * step).collect();
            candidates.push(base.clone());
            for offset in 1..3usize.min(step) {
                candidates.push(base.iter().map(|&b| (b + offset) % n).collect());
            }
        } else {
            // All combinations of k from n
            let indices: Vec<usize> = (0..n).collect();
            for combo in combinations(&indices, k) {
                candidates.push(combo);
            }
        }
    }
    candidates
}

fn combinations(items: &[usize], k: usize) -> Vec<Vec<usize>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    let mut result = Vec::new();
    for (i, &item) in items.iter().enumerate() {
        for mut rest in combinations(&items[i + 1..], k - 1) {
            let mut combo = vec![item];
            combo.append(&mut rest);
            result.push(combo);
        }
    }
    result
}

/// Type 1: Per-frame independent RLE with per-frame bboxes.
fn encode_type1(
    frames_pixels: &[Vec<u8>],
    fw: u32,
    fh: u32,
    bpp: u8,
    palette: Option<&[u8]>,
) -> (Vec<u8>, String) {
    let n = frames_pixels.len();
    let mut frame_datas = Vec::new();

    for f in frames_pixels {
        let (bx, by, bw, bh) = get_frame_bbox(f, fw, fh);
        if bw == 0 || bh == 0 {
            frame_datas.push(vec![0u8, 0, 0, 0]);
        } else {
            let mut cropped = crop_pixels(f, fw, bx, by, bw, bh);
            if bpp < 4 {
                if let Some(pal) = palette {
                    cropped = quantize_pixels(&cropped, pal);
                }
            }
            let rle = ext_nibble_rle_encode(&cropped, bpp);
            let mut fd = vec![bx, by, bw, bh];
            fd.extend_from_slice(&rle);
            frame_datas.push(fd);
        }
    }

    let mut block = Vec::new();
    block.push(n as u8);
    block.push(1); // type 1
    block.push(bpp);
    if bpp < 4 {
        if let Some(pal) = palette {
            block.extend_from_slice(&pack_palette(pal));
        }
    }
    let mut offset = 0u16;
    for fd in &frame_datas {
        block.push((offset & 0xFF) as u8);
        block.push(((offset >> 8) & 0xFF) as u8);
        offset += fd.len() as u16;
    }
    for fd in &frame_datas {
        block.extend_from_slice(fd);
    }

    (block, "PF".to_string())
}

/// Type 2: Sequential XOR + RLE (animation-wide bbox).
fn encode_type2(
    frames_pixels: &[Vec<u8>],
    fw: u32,
    fh: u32,
    bpp: u8,
    palette: Option<&[u8]>,
) -> (Vec<u8>, String) {
    let n = frames_pixels.len();
    let (bx, by, bw, bh) = get_bbox(frames_pixels, fw, fh);
    let mut cropped: Vec<Vec<u8>> = frames_pixels
        .iter()
        .map(|f| crop_pixels(f, fw, bx, by, bw, bh))
        .collect();
    if bpp < 4 {
        if let Some(pal) = palette {
            cropped = cropped.iter().map(|f| quantize_pixels(f, pal)).collect();
        }
    }

    let mut frame_rles = vec![ext_nibble_rle_encode(&cropped[0], bpp)];
    for i in 1..n {
        let xor_diff: Vec<u8> = cropped[i]
            .iter()
            .zip(cropped[i - 1].iter())
            .map(|(&a, &b)| a ^ b)
            .collect();
        frame_rles.push(ext_nibble_rle_encode(&xor_diff, bpp));
    }

    let mut block = Vec::new();
    block.push(n as u8);
    block.push(2); // type 2
    block.push(bpp);
    if bpp < 4 {
        if let Some(pal) = palette {
            block.extend_from_slice(&pack_palette(pal));
        }
    }
    block.push(bx);
    block.push(by);
    block.push(bw);
    block.push(bh);
    let mut offset = 0u16;
    for rle in &frame_rles {
        block.push((offset & 0xFF) as u8);
        block.push(((offset >> 8) & 0xFF) as u8);
        offset += rle.len() as u16;
    }
    for rle in &frame_rles {
        block.extend_from_slice(rle);
    }

    let total: usize = frame_rles.iter().map(|r| r.len()).sum();
    (block, format!("XR {}x{} data={}b", bw, bh, total))
}

/// Type 3: Referenced XOR + RLE (with optional row-delta).
fn encode_type3(
    frames_pixels: &[Vec<u8>],
    fw: u32,
    fh: u32,
    bpp: u8,
    palette: Option<&[u8]>,
    use_rowdelta: bool,
) -> (Vec<u8>, String) {
    let n = frames_pixels.len();
    let (bx, by, bw, bh) = get_bbox(frames_pixels, fw, fh);
    let mut cropped: Vec<Vec<u8>> = frames_pixels
        .iter()
        .map(|f| crop_pixels(f, fw, bx, by, bw, bh))
        .collect();
    if bpp < 4 {
        if let Some(pal) = palette {
            cropped = cropped.iter().map(|f| quantize_pixels(f, pal)).collect();
        }
    }

    let npix = bw as usize * bh as usize;
    let use_bitpack = bpp == 1;

    let encode_frame = |pixels: &[u8]| -> Vec<u8> {
        let px = if use_rowdelta {
            row_delta(pixels, bw as usize, bh as usize)
        } else {
            pixels.to_vec()
        };
        if use_bitpack {
            pack_bits(&px)
        } else {
            ext_nibble_rle_encode(&px, bpp)
        }
    };

    let mut refs = Vec::new();
    let mut frame_data = Vec::new();
    for i in 0..n {
        let mut best_ref = 255u8;
        let mut best_enc = encode_frame(&cropped[i]);
        for r in 0..i {
            let xor_diff: Vec<u8> = cropped[i]
                .iter()
                .zip(cropped[r].iter())
                .map(|(&a, &b)| a ^ b)
                .collect();
            let enc = encode_frame(&xor_diff);
            if enc.len() < best_enc.len() {
                best_enc = enc;
                best_ref = r as u8;
            }
        }
        refs.push(best_ref);
        frame_data.push(best_enc);
    }

    let enc_byte = 3u8 | if use_rowdelta { 0x80 } else { 0 };
    let mut block = Vec::new();
    block.push(n as u8);
    block.push(enc_byte);
    block.push(bpp);
    if bpp < 4 {
        if let Some(pal) = palette {
            block.extend_from_slice(&pack_palette(pal));
        }
    }
    block.push(bx);
    block.push(by);
    block.push(bw);
    block.push(bh);
    for &r in &refs {
        block.push(r);
    }
    let mut offset = 0u16;
    for d in &frame_data {
        block.push((offset & 0xFF) as u8);
        block.push(((offset >> 8) & 0xFF) as u8);
        offset += d.len() as u16;
    }
    for d in &frame_data {
        block.extend_from_slice(d);
    }

    let total: usize = frame_data.iter().map(|d| d.len()).sum();
    let ref_count = refs.iter().filter(|&&r| r != 255).count();
    let rd_tag = if use_rowdelta { "+RD" } else { "" };
    let tag = if use_bitpack { "BP" } else { "RX" };
    (
        block,
        format!(
            "{}{} {}x{} data={}b refs={}/{}",
            tag, rd_tag, bw, bh, total, ref_count, n
        ),
    )
}

/// Type 4: Referenced XOR + EG-2 with per-frame diff modes.
fn encode_type4(
    frames_pixels: &[Vec<u8>],
    fw: u32,
    fh: u32,
    bpp: u8,
    palette: Option<&[u8]>,
) -> (Vec<u8>, String) {
    let n = frames_pixels.len();
    let (bx, by, bw, bh) = get_bbox(frames_pixels, fw, fh);
    let mut cropped: Vec<Vec<u8>> = frames_pixels
        .iter()
        .map(|f| crop_pixels(f, fw, bx, by, bw, bh))
        .collect();
    if bpp < 4 {
        if let Some(pal) = palette {
            cropped = cropped.iter().map(|f| quantize_pixels(f, pal)).collect();
        }
    }
    let npix = bw as usize * bh as usize;

    let mut refs = Vec::new();
    let mut frame_data = Vec::new();
    let mut modes = Vec::new();
    let mut orders = Vec::new();

    for i in 0..n {
        let mut best_ref = 255u8;
        let (mut best_enc, mut best_mode, mut best_order) =
            eg2_encode_frame(&cropped[i], bpp, bw as usize, bh as usize);
        for r in 0..i {
            let xor_diff: Vec<u8> = cropped[i]
                .iter()
                .zip(cropped[r].iter())
                .map(|(&a, &b)| a ^ b)
                .collect();
            let (enc, mode, order) = eg2_encode_frame(&xor_diff, bpp, bw as usize, bh as usize);
            if enc.len() < best_enc.len() {
                best_enc = enc;
                best_ref = r as u8;
                best_mode = mode;
                best_order = order;
            }
        }
        refs.push(best_ref);
        frame_data.push(best_enc);
        modes.push(best_mode);
        orders.push(best_order);
    }

    let mut block = Vec::new();
    block.push(n as u8);
    block.push(4); // type 4
    block.push(bpp);
    if bpp < 4 {
        if let Some(pal) = palette {
            block.extend_from_slice(&pack_palette(pal));
        }
    }
    block.push(bx);
    block.push(by);
    block.push(bw);
    block.push(bh);
    for &r in &refs {
        block.push(r);
    }
    let mut offset = 0u16;
    for d in &frame_data {
        block.push((offset & 0xFF) as u8);
        block.push(((offset >> 8) & 0xFF) as u8);
        offset += d.len() as u16;
    }
    for d in &frame_data {
        block.extend_from_slice(d);
    }

    let total: usize = frame_data.iter().map(|d| d.len()).sum();
    let ref_count = refs.iter().filter(|&&r| r != 255).count();
    let mode_names = ['_', 'L', 'U', 'D', 'P'];
    let mode_str: String = modes
        .iter()
        .zip(orders.iter())
        .map(|(&m, &o)| {
            let mi = if m == 4 { 4 } else { m as usize };
            format!("{}{}", mode_names[mi], o)
        })
        .collect();

    (
        block,
        format!(
            "EG {}x{} data={}b refs={}/{} m={}",
            bw, bh, total, ref_count, n, mode_str
        ),
    )
}

/// Type 5: Per-frame bbox + EG-2 (no cross-frame refs).
fn encode_type5(
    frames_pixels: &[Vec<u8>],
    fw: u32,
    fh: u32,
    bpp: u8,
    palette: Option<&[u8]>,
) -> (Vec<u8>, String) {
    let n = frames_pixels.len();
    let mut frame_datas = Vec::new();
    let mut frame_infos = Vec::new();

    for f in frames_pixels {
        let (bx, by, bw, bh) = get_frame_bbox(f, fw, fh);
        if bw == 0 || bh == 0 {
            frame_datas.push(vec![0u8, 0, 0, 0]);
            frame_infos.push((0u8, 0u8));
            continue;
        }
        let mut cropped = crop_pixels(f, fw, bx, by, bw, bh);
        if bpp < 4 {
            if let Some(pal) = palette {
                cropped = quantize_pixels(&cropped, pal);
            }
        }
        let (enc, _mode, _order) = eg2_encode_frame(&cropped, bpp, bw as usize, bh as usize);
        let mut fd = vec![bx, by, bw, bh];
        fd.extend_from_slice(&enc);
        frame_datas.push(fd);
        frame_infos.push((bw, bh));
    }

    let mut block = Vec::new();
    block.push(n as u8);
    block.push(5); // type 5
    block.push(bpp);
    if bpp < 4 {
        if let Some(pal) = palette {
            block.extend_from_slice(&pack_palette(pal));
        }
    }
    let mut offset = 0u16;
    for fd in &frame_datas {
        block.push((offset & 0xFF) as u8);
        block.push(((offset >> 8) & 0xFF) as u8);
        offset += fd.len() as u16;
    }
    for fd in &frame_datas {
        block.extend_from_slice(fd);
    }

    let total: usize = frame_datas.iter().map(|d| d.len()).sum();
    let max_bw = frame_infos.iter().map(|(w, _)| *w).max().unwrap_or(0);
    let max_bh = frame_infos.iter().map(|(_, h)| *h).max().unwrap_or(0);
    (block, format!("PF {}x{} data={}b", max_bw, max_bh, total))
}

/// Type 6: Hybrid per-frame bbox OR union+ref.
fn encode_type6(
    frames_pixels: &[Vec<u8>],
    fw: u32,
    fh: u32,
    bpp: u8,
    palette: Option<&[u8]>,
) -> (Vec<u8>, String) {
    let n = frames_pixels.len();
    let (ubx, uby, ubw, ubh) = get_bbox(frames_pixels, fw, fh);
    let mut u_cropped: Vec<Vec<u8>> = frames_pixels
        .iter()
        .map(|f| crop_pixels(f, fw, ubx, uby, ubw, ubh))
        .collect();
    if bpp < 4 {
        if let Some(pal) = palette {
            u_cropped = u_cropped.iter().map(|f| quantize_pixels(f, pal)).collect();
        }
    }
    let unpix = ubw as usize * ubh as usize;

    // Per-frame bboxes
    let pf_bboxes: Vec<(u8, u8, u8, u8)> = frames_pixels
        .iter()
        .map(|f| get_frame_bbox(f, fw, fh))
        .collect();
    let mut pf_cropped: Vec<Vec<u8>> = Vec::new();
    for (i, f) in frames_pixels.iter().enumerate() {
        let (bx, by, bw, bh) = pf_bboxes[i];
        if bw == 0 {
            pf_cropped.push(vec![]);
        } else {
            let mut c = crop_pixels(f, fw, bx, by, bw, bh);
            if bpp < 4 {
                if let Some(pal) = palette {
                    c = quantize_pixels(&c, pal);
                }
            }
            pf_cropped.push(c);
        }
    }

    let mut flags: Vec<u8> = Vec::new();
    let mut frame_data: Vec<Vec<u8>> = Vec::new();

    for i in 0..n {
        // Option A: per-frame bbox (like T5)
        let (pbx, pby, pbw, pbh) = pf_bboxes[i];
        let best_pf = if pbw == 0 {
            vec![0u8, 0, 0, 0]
        } else {
            let (enc_pf, _, _) = eg2_encode_frame(&pf_cropped[i], bpp, pbw as usize, pbh as usize);
            let mut fd = vec![pbx, pby, pbw, pbh];
            fd.extend_from_slice(&enc_pf);
            fd
        };

        // Option B: union bbox, no ref
        let (best_union_enc, _, _) =
            eg2_encode_frame(&u_cropped[i], bpp, ubw as usize, ubh as usize);
        let mut best_ref = 255u8;
        let mut best_union = best_union_enc;

        // Option C: union bbox + ref to prev frame
        for r in 0..i {
            if flags[r] >= 254 {
                continue;
            }
            let xor_diff: Vec<u8> = u_cropped[i]
                .iter()
                .zip(u_cropped[r].iter())
                .map(|(&a, &b)| a ^ b)
                .collect();
            let (enc, _, _) = eg2_encode_frame(&xor_diff, bpp, ubw as usize, ubh as usize);
            if enc.len() < best_union.len() {
                best_union = enc;
                best_ref = r as u8;
            }
        }

        if best_pf.len() < best_union.len() {
            flags.push(254); // per-frame bbox marker
            frame_data.push(best_pf);
        } else {
            flags.push(best_ref);
            frame_data.push(best_union);
        }
    }

    let mut block = Vec::new();
    block.push(n as u8);
    block.push(6); // type 6
    block.push(bpp);
    if bpp < 4 {
        if let Some(pal) = palette {
            block.extend_from_slice(&pack_palette(pal));
        }
    }
    block.push(ubx);
    block.push(uby);
    block.push(ubw);
    block.push(ubh);
    for &f in &flags {
        block.push(f);
    }
    let mut offset = 0u16;
    for d in &frame_data {
        block.push((offset & 0xFF) as u8);
        block.push(((offset >> 8) & 0xFF) as u8);
        offset += d.len() as u16;
    }
    for d in &frame_data {
        block.extend_from_slice(d);
    }

    let total: usize = frame_data.iter().map(|d| d.len()).sum();
    let pf_count = flags.iter().filter(|&&f| f == 254).count();
    let ref_count = flags.iter().filter(|&&f| f < 254 && f != 255).count();
    (
        block,
        format!(
            "HY {}x{} data={}b pf={}/{} refs={}/{}",
            ubw, ubh, total, pf_count, n, ref_count, n
        ),
    )
}

/// Try T4/T5/T6 and pick smallest. Returns (block, info_string).
pub fn encode_animation(
    name: &str,
    frames_pixels: &[Vec<u8>],
    fw: u32,
    fh: u32,
    bpp_override: Option<u8>,
    palette_override: Option<&[u8]>,
) -> (Vec<u8>, String) {
    let bpp = bpp_override.unwrap_or_else(|| min_bpp_for_frames(frames_pixels));
    let built_pal;
    let palette = if bpp < 4 {
        match palette_override {
            Some(p) => Some(p),
            None => {
                built_pal = build_palette(frames_pixels, bpp);
                Some(built_pal.as_slice())
            }
        }
    } else {
        None
    };
    let n = frames_pixels.len();

    let (b4, i4) = encode_type4(frames_pixels, fw, fh, bpp, palette);
    let (b5, i5) = encode_type5(frames_pixels, fw, fh, bpp, palette);
    let (b6, i6) = encode_type6(frames_pixels, fw, fh, bpp, palette);

    let candidates = [("T4", b4, i4), ("T5", b5, i5), ("T6", b6, i6)];
    let best_idx = candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, (_, b, _))| b.len())
        .unwrap()
        .0;

    let (best_tag, ref best_block, ref best_info) = candidates[best_idx];
    let others: String = candidates
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != best_idx)
        .map(|(_, (t, b, _))| format!("{}={}b", t, b.len()))
        .collect::<Vec<_>>()
        .join(" ");

    let bpp_tag = format!(" [{}bpp]", bpp);
    let info = format!(
        "    {:12}: {:2}f, {} {}b{} {} ({})",
        name,
        n,
        best_info,
        best_block.len(),
        bpp_tag,
        best_tag,
        others
    );
    (best_block.clone(), info)
}
