# Ashen Edge — Data Layout

PICO-8 gives us several storage buckets with different compression
characteristics. This document explains how the game's data is split
across them and why.

---

## Budgets at a glance

| Section     | Used       | Limit  | %    |
|-------------|------------|--------|------|
| `__gfx__`   | 3,754 b    | 8,192  | 45%  |
| `__map__`   | 3,063 b    | 4,096  | 74%  |
| `__lua__`   |            |        |      |
| — tokens    | 5,672      | 8,192  | 69%  |
| — chars     | 39,228     | 65,535 | 60%  |
| — compressed| 12,203 b   | 15,616 | 78%  |

---

## `__gfx__` — Player & entity animations (0x0000)

Holds one **character chunk**: a contiguous binary blob read at runtime
with `read_anim()` / `decode_rle()`. Layout:

```
[na:u8][cell_w:u8][cell_h:u8][off0:u16][off1:u16]...[block0][block1]...
```

Each block starts with a 3-byte header:
- `[nf:u8][enc:u8][bpp:u8]` — frame count, encoding type, bits per pixel
- followed by optional palette nibbles (when bpp < 4)

### Encoding types

**Type 0 — Keyframe + Delta (KD)**
Best for looping animations with many similar frames.
```
[nkeys][bx][by][bw][bh][key_idx...][key_sz:u16...][frame_key:u8...][delta_off:u16...][data]
```
Each frame stores only the pixels that changed from its keyframe (delta-skip encoding).

**Type 1 — Per-frame RLE (PF)**
Best for single frames or animations with large per-frame changes.
```
[frame_off:u16...][bx][by][bw][bh][rle_data]...
```
Each frame is compressed independently with extended nibble-RLE.

### Unified RLE format (all bpp)

```
byte = (color_index << run_bits) | (run - 1)
run_bits = 8 - bpp
```

Escape when `run - 1 == run_mask`: read one more byte `ext`, actual run = `(run_mask + 1) + ext`.

| bpp | run_bits | max inline run |
|-----|----------|----------------|
| 1   | 7        | 128            |
| 2   | 6        | 64             |
| 3   | 5        | 32             |
| 4   | 4        | 16             |

`bpp=4` is bit-for-bit identical to the original nibble-RLE.

### Current contents

| Animation   | Frames | bpp | Encoding | Compressed |
|-------------|--------|-----|----------|------------|
| idle        | 8      | 2   | KD       |            |
| run         | 12     | 2   | KD       |            |
| jump        | 10     | 2   | KD       |            |
| fall        | 4      | 2   | KD       |            |
| hit         | 4      | 2   | KD       |            |
| land        | 8      | 2   | KD       |            |
| attack1     | 10     | 2   | KD       |            |
| cross_slice | 13     | 2   | KD       |            |
| sweep       | 14     | 2   | KD       |            |
| death       | 18     | 2   | KD       |            |
| door        | 15     | 2   | KD       |            |
| sw_start    | 7      | 2   | KD       |            |
| sw_idle     | 1      | 2   | PF       |            |
| sw_down     | 4      | 2   | KD       |            |
| **Total**   | **128**|     |          | **3,754 b**|

bpp is determined automatically by `min_bpp_for_frames()` — all current
animations use only 3–4 colors so they all compress to 2bpp (palette stored inline).

---

## `__map__` — Level & tile data (0x2000)

Read at runtime via `map_base = 0x2000`. Layout:

```
[num_tiles:u8][num_layers:u8][map_w:u16][map_h:u16]
[spawn_x:u16][spawn_y:u16][tile_blob_size:u16]
[layer_mode:u8 × num_layers]
[tile_blob]   ← extended nibble-RLE, single blob for all tiles
[layer_0_data]
[layer_1_data]
[num_entities:u8][type:u8 x:u8 y:u8 group:u8 × num_entities]
```

### Tile blob

All runtime tile pixels (83 tiles × 256 pixels each = 21,248 pixels raw)
are concatenated and compressed as a single extended nibble-RLE stream.
Result: **2,137 b** (20% of raw).

Tiles are stored in two groups:
- **Sprite sheet tiles** (37): rendered via `spr()` / sprite sheet memory
- **BG tiles** (46): copied to user RAM at `0x4300+` on level load,
  rendered via `memcpy` into screen buffer

### Layer encoding (auto-selected per layer)

| Mode | Format | Used for |
|------|--------|----------|
| 0 — RLE | `[cell][run]` pairs | dense layers |
| 1 — TiledFill | repeating pattern + bbox | BG layer (246 b vs 3008 b RLE) |
| 2 — PackBits | literal runs + repeat runs | Main layer (657 b) |

### Current contents (128×64 map, 2 layers)

| Component      | Size    |
|----------------|---------|
| Header         | 14 b    |
| Tile blob      | 2,137 b |
| BG layer       | 246 b   |
| Main layer     | 657 b   |
| Entities       | 9 b     |
| **Total**      | **3,063 b** / 4,096 |

---

## `__lua__` — Code + title/font data

The Lua section holds all game code plus two large data strings that
cannot fit in `__gfx__` without exceeding the 8,192-byte sprite limit.

### Why strings for title & font?

Adding lowercase letters to the font pushed `__gfx__` to 8,328 b (136 b
over budget). Moving the title image (2,023 b) and font (1,986 b) to
Lua string literals freed `__gfx__` to 45%.

The tradeoff:
- String literals cost **1 token** regardless of size (very token-efficient)
- They do count against the **character** and **compressed** budgets
- PICO-8's compressor handles binary escape sequences well

### Runtime decode flow

At startup (`cache_anims()`):

```
title_data (Lua string)  ──poke──► 0x4300  ──read_anim/decode_rle──► acache[a_title]
font_data  (Lua string)  ──poke──► 0x4ae7  ──read_anim/decode_rle──► acache[a_font]
char_base  (gfx, 0x0000) ──────────────────── read_anim/decode_rle──► acache[1..14]
```

`0x4300`–`0x5DFF` is free user RAM (not loaded from cart). Title and font
share this region (non-overlapping) since both are decoded once into Lua
tables and the RAM is never reused.

BG tiles are loaded separately to `0x4300+` during level load (after
`cache_anims()` has finished with that region).

### String encoding

Binary data is encoded as Lua string literals using 3-digit decimal escapes:

```python
f'\\{b:03d}'  # e.g. byte 31 → \031  (never \31 which is greedy-parsed)
```

Printable ASCII characters are stored literally; only `"` and `\` are
additionally escaped.

### Current Lua budget

| Metric     | Used   | Limit  | %    |
|------------|--------|--------|------|
| Tokens     | 5,672  | 8,192  | 69%  |
| Chars      | 39,228 | 65,535 | 60%  |
| Compressed | 12,203 | 15,616 | 78%  |

~22% compressed budget remains for game code.

---

## Free RAM layout (runtime)

```
0x0000–0x00FF   system / draw state (pico-8 internal)
0x0100–0x3FFF   sprite sheet / map / flags / sfx / music (cart sections)
0x4000–0x42FF   sprite flags + extra (partially used by pico-8)
0x4300–0x4AE6   title mini-chunk (2,023 b, poked from title_data string)
0x4AE7–0x52A8   font mini-chunk (1,986 b, poked from font_data string)
0x52A9–0x5DFF   BG tile pixels (46 tiles × 128 b = 5,888 b max)
0x5E00–0x7FFF   screen buffer (read/write, 128×128 px at 4bpp)
```

---

## Build pipeline

```
make build   → python3 build.py
make count   → python3 count_tokens.py
make minify  → python3 minify.py
make edit    → python3 level_editor.py
```

`build.py` selects encoding type and bpp automatically — no manual
tuning required. Output is `ashen_edge.p8`.
