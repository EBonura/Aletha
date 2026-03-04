/// Configuration constants matching build.py.

use std::path::{Path, PathBuf};

pub const TRANS: u8 = 14; // transparency color index (pink)
pub const CELL_W: u32 = 91;
pub const CELL_H: u32 = 19;
pub const TILE_SIZE: u32 = 16;
pub const TILESET_COLS: u32 = 18;
pub const TILESET_ROWS: u32 = 16;
pub const BG_TILESET_COLS: u32 = 19;
pub const BG_TILESET_ROWS: u32 = 9;

pub const P8_PALETTE: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0 black
    (29, 43, 83),     // 1 dark blue
    (126, 37, 83),    // 2 dark purple
    (0, 135, 81),     // 3 dark green
    (171, 82, 54),    // 4 brown
    (95, 87, 79),     // 5 dark grey
    (194, 195, 199),  // 6 light grey
    (255, 241, 232),  // 7 white
    (255, 0, 77),     // 8 red
    (255, 163, 0),    // 9 orange
    (255, 236, 39),   // 10 yellow
    (0, 228, 54),     // 11 green
    (41, 173, 255),   // 12 blue
    (131, 118, 156),  // 13 lavender
    (255, 119, 168),  // 14 pink (transparent)
    (255, 204, 170),  // 15 peach
];

pub const BAND_RANGES: [(u8, u8); 5] = [(0, 20), (21, 45), (46, 100), (101, 185), (186, 255)];
pub const BAND_COLORS: [u8; 5] = [0, 5, 13, 6, 7]; // black, dk grey, lavender, lt grey, white

pub const SPIDER_W: u32 = 16;
pub const SPIDER_H: u32 = 16;
pub const WHEELBOT_W: u32 = 48;
pub const WHEELBOT_H: u32 = 26;
pub const HELLBOT_W: u32 = 92;
pub const HELLBOT_H: u32 = 36;
pub const BOSS_W: u32 = 48;
pub const BOSS_H: u32 = 32;
pub const BOX_S: u32 = 13;
pub const PORTAL_SRC_W: u32 = 28;
pub const PORTAL_SRC_H: u32 = 41;
pub const PORTAL_CROP_Y: u32 = 30;
pub const PORTAL_W: u32 = 28;
pub const PORTAL_H: u32 = 11; // PORTAL_SRC_H - 30
pub const TORCH_W: u32 = 16;
pub const TORCH_H: u32 = 16;

pub const FONT_CHARS: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!.,:-'?/()\u{97}\u{8e}\u{94}\u{83}\u{8b}\u{91}";

/// Virtual address space constants.
pub const VGAP: usize = 0x3100;
pub const GAP_SIZE: usize = 0x100;
pub const SFX_PHYS_END: usize = 0x4300;
pub const TOTAL_VIRT: usize = VGAP + (SFX_PHYS_END - VGAP - GAP_SIZE); // 16896

pub fn vaddr_to_phys(va: usize) -> usize {
    if va < VGAP { va } else { va + GAP_SIZE }
}

/// Player animation definitions: (name, filename, frame_count_override).
pub struct AnimDef {
    pub name: &'static str,
    pub filename: &'static str,
    pub nframes: Option<u32>,
}

pub const ANIMS: &[AnimDef] = &[
    AnimDef { name: "idle", filename: "idle.png", nframes: None },
    AnimDef { name: "run", filename: "run with VFX.png", nframes: None },
    AnimDef { name: "jump", filename: "jump.png", nframes: None },
    AnimDef { name: "fall", filename: "fall.png", nframes: None },
    AnimDef { name: "hit", filename: "hit.png", nframes: None },
    AnimDef { name: "land", filename: "land with VFX.png", nframes: None },
    AnimDef { name: "attack1", filename: "attack 1 with VFX.png", nframes: None },
    AnimDef { name: "cross_slice", filename: "Cross Slice with VFX.png", nframes: None },
    AnimDef { name: "sweep", filename: "Sweep Attack with VFX.png", nframes: None },
    AnimDef { name: "death", filename: "death.png", nframes: None },
];

pub struct SpiderAnimDef {
    pub name: &'static str,
    pub files: Vec<&'static str>,
    pub nframes: Vec<Option<u32>>,
}

pub fn spider_anims() -> Vec<SpiderAnimDef> {
    vec![
        SpiderAnimDef { name: "sp_idle", files: vec!["idle.png"], nframes: vec![None] },
        SpiderAnimDef { name: "sp_walk", files: vec!["walk.png"], nframes: vec![None] },
        SpiderAnimDef { name: "sp_attack", files: vec!["prep_attack.png", "attack.png"], nframes: vec![None, Some(2)] },
        SpiderAnimDef { name: "sp_hit", files: vec!["hit.png"], nframes: vec![None] },
        SpiderAnimDef { name: "sp_death", files: vec!["death.png"], nframes: vec![None] },
    ]
}

pub struct WheelbotAnimDef {
    pub name: &'static str,
    pub filename: &'static str,
    pub src_fw: u32,
    pub src_fh: u32,
    pub nframes: Option<u32>,
    pub frame_select: Option<Vec<usize>>,
}

pub fn wheelbot_anims() -> Vec<WheelbotAnimDef> {
    vec![
        WheelbotAnimDef { name: "wb_idle", filename: "idle 112x26.png", src_fw: 32, src_fh: 26, nframes: None, frame_select: None },
        WheelbotAnimDef { name: "wb_move", filename: "move 112x26.png", src_fw: 32, src_fh: 26, nframes: None, frame_select: None },
        WheelbotAnimDef { name: "wb_charge", filename: "charge 112x26.png", src_fw: 48, src_fh: 26, nframes: None, frame_select: None },
        WheelbotAnimDef { name: "wb_shoot", filename: "shoot 112x26.png", src_fw: 48, src_fh: 26, nframes: None, frame_select: None },
        WheelbotAnimDef { name: "wb_firedash", filename: "fire dash 112x26.png", src_fw: 112, src_fh: 26, nframes: None, frame_select: None },
        WheelbotAnimDef { name: "wb_wake", filename: "wake 112x26.png", src_fw: 32, src_fh: 26, nframes: None, frame_select: None },
        WheelbotAnimDef { name: "wb_damaged", filename: "damaged 112x26.png", src_fw: 32, src_fh: 26, nframes: None, frame_select: None },
        WheelbotAnimDef { name: "wb_death", filename: "death 112x26.png", src_fw: 32, src_fh: 26, nframes: None, frame_select: None },
    ]
}

pub struct HellbotAnimDef {
    pub name: &'static str,
    pub filename: &'static str,
    pub nframes: Option<u32>,
}

pub fn hellbot_anims() -> Vec<HellbotAnimDef> {
    vec![
        HellbotAnimDef { name: "hb_idle", filename: "idle 92x36.png", nframes: None },
        HellbotAnimDef { name: "hb_run", filename: "run 92x36.png", nframes: None },
        HellbotAnimDef { name: "hb_attack", filename: "attack 92x36.png", nframes: None },
        HellbotAnimDef { name: "hb_shoot", filename: "shoot 92x36.png", nframes: None },
        HellbotAnimDef { name: "hb_hit", filename: "hit 92x36.png", nframes: None },
        HellbotAnimDef { name: "hb_death", filename: "death 92x36.png", nframes: None },
    ]
}

pub struct BossAnimDef {
    pub name: &'static str,
    pub filename: &'static str,
    pub src_fw: u32,
    pub src_fh: u32,
    pub nframes: Option<u32>,
    pub frame_select: Option<Vec<usize>>,
}

pub fn boss_anims() -> Vec<BossAnimDef> {
    vec![
        BossAnimDef { name: "bk_idle", filename: "idle(32x32).png", src_fw: 32, src_fh: 32, nframes: None, frame_select: Some(vec![0,2,4,6,8,10]) },
        BossAnimDef { name: "bk_run", filename: "Run (32x32).png", src_fw: 32, src_fh: 32, nframes: None, frame_select: None },
        BossAnimDef { name: "bk_attack", filename: "Double Slash no VFX (48x32).png", src_fw: 48, src_fh: 32, nframes: None, frame_select: Some(vec![0,2,4,6,8,10,12,13]) },
        BossAnimDef { name: "bk_charge", filename: "charge(48x32).png", src_fw: 48, src_fh: 32, nframes: None, frame_select: None },
        BossAnimDef { name: "bk_hit", filename: "Hit (32x32)).png", src_fw: 32, src_fh: 32, nframes: None, frame_select: None },
        BossAnimDef { name: "bk_death", filename: "death or teleport (168x79).png", src_fw: 168, src_fh: 79, nframes: None, frame_select: Some(vec![0,1,2]) },
    ]
}

/// Animation speeds indexed by animation constant.
pub fn aspd_table(n_player: usize, n_ent: usize, n_spider: usize, n_wb: usize, n_hb: usize, n_boss: usize) -> Vec<u8> {
    let mut aspd = vec![
        6, 5, 5, 5, 5, 2, 6, 6, 4, 6, // player: idle,run,jump,fall,hit,land,atk1,xslice,sweep,death
        6, 6, 6, 8, // entity: door,sw_start,sw_idle,sw_down
        30, 0, // title, font
    ];
    aspd.extend_from_slice(&[8, 6, 5, 5, 6]); // spider: idle,walk,attack,hit,death
    aspd.extend_from_slice(&[8, 5, 6, 5, 4, 6, 5, 6]); // wheelbot
    aspd.extend_from_slice(&[8, 5, 5, 5, 5, 6]); // hellbot
    aspd.extend_from_slice(&[8, 5, 5, 5, 5, 6]); // boss
    aspd.extend_from_slice(&[6, 6, 0]); // portal, torch, box
    aspd
}
