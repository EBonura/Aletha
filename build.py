#!/usr/bin/env python3
"""
Ashen Edge build script.
Builds a PICO-8 .p8 cart from sprite sheets and level data.
Vertical strip PNGs, 91x19 cells, 4 source colors -> 3 PICO-8 colors.
Two encoding types per animation (picks smaller):
  Type 0 (KD): keyframe+delta with animation-wide bbox
  Type 1 (PF): per-frame independent RLE with per-frame bboxes

Level pipeline:
  Reads level_data.json (map, flags, transforms).
  Slices tileset PNG, deduplicates, applies color remap.
  Flattens rotation transforms into pre-rendered tiles.
  Packs compressed tile pixels + map data into __map__ of output cart.
"""

import os, json, hashlib, re, struct, math
from PIL import Image
from itertools import combinations

ASSET_DIR = "/tmp/assets/All 3 Sprites/assassin"
DIR = os.path.dirname(os.path.abspath(__file__))
OUTPUT_P8 = os.path.join(DIR, "ashen_edge.p8")
LEVEL_JSON = os.path.join(DIR, "level_data.json")
TRANS = 14  # transparency color index
CELL_W, CELL_H = 91, 19

# ── Tileset config ──
TILESET_PNG = "/Users/ebonura/Downloads/DARK Edition 2/Tileset/DARK Edition Tileset No background.png"
TILE_SIZE = 16
TILESET_COLS = 18
TILESET_ROWS = 16

# Luminance band → PICO-8 color mapping (matches editor defaults)
BAND_RANGES = [(0, 20), (21, 45), (46, 100), (101, 185), (186, 255)]
BAND_COLORS = [0, 5, 13, 6, 7]  # black, dk grey, lavender, lt grey, white

# PICO-8 palette
P8_PALETTE = [
    (0, 0, 0),       # 0 black
    (29, 43, 83),     # 1 dark blue
    (126, 37, 83),    # 2 dark purple
    (0, 135, 81),     # 3 dark green
    (171, 82, 54),    # 4 brown
    (95, 87, 79),     # 5 dark grey
    (194, 195, 199),  # 6 light grey
    (255, 241, 232),  # 7 white
    (255, 0, 77),     # 8 red
    (255, 163, 0),    # 9 orange
    (255, 236, 39),   # 10 yellow
    (0, 228, 54),     # 11 green
    (41, 173, 255),   # 12 blue
    (131, 118, 156),  # 13 lavender
    (255, 119, 168),  # 14 pink (=transparent)
    (255, 204, 170),  # 15 peach
]

# Animations: (name, filename, frame_count or None=auto)
ANIMS = [
    ("idle",        "idle.png",                          None),
    ("run",         "run with VFX.png",                  None),
    ("jump",        "jump.png",                          None),
    ("fall",        "fall.png",                          None),
    ("hit",         "hit.png",                           None),
    ("land",        "land with VFX.png",                 None),
    ("attack1",     "attack 1 with VFX.png",             None),
    ("cross_slice", "Cross Slice with VFX.png",          None),
    ("sweep",       "Sweep Attack with VFX.png",         None),
    ("death",       "death.png",                         None),
]

# ── Color mapping ──

_color_cache = {}

def nearest_p8(r, g, b):
    key = (r, g, b)
    if key in _color_cache:
        return _color_cache[key]
    best_i = 0
    best_d = float('inf')
    for i, (pr, pg, pb) in enumerate(P8_PALETTE):
        if i == TRANS:
            continue
        d = (r - pr) ** 2 + (g - pg) ** 2 + (b - pb) ** 2
        if d < best_d:
            best_d = d
            best_i = i
    _color_cache[key] = best_i
    return best_i


# ── Frame extraction (vertical strip format) ──

def extract_frames(img_path, nframes=None):
    """Extract frames from a vertical strip PNG. Returns list of pixel arrays."""
    img = Image.open(os.path.join(ASSET_DIR, img_path)).convert("RGBA")
    w, h = img.size
    if nframes is None:
        nframes = h // CELL_H
    frames = []
    for f in range(nframes):
        y0 = f * CELL_H
        pixels = []
        for y in range(CELL_H):
            for x in range(CELL_W):
                r, g, b, a = img.getpixel((x, y0 + y))
                if a == 0:
                    pixels.append(TRANS)
                else:
                    pixels.append(nearest_p8(r, g, b))
        frames.append(pixels)
    return frames


# ── Shared compression helpers ──

def crop_pixels(f, fw, bx, by, bw, bh):
    cropped = []
    for y in range(by, by + bh):
        for x in range(bx, bx + bw):
            cropped.append(f[y * fw + x])
    return cropped


def nibble_rle_encode(pixels):
    out = bytearray()
    cur_color = pixels[0]
    cur_count = 1

    def emit(color, count):
        while count > 0:
            run = min(count, 16)
            out.append((color << 4) | (run - 1))
            count -= run

    for p in pixels[1:]:
        if p == cur_color and cur_count < 16:
            cur_count += 1
        else:
            emit(cur_color, cur_count)
            cur_color = p
            cur_count = 1
    emit(cur_color, cur_count)
    return out


def delta_encode_skip(base_pixels, frame_pixels):
    entries = [(i, frame_pixels[i]) for i in range(len(base_pixels))
               if base_pixels[i] != frame_pixels[i]]
    out = bytearray()
    if len(entries) >= 255:
        out.append(0xFF)
        n = len(entries)
        out.append(n & 0xFF)
        out.append((n >> 8) & 0xFF)
    else:
        out.append(len(entries))
    if not entries:
        return out
    out.append(entries[0][0] & 0xFF)
    out.append((entries[0][0] >> 8) & 0xFF)
    out.append(entries[0][1])
    for j in range(1, len(entries)):
        gap = entries[j][0] - entries[j - 1][0]
        color = entries[j][1]
        skip = gap - 1
        if skip <= 14:
            out.append((skip << 4) | color)
        elif skip <= 14 + 255:
            out.append((15 << 4) | color)
            out.append(skip - 15)
        else:
            out.append((15 << 4) | color)
            out.append(0xFF)
            out.append(skip & 0xFF)
            out.append((skip >> 8) & 0xFF)
    return out


def count_diffs(a, b):
    return sum(1 for x, y in zip(a, b) if x != y)


def get_bbox(frames, fw, fh):
    min_x, min_y = fw - 1, fh - 1
    max_x, max_y = 0, 0
    for f in frames:
        for idx, c in enumerate(f):
            if c != TRANS:
                x, y = idx % fw, idx // fw
                min_x = min(min_x, x)
                min_y = min(min_y, y)
                max_x = max(max_x, x)
                max_y = max(max_y, y)
    return min_x, min_y, max_x - min_x + 1, max_y - min_y + 1


def get_frame_bbox(f, fw, fh):
    min_x, min_y = fw - 1, fh - 1
    max_x, max_y = 0, 0
    has_pixels = False
    for idx, c in enumerate(f):
        if c != TRANS:
            x, y = idx % fw, idx // fw
            min_x = min(min_x, x)
            min_y = min(min_y, y)
            max_x = max(max_x, x)
            max_y = max(max_y, y)
            has_pixels = True
    if not has_pixels:
        return 0, 0, 0, 0
    return min_x, min_y, max_x - min_x + 1, max_y - min_y + 1


# ── Type 0: Keyframe + Delta (animation-wide bbox) ──

def pick_keyframes_candidates(frames_pixels):
    n = len(frames_pixels)
    candidates = []
    max_keys = min(n, 4) if n <= 8 else min(n, 3)
    if n > 16:
        max_keys = min(n, 2)
    for k in range(1, max_keys + 1):
        if n > 12 and k >= 3:
            step = n // k
            base = [i * step for i in range(k)]
            candidates.append(base)
            for offset in range(1, min(3, step)):
                candidates.append([(b + offset) % n for b in base])
        else:
            for combo in combinations(range(n), k):
                candidates.append(list(combo))
    return candidates


def encode_kd_with_keys(frames_pixels, key_indices):
    n = len(frames_pixels)
    nkeys = len(key_indices)
    assignments = []
    for i in range(n):
        best_k_idx = min(range(nkeys),
                         key=lambda ki: count_diffs(frames_pixels[key_indices[ki]], frames_pixels[i]))
        assignments.append(best_k_idx)
    key_rles = [nibble_rle_encode(frames_pixels[ki]) for ki in key_indices]
    deltas = []
    for i in range(n):
        base_idx = key_indices[assignments[i]]
        deltas.append(delta_encode_skip(frames_pixels[base_idx], frames_pixels[i]))
    data = bytearray()
    for kr in key_rles:
        data.extend(kr)
    delta_offsets = []
    for d in deltas:
        delta_offsets.append(len(data))
        data.extend(d)
    return key_rles, assignments, delta_offsets, data


def encode_type0(name, frames_pixels, fw, fh):
    n = len(frames_pixels)
    bx, by, bw, bh = get_bbox(frames_pixels, fw, fh)
    cropped = [crop_pixels(f, fw, bx, by, bw, bh) for f in frames_pixels]
    candidates = pick_keyframes_candidates(cropped)
    best_block = None
    best_info = ""
    for key_indices in candidates:
        key_rles, assignments, delta_offsets, data = encode_kd_with_keys(cropped, key_indices)
        nkeys = len(key_indices)
        block = bytearray()
        block.append(n)
        block.append(0)  # type 0
        block.append(nkeys)
        block.append(bx)
        block.append(by)
        block.append(bw)
        block.append(bh)
        for ki in key_indices:
            block.append(ki)
        for kr in key_rles:
            block.append(len(kr) & 0xFF)
            block.append((len(kr) >> 8) & 0xFF)
        for a in assignments:
            block.append(a)
        for off in delta_offsets:
            block.append(off & 0xFF)
            block.append((off >> 8) & 0xFF)
        block.extend(data)
        if best_block is None or len(block) < len(best_block):
            best_block = block
            total_keys = sum(len(kr) for kr in key_rles)
            best_info = f"KD {nkeys}k {bw}x{bh} keys={total_keys}b"
    return best_block, best_info


# ── Type 1: Per-frame independent RLE ──

def encode_type1(name, frames_pixels, fw, fh):
    n = len(frames_pixels)
    frame_datas = []
    for f in frames_pixels:
        bx, by, bw, bh = get_frame_bbox(f, fw, fh)
        if bw == 0 or bh == 0:
            frame_datas.append(bytearray([0, 0, 0, 0]))
        else:
            cropped = crop_pixels(f, fw, bx, by, bw, bh)
            rle = nibble_rle_encode(cropped)
            fd = bytearray([bx, by, bw, bh])
            fd.extend(rle)
            frame_datas.append(fd)
    block = bytearray()
    block.append(n)
    block.append(1)  # type 1
    offset = 0
    for fd in frame_datas:
        block.append(offset & 0xFF)
        block.append((offset >> 8) & 0xFF)
        offset += len(fd)
    for fd in frame_datas:
        block.extend(fd)
    return block, "PF"


# ── Pick best encoding per animation ──

def encode_animation(name, frames_pixels, fw, fh):
    n = len(frames_pixels)
    kd_block, kd_info = encode_type0(name, frames_pixels, fw, fh)
    pf_block, pf_info = encode_type1(name, frames_pixels, fw, fh)
    if len(pf_block) < len(kd_block):
        info = f"    {name:12s}: {n:2d}f, PF {len(pf_block)}b (kd={len(kd_block)}b)"
        return pf_block, info
    else:
        info = f"    {name:12s}: {n:2d}f, {kd_info} {len(kd_block)}b (pf={len(pf_block)}b)"
        return kd_block, info


# ── GFX output ──

def bytes_to_gfx(data):
    row_bytes = 64
    total_rows = 128  # PICO-8 expects exactly 128 rows
    padded = bytearray(data)
    padded.extend(b'\x00' * (total_rows * row_bytes - len(padded)))
    lines = []
    for row in range(total_rows):
        row_data = padded[row * row_bytes:(row + 1) * row_bytes]
        hex_str = ""
        for b in row_data:
            lo = b & 0x0F
            hi = (b >> 4) & 0x0F
            hex_str += f"{lo:x}{hi:x}"
        lines.append(hex_str)
    return "\n".join(lines)


# ── Tileset processing ──

def slice_tileset():
    """Slice tileset PNG into 16x16 tiles, deduplicate, return list of PIL Images."""
    img = Image.open(TILESET_PNG).convert("RGBA")
    tiles = []  # list of (name, PIL.Image)
    seen_hashes = {}
    for r in range(TILESET_ROWS):
        for c in range(TILESET_COLS):
            x0, y0 = c * TILE_SIZE, r * TILE_SIZE
            tile = img.crop((x0, y0, x0 + TILE_SIZE, y0 + TILE_SIZE))
            # Skip fully transparent/empty tiles
            pixels = list(tile.getdata())
            if all(p[3] == 0 for p in pixels):
                continue
            # Dedup by pixel hash
            grey = tile.convert("L").tobytes()
            h = hashlib.md5(grey).hexdigest()
            if h in seen_hashes:
                continue
            # Check 8 dihedral transforms for duplicates
            is_dup = False
            for rot in range(4):
                for flip in [False, True]:
                    t = tile.rotate(-rot * 90, expand=False)
                    if flip:
                        t = t.transpose(Image.FLIP_LEFT_RIGHT)
                    th = hashlib.md5(t.convert("L").tobytes()).hexdigest()
                    if th in seen_hashes:
                        is_dup = True
                        break
                if is_dup:
                    break
            if is_dup:
                continue
            name = f"T_{r:02d}_{c:02d}"
            seen_hashes[h] = len(tiles)
            tiles.append((name, tile))
    return tiles


def remap_tile_colors(tile_img):
    """Convert greyscale tile to PICO-8 colors using luminance bands.
    Returns list of 256 P8 color indices (row-major 16x16)."""
    grey = tile_img.convert("LA")  # luminance + alpha
    pixels = list(grey.getdata())
    result = []
    for lum, alpha in pixels:
        if alpha < 128:
            result.append(TRANS)
        else:
            color = BAND_COLORS[-1]  # default to last band
            for i, (lo, hi) in enumerate(BAND_RANGES):
                if lo <= lum <= hi:
                    color = BAND_COLORS[i]
                    break
            result.append(color)
    return result


def apply_transform(pixels, rot, hflip, vflip):
    """Apply rotation + flip to a 16x16 pixel array.
    Editor order: rotate → vflip → hflip (canvas transform order)."""
    size = TILE_SIZE
    # Start with 2D grid
    grid = []
    for y in range(size):
        grid.append(pixels[y * size:(y + 1) * size])

    # Apply rotation (CW)
    for _ in range(rot % 4):
        new_grid = []
        for x in range(size):
            row = []
            for y in range(size - 1, -1, -1):
                row.append(grid[y][x])
            new_grid.append(row)
        grid = new_grid

    # Apply vflip
    if vflip:
        grid = grid[::-1]

    # Apply hflip
    if hflip:
        grid = [row[::-1] for row in grid]

    return [p for row in grid for p in row]


def pixels_to_spritesheet_bytes(pixels):
    """Convert 16x16 pixel array (P8 color indices) to 128 bytes in PICO-8
    sprite sheet format: each byte = lo_nibble(left_pixel) | hi_nibble(right_pixel).
    Pixels are stored row by row, but within the sprite sheet each 8-pixel-wide
    column is stored as a block."""
    # PICO-8 sprite sheet: 128 pixels wide, each row = 64 bytes
    # For a 16x16 tile at position (col, row) in the 8x8-tile grid:
    # We just need the raw nibble pairs for 16 rows of 16 pixels
    out = bytearray()
    for y in range(TILE_SIZE):
        for x in range(0, TILE_SIZE, 2):
            lo = pixels[y * TILE_SIZE + x] & 0x0F
            hi = pixels[y * TILE_SIZE + x + 1] & 0x0F
            out.append(lo | (hi << 4))
    return out


def read_level_json(json_path):
    """Read level data from level_data.json.
    Returns (map_w, map_h, map_grid, xform_grid, spawn_x, spawn_y, flags, band_colors)
    where map_grid[y][x] = tile_index (255=empty), xform_grid[y][x] = packed xform."""
    with open(json_path) as f:
        data = json.load(f)

    if "width" not in data:
        return None

    w = data["width"]
    h = data["height"]
    sx = data.get("spawnX", -1)
    sy = data.get("spawnY", -1)

    map_grid = []
    for row in data["map"]:
        map_grid.append([int(v) for v in row])

    xform_grid = []
    if "mapXform" in data:
        for row in data["mapXform"]:
            xform_grid.append([int(v) for v in row])
    else:
        for y in range(h):
            xform_grid.append([0] * w)

    flags = [0] * 256
    if "flags" in data:
        for i, f in enumerate(data["flags"]):
            if i < 256:
                flags[i] = int(f)

    band_colors = None
    if "bandColors" in data:
        band_colors = [int(c) for c in data["bandColors"]]

    return w, h, map_grid, xform_grid, sx, sy, flags, band_colors


def build_level_data(tileset, map_data):
    """Build runtime tile + map data for __map__ section.

    Args:
        tileset: list of (name, PIL.Image) from slice_tileset()
        map_data: tuple from read_level_json()

    Returns:
        (map_bytes, num_rt_tiles, tile_flags, gen_lines)
        map_bytes: bytearray for __map__ section
        num_rt_tiles: number of runtime tiles
        tile_flags: dict of runtime_tile_id -> flag byte
        gen_lines: list of Lua code lines for generated block
    """
    map_w, map_h, map_grid, xform_grid, spawn_x, spawn_y, editor_flags, band_colors = map_data

    # Use band colors from level data if available
    global BAND_COLORS
    if band_colors and len(band_colors) == len(BAND_COLORS):
        BAND_COLORS = band_colors

    print(f"\n=== LEVEL DATA ===")
    print(f"  Map size: {map_w}x{map_h} ({map_w*map_h} cells)")
    print(f"  Spawn: ({spawn_x}, {spawn_y})")

    # Step 1: Collect all used (tile_id, rot_group) pairs
    # rot_group: 0 = needs base only (rot 0,2), 1 = needs rot90 (rot 1,3)
    used_combos = set()  # (tile_id, rot, hflip, vflip)
    for y in range(map_h):
        for x in range(map_w):
            ti = map_grid[y][x]
            if ti == 255:
                continue
            xf = xform_grid[y][x]
            rot = xf & 3
            hflip = bool(xf & 4)
            vflip = bool(xf & 8)
            used_combos.add((ti, rot, hflip, vflip))

    # Step 2: Determine runtime tiles needed
    # For each base tile used: need base version. If rot 1 or 3 used, also need rot90 version.
    # Runtime xform is just (flip_x, flip_y) for spr().
    #
    # Mapping editor xform → (runtime_tile_variant, spr_flip_x, spr_flip_y):
    #   rot=0: base, flip_x=hflip, flip_y=vflip
    #   rot=2: base, flip_x=!hflip, flip_y=!vflip  (180° = hflip+vflip of base)
    #   rot=1: rot90, flip_x=vflip, flip_y=hflip  (derived from transform math)
    #   rot=3: rot90, flip_x=!vflip, flip_y=!hflip (270° = 90°+180°)
    #
    # But this transform math is tricky to get right. Instead, we flatten all
    # transforms at build time: each unique (tile, full_xform) = 1 runtime tile.
    # This avoids flip math errors. Runtime has NO flips, just spr() calls.

    # Collect unique transformed pixel data
    rt_tiles = []       # list of pixel arrays (each 256 P8 colors)
    rt_tile_map = {}    # (editor_tile_id, packed_xform) -> runtime_tile_id (1-based)
    rt_tile_flags = {}  # runtime_tile_id -> flag byte

    for ti, rot, hflip, vflip in sorted(used_combos):
        if ti >= len(tileset):
            print(f"  WARNING: tile index {ti} out of range, skipping")
            continue
        name, tile_img = tileset[ti]
        base_pixels = remap_tile_colors(tile_img)
        transformed = apply_transform(base_pixels, rot, hflip, vflip)

        # Check if this exact pixel data already exists
        t_hash = hashlib.md5(bytes(transformed)).hexdigest()
        found = False
        for rt_id, existing in enumerate(rt_tiles):
            if hashlib.md5(bytes(existing)).hexdigest() == t_hash:
                packed_xf = rot | (int(hflip) << 2) | (int(vflip) << 3)
                rt_tile_map[(ti, packed_xf)] = rt_id + 1
                found = True
                break

        if not found:
            rt_id = len(rt_tiles)
            rt_tiles.append(transformed)
            packed_xf = rot | (int(hflip) << 2) | (int(vflip) << 3)
            rt_tile_map[(ti, packed_xf)] = rt_id + 1
            # Carry over editor flags (sprite+1 index in editor flags)
            rt_tile_flags[rt_id + 1] = editor_flags[ti] if ti < len(editor_flags) else 0

    num_rt = len(rt_tiles)
    print(f"  Used tile combos: {len(used_combos)}")
    print(f"  Runtime tiles (after dedup): {num_rt}")

    if num_rt > 63:
        print(f"  WARNING: {num_rt} runtime tiles exceeds 63 tile limit!")
        print(f"  (sprite sheet can hold 64 16x16 tiles, ID 0 reserved for empty)")

    # Step 3: Compress tile pixel data (raw sprite sheet bytes, not RLE)
    # Each tile = 128 bytes in PICO-8 format. We store them sequentially.
    # At runtime, poke directly into sprite sheet memory.
    tile_data = bytearray()
    for pixels in rt_tiles:
        tile_data.extend(pixels_to_spritesheet_bytes(pixels))
    print(f"  Tile pixel data: {len(tile_data)} bytes ({num_rt} × 128)")

    # Step 4: Build runtime map (2-byte RLE: cell_byte, run_length)
    # cell_byte: 0 = empty, 1-63 = runtime tile ID (no flips needed since flattened)
    map_rle = bytearray()
    for y in range(map_h):
        x = 0
        while x < map_w:
            ti = map_grid[y][x]
            xf = xform_grid[y][x]
            if ti == 255:
                cell = 0
            else:
                packed_xf = xf  # rot | hflip<<2 | vflip<<3
                rt_id = rt_tile_map.get((ti, packed_xf), 0)
                cell = rt_id  # 1-based, 0=empty
            # Count run
            run = 1
            while x + run < map_w and run < 255:
                nti = map_grid[y][x + run]
                nxf = xform_grid[y][x + run]
                if nti == 255:
                    ncell = 0
                else:
                    npacked = nxf
                    ncell = rt_tile_map.get((nti, npacked), 0)
                if ncell != cell:
                    break
                run += 1
            map_rle.append(cell)
            map_rle.append(run)
            x += run
    print(f"  Map RLE data: {len(map_rle)} bytes")

    # Step 5: Pack into __map__ format
    # Header: 10 bytes
    header = bytearray()
    header.append(num_rt)                           # num_tiles (u8)
    header.append(map_w & 0xFF)                     # map_w lo
    header.append((map_w >> 8) & 0xFF)              # map_w hi
    header.append(map_h & 0xFF)                     # map_h lo
    header.append((map_h >> 8) & 0xFF)              # map_h hi
    sx = spawn_x if spawn_x >= 0 else 0xFFFF
    sy = spawn_y if spawn_y >= 0 else 0xFFFF
    header.append(sx & 0xFF)
    header.append((sx >> 8) & 0xFF)
    header.append(sy & 0xFF)
    header.append((sy >> 8) & 0xFF)
    tile_data_len = len(tile_data)
    header.append(tile_data_len & 0xFF)             # tile_data_size lo
    header.append((tile_data_len >> 8) & 0xFF)      # tile_data_size hi

    total_bytes = len(header) + len(tile_data) + len(map_rle)
    print(f"  Total __map__ data: {total_bytes} bytes (limit: 4096)")

    map_section = bytearray()
    map_section.extend(header)
    map_section.extend(tile_data)
    map_section.extend(map_rle)

    overflow_str = None
    if total_bytes > 4096:
        # Overflow: put map RLE in Lua string, tile data in __map__
        print(f"  OVERFLOW: {total_bytes - 4096} bytes over limit")
        print(f"  Tile data ({len(header) + len(tile_data)}b) in __map__, map RLE ({len(map_rle)}b) in Lua string")
        map_section = bytearray()
        map_section.extend(header)
        map_section.extend(tile_data)
        # Encode map RLE as Lua hex string
        overflow_hex = "".join(f"\\x{b:02x}" for b in map_rle)
        overflow_str = f'map_rle_str="{overflow_hex}"'
    else:
        overflow_str = None

    # Step 6: Generate Lua metadata
    gen = []
    gen.append(f"-- level: {map_w}x{map_h}, {num_rt} tiles, {total_bytes}b")
    gen.append(f"map_base=0x2000")
    gen.append(f"lvl_w={map_w} lvl_h={map_h}")
    gen.append(f"lvl_nt={num_rt}")
    if spawn_x >= 0:
        gen.append(f"spn_x={spawn_x} spn_y={spawn_y}")
    else:
        gen.append(f"spn_x=0 spn_y=0")
    if overflow_str:
        gen.append(f"map_in_str=true")
        gen.append(overflow_str)
    else:
        gen.append(f"map_in_str=false")

    # Tile flags table (runtime tile ID → flag byte)
    flag_entries = []
    for rt_id in range(1, num_rt + 1):
        f = rt_tile_flags.get(rt_id, 0)
        if f != 0:
            flag_entries.append(f"tflg[{rt_id}]={f}")
    if flag_entries:
        gen.append("tflg={}")
        gen.extend(flag_entries)
    else:
        gen.append("tflg={}")

    return map_section, num_rt, rt_tile_flags, gen


def bytes_to_map_hex(data):
    """Convert bytes to __map__ section hex format (32 rows of 256 hex chars)."""
    padded = bytearray(data)
    padded.extend(b'\x00' * (4096 - len(padded)))
    lines = []
    row_bytes = 128  # 128 bytes per row = 256 hex chars
    for row in range(32):
        row_data = padded[row * row_bytes:(row + 1) * row_bytes]
        lines.append("".join(f"{b:02x}" for b in row_data))
    return "\n".join(lines)


# ── Build ──

def build_cart():
    print("Loading sprite sheets...")
    print(f"  Cell size: {CELL_W}x{CELL_H}")

    print("\nExtracting frames...")
    all_frames = {}
    for name, fname, nf in ANIMS:
        frames = extract_frames(fname, nf)
        all_frames[name] = frames
        print(f"    {name}: {len(frames)} frames from {fname}")

    print("\nCompressing animations...")
    anim_blocks = []
    total_frames = 0
    for name, fname, _ in ANIMS:
        frames = all_frames[name]
        block, info = encode_animation(name, frames, CELL_W, CELL_H)
        anim_blocks.append((name, block))
        total_frames += len(frames)
        print(info)

    # Character chunk header
    num_anims = len(anim_blocks)
    anim_offsets = []
    anim_data = bytearray()
    for name, block in anim_blocks:
        anim_offsets.append(len(anim_data))
        anim_data.extend(block)

    char_chunk = bytearray()
    char_chunk.append(num_anims)
    char_chunk.append(CELL_W)
    char_chunk.append(CELL_H)
    for off in anim_offsets:
        char_chunk.append(off & 0xFF)
        char_chunk.append((off >> 8) & 0xFF)
    char_chunk.extend(anim_data)

    total = len(char_chunk)
    print(f"\n=== TOTAL ===")
    print(f"  {num_anims} anims, {total_frames} frames")
    print(f"  Total: {total} / 8192 bytes ({total*100//8192}%)")

    if total > 8192:
        print(f"  WARNING: exceeds sprite memory by {total - 8192} bytes!")
        print(f"  (but generating cart anyway for testing)")

    gfx = bytes_to_gfx(char_chunk)
    # Compute per-frame body anchor X (center of dark/body pixels)
    print("\nComputing body anchors...")
    anchor_lines = []
    for ai, (name, fname, _) in enumerate(ANIMS):
        frames = all_frames[name]
        centers = []
        for f in frames:
            body_xs = []
            for idx, c in enumerate(f):
                if c != TRANS and c == 0:  # black = body
                    body_xs.append(idx % CELL_W)
            if body_xs:
                centers.append((min(body_xs) + max(body_xs)) // 2)
            else:
                centers.append(15)  # fallback to idle center
        vals = ",".join(str(c) for c in centers)
        anchor_lines.append(f"anc[{ai+1}]={{{vals}}}")
        print(f"    {name}: {centers}")

    # Build generated data block
    gen_lines = []
    gen_lines.append(f"-- {total_frames} frames, {num_anims} anims")
    gen_lines.append(f"-- compressed: {total}/8192 bytes ({total*100//8192}%)")
    gen_lines.append(f"char_base=0")
    gen_lines.append(f"cell_w={CELL_W} cell_h={CELL_H}")
    gen_lines.append(f"trans={TRANS}")
    # anim indices
    for ai, (name, fname, _) in enumerate(ANIMS):
        anim_var = {
            "idle": "a_idle", "run": "a_run", "jump": "a_jump",
            "fall": "a_fall", "hit": "a_hit", "land": "a_land",
            "attack1": "a_atk1", "cross_slice": "a_xslice",
            "sweep": "a_sweep", "death": "a_death",
        }[name]
        gen_lines.append(f"{anim_var}={ai+1}")
    # anchor data
    gen_lines.append("anc={}")
    gen_lines.extend(anchor_lines)

    generated_block = "\n".join(gen_lines)

    # ── Process level data ──
    level_gen_lines = []
    map_hex = ""

    print("\nLoading tileset...")
    tileset = slice_tileset()
    print(f"  {len(tileset)} unique tiles from tileset")

    if os.path.exists(LEVEL_JSON):
        print(f"\nReading level data from {LEVEL_JSON}...")
        map_data = read_level_json(LEVEL_JSON)
        if map_data:
            map_section, num_rt, tile_flags, level_gen_lines = build_level_data(tileset, map_data)
            map_hex = bytes_to_map_hex(map_section)
        else:
            print("  No map data found in JSON, skipping level processing")
    else:
        print(f"\n  No level data at {LEVEL_JSON}, skipping level processing")

    # Combine generated blocks
    if level_gen_lines:
        generated_block += "\n" + "\n".join(level_gen_lines)

    # Read game lua, inject generated data
    game_lua_path = os.path.join(DIR, "ashen_edge.lua")
    with open(game_lua_path) as f:
        game_lua = f.read()

    # Replace between markers
    marker_start = "--##generated##"
    marker_end = "--##end##"
    i0 = game_lua.index(marker_start)
    i1 = game_lua.index(marker_end) + len(marker_end)
    lua_code = game_lua[:i0] + marker_start + "\n" + generated_block + "\n" + marker_end + game_lua[i1:]

    # Build map section
    map_section = ""
    if map_hex:
        map_section = f"\n__map__\n{map_hex}"

    # Write single output cart
    p8 = f"""pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
{lua_code}
__gfx__
{gfx}{map_section}
"""

    with open(OUTPUT_P8, "w") as f:
        f.write(p8)
    print(f"\nWrote cart: {OUTPUT_P8}")


if __name__ == "__main__":
    build_cart()
