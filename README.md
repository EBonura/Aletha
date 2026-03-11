# Aletha

A PICO-8 action-platformer set inside **The Hollowed Furnace** -a colossal ancient kiln that consumes the fire of the world.

You play as **Aletha**, a tempered figure born of the furnace itself, who returns to shut it down before it drains what's left. Fight constructs and vermin, extinguish ignition points, and descend toward the Heart of the Kiln.

## Controls

- Arrow keys: move / climb
- Up: jump
- Down: interact (switches, portals)
- O (Z): attack
- X: sweep attack

## How It Fits in a PICO-8 Cart

PICO-8 carts have hard limits: 8,192 tokens of Lua, 8 KB of sprite memory, 4 KB of map memory, and 15,616 bytes of compressed code. This game has 128 frames of player animation alone, plus 5 enemy types, tilesets, a bitmap font, and title screen art. None of that fits in a normal PICO-8 cart -so the build pipeline does some unusual things to make it work.

### Virtual Memory Mapper

PICO-8's ROM is laid out as separate sections in fixed memory:

```
0x0000-0x1FFF  sprite sheet   (8,192 bytes)
0x2000-0x2FFF  map            (4,096 bytes)
0x3000-0x30FF  sprite flags   (  256 bytes)
0x3100-0x31FF  music          (  256 bytes)  <- can't use, reserved for music
0x3200-0x42FF  SFX            (4,352 bytes)
```

The build tool treats everything except the music region as a single contiguous 16,640-byte virtual address space. Data chunks (animations, tiles, map, entities) are packed sequentially into this space. The allocator converts virtual addresses to physical ones, transparently skipping the 256-byte music gap at `0x3100`. A single data blob can start in the sprite sheet, flow through the map section and sprite flags, jump over the music region, and continue into SFX memory -all invisible to the runtime, which just calls `peek()` at the address the build tool assigned.

This means **sprite sheet memory doesn't hold sprites** and **SFX memory doesn't hold sound effects**. They're just bytes. The game's actual music and SFX are loaded from a separate cart (`music.p8`) into the music region at startup.

### Compression Pipeline

Raw pixel data for all animations would be ~100 KB. The build tool compresses it down to fit in the ~16 KB of available ROM through several layers:

**Automatic BPP reduction** -The build tool analyzes each animation's actual color usage. Most animations use only 3-4 colors (the game's palette is deliberately constrained), so they compress to 2 bits per pixel instead of 4. The inline palette (mapping 2-bit indices back to PICO-8 colors) is stored in the animation header. This alone halves the data for most animations.

**EG-2 encoding** -The core compressor. Pixel data is first passed through a differential predictor (raw, left, up, diagonal, or Paeth -the build tool tries all five and picks the smallest). The residuals are then encoded as a bitstream using Exponential-Golomb coding of zero-runs. The encoder picks the optimal Golomb parameter `k` per frame. This is particularly effective on sprite data where large regions are transparent (long zero-runs).

**Keyframe + Delta (KD) encoding** -For looping animations with many similar frames (idle, run, attack). The build tool performs a combinatorial search for the best set of keyframes, assigns each frame to its nearest keyframe, and stores only the XOR delta. The delta is then EG-2 compressed -since most pixels don't change between frames, the delta is almost all zeros, which compresses extremely well.

**Per-frame RLE (PF) encoding** -For single frames or animations where every frame is very different. Each frame is independently compressed with extended nibble-RLE. The build tool tries both KD and PF for every animation and picks whichever is smaller.

**Tile blob compression** -All 83 tile images (16x16, ~21 KB raw) are concatenated into a single stream and EG-2 compressed as one unit, exploiting cross-tile redundancy. Result: 2,137 bytes (10:1 ratio).

**Map layer encoding** -Each map layer uses an automatically-selected encoding: RLE for dense layers, TiledFill (repeating pattern + bounding box) for the background layer (246 bytes vs 3,008 bytes with RLE), or PackBits for the main gameplay layer.

### Overflow into Lua Strings

Even with all this compression, the font and title image didn't fit in ROM. Adding lowercase letters pushed `__gfx__` 136 bytes over the 8,192-byte limit. The solution: encode them as Lua string literals using `\nnn` escape sequences and `poke()` them into free user RAM (`0x4300+`) at startup, where the same `read_anim()` / `decode_eg2()` decoder unpacks them. A string literal costs only 1 token regardless of length -very token-efficient, at the cost of character and compressed-code budget.

### Runtime Decode

At startup, everything is decoded into Lua tables in a single pass:

```
ROM (gfx+map+gff+sfx)  --peek()-->  read_anim() --> decode_eg2() --> acache[]
Lua strings             --poke()-->  user RAM     --> same decoder --> acache[]
```

Animation frames become flat pixel strings drawn with `pset()` per pixel. Tiles are decoded to sprite sheet memory and drawn with `spr()` / `memcpy`. After decoding, the ROM is never read again -the game runs entirely from Lua tables.

### Budget (current)

| Resource   | Used    | Limit   | %    |
|------------|---------|---------|------|
| Tokens     | ~8,040  | 8,192   | 98%  |
| Characters | 39,228  | 65,535  | 60%  |
| Compressed | 12,203  | 15,616  | 78%  |
| Sprite mem | 3,754   | 8,192   | 45%  |
| Map mem    | 3,063   | 4,096   | 74%  |

## Building

Requires [PICO-8](https://www.lexaloffle.com/pico-8.php) and a Rust toolchain.

```
make build     # Build the .p8 cart from source assets + Lua
make export    # Build + export to HTML (output in export/)
make count     # Show token/char/compressed usage
make minify    # Generate minified Lua
make edit      # Launch the level editor
```

The Rust build tool (`tools/build-cart/`) processes PNG sprite sheets into compressed animation data, encodes the level map, and assembles the final cart. HTML export is done natively (no PICO-8 binary needed) -it reads `pico8.dat` for the web player template.

## Project Structure

```
aletha.lua            Main game source (Lua)
music.p8              Music/SFX cart (composed in PICO-8 tracker)
level_editor.py       Tiled-style level editor (Python + pygame)
level_editor.html     Web-based level editor
minify.py             Lua minifier (token-aware)
count_tokens.py       PICO-8 token/char/compressed counter
tools/build-cart/     Rust build pipeline
  src/eg2.rs            EG-2 compressor (Exp-Golomb + differential predictors)
  src/animation.rs      KD/PF animation encoders with combinatorial keyframe search
  src/frame.rs          PNG frame extraction, color quantization, bbox cropping
  src/cart.rs           Virtual memory allocator and .p8 assembly
  src/level.rs          Map layer encoding (RLE, TiledFill, PackBits)
  src/tileset.rs        Tile extraction and blob compression
  src/html_export.rs    Native HTML export (pxa compression, label embedding)
  src/music.rs          Music/SFX cart merging
assets/               Source sprite sheets and fonts
export/               HTML export (deployed to itch.io)
docs/                 Design document and data layout reference
```

## Deployment

Pushing changes to `export/` on `main` triggers a GitHub Actions workflow that deploys to itch.io via [Butler](https://itch.io/docs/butler/).

## Assets

All character/entity sprite sheets are from [Sci-Fi Platformer Dark Edition](https://penusbmic.itch.io/sci-fi-platformer-dark-edition) by penusbmic. Most have been tweaked or modified (recolored, resized, trimmed) to fit within PICO-8's constraints.

Tileset, title screen, UI elements, and font are original or heavily reworked derivatives.

## License

Game code is provided as-is for reference. Asset license follows the original pack's terms -see the [asset page](https://penusbmic.itch.io/sci-fi-platformer-dark-edition) for details.
