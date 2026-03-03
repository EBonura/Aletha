#!/usr/bin/env python3
"""Measure Hell Bot sprite data cost with various optimization options."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from build import extract_frames_custom, encode_animation, min_bpp_for_frames, TRANS

HELLBOT_DIR = os.path.expanduser("~/Downloads/DARK Edition/Sprites/Hell Bot DARK")
HELLBOT_W, HELLBOT_H = 92, 36
HELLBOT_ANIMS = [
    ("hb_idle",   "idle 92x36.png",   None),
    ("hb_run",    "run 92x36.png",    None),
    ("hb_attack", "attack 92x36.png", None),
    ("hb_shoot",  "shoot 92x36.png",  None),
    ("hb_hit",    "hit 92x36.png",    None),
    ("hb_death",  "death 92x36.png",  None),
]

# Load all frames once
all_frames = {}
for name, fname, nf in HELLBOT_ANIMS:
    all_frames[name] = extract_frames_custom(fname, HELLBOT_DIR, HELLBOT_W, HELLBOT_H, nf)

# Color analysis
print("=== Color analysis per animation ===")
for name in all_frames:
    frames = all_frames[name]
    colors = set(c for f in frames for c in f if c != TRANS)
    bpp = min_bpp_for_frames(frames)
    print(f"  {name:12s}: {len(frames):2d}f, {len(colors)} non-trans colors ({bpp}bpp): {sorted(colors)}")

# Baseline
print("\n=== Baseline (auto bpp) ===")
overhead = 3 + len(HELLBOT_ANIMS) * 2
total = overhead
for name in all_frames:
    block, info = encode_animation(name, all_frames[name], HELLBOT_W, HELLBOT_H)
    total += len(block)
    print(f"  {info}")
print(f"  TOTAL: {total}b (budget 975b, over by {total-975}b)")

# Aggressive frame trim
print("\n=== Aggressive trim: death(6f) + shoot(3f) + attack(3f) + run(4f) ===")
total2 = overhead
for name in all_frames:
    frames = all_frames[name]
    if name == "hb_death":
        frames = frames[::2]  # 11 -> 6
    elif name == "hb_shoot":
        frames = frames[::2]  # 6 -> 3
    elif name == "hb_attack":
        frames = frames[::2]  # 6 -> 3
    elif name == "hb_run":
        frames = frames[::2]  # 8 -> 4
    block, info = encode_animation(name, frames, HELLBOT_W, HELLBOT_H)
    total2 += len(block)
    print(f"  {info}")
print(f"  TOTAL: {total2}b (budget 975b, {'FITS!' if total2 <= 975 else f'over by {total2-975}b'})")

# Maximum trim
print("\n=== Maximum trim: death(4f) + shoot(2f) + attack(2f) + run(4f) ===")
total3 = overhead
for name in all_frames:
    frames = all_frames[name]
    if name == "hb_death":
        frames = frames[::3]  # 11 -> 4
    elif name == "hb_shoot":
        frames = frames[::3]  # 6 -> 2
    elif name == "hb_attack":
        frames = frames[::3]  # 6 -> 2
    elif name == "hb_run":
        frames = frames[::2]  # 8 -> 4
    block, info = encode_animation(name, frames, HELLBOT_W, HELLBOT_H)
    total3 += len(block)
    print(f"  {info}")
print(f"  TOTAL: {total3}b (budget 975b, {'FITS!' if total3 <= 975 else f'over by {total3-975}b'})")

# What if we free space by moving title to cart 2?
print("\n=== Space if title moved to cart 2 ===")
print(f"  Current free: 975b")
print(f"  Title: 1,431b")
print(f"  If title moved: {975+1431}b available")
print(f"  Baseline hellbot: {total-overhead+overhead}b - {'FITS!' if total <= 975+1431 else f'over by {total-975-1431}b'}")
print(f"  Aggressive trim: {total2}b - {'FITS!' if total2 <= 975+1431 else f'over by {total2-975-1431}b'}")
