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
BG_TILESET_PNG = os.path.join(DIR, "bg_tileset.png")
TILE_SIZE = 16
TILESET_COLS = 18
TILESET_ROWS = 16
BG_TILESET_COLS = 19
BG_TILESET_ROWS = 6

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


def ext_nibble_rle_encode(pixels):
    """Extended nibble-RLE: allows runs > 15 via escape byte.
    Standard: (color << 4) | (run - 1), run 1..15
    Escape:   (color << 4) | 0xF, then ext_byte, run = 16 + ext_byte (16..271)"""
    out = bytearray()
    cur_color = pixels[0]
    cur_count = 1

    def emit(color, count):
        while count > 0:
            if count <= 15:
                out.append((color << 4) | (count - 1))
                count = 0
            else:
                ext = min(count - 16, 255)
                out.append((color << 4) | 0xF)
                out.append(ext)
                count -= (16 + ext)

    for p in pixels[1:]:
        if p == cur_color:
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
    key_rles = [ext_nibble_rle_encode(frames_pixels[ki]) for ki in key_indices]
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
            rle = ext_nibble_rle_encode(cropped)
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

def tile_remap_hash(tile_img):
    """Hash a tile by its remapped PICO-8 colors (not raw grayscale)."""
    pixels = remap_tile_colors(tile_img)
    return hashlib.md5(bytes(pixels)).hexdigest()


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
            # Dedup by remapped color hash
            h = tile_remap_hash(tile)
            if h in seen_hashes:
                continue
            # Check 8 dihedral transforms for duplicates
            is_dup = False
            for rot in range(4):
                for flip in [False, True]:
                    t = tile.rotate(-rot * 90, expand=False)
                    if flip:
                        t = t.transpose(Image.FLIP_LEFT_RIGHT)
                    th = tile_remap_hash(t)
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


def slice_bg_tileset():
    """Slice BG tileset PNG into 16x16 tiles, deduplicate, return list of PIL Images."""
    img = Image.open(BG_TILESET_PNG).convert("RGBA")
    # Crop to 16px grid (304x96)
    cw = (img.width // TILE_SIZE) * TILE_SIZE
    ch = (img.height // TILE_SIZE) * TILE_SIZE
    img = img.crop((0, 0, cw, ch))
    tiles = []
    seen_hashes = {}
    for r in range(ch // TILE_SIZE):
        for c in range(cw // TILE_SIZE):
            x0, y0 = c * TILE_SIZE, r * TILE_SIZE
            tile = img.crop((x0, y0, x0 + TILE_SIZE, y0 + TILE_SIZE))
            pixels = list(tile.getdata())
            if all(p[3] == 0 for p in pixels):
                continue
            h = tile_remap_hash(tile)
            if h in seen_hashes:
                continue
            name = f"BG_{r:02d}_{c:02d}"
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
    Returns (map_w, map_h, map_grids, xform_grids, spawn_x, spawn_y, flags, band_colors, parallax)
    where map_grids[layer][y][x] = tile_index (255=empty), xform_grids[layer][y][x] = packed xform.
    2 layers: 0=BG, 1=Main."""
    with open(json_path) as f:
        data = json.load(f)

    if "width" not in data:
        return None

    w = data["width"]
    h = data["height"]
    sx = data.get("spawnX", -1)
    sy = data.get("spawnY", -1)

    def parse_grid(rows):
        return [[int(v) for v in row] for row in rows]

    def empty_grid():
        return [[255] * w for _ in range(h)]

    def zero_grid():
        return [[0] * w for _ in range(h)]

    if "layers" in data:
        map_grids = []
        xform_grids = []
        for layer in data["layers"]:
            map_grids.append(parse_grid(layer["map"]))
            xform_grids.append(parse_grid(layer["xform"]))
        # Only keep first 2 layers (BG, Main) — FG was removed
        map_grids = map_grids[:2]
        xform_grids = xform_grids[:2]
        # v2→v3 migration: if no bgTiles field, BG layer had main tileset indices — clear it
        if "bgTiles" not in data and data.get("version", 2) < 3:
            map_grids[0] = empty_grid()
            xform_grids[0] = zero_grid()
    else:
        # v1 migration: single map → Main layer (index 1)
        map_grids = [empty_grid(), parse_grid(data["map"])]
        xf = parse_grid(data["mapXform"]) if "mapXform" in data else zero_grid()
        xform_grids = [zero_grid(), xf]

    flags = [0] * 256
    if "flags" in data:
        for i, f in enumerate(data["flags"]):
            if i < 256:
                flags[i] = int(f)

    band_colors = None
    if "bandColors" in data:
        band_colors = [int(c) for c in data["bandColors"]]

    parallax = [0.5, 1.0]  # defaults: BG, Main
    if "parallax" in data:
        parallax = [float(v) for v in data["parallax"]][:2]

    return w, h, map_grids, xform_grids, sx, sy, flags, band_colors, parallax


## -- Map layer encoding modes --

def encode_rle(cell_grid, map_w, map_h):
    """Mode 0: Standard 2-byte RLE (cell, run) pairs."""
    out = bytearray()
    for y in range(map_h):
        x = 0
        while x < map_w:
            cell = cell_grid[y][x]
            run = 1
            while x + run < map_w and run < 255:
                if cell_grid[y][x + run] != cell:
                    break
                run += 1
            out.append(cell)
            out.append(run)
            x += run
    return out


def detect_tiling(cell_grid, map_w, map_h):
    """Mode 1: Detect repeating rectangular pattern.
    Returns (cost, tw, th, dx, dy, rw, rh, tile_cells) or None."""
    # Find bounding box of non-empty cells
    min_x, min_y = map_w, map_h
    max_x, max_y = -1, -1
    for y in range(map_h):
        for x in range(map_w):
            if cell_grid[y][x] != 0:
                min_x = min(min_x, x)
                min_y = min(min_y, y)
                max_x = max(max_x, x)
                max_y = max(max_y, y)
    if max_x < 0:
        return None  # empty layer
    bw = max_x - min_x + 1
    bh = max_y - min_y + 1

    best = None
    for tw in range(1, bw + 1):
        for th in range(1, bh + 1):
            cost = 6 + tw * th  # header + pattern cells
            if best and cost >= best[0]:
                continue
            # Verify modulo tiling
            ok = True
            for y in range(min_y, max_y + 1):
                if not ok:
                    break
                for x in range(min_x, max_x + 1):
                    ref_y = min_y + (y - min_y) % th
                    ref_x = min_x + (x - min_x) % tw
                    if cell_grid[y][x] != cell_grid[ref_y][ref_x]:
                        ok = False
                        break
            if ok:
                tile_cells = []
                for ty in range(th):
                    for tx in range(tw):
                        tile_cells.append(cell_grid[min_y + ty][min_x + tx])
                best = (cost, tw, th, min_x, min_y, bw, bh, tile_cells)
    return best


def encode_packbits(cell_grid, map_w, map_h):
    """Mode 2: PackBits encoding.
    Control 0..127: next ctrl+1 bytes are literals.
    Control 128..255: next byte repeated (ctrl-125) times (3..130)."""
    flat = []
    for y in range(map_h):
        for x in range(map_w):
            flat.append(cell_grid[y][x])
    out = bytearray()
    i = 0
    n = len(flat)
    while i < n:
        # Check for run of 3+
        run = 1
        while i + run < n and run < 130 and flat[i + run] == flat[i]:
            run += 1
        if run >= 3:
            out.append(125 + run)  # 128..255
            out.append(flat[i])
            i += run
        else:
            # Collect literals until we hit a run of 3+
            lit_start = i
            lit_len = 0
            while i < n and lit_len < 128:
                r = 1
                while i + r < n and r < 130 and flat[i + r] == flat[i]:
                    r += 1
                if r >= 3:
                    break
                lit_len += 1
                i += 1
            if lit_len > 0:
                out.append(lit_len - 1)  # 0..127
                out.extend(flat[lit_start:lit_start + lit_len])
    return out


def encode_layer(cell_grid, map_w, map_h, label=""):
    """Try all encoding modes, return (mode, data_bytes, description)."""
    # Mode 0: Standard RLE
    rle = encode_rle(cell_grid, map_w, map_h)
    best_mode, best_data = 0, rle
    desc = f"RLE {len(rle)}b"

    # Mode 1: Tiled fill
    tiling = detect_tiling(cell_grid, map_w, map_h)
    if tiling and tiling[0] < len(best_data):
        cost, tw, th, dx, dy, rw, rh, cells = tiling
        data = bytearray([tw, th, dx, dy, rw, rh])
        data.extend(cells)
        best_mode, best_data = 1, data
        desc = f"TiledFill {tw}x{th} at ({dx},{dy}) {rw}x{rh} = {len(data)}b (RLE was {len(rle)}b)"

    # Mode 2: PackBits
    packbits = encode_packbits(cell_grid, map_w, map_h)
    if len(packbits) < len(best_data):
        best_mode, best_data = 2, packbits
        desc = f"PackBits {len(packbits)}b (RLE was {len(rle)}b)"

    return best_mode, best_data, desc


def build_level_data(tileset, bg_tileset, map_data):
    """Build runtime tile + map data for __map__ section.

    Args:
        tileset: list of (name, PIL.Image) from slice_tileset() — main tiles
        bg_tileset: list of (name, PIL.Image) from slice_bg_tileset() — BG tiles
        map_data: tuple from read_level_json()

    Returns:
        (map_bytes, num_rt_tiles, tile_flags, gen_lines)
        map_bytes: bytearray for __map__ section
        num_rt_tiles: number of runtime tiles
        tile_flags: dict of runtime_tile_id -> flag byte
        gen_lines: list of Lua code lines for generated block
    """
    map_w, map_h, map_grids, xform_grids, spawn_x, spawn_y, editor_flags, band_colors, parallax = map_data
    num_layers = len(map_grids)
    # Layer 0 = BG (uses bg_tileset), Layer 1 = Main (uses tileset)
    layer_tilesets = [bg_tileset, tileset]

    # Use band colors from level data if available
    global BAND_COLORS
    if band_colors and len(band_colors) == len(BAND_COLORS):
        BAND_COLORS = band_colors

    print(f"\n=== LEVEL DATA ===")
    print(f"  Map size: {map_w}x{map_h} ({map_w*map_h} cells), {num_layers} layers")
    print(f"  Spawn: ({spawn_x}, {spawn_y})")

    # Step 1: Collect used tiles per layer (each layer uses its own tileset)
    # Per-layer sets: used_base[L] and used_rot90[L]
    used_base = [set() for _ in range(num_layers)]
    used_rot90 = [set() for _ in range(num_layers)]
    for L in range(num_layers):
        for y in range(map_h):
            for x in range(map_w):
                ti = map_grids[L][y][x]
                if ti == 255:
                    continue
                rot = xform_grids[L][y][x] & 3
                if rot == 0 or rot == 2:
                    used_base[L].add(ti)
                else:
                    used_rot90[L].add(ti)

    # Step 2: Build runtime tiles from both tilesets into a shared pool
    rt_tiles = []       # pixel arrays (256 P8 colors each)
    rt_tile_flags = {}  # runtime_tile_id -> flag byte
    # Per-layer maps: base_rt[L][ti] -> rt_id, rot90_rt[L][ti] -> rt_id
    base_rt = [{} for _ in range(num_layers)]
    rot90_rt = [{} for _ in range(num_layers)]

    def add_tile_variant(pixels, L, ti, is_rot90):
        """Add a pixel variant to the runtime pool, deduplicating by hash."""
        t_hash = hashlib.md5(bytes(pixels)).hexdigest()
        # Check existing
        for rt_id, existing in enumerate(rt_tiles):
            if hashlib.md5(bytes(existing)).hexdigest() == t_hash:
                if is_rot90:
                    rot90_rt[L][ti] = rt_id + 1
                else:
                    base_rt[L][ti] = rt_id + 1
                return
        # New tile
        rt_id = len(rt_tiles)
        rt_tiles.append(pixels)
        if is_rot90:
            rot90_rt[L][ti] = rt_id + 1
        else:
            base_rt[L][ti] = rt_id + 1
        # Only main layer tiles (L=1) get collision flags
        if L == 1:
            rt_tile_flags[rt_id + 1] = editor_flags[ti] if ti < len(editor_flags) else 0

    # Process main layer (1) first → sprite sheet tiles (max 64).
    # Then BG layer (0) → user memory tiles (rendered via memcpy).
    def process_layer_tiles(L):
        ts = layer_tilesets[L]
        all_used = sorted(used_base[L] | used_rot90[L])
        for ti in all_used:
            if ti >= len(ts):
                print(f"  WARNING: layer {L} tile index {ti} out of range (tileset has {len(ts)}), skipping")
                continue
            name, tile_img = ts[ti]
            base_pixels = remap_tile_colors(tile_img)
            if ti in used_base[L]:
                add_tile_variant(base_pixels, L, ti, False)
            if ti in used_rot90[L]:
                rot90_pixels = apply_transform(base_pixels, 1, False, False)
                add_tile_variant(rot90_pixels, L, ti, True)

    process_layer_tiles(1)  # Main → sprite sheet
    num_spr_tiles = len(rt_tiles)
    process_layer_tiles(0)  # BG → user memory
    num_rt = len(rt_tiles)

    print(f"  BG: {len(used_base[0]|used_rot90[0])} editor tiles, Main: {len(used_base[1]|used_rot90[1])} editor tiles")
    print(f"  Runtime tiles: {num_rt} ({num_spr_tiles} spr + {num_rt - num_spr_tiles} bg)")

    # Sprite sheet: 128x128px = 8x8 grid of 16x16 tiles = 64 max
    if num_spr_tiles > 64:
        print(f"  ERROR: {num_spr_tiles} main tiles exceeds 64 sprite sheet limit!")
    # Main layer cell byte: (tile_id << 2) | flip bits → max 63 tile IDs
    main_rt_ids = set(base_rt[1].values()) | set(rot90_rt[1].values())
    max_main_rt = max(main_rt_ids) if main_rt_ids else 0
    if max_main_rt > 63:
        print(f"  WARNING: main layer uses rt_id up to {max_main_rt}, exceeds 63 tile limit!")
    # BG tiles in user memory: 0x4300-0x5FFF = 7424 bytes, 128 bytes/tile = 58 max
    num_bg_tiles = num_rt - num_spr_tiles
    if num_bg_tiles * 128 > 7424:
        print(f"  WARNING: {num_bg_tiles} BG tiles ({num_bg_tiles*128}b) exceeds user memory (7424b)!")

    def editor_to_cell(L, ti, xf):
        """Convert editor (tile_id, packed_xform) to cell byte for layer L.
        BG layer (0): cell = rt_tile_id (no flip bits, max 255 tiles)
        Main layer (1): cell = (rt_tile_id << 2) | (flip_x << 1) | flip_y (max 63 tiles)
        Returns 0 for empty."""
        if ti == 255:
            return 0
        rot = xf & 3
        hflip = bool(xf & 4)
        vflip = bool(xf & 8)
        if rot == 0 or rot == 2:
            rt_id = base_rt[L].get(ti, 0)
        else:
            rt_id = rot90_rt[L].get(ti, 0)
        if rt_id == 0:
            return 0
        # BG layer: no flip bits, just tile id
        if L == 0:
            return rt_id
        if rot >= 2:
            fx = int(not hflip)
            fy = int(not vflip)
        else:
            fx = int(hflip)
            fy = int(vflip)
        return (rt_id << 2) | (fx << 1) | fy

    # Step 3: Extended nibble-RLE compress all tile pixels as single blob
    # Format: (color<<4 | run-1) for runs 1..15
    #         (color<<4 | 0xF) + ext_byte for runs 16..271
    # No per-tile offset table needed — decoded sequentially.
    all_pixels = []
    for pixels in rt_tiles:
        all_pixels.extend(pixels)

    tile_blob = ext_nibble_rle_encode(all_pixels)
    old_size = sum(len(nibble_rle_encode(p)) for p in rt_tiles) + num_rt * 2
    raw_size = num_rt * 128
    print(f"  Tile pixels: {len(tile_blob)}b compressed"
          f" (from {raw_size}b, {len(tile_blob)*100//raw_size}%)"
          f" (old: {old_size}b, saved {old_size - len(tile_blob)}b)")

    # Step 4: Build cell grids, then auto-encode each layer
    layer_modes = []
    layer_data = []
    for L in range(num_layers):
        mg = map_grids[L]
        xg = xform_grids[L]
        cell_grid = []
        for y in range(map_h):
            row = []
            for x in range(map_w):
                row.append(editor_to_cell(L, mg[y][x], xg[y][x]))
            cell_grid.append(row)
        label = ['BG', 'Main'][L]
        mode, data, desc = encode_layer(cell_grid, map_w, map_h, label)
        layer_modes.append(mode)
        layer_data.append(data)
        print(f"  Layer {L} ({label}): mode {mode} {desc}")

    # Step 5: Pack into __map__ format
    # Header (12 + num_layers bytes):
    #   num_tiles:      u8
    #   num_layers:     u8
    #   map_w:          u16 LE
    #   map_h:          u16 LE
    #   spawn_x:        u16 LE (0xFFFF = none)
    #   spawn_y:        u16 LE (0xFFFF = none)
    #   tile_blob_size: u16 LE
    #   layer_mode[0..nl-1]: u8 each  (NEW)
    # Then: tile_blob (extended RLE compressed pixel data, single blob)
    # Then: layer 0 data, layer 1 data
    header = bytearray()
    header.append(num_rt)
    header.append(num_layers)
    header.append(map_w & 0xFF)
    header.append((map_w >> 8) & 0xFF)
    header.append(map_h & 0xFF)
    header.append((map_h >> 8) & 0xFF)
    sx = spawn_x if spawn_x >= 0 else 0xFFFF
    sy = spawn_y if spawn_y >= 0 else 0xFFFF
    header.append(sx & 0xFF)
    header.append((sx >> 8) & 0xFF)
    header.append(sy & 0xFF)
    header.append((sy >> 8) & 0xFF)
    header.append(len(tile_blob) & 0xFF)
    header.append((len(tile_blob) >> 8) & 0xFF)
    for m in layer_modes:
        header.append(m)

    map_section = bytearray()
    map_section.extend(header)
    map_section.extend(tile_blob)
    for ld in layer_data:
        map_section.extend(ld)

    total_bytes = len(map_section)
    print(f"  Total __map__: {total_bytes}/4096 bytes ({total_bytes*100//4096}%)")
    if total_bytes > 4096:
        print(f"  ERROR: exceeds 4096 by {total_bytes - 4096} bytes!")

    # Step 6: Generate Lua metadata
    gen = []
    gen.append(f"-- level: {map_w}x{map_h}, {num_rt} tiles, {num_layers} layers, {total_bytes}b")
    gen.append(f"map_base=0x2000")
    gen.append(f"lvl_w={map_w} lvl_h={map_h}")
    gen.append(f"lvl_nt={num_rt} lvl_nl={num_layers} lvl_nst={num_spr_tiles}")
    if spawn_x >= 0:
        gen.append(f"spn_x={spawn_x} spn_y={spawn_y}")
    else:
        gen.append(f"spn_x=0 spn_y=0")

    # Layer parallax (Lua table: 1=bg,2=main,3=fg)
    px_vals = ",".join(str(p) for p in parallax)
    gen.append(f"lplx={{{px_vals}}}")

    # Tile flags table
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

    print("\nLoading tilesets...")
    tileset = slice_tileset()
    print(f"  {len(tileset)} unique main tiles from tileset")
    bg_tileset = slice_bg_tileset()
    print(f"  {len(bg_tileset)} unique BG tiles from bg_tileset")

    if os.path.exists(LEVEL_JSON):
        print(f"\nReading level data from {LEVEL_JSON}...")
        map_data = read_level_json(LEVEL_JSON)
        if map_data:
            map_section, num_rt, tile_flags, level_gen_lines = build_level_data(tileset, bg_tileset, map_data)
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
