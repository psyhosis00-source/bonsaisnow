//! "Window bonsai": pixel-art snowy pines that grow on the top edges of the window being used and on the taskbar,
//! some of them dressed up for Christmas, with the odd snowman who builds himself and hops now and then.
//! A quick swipe of the pointer cuts a full-grown tree or knocks a snowman to bits.
//!
//! Everything here is plain data and maths, so it is tested on any platform. Trees, snowmen, falling needles and
//! snow and the swipe trail all end up as square [`Sprite`]s (see `PIXEL_LUMP`) that the overlay draws after the
//! weather, in the same pass.
//!
//! Coordinates are the overlay's: DIPs, y pointing up from the bottom of the work area. The taskbar is the
//! ground at y = 0.

use std::collections::HashMap;

use crate::geometry::{Rect, V2};
use crate::simulation::{Pace, Sprite, WindowId, WindowSurface};

/// Size of one art pixel in DIPs.
pub const PX: f32 = 2.0;
/// The taskbar, which is the ground. No window has this id.
pub const GROUND: WindowId = 0;
/// Marks a sprite as a flat, hard-edged coloured square; the shaders draw it as such.
pub const PIXEL_LUMP: f32 = -1000.0;

const VARIANTS: usize = 8;
const MAX_TREES: usize = 36;
const MAX_SNOWMEN: usize = 3;
const MAX_BITS: usize = 1500;
const CUT_SPEED: f32 = 1000.0;
const GRAVITY: f32 = 620.0;
const ACTIVE_FOR: f32 = 2.5;
/// Seconds one hop of the snowman takes: five frames at ten a second.
const HOP: f32 = 0.5;

const fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

const OUT: u8 = 0;
const G1: u8 = 1;
const G2: u8 = 2;
const G3: u8 = 3;
const FROST: u8 = 4;
const FROST2: u8 = 5;
const FROST3: u8 = 6;
const TRUNK: u8 = 7;
const TRUNK2: u8 = 8;
const RED: u8 = 9;
const RED_DARK: u8 = 10;
const BLUE: u8 = 11;
const YELLOW: u8 = 12;
const LIME: u8 = 13;
const GOLD: u8 = 14;
const GOLD_PALE: u8 = 15;
/// The snowman's colours start here.
const SM: u8 = 16;
const SM_ICE: u8 = SM + 10;
const SM_COAL: u8 = SM + 1;
const SM_RED: u8 = SM + 13;
const SM_BROWN: u8 = SM + 7;

const PALETTE: [[f32; 3]; 34] = [
    rgb(11, 36, 23),
    rgb(23, 73, 44),
    rgb(34, 112, 58),
    rgb(52, 147, 63),
    rgb(246, 251, 255),
    rgb(196, 227, 244),
    rgb(141, 191, 220),
    rgb(122, 74, 40),
    rgb(77, 46, 25),
    rgb(214, 40, 40),
    rgb(150, 20, 25),
    rgb(80, 150, 255),
    rgb(255, 215, 70),
    rgb(110, 240, 110),
    rgb(240, 190, 40),
    rgb(255, 240, 150),
    rgb(0, 0, 0),
    rgb(33, 33, 33),
    rgb(35, 16, 8),
    rgb(66, 64, 64),
    rgb(78, 97, 95),
    rgb(97, 97, 97),
    rgb(102, 61, 43),
    rgb(115, 80, 64),
    rgb(154, 0, 0),
    rgb(163, 0, 0),
    rgb(177, 229, 242),
    rgb(191, 0, 0),
    rgb(212, 240, 247),
    rgb(217, 26, 26),
    rgb(240, 248, 250),
    rgb(254, 87, 34),
    rgb(254, 131, 93),
    rgb(255, 255, 255),
];

/// Five frames of a hop, drawn on a 17 by 29 grid with the ground under the last row. Letters are colours in
/// the order of the palette entries from [`SM`] on; a dot is empty.
const SNOWMAN: [[&str; 29]; 5] = [
    [
        ".................",
        ".................",
        "......aaaaaaa....",
        "......adffdda....",
        ".....afddbba.....",
        ".....afdbbba.....",
        "..aa.alliaaaa....",
        "..afaaaaafffda...",
        "..abdfffdbbbba...",
        "...bbbbbbbbbb....",
        "....booborre.....",
        "...bqpoobore.....",
        "....bbbboore.....",
        "....eooooree.....",
        "....eeeeijlre....",
        "...enjijjnorrcc..",
        ".cceoboojnoorcgc.",
        "chhekoooinnorrchc",
        ".ccekbommccckkhhc",
        "..ekccccchhgggcc.",
        ".ekchhgggcccccre.",
        ".ekmcccccooooore.",
        ".ekmoboooooooore.",
        ".ekmoooooooooore.",
        ".ekmmooooooommke.",
        ".ekkmmmooommmkke.",
        "..ekkkmmmmmkkke..",
        "...eekkkkkkkee...",
        ".....eeeeeee.....",
    ],
    [
        ".................",
        "......aaaaaaa....",
        "......adffdda....",
        ".....afddbba.....",
        ".....afdbbba.....",
        "..aa.alliaaaa....",
        "..afaaaaafffda...",
        "..abdfffdbbbba...",
        "...bbbbbbbbbb....",
        "....booborre.....",
        "...bqpoobore.....",
        "....bbbboore.....",
        "....eooooree.....",
        "....eeeeijnre....",
        ".ccenjijjnorrcc..",
        "chheoboojnoorcgc.",
        ".ccekoooinnorrchc",
        "..ekcccccccckkhhc",
        "..echhggghhgggcc.",
        ".ekmccccccccccre.",
        ".ekmoooooooooore.",
        ".ekmoboooooooore.",
        ".ekmoooooooooore.",
        ".ekmmooooooommke.",
        ".ekkmmmooommmkke.",
        "..ekkkmmmmmkkke..",
        "...eekkkkkkkee...",
        ".....eeeeeee.....",
        ".................",
    ],
    [
        "......aaaaaaa....",
        "......adffdda....",
        ".....afddbba.....",
        ".....afdbbba.....",
        "..aa.alliaaaa....",
        "..afaaaaafffda...",
        "..abdfffdbbbba...",
        "...bbbbbbbbbb....",
        "....booborre.....",
        "...bqpoobore.....",
        "....bbbboore.....",
        "....eooooree.....",
        "....eeeeijnre....",
        "...enjijjnorrcc..",
        ".cceoboojnoorcgc.",
        "chhekoooinnorrchc",
        ".ccekbommccckkhhc",
        "..ekccccchhgggcc.",
        ".ekchhgggcccccre.",
        ".ekmcccccooooore.",
        ".ekmoboooooooore.",
        ".ekmoooooooooore.",
        ".ekmmooooooommke.",
        ".ekkmmmooommmkke.",
        "..ekkkmmmmmkkke..",
        "...eekkkkkkkee...",
        ".....eeeeeee.....",
        ".................",
        ".................",
    ],
    [
        "......aaaaaaa....",
        "......adffdda....",
        ".....afddbba.....",
        ".....afdbbba.....",
        "..aa.alliaaaa....",
        "..afaaaaafffda...",
        "..abdfffdbbbba...",
        "...bbbbbbbbbb....",
        "....dooooooe.....",
        "....dooborre.....",
        "...bqpoobore.....",
        "....bbbboore.....",
        "....eooooree.....",
        "....eeeeijnre....",
        "...enjijjnorrcc..",
        ".cceoboojnoorcgc.",
        "chhekoooinnorrchc",
        ".ccekbommccckkhhc",
        "..ekccccchhgggcc.",
        ".ekchhgggcccccre.",
        ".ekmcccccooooore.",
        ".ekmoboooooooore.",
        ".ekmoooooooooore.",
        ".ekmmooooooommke.",
        ".ekkmmmooommmkke.",
        "..ekkkmmmmmkkke..",
        "...eekkkkkkkee...",
        ".....eeeeeee.....",
        ".................",
    ],
    [
        ".................",
        ".................",
        "......aaaaaaa....",
        "......adffdda....",
        ".....afddbba.....",
        ".....afdbbba.....",
        "..aa.alliaaaa....",
        "..afaaaaafffda...",
        "..abdfffdbbbba...",
        "...bbbbbbbbbb....",
        "....booborre.....",
        "...bqpoobore.....",
        "....bbbboore.....",
        "....eooooree.....",
        "....eeeeijnre....",
        "...enjijjnorrcc..",
        ".cceoboojnoorcgc.",
        "chhekoooinnorrchc",
        ".ccekbommccckkhhc",
        "..ekccccchhgggcc.",
        ".ekchhgggcccccre.",
        ".ekmcccccooooore.",
        ".ekmoboooooooore.",
        ".ekmoooooooooore.",
        ".ekmmooooooommke.",
        ".ekkmmmooommmkke.",
        "..ekkkmmmmmkkke..",
        "...eekkkkkkkee...",
        ".....eeeeeee.....",
    ],
];

/// A bow for the top of a tree (R red, K dark red) and a star (Y gold, y pale).
const BOW: [&str; 5] = ["RR.....RR", "RRR...RRR", ".RRRRRRR.", "..RRKRR..", ".RR...RR."];
const STAR: [&str; 6] = ["...Y...", "..YYY..", "YYYyYYY", ".YYyYY.", ".YYYYY.", ".YY.YY."];

// ---------------------------------------------------------------------------------------------- random numbers

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 32) as u32
    }

    /// 0 up to, not including, 1.
    fn f(&mut self) -> f32 {
        (self.next() >> 8) as f32 / 16_777_216.0
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.f()
    }

    fn pick(&mut self, colors: &[u8]) -> u8 {
        colors[self.next() as usize % colors.len()]
    }
}

// ---------------------------------------------------------------------------------------------- pixel art

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Sprout,
    Sapling,
    Mature,
}

impl Stage {
    fn of(growth: f32) -> Stage {
        if growth < 0.22 {
            Stage::Sprout
        } else if growth < 0.55 {
            Stage::Sapling
        } else {
            Stage::Mature
        }
    }
}

/// What a grown tree looks like. The three dressed ones are plain pines until they are full-grown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Species {
    Pine,
    /// Red bow on top, coloured lights.
    Bow,
    /// Gold star on top, warm lights.
    Star,
    /// Gold star on top, a red garland winding down.
    Garland,
    Snowman,
}

struct Preset {
    tiers: usize,
    tier_height: usize,
    step: f32,
    trunk: usize,
    trunk_width: usize,
}

/// Mature pines differ in height (28 to 49 rows) and a little in breadth; the young ones in size too.
fn preset(stage: usize, variant: usize) -> Preset {
    match stage {
        0 => Preset { tiers: 1, tier_height: 4, step: 1.2, trunk: 2, trunk_width: 1 },
        1 => Preset { tiers: 2 + variant % 3, tier_height: 6, step: 1.8, trunk: 3, trunk_width: 2 },
        _ => Preset {
            tiers: [5, 6, 7, 6, 8, 5, 7, 6][variant % VARIANTS],
            tier_height: 7 + variant % 2,
            step: 2.0 + (variant % 3) as f32 * 0.15,
            trunk: 5 + variant % 2,
            trunk_width: 3,
        },
    }
}

/// A picture as a list of coloured cells: column from the middle, row up from the base, palette index.
pub struct Art {
    pub width: usize,
    pub height: usize,
    cells: Vec<(i16, i16, u8)>,
}

fn half_top(p: &Preset, tier: usize) -> f32 {
    0.4 + tier as f32 * p.step * 0.9
}

fn half_bottom(p: &Preset, tier: usize) -> f32 {
    half_top(p, tier) + p.step * 1.5 + 0.8
}

/// A pine of stacked, ragged cones. The open top edge of every cone gets frost, the underside gets shade.
/// Dressed species get their lights, garland and topper on top of the same shape.
fn build_art(p: &Preset, seed: u64, species: Species) -> Art {
    let mut rng = Rng(seed | 1);
    let mut deco = Rng((seed ^ 0xA5A5_5A5A_1234_5678) | 1); // the dressing leaves the shape and shading alone
    let last = p.tiers - 1;
    let max_half = half_bottom(p, last).ceil() as usize + 2;
    let width = max_half * 2 + 1;
    let centre = max_half;
    let height = last * (p.tier_height - 3) + p.tier_height + p.trunk;
    let mut grid = vec![vec![-1i8; width]; height];

    for tier in 0..p.tiers {
        let y0 = tier * (p.tier_height - 3); // tiers overlap by three rows
        for row in 0..p.tier_height {
            let t = row as f32 / (p.tier_height - 1) as f32;
            let v = half_top(p, tier) + (half_bottom(p, tier) - half_top(p, tier)) * t;
            let mut half = (v + (rng.f() - 0.5) * 0.9).round().max(0.0) as i32;
            if tier == 0 && row == 0 {
                half = 0; // a sharp tip
            }
            if row == p.tier_height - 1 {
                half += 1; // the boughs droop past the tier
            }
            for dx in -half..=half {
                // ragged lower edge
                if row == p.tier_height - 1 && dx.abs() >= half - 1 && ((dx + tier as i32) & 1) != 0 {
                    continue;
                }
                grid[y0 + row][(centre as i32 + dx) as usize] = tier as i8;
            }
        }
    }
    let trunk_left = centre - (p.trunk_width >> 1);
    for row in grid.iter_mut().skip(height - p.trunk) {
        for cell in &mut row[trunk_left..trunk_left + p.trunk_width] {
            *cell = 100;
        }
    }

    let at = |y: i32, x: i32| -> i8 {
        if y < 0 || y >= height as i32 || x < 0 || x >= width as i32 { -1 } else { grid[y as usize][x as usize] }
    };
    let mut cells = Vec::new();
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let k = at(y, x);
            if k < 0 {
                continue;
            }
            let (dx, dy) = ((x - centre as i32) as i16, (height as i32 - 1 - y) as i16);
            let mut color = if k == 100 {
                if x == (trunk_left + p.trunk_width - 1) as i32 || rng.f() < 0.25 { TRUNK2 } else { TRUNK }
            } else {
                let (up, up2, down) = (at(y - 1, x), at(y - 2, x), at(y + 1, x));
                let (left, right) = (at(y, x - 1), at(y, x + 1));
                if up < 0 {
                    if rng.f() < 0.88 { FROST } else { FROST2 } // snow cap of the tier
                } else if up2 < 0 && rng.f() < 0.5 {
                    FROST2
                } else if (left < 0 || right < 0) && rng.f() < 0.28 {
                    FROST3 // rime on the sides
                } else if down != k {
                    if rng.f() < 0.5 { OUT } else { G1 } // shade under the tier
                } else {
                    let v = rng.f();
                    if x > centre as i32 && v < 0.5 {
                        G1
                    } else if v < 0.2 {
                        G3
                    } else if v < 0.7 {
                        G2
                    } else {
                        G1
                    }
                }
            };
            if matches!(color, G1 | G2 | G3) {
                color = dress(species, color, dx, dy, &mut deco);
            }
            cells.push((dx, dy, color));
        }
    }

    let mut height = height;
    let topper: &[&str] = match species {
        Species::Bow => &BOW,
        Species::Star | Species::Garland => &STAR,
        _ => &[],
    };
    for (r, line) in topper.iter().enumerate() {
        let offset = line.len() as i16 / 2;
        for (c, ch) in line.bytes().enumerate() {
            let color = match ch {
                b'R' => RED,
                b'K' => RED_DARK,
                b'Y' => GOLD,
                b'y' => GOLD_PALE,
                _ => continue,
            };
            // the bottom row of the topper sits on the tip
            let dy = (height - 1 + topper.len() - 1 - r) as i16;
            cells.retain(|cell| !(cell.0 == c as i16 - offset && cell.1 == dy));
            cells.push((c as i16 - offset, dy, color));
        }
    }
    if !topper.is_empty() {
        height += topper.len() - 1;
    }
    Art { width, height, cells }
}

/// Lights and garland for a green cell of a dressed tree.
fn dress(species: Species, color: u8, dx: i16, dy: i16, rng: &mut Rng) -> u8 {
    match species {
        Species::Bow if rng.f() < 0.11 => rng.pick(&[RED, BLUE, YELLOW, LIME]),
        Species::Star if rng.f() < 0.10 => rng.pick(&[GOLD, GOLD_PALE, YELLOW]),
        Species::Garland => {
            let band = (dx + 2 * dy).rem_euclid(13);
            if band == 0 {
                RED
            } else if band == 1 {
                RED_DARK
            } else if rng.f() < 0.05 {
                GOLD
            } else {
                color
            }
        }
        _ => color,
    }
}

fn build_snowman() -> Vec<Art> {
    SNOWMAN
        .iter()
        .map(|rows| {
            let height = rows.len();
            let mut cells = Vec::new();
            for (r, line) in rows.iter().enumerate() {
                for (c, ch) in line.bytes().enumerate() {
                    if ch != b'.' {
                        cells.push((c as i16 - 8, (height - 1 - r) as i16, SM + (ch - b'a')));
                    }
                }
            }
            Art { width: rows[0].len(), height, cells }
        })
        .collect()
}

fn pixel(position: V2, size: f32, color: u8, alpha: f32) -> Sprite {
    Sprite {
        position,
        velocity: V2::ZERO,
        size,
        alpha,
        soft: 0.0,
        tint_amount: 1.0,
        tint: PALETTE[color as usize],
        lump: PIXEL_LUMP,
    }
}

// ---------------------------------------------------------------------------------------------- the forest

/// What the user is doing, which decides where trees grow.
pub struct Input {
    /// The window that has the focus; [`GROUND`] if none.
    pub foreground: WindowId,
    /// Typing or clicking lately: the taskbar grows trees too.
    pub typing: bool,
}

/// A place trees can stand: the top edge of a window, or the ground.
#[derive(Clone, PartialEq)]
struct Slot {
    id: WindowId,
    frame: Rect,
    blockers: Vec<Rect>,
}

impl Slot {
    fn top(&self) -> f32 {
        self.frame.max_y()
    }

    /// Is the point hidden behind a window in front?
    fn blocked(&self, x: f32, y: f32) -> bool {
        self.id != GROUND && self.blockers.iter().any(|b| x >= b.min_x() && x <= b.max_x() && y >= b.min_y() && y <= b.max_y())
    }
}

struct Tree {
    slot: WindowId,
    /// Distance from the slot's left edge.
    x: f32,
    growth: f32,
    variant: usize,
    species: Species,
    stage: Stage,
    /// Snowmen: seconds into the hop being played, or minus the seconds to wait for the next one.
    hop: f32,
    /// Seconds without a place to stand, as when its window is minimized.
    orphaned: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Needle,
    Flake,
}

struct Rest {
    slot: WindowId,
    column: usize,
    level: u8,
    life: f32,
}

/// A needle or a bit of snow, falling or lying in a small pile that thins away.
struct Bit {
    position: V2,
    velocity: V2,
    kind: Kind,
    color: u8,
    bounces: u8,
    phase: f32,
    rest: Option<Rest>,
}

struct Streak {
    from: V2,
    to: V2,
    age: f32,
}

pub struct Forest {
    width: f32,
    /// Young pines by stage (sprout, sapling, mature) and variant.
    pines: Vec<Vec<Art>>,
    /// Dressed mature trees by species (bow, star, garland) and variant.
    dressed: Vec<Vec<Art>>,
    snowman: Vec<Art>,
    slots: Vec<Slot>,
    trees: Vec<Tree>,
    bits: Vec<Bit>,
    streaks: Vec<Streak>,
    piles: HashMap<WindowId, Vec<u8>>,
    planting: HashMap<WindowId, f32>,
    active: HashMap<WindowId, f32>,
    cursor: Option<V2>,
    rng: Rng,
    time: f32,
    out: Vec<Sprite>,
}

impl Forest {
    pub fn new(width: f32, _height: f32) -> Forest {
        let seed = |stage: usize, v: usize| (stage * 977 + v * 7919 + 13) as u64;
        let pines = (0..3).map(|s| (0..VARIANTS).map(|v| build_art(&preset(s, v), seed(s, v), Species::Pine)).collect()).collect();
        let dressed = [Species::Bow, Species::Star, Species::Garland]
            .iter()
            .map(|&sp| (0..VARIANTS).map(|v| build_art(&preset(2, v), seed(2, v), sp)).collect())
            .collect();
        Forest {
            width,
            pines,
            dressed,
            snowman: build_snowman(),
            slots: Vec::new(),
            trees: Vec::new(),
            bits: Vec::new(),
            streaks: Vec::new(),
            piles: HashMap::new(),
            planting: HashMap::new(),
            active: HashMap::new(),
            cursor: None,
            rng: Rng(0x9E37_79B9_7F4A_7C15),
            time: 0.0,
            out: Vec::new(),
        }
    }

    pub fn resize(&mut self, width: f32, _height: f32) {
        self.width = width;
        self.slots.clear();
    }

    /// How many sprites [`Forest::write`] will produce.
    pub fn sprite_count(&self) -> usize {
        self.out.len()
    }

    pub fn write(&self, sprites: &mut [Sprite]) -> usize {
        let n = self.out.len().min(sprites.len());
        sprites[..n].copy_from_slice(&self.out[..n]);
        n
    }

    pub fn pace(&self) -> Pace {
        if !self.streaks.is_empty() || self.bits.iter().any(|b| b.rest.is_none()) {
            Pace::Interactive
        } else if !self.bits.is_empty() || self.trees.iter().any(|t| t.hop >= 0.0) {
            Pace::Lively
        } else {
            Pace::Calm
        }
    }

    fn slot(&self, id: WindowId) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    fn art(&self, tree: &Tree) -> &Art {
        match tree.species {
            Species::Snowman => &self.snowman[if tree.hop >= 0.0 { ((tree.hop * 10.0) as usize).min(4) } else { 0 }],
            Species::Pine => &self.pines[tree.stage as usize][tree.variant],
            species if tree.stage == Stage::Mature => &self.dressed[species as usize - 1][tree.variant],
            _ => &self.pines[tree.stage as usize][tree.variant],
        }
    }

    /// Advances everything; returns whether the picture changed.
    pub fn step(&mut self, dt: f32, surfaces: &[WindowSurface], cursor: V2, input: &Input) -> bool {
        let dt = dt.clamp(0.0, 0.05);
        self.time += dt;
        let mut dirty = false;

        let mut slots: Vec<Slot> = surfaces.iter().map(|s| Slot { id: s.id, frame: s.frame, blockers: s.blockers.clone() }).collect();
        slots.push(Slot { id: GROUND, frame: Rect { x: 0.0, y: 0.0, width: self.width, height: 0.0 }, blockers: Vec::new() });
        if slots != self.slots {
            self.slots = slots;
            dirty = true;
        }

        // Where the user is working, trees grow, for a moment after as well.
        if input.foreground != GROUND {
            self.active.insert(input.foreground, ACTIVE_FOR);
        }
        if input.typing {
            self.active.insert(GROUND, ACTIVE_FOR);
        }
        self.active.retain(|_, left| {
            *left -= dt;
            *left > 0.0
        });

        for i in 0..self.slots.len() {
            let id = self.slots[i].id;
            if !self.active.contains_key(&id) {
                continue;
            }
            let clock = self.planting.entry(id).or_insert(0.0);
            *clock += dt;
            if *clock > 0.8 {
                *clock = 0.0;
                dirty |= self.plant(i);
            }
        }

        dirty |= self.grow(dt);
        dirty |= self.harvest(dt, cursor);
        self.move_bits(dt);

        for streak in &mut self.streaks {
            streak.age += dt;
        }
        self.streaks.retain(|s| s.age < 0.14);

        if dirty || !self.bits.is_empty() || !self.streaks.is_empty() {
            self.rebuild();
            return true;
        }
        false
    }

    /// Tries to start something on a slot; returns whether it did.
    fn plant(&mut self, i: usize) -> bool {
        let (id, left, width) = (self.slots[i].id, self.slots[i].frame.x, self.slots[i].frame.width);
        let top = self.slots[i].top();
        if width < 48.0 || self.trees.len() >= MAX_TREES {
            return false;
        }
        let cap = ((width / (PX * 16.0)) as usize).clamp(1, 8);
        if self.trees.iter().filter(|t| t.slot == id).count() >= cap {
            return false;
        }
        // Now and then a snowman, a third of the trees come dressed for Christmas.
        let roll = self.rng.f();
        let snowmen = self.trees.iter().filter(|t| t.species == Species::Snowman).count();
        let here = self.trees.iter().any(|t| t.slot == id && t.species == Species::Snowman);
        let species = if roll < 0.10 && !here && snowmen < MAX_SNOWMEN {
            Species::Snowman
        } else if roll < 0.40 {
            [Species::Bow, Species::Star, Species::Garland][self.rng.next() as usize % 3]
        } else {
            Species::Pine
        };
        let room = if species == Species::Snowman { 36.0 } else { 26.0 };
        let x = self.rng.range(20.0, width - 20.0);
        if self.trees.iter().any(|t| t.slot == id && (t.x - x).abs() < room) {
            return false;
        }
        let at = left + x;
        if at < 20.0 || at > self.width - 20.0 || self.slots[i].blocked(at, top - 1.0) {
            return false;
        }
        let variant = self.rng.next() as usize % VARIANTS;
        let hop = -self.rng.range(2.0, 4.0);
        self.trees.push(Tree { slot: id, x, growth: 0.0, variant, species, stage: Stage::Sprout, hop, orphaned: 0.0 });
        true
    }

    fn grow(&mut self, dt: f32) -> bool {
        let mut changed = false;
        let mut puffs = Vec::new();
        let mut gone = Vec::new();
        for (index, tree) in self.trees.iter_mut().enumerate() {
            let Some(slot) = self.slots.iter().find(|s| s.id == tree.slot) else {
                tree.orphaned += dt;
                if tree.orphaned > 30.0 {
                    gone.push(index);
                }
                continue;
            };
            tree.orphaned = 0.0;
            if tree.x > slot.frame.width - 6.0 {
                gone.push(index); // the window was made too narrow for it
                continue;
            }
            let snowman = tree.species == Species::Snowman;
            if tree.growth < 1.0 && self.active.contains_key(&tree.slot) {
                let seconds = if snowman {
                    7.0
                } else if tree.slot == GROUND {
                    22.0
                } else {
                    16.0
                };
                tree.growth = (tree.growth + dt / seconds).min(1.0);
                // a snowman is finished all at once; before that he is built up from the bottom
                let stage = if snowman {
                    if tree.growth >= 1.0 { Stage::Mature } else { Stage::Sprout }
                } else {
                    Stage::of(tree.growth)
                };
                changed |= snowman;
                if stage != tree.stage {
                    tree.stage = stage;
                    changed = true;
                    puffs.push(V2::new(slot.frame.x + tree.x, slot.top() + 6.0));
                }
            }
            if snowman && tree.stage == Stage::Mature {
                if tree.hop < 0.0 {
                    tree.hop = (tree.hop + dt).min(0.0);
                } else {
                    tree.hop += dt;
                    changed = true;
                    if tree.hop >= HOP {
                        tree.hop = -self.rng.range(2.5, 6.0);
                    }
                }
            }
        }
        for &index in gone.iter().rev() {
            self.trees.remove(index);
            changed = true;
        }
        for at in puffs {
            for _ in 0..6 {
                let (x, y) = (self.rng.range(-8.0, 8.0), self.rng.range(0.0, 14.0));
                let vx = self.rng.range(-20.0, 20.0);
                let color = self.rng.pick(&[FROST, FROST2]);
                self.spawn_bit(Kind::Flake, color, V2::new(at.x + x, at.y + y), V2::new(vx, 10.0));
            }
        }
        changed
    }

    /// The pointer is a pruner: a fast swipe through a full-grown tree, or a finished snowman, cuts it down.
    fn harvest(&mut self, dt: f32, cursor: V2) -> bool {
        if cursor.x < -5000.0 || cursor.y < -5000.0 || dt <= 0.0 {
            self.cursor = None;
            return false;
        }
        let Some(previous) = self.cursor.replace(cursor) else { return false };
        let travel = cursor - previous;
        let distance = travel.length();
        if distance / dt < CUT_SPEED {
            return false;
        }
        self.streaks.push(Streak { from: previous, to: cursor, age: 0.0 });

        let steps = ((distance / 6.0).ceil() as usize).max(1);
        let mut hit = Vec::new();
        for (index, tree) in self.trees.iter().enumerate() {
            if tree.stage != Stage::Mature {
                continue;
            }
            let Some(slot) = self.slot(tree.slot) else { continue };
            let x = slot.frame.x + tree.x;
            if slot.blocked(x, slot.top() - 1.0) {
                continue;
            }
            let art = self.art(tree);
            let half = art.width as f32 * PX * 0.4;
            let (low, high) = (slot.top() + 3.0, slot.top() + art.height as f32 * PX);
            let touched = (0..=steps).any(|k| {
                let p = previous + travel * (k as f32 / steps as f32);
                p.x > x - half && p.x < x + half && p.y > low && p.y < high
            });
            if touched {
                hit.push(index);
            }
        }
        for &index in hit.iter().rev() {
            self.cut(index, travel);
        }
        !hit.is_empty()
    }

    fn cut(&mut self, index: usize, swipe: V2) {
        let tree = self.trees.remove(index);
        let (x, top, w, h) = {
            let Some(slot) = self.slot(tree.slot) else { return };
            let art = self.art(&tree);
            (slot.frame.x + tree.x, slot.top(), art.width as f32 * PX, art.height as f32 * PX)
        };
        let area = (x, top, w, h);
        match tree.species {
            Species::Snowman => {
                self.burst(Kind::Flake, 55, &[FROST, FROST2, SM_ICE], area, swipe.x);
                self.burst(Kind::Needle, 14, &[SM_COAL, SM_RED, SM_RED, SM_BROWN], area, swipe.x);
            }
            species => {
                self.burst(Kind::Needle, 30, &[G1, G2, G3], area, swipe.x);
                self.burst(Kind::Flake, 40, &[FROST, FROST, FROST2], area, swipe.x);
                if species != Species::Pine {
                    self.burst(Kind::Needle, 12, &[RED, BLUE, YELLOW, GOLD, LIME], area, swipe.x);
                }
            }
        }
    }

    /// `area` is the left-right middle, the bottom, the width and the height of what was cut.
    fn burst(&mut self, kind: Kind, count: usize, colors: &[u8], area: (f32, f32, f32, f32), swipe_x: f32) {
        let (x, top, w, h) = area;
        for _ in 0..count {
            let position = V2::new(x + self.rng.range(-0.4, 0.4) * w, top + self.rng.range(0.0, 1.0) * h);
            let velocity = match kind {
                Kind::Needle => V2::new(self.rng.range(-130.0, 130.0) + swipe_x * 4.0, self.rng.range(60.0, 280.0)),
                Kind::Flake => V2::new(self.rng.range(-60.0, 60.0), self.rng.range(0.0, 90.0)),
            };
            let color = self.rng.pick(colors);
            self.spawn_bit(kind, color, position, velocity);
        }
    }

    fn spawn_bit(&mut self, kind: Kind, color: u8, position: V2, velocity: V2) {
        if self.bits.len() >= MAX_BITS {
            return;
        }
        let phase = self.rng.range(0.0, 6.28);
        self.bits.push(Bit { position, velocity, kind, color, bounces: 0, phase, rest: None });
    }

    /// Gravity, bounces off the top edges of windows and the taskbar, and the piles that lie there and thin away.
    fn move_bits(&mut self, dt: f32) {
        let (slots, piles, rng, time, width) = (&self.slots, &mut self.piles, &mut self.rng, self.time, self.width);
        self.bits.retain_mut(|b| {
            if let Some(rest) = &mut b.rest {
                rest.life -= dt;
                let present = slots.iter().any(|s| s.id == rest.slot && (rest.column as f32) * PX < s.frame.width);
                if rest.life > 0.0 && present {
                    return true;
                }
                if let Some(h) = piles.get_mut(&rest.slot).and_then(|v| v.get_mut(rest.column)) {
                    *h = h.saturating_sub(1);
                }
                return false;
            }
            match b.kind {
                Kind::Needle => b.velocity.y -= GRAVITY * dt,
                Kind::Flake => {
                    b.velocity.y = (b.velocity.y - 140.0 * dt).max(-70.0);
                    let sway = (time * 2.0 + b.phase).sin() * 14.0;
                    b.velocity.x += (sway - b.velocity.x) * (dt * 2.0).min(1.0);
                }
            }
            let before = b.position;
            b.position = b.position + b.velocity * dt;
            if b.velocity.y < 0.0 {
                let mut landing: Option<&Slot> = None;
                for s in slots {
                    let top = s.top();
                    if b.position.x >= s.frame.min_x()
                        && b.position.x <= s.frame.max_x()
                        && before.y >= top
                        && b.position.y <= top
                        && !s.blocked(b.position.x, top - 1.0)
                        && landing.is_none_or(|l| top > l.top())
                    {
                        landing = Some(s);
                    }
                }
                if let Some(s) = landing {
                    if b.kind == Kind::Needle && b.bounces < 2 {
                        b.position.y = s.top() + 0.5;
                        b.velocity.y *= -0.35;
                        b.velocity.x *= 0.6;
                        b.bounces += 1;
                        return true;
                    }
                    let column = ((b.position.x - s.frame.x) / PX).max(0.0) as usize;
                    let heights = piles.entry(s.id).or_default();
                    if heights.len() <= column {
                        heights.resize(column + 1, 0);
                    }
                    if heights[column] >= 4 {
                        return false;
                    }
                    let level = heights[column];
                    heights[column] += 1;
                    let life = (4.0 + rng.f() * 4.0 - level as f32 * 0.9).max(1.2);
                    b.rest = Some(Rest { slot: s.id, column, level, life });
                    b.velocity = V2::ZERO;
                    return true;
                }
            }
            b.position.y > -20.0 && b.position.x > -20.0 && b.position.x < width + 20.0
        });
    }

    /// Lays out every visible square, back to front.
    fn rebuild(&mut self) {
        let mut out = std::mem::take(&mut self.out);
        out.clear();

        for tree in &self.trees {
            let Some(slot) = self.slot(tree.slot) else { continue };
            let x = slot.frame.x + tree.x;
            if slot.blocked(x, slot.top() - 1.0) {
                continue;
            }
            let art = self.art(tree);
            // a snowman still being built shows only his lower rows
            let shown = if tree.species == Species::Snowman && tree.growth < 1.0 {
                (tree.growth * art.height as f32).ceil() as i16
            } else {
                i16::MAX
            };
            let centre = (x / PX).round() * PX;
            let base = slot.top();
            for &(dx, dy, color) in &art.cells {
                if dy < shown {
                    out.push(pixel(V2::new(centre + dx as f32 * PX, base + (dy as f32 + 0.5) * PX), PX, color, 1.0));
                }
            }
        }

        for bit in &self.bits {
            match &bit.rest {
                Some(rest) => {
                    let Some(slot) = self.slot(rest.slot) else { continue };
                    let x = slot.frame.x + rest.column as f32 * PX + PX / 2.0;
                    if slot.blocked(x, slot.top() - 1.0) {
                        continue;
                    }
                    let y = slot.top() + (rest.level as f32 + 0.5) * PX;
                    out.push(pixel(V2::new(x, y), PX, bit.color, (rest.life / 1.2).min(1.0)));
                }
                None => out.push(pixel(bit.position, PX, bit.color, 1.0)),
            }
        }

        for streak in &self.streaks {
            let fade = 1.0 - streak.age / 0.14;
            let line = streak.to - streak.from;
            let n = ((line.length() / 4.0).ceil() as usize).max(1);
            for k in 0..=n {
                out.push(pixel(streak.from + line * (k as f32 / n as f32), PX, FROST, fade));
            }
        }
        self.out = out;
    }
}

/// The livelier of two paces.
pub fn busiest(a: Pace, b: Pace) -> Pace {
    let rank = |p: Pace| match p {
        Pace::Calm => 0,
        Pace::Lively => 1,
        Pace::Interactive => 2,
    };
    if rank(b) > rank(a) { b } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> WindowSurface {
        WindowSurface {
            id: 7,
            frame: Rect { x: 200.0, y: 300.0, width: 400.0, height: 300.0 },
            blockers: Vec::new(),
            corner_radius: 0.0,
            corner_exponent: 2.0,
        }
    }

    const NOBODY: V2 = V2::new(-9999.0, -9999.0);

    fn grown() -> Forest {
        let mut forest = Forest::new(1000.0, 800.0);
        let focus = Input { foreground: 7, typing: false };
        for _ in 0..1200 {
            forest.step(0.05, &[window()], NOBODY, &focus);
        }
        forest
    }

    #[test]
    fn pines_have_frost_a_trunk_and_differ_in_height() {
        let forest = Forest::new(1000.0, 800.0);
        for stage in &forest.pines {
            for art in stage {
                assert!(art.cells.iter().any(|c| c.2 == FROST));
                assert!(art.cells.iter().any(|c| c.2 == TRUNK || c.2 == TRUNK2));
                assert!(art.cells.iter().all(|c| c.1 >= 0 && (c.1 as usize) < art.height));
            }
        }
        let mut heights: Vec<usize> = forest.pines[2].iter().map(|a| a.height).collect();
        heights.sort();
        heights.dedup();
        assert!(heights.len() >= 4, "mature pines should come in several heights: {heights:?}");
    }

    #[test]
    fn dressed_trees_have_their_toppers_and_decorations() {
        let forest = Forest::new(1000.0, 800.0);
        let has = |art: &Art, color: u8| art.cells.iter().any(|c| c.2 == color);
        assert!(forest.dressed[0].iter().all(|a| has(a, RED) && has(a, BLUE)), "bow tree");
        assert!(forest.dressed[1].iter().all(|a| has(a, GOLD) && has(a, GOLD_PALE)), "star tree");
        assert!(forest.dressed[2].iter().all(|a| has(a, GOLD) && has(a, RED)), "garland tree");
        for (dressed, plain) in forest.dressed[0].iter().zip(&forest.pines[2]) {
            assert!(dressed.height > plain.height, "the topper adds height");
            assert!(dressed.cells.iter().all(|c| c.1 >= 0 && (c.1 as usize) < dressed.height));
        }
    }

    #[test]
    fn the_snowman_has_five_frames_that_hop() {
        let forest = Forest::new(1000.0, 800.0);
        assert_eq!(forest.snowman.len(), 5);
        assert!(forest.snowman.iter().all(|a| a.width == 17 && a.height == 29));
        let lowest = |a: &Art| a.cells.iter().map(|c| c.1).min().unwrap();
        assert_eq!(lowest(&forest.snowman[0]), 0, "standing on the ground");
        assert!(lowest(&forest.snowman[2]) > 0, "in the air");
        assert!(forest.snowman[0].cells.iter().any(|c| c.2 == SM_RED), "the scarf");
    }

    #[test]
    fn trees_grow_only_where_the_user_works() {
        let mut idle = Forest::new(1000.0, 800.0);
        let away = Input { foreground: GROUND, typing: false };
        for _ in 0..200 {
            idle.step(0.05, &[window()], NOBODY, &away);
        }
        assert!(idle.trees.is_empty());

        let forest = grown();
        assert!(!forest.trees.is_empty());
        assert!(forest.trees.iter().all(|t| t.slot == 7));
        assert!(forest.trees.iter().any(|t| t.stage == Stage::Mature));
        assert!(forest.sprite_count() > 0);
    }

    #[test]
    fn a_fast_swipe_cuts_something_grown_and_a_slow_one_does_not() {
        let mut forest = grown();
        let focus = Input { foreground: 7, typing: false };
        let tree = forest.trees.iter().find(|t| t.stage == Stage::Mature).unwrap();
        let (x, y) = (200.0 + tree.x, 600.0 + 30.0);
        let before = forest.trees.len();

        forest.step(0.016, &[window()], V2::new(x - 30.0, y), &focus);
        forest.step(0.016, &[window()], V2::new(x - 29.0, y), &focus);
        assert_eq!(forest.trees.len(), before, "slow movement must not cut");

        forest.step(0.016, &[window()], V2::new(x + 30.0, y), &focus);
        assert!(forest.trees.len() < before);
        assert!(!forest.bits.is_empty());
    }

    #[test]
    fn a_snowman_builds_up_from_the_bottom_then_hops() {
        let mut forest = Forest::new(1000.0, 800.0);
        let focus = Input { foreground: 7, typing: false };
        forest.step(0.016, &[window()], NOBODY, &focus);
        forest.trees.push(Tree { slot: 7, x: 100.0, growth: 0.3, variant: 0, species: Species::Snowman, stage: Stage::Sprout, hop: -0.1, orphaned: 0.0 });
        forest.step(0.016, &[window()], NOBODY, &focus);
        let top = forest.out.iter().map(|s| s.position.y).fold(0.0, f32::max);
        assert!(top < 600.0 + 29.0 * PX * 0.5, "only the lower rows are there so far");
        for _ in 0..400 {
            forest.step(0.05, &[window()], NOBODY, &focus);
        }
        let snowman = forest.trees.iter().find(|t| t.species == Species::Snowman).unwrap();
        assert_eq!(snowman.stage, Stage::Mature);
    }

    #[test]
    fn falling_snow_lands_on_the_window_below_and_fades() {
        let mut forest = Forest::new(1000.0, 800.0);
        let away = Input { foreground: GROUND, typing: false };
        forest.step(0.016, &[window()], NOBODY, &away);
        forest.spawn_bit(Kind::Flake, FROST, V2::new(300.0, 640.0), V2::new(0.0, -50.0));
        for _ in 0..200 {
            forest.step(0.016, &[window()], NOBODY, &away);
        }
        assert!(forest.bits.iter().any(|b| b.rest.as_ref().is_some_and(|r| r.slot == 7)));
        for _ in 0..1000 {
            forest.step(0.016, &[window()], NOBODY, &away);
        }
        assert!(forest.bits.is_empty());
    }
}
