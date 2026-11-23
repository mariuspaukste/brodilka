//! World map: procedural generation, tiles, objects, pathfinding.
use crate::data::*;
use crate::rng::Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub const W: i32 = 160;
pub const H: i32 = 160;
pub const N: usize = (W * H) as usize;

pub const T_WATER: u8 = 0;
pub const T_SAND: u8 = 1;
pub const T_GRASS: u8 = 2;
pub const T_DIRT: u8 = 3;
pub const T_COBBLE: u8 = 4;
pub const T_FLOOR: u8 = 5;
pub const T_MOUNT: u8 = 6;
pub const T_BRIDGE: u8 = 7;
pub const T_CAVE: u8 = 8;

pub const O_NONE: u8 = 0;
pub const O_TREE: u8 = 1;
pub const O_WALL: u8 = 2;
pub const O_DOOR: u8 = 3;
pub const O_ANVIL: u8 = 4;
pub const O_FORGE: u8 = 5;
pub const O_GRAVE: u8 = 6;
pub const O_TENT: u8 = 7;
pub const O_FIRE: u8 = 8;
pub const O_ANKH: u8 = 9;
pub const O_LAMP: u8 = 10;
pub const O_FOUNTAIN: u8 = 11;
pub const O_BUSH: u8 = 12;
pub const O_BOULDER: u8 = 13;
pub const O_FENCE: u8 = 14;
pub const O_FLOWERS: u8 = 15;
pub const O_HOARD: u8 = 16;
pub const O_SIGN: u8 = 17;

pub struct Building {
    pub x: i32,
    pub y: i32,
    pub vendor: usize,
    pub style: u8,
}

pub struct Map {
    pub t: Vec<u8>,
    pub o: Vec<u8>,
    pub shade: Vec<u8>,
    pub style: Vec<u8>,
    pub res: Vec<u8>,
    pub regrow: Vec<(usize, f32)>,
    pub town: (i32, i32, i32, i32),
    pub start: (i32, i32),
    pub npcs: Vec<(usize, i32, i32)>,
    pub spawns: Vec<(&'static str, i32, i32, i32, usize)>,
    pub ankhs: Vec<(i32, i32)>,
    pub buildings: Vec<Building>,
    pub pois: Vec<(&'static str, i32, i32)>,
    // pathfinding scratch
    pf_g: Vec<f32>,
    pf_par: Vec<i32>,
    pf_stamp: Vec<u32>,
    pf_gen: u32,
}

#[inline]
pub fn idx(x: i32, y: i32) -> usize {
    (y * W + x) as usize
}
#[inline]
pub fn inb(x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < W && y < H
}

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xffffff) as f32 / 16_777_216.0
}
fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - xi as f32;
    let fy = y - yi as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let a = hash(xi, yi, seed);
    let b = hash(xi + 1, yi, seed);
    let c = hash(xi, yi + 1, seed);
    let d = hash(xi + 1, yi + 1, seed);
    let top = a + (b - a) * sx;
    let bot = c + (d - c) * sx;
    top + (bot - top) * sy
}
fn fbm(x: f32, y: f32, seed: u32) -> f32 {
    let mut amp = 0.5;
    let mut f = 1.0;
    let mut sum = 0.0;
    let mut norm = 0.0;
    for o in 0..4 {
        sum += vnoise(x * f, y * f, seed.wrapping_add(o * 101)) * amp;
        norm += amp;
        amp *= 0.5;
        f *= 2.0;
    }
    sum / norm
}

pub fn blocks(o: u8) -> bool {
    !matches!(o, O_NONE | O_DOOR | O_BUSH | O_FLOWERS)
}

#[derive(Copy, Clone, PartialEq)]
struct Node {
    f: f32,
    i: usize,
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

pub const DIRS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

impl Map {
    pub fn tile(&self, x: i32, y: i32) -> u8 {
        if inb(x, y) {
            self.t[idx(x, y)]
        } else {
            T_WATER
        }
    }
    pub fn obj(&self, x: i32, y: i32) -> u8 {
        if inb(x, y) {
            self.o[idx(x, y)]
        } else {
            O_NONE
        }
    }
    pub fn passable(&self, x: i32, y: i32) -> bool {
        if !inb(x, y) {
            return false;
        }
        let i = idx(x, y);
        let t = self.t[i];
        t != T_WATER && t != T_MOUNT && !blocks(self.o[i])
    }
    /// can step from (x,y) by (dx,dy) without cutting corners
    pub fn can_step(&self, x: i32, y: i32, dx: i32, dy: i32) -> bool {
        if !self.passable(x + dx, y + dy) {
            return false;
        }
        if dx != 0 && dy != 0 {
            return self.passable(x + dx, y) && self.passable(x, y + dy);
        }
        true
    }
    pub fn in_town(&self, x: i32, y: i32) -> bool {
        let (x0, y0, x1, y1) = self.town;
        x >= x0 - 3 && y >= y0 - 3 && x < x1 + 3 && y < y1 + 3
    }

    /// A* to any passable tile within `range` (chebyshev) of goal.
    pub fn find_path(
        &mut self,
        sx: i32,
        sy: i32,
        gx: i32,
        gy: i32,
        range: i32,
    ) -> Option<Vec<(i32, i32)>> {
        if !inb(sx, sy) {
            return None;
        }
        let done = |x: i32, y: i32| (x - gx).abs().max((y - gy).abs()) <= range;
        if done(sx, sy) {
            return Some(vec![]);
        }
        self.pf_gen = self.pf_gen.wrapping_add(1);
        let gen = self.pf_gen;
        let h = |x: i32, y: i32| {
            let dx = ((x - gx).abs() - range).max(0) as f32;
            let dy = ((y - gy).abs() - range).max(0) as f32;
            dx.max(dy) + 0.414 * dx.min(dy)
        };
        let mut open = BinaryHeap::new();
        let s = idx(sx, sy);
        self.pf_stamp[s] = gen;
        self.pf_g[s] = 0.0;
        self.pf_par[s] = -1;
        open.push(Node { f: h(sx, sy), i: s });
        let mut expanded = 0;
        while let Some(Node { i, .. }) = open.pop() {
            let x = (i as i32) % W;
            let y = (i as i32) / W;
            if done(x, y) {
                let mut path = vec![];
                let mut c = i as i32;
                while c >= 0 && c as usize != s {
                    path.push((c % W, c / W));
                    c = self.pf_par[c as usize];
                }
                path.reverse();
                return Some(path);
            }
            expanded += 1;
            if expanded > 9000 {
                return None;
            }
            let g = self.pf_g[i];
            for &(dx, dy) in DIRS.iter() {
                if !self.can_step(x, y, dx, dy) {
                    continue;
                }
                let n = idx(x + dx, y + dy);
                let ng = g + if dx != 0 && dy != 0 { 1.414 } else { 1.0 };
                if self.pf_stamp[n] != gen || ng < self.pf_g[n] {
                    self.pf_stamp[n] = gen;
                    self.pf_g[n] = ng;
                    self.pf_par[n] = i as i32;
                    open.push(Node {
                        f: ng + h(x + dx, y + dy),
                        i: n,
                    });
                }
            }
        }
        None
    }

    pub fn tick(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.regrow.len() {
            self.regrow[i].1 -= dt;
            if self.regrow[i].1 <= 0.0 {
                let ix = self.regrow[i].0;
                self.res[ix] = 12;
                self.regrow.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn set(&mut self, x: i32, y: i32, t: u8, o: u8) {
        if inb(x, y) {
            let i = idx(x, y);
            self.t[i] = t;
            self.o[i] = o;
        }
    }
    fn set_o(&mut self, x: i32, y: i32, o: u8) {
        if inb(x, y) {
            let i = idx(x, y);
            self.o[i] = o;
        }
    }

    fn carve_road(&mut self, a: (i32, i32), b: (i32, i32), seed: u32) {
        let (ax, ay) = (a.0 as f32, a.1 as f32);
        let (bx, by) = (b.0 as f32, b.1 as f32);
        let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
        let steps = (len * 2.0) as i32;
        let (nx, ny) = (-(by - ay) / len, (bx - ax) / len);
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let wob = (vnoise(t * 6.0, 0.5, seed) - 0.5) * 8.0 * (t * std::f32::consts::PI).sin();
            let x = (ax + (bx - ax) * t + nx * wob).round() as i32;
            let y = (ay + (by - ay) * t + ny * wob).round() as i32;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1)] {
                let (px, py) = (x + ox, y + oy);
                if !inb(px, py) {
                    continue;
                }
                let i = idx(px, py);
                match self.t[i] {
                    T_WATER => self.t[i] = T_BRIDGE,
                    T_COBBLE | T_FLOOR | T_BRIDGE | T_CAVE => {}
                    _ => self.t[i] = T_DIRT,
                }
                if matches!(self.o[i], O_TREE | O_BUSH | O_BOULDER | O_FLOWERS) {
                    self.o[i] = O_NONE;
                }
            }
        }
    }

    fn building(&mut self, bx: i32, by: i32, style: u8, vendor: usize) {
        for y in by..by + 6 {
            for x in bx..bx + 8 {
                let edge = x == bx || x == bx + 7 || y == by || y == by + 5;
                let i = idx(x, y);
                self.t[i] = T_FLOOR;
                self.o[i] = if edge { O_WALL } else { O_NONE };
                self.style[i] = style;
            }
        }
        self.set_o(bx + 3, by + 5, O_DOOR);
        self.set_o(bx + 4, by + 5, O_DOOR);
        self.set_o(bx + 4, by + 6, O_SIGN);
        self.buildings.push(Building {
            x: bx,
            y: by,
            vendor,
            style,
        });
        self.npcs.push((vendor, bx + 3, by + 2));
    }
}

pub fn generate(seed: u32) -> Map {
    let mut rng = Rng::new(seed as u64 ^ 0xABCD);
    let mut m = Map {
        t: vec![T_GRASS; N],
        o: vec![O_NONE; N],
        shade: vec![0; N],
        style: vec![0; N],
        res: vec![12; N],
        regrow: vec![],
        town: (62, 64, 98, 92),
        start: (80, 79),
        npcs: vec![],
        spawns: vec![],
        ankhs: vec![],
        buildings: vec![],
        pois: vec![],
        pf_g: vec![0.0; N],
        pf_par: vec![-1; N],
        pf_stamp: vec![0; N],
        pf_gen: 0,
    };
    let town_c = (80, 78);
    let grave = (124, 32);
    let orcs = (32, 124);
    let hills = (34, 34);
    let lair = (128, 128);
    let pois: [((i32, i32), f32); 5] = [
        (town_c, 26.0),
        (grave, 15.0),
        (orcs, 14.0),
        (hills, 15.0),
        (lair, 15.0),
    ];

    let mut elev = vec![0f32; N];
    let mut moist = vec![0f32; N];
    for y in 0..H {
        for x in 0..W {
            let i = idx(x, y);
            let mut e = fbm(x as f32 / 26.0, y as f32 / 26.0, seed);
            let edge = x.min(y).min(W - 1 - x).min(H - 1 - y) as f32;
            if edge < 12.0 {
                e -= (12.0 - edge) * 0.05;
            }
            for &((px, py), r) in pois.iter() {
                let d = (((x - px).pow(2) + (y - py).pow(2)) as f32).sqrt();
                if d < r {
                    let t = ((r - d) / (r * 0.4)).min(1.0);
                    e += (0.5 - e) * t;
                }
            }
            elev[i] = e;
            moist[i] = fbm(x as f32 / 18.0 + 50.0, y as f32 / 18.0 + 50.0, seed ^ 0x77);
            m.shade[i] = (hash(x, y, seed ^ 0x55) * 255.0) as u8;
        }
    }
    for i in 0..N {
        let e = elev[i];
        let mo = moist[i];
        m.t[i] = if e < 0.33 {
            T_WATER
        } else if e < 0.36 {
            T_SAND
        } else if e < 0.69 {
            T_GRASS
        } else {
            T_MOUNT
        };
        if m.t[i] == T_GRASS {
            let r = rng.f();
            if mo > 0.56 && r < 0.42 {
                m.o[i] = O_TREE;
            } else if r < 0.025 {
                m.o[i] = O_TREE;
            } else if r < 0.045 {
                m.o[i] = O_BUSH;
            } else if r < 0.06 {
                m.o[i] = O_FLOWERS;
            }
        }
        if m.t[i] == T_GRASS && mo < 0.35 && rng.f() < 0.05 {
            m.t[i] = T_DIRT;
        }
    }

    // --- graveyard
    for y in grave.1 - 8..grave.1 + 8 {
        for x in grave.0 - 10..grave.0 + 10 {
            let edge =
                x == grave.0 - 10 || x == grave.0 + 9 || y == grave.1 - 8 || y == grave.1 + 7;
            let gap = (x - grave.0).abs() <= 1 || (y - grave.1).abs() <= 1;
            let t = if rng.f() < 0.5 { T_DIRT } else { T_GRASS };
            let o = if edge && !gap {
                O_FENCE
            } else if !edge && (x - grave.0 + 10) % 3 == 1 && (y - grave.1 + 8) % 3 == 1 && !gap {
                O_GRAVE
            } else {
                O_NONE
            };
            m.set(x, y, t, o);
        }
    }
    m.pois.push(("Graveyard", grave.0, grave.1));
    // --- orc camp
    for y in orcs.1 - 9..=orcs.1 + 9 {
        for x in orcs.0 - 9..=orcs.0 + 9 {
            let d = (((x - orcs.0).pow(2) + (y - orcs.1).pow(2)) as f32).sqrt();
            if d < 9.0 {
                m.set(x, y, T_DIRT, O_NONE);
            }
        }
    }
    m.set_o(orcs.0, orcs.1, O_FIRE);
    for (dx, dy) in [(-5, -3), (4, -5), (5, 3), (-3, 5), (-6, 2)] {
        m.set_o(orcs.0 + dx, orcs.1 + dy, O_TENT);
    }
    m.pois.push(("Orc Camp", orcs.0, orcs.1));
    // --- rocky hills
    for y in hills.1 - 13..=hills.1 + 13 {
        for x in hills.0 - 13..=hills.0 + 13 {
            if !inb(x, y) {
                continue;
            }
            let d = (((x - hills.0).pow(2) + (y - hills.1).pow(2)) as f32).sqrt();
            if d < 13.0 {
                let i = idx(x, y);
                if m.t[i] == T_GRASS && m.o[i] == O_TREE {
                    m.o[i] = O_NONE;
                }
                if m.t[i] == T_GRASS && rng.f() < 0.4 {
                    m.t[i] = T_DIRT;
                }
                if m.o[i] == O_NONE && d > 3.0 && rng.f() < 0.07 {
                    m.o[i] = O_BOULDER;
                }
            }
        }
    }
    m.pois.push(("Troll Hills", hills.0, hills.1));
    // --- dragon lair: ring of mountains, opening to the west
    for y in lair.1 - 11..=lair.1 + 11 {
        for x in lair.0 - 11..=lair.0 + 11 {
            let d = (((x - lair.0).pow(2) + (y - lair.1).pow(2)) as f32).sqrt();
            if d < 6.5 {
                m.set(x, y, T_CAVE, O_NONE);
            } else if d < 10.0 {
                let opening = x < lair.0 && (y - lair.1).abs() <= 1;
                if opening {
                    m.set(x, y, T_CAVE, O_NONE);
                } else {
                    m.set(x, y, T_MOUNT, O_NONE);
                }
            }
        }
    }
    m.set_o(lair.0 + 2, lair.1 - 2, O_HOARD);
    m.set_o(lair.0 + 3, lair.1 - 1, O_HOARD);
    m.pois.push(("Dragon Lair", lair.0, lair.1));

    // --- roads from the town gates
    let (x0, y0, x1, y1) = m.town;
    m.carve_road((x1, y0 + 13), (grave.0 - 2, grave.1 + 8), seed ^ 1);
    m.carve_road((x0, y0 + 13), (orcs.0 + 6, orcs.1 - 8), seed ^ 2);
    m.carve_road(((x0 + x1) / 2, y0), (hills.0 + 8, hills.1 + 8), seed ^ 3);
    m.carve_road(((x0 + x1) / 2, y1), (lair.0 - 13, lair.1), seed ^ 4);
    m.carve_road((lair.0 - 13, lair.1), (lair.0 - 7, lair.1), seed ^ 5);

    // --- ankh shrines along the roads
    for &(ax, ay) in &[
        (grave.0 - 16, grave.1 + 14),
        (orcs.0 + 14, orcs.1 - 14),
        (hills.0 + 14, hills.1 + 12),
    ] {
        for y in ay - 1..=ay + 1 {
            for x in ax - 1..=ax + 1 {
                m.set(x, y, T_COBBLE, O_NONE);
            }
        }
        m.set_o(ax, ay, O_ANKH);
        m.ankhs.push((ax, ay));
    }

    // --- town
    for y in y0..y1 {
        for x in x0..x1 {
            m.set(x, y, T_COBBLE, O_NONE);
        }
    }
    m.building(x0 + 2, y0 + 2, 1, V_BANKER);
    m.building(x0 + 14, y0 + 2, 2, V_MAGE);
    m.building(x0 + 26, y0 + 2, 3, V_HEALER);
    m.building(x0 + 2, y0 + 16, 4, V_SMITH);
    m.building(x0 + 14, y0 + 16, 3, V_PROV);
    m.building(x0 + 26, y0 + 16, 2, V_PROV);
    // last building is an inn: make its NPC a provisioner variant already pushed; keep it.
    m.set_o(x0 + 3, y0 + 23, O_FORGE);
    m.set_o(x0 + 6, y0 + 23, O_ANVIL);
    for (dx, dy) in [(17, 12), (18, 12), (17, 13), (18, 13)] {
        m.set_o(x0 + dx, y0 + dy, O_FOUNTAIN);
    }
    for (dx, dy) in [
        (1, 10),
        (34, 10),
        (12, 13),
        (23, 13),
        (1, 25),
        (34, 25),
        (12, 9),
        (23, 9),
    ] {
        m.set_o(x0 + dx, y0 + dy, O_LAMP);
    }
    for dx in [4, 8, 16, 20, 28, 32] {
        m.set_o(x0 + dx, y0 + 9, O_FLOWERS);
    }
    m.npcs.push((V_GUARD, x0 + 15, y0 + 11));
    m.npcs.push((V_GUARD, x0 + 1, y0 + 13));
    m.npcs.push((V_GUARD, x1 - 1, y0 + 14));
    m.start = (x0 + 18, y0 + 15);
    m.pois.push(("Town", town_c.0, town_c.1));

    // --- spawn regions (monster, x, y, radius, max)
    m.spawns = vec![
        ("rabbit", 80, 108, 10, 5),
        ("rabbit", 48, 80, 10, 4),
        ("rabbit", 108, 70, 9, 4),
        ("rat", 104, 100, 8, 3),
        ("rat", 56, 98, 8, 3),
        ("mongbat", 70, 50, 8, 3),
        ("mongbat", 130, 80, 10, 3),
        ("slime", 82, 128, 10, 3),
        ("slime", 110, 52, 8, 2),
        ("wolf", 44, 66, 11, 4),
        ("wolf", 112, 110, 11, 3),
        ("skeleton", grave.0, grave.1, 8, 5),
        ("zombie", grave.0, grave.1, 8, 4),
        ("lich", grave.0, grave.1, 3, 1),
        ("orc", orcs.0, orcs.1, 8, 6),
        ("orclord", orcs.0, orcs.1, 3, 1),
        ("troll", hills.0, hills.1, 10, 2),
        ("ettin", hills.0, hills.1, 10, 2),
        ("dragon", lair.0, lair.1, 2, 1),
    ];
    m
}

/// Minimap colour for a tile, RGBA bytes.
pub fn mini_color(m: &Map, i: usize) -> [u8; 4] {
    let c: u32 = match (m.t[i], m.o[i]) {
        (_, O_WALL) => 0x5a4a3aff,
        (_, O_TREE) => 0x2e5a22ff,
        (T_WATER, _) => 0x2a5582ff,
        (T_SAND, _) => 0xcdb882ff,
        (T_GRASS, _) => 0x5b8a38ff,
        (T_DIRT, _) => 0x86684aff,
        (T_COBBLE, _) => 0x9a9488ff,
        (T_FLOOR, _) => 0x8f6a42ff,
        (T_MOUNT, _) => 0x6e675cff,
        (T_BRIDGE, _) => 0x7a5432ff,
        (T_CAVE, _) => 0x40382fff,
        _ => 0x000000ff,
    };
    let s = m.shade[i] as f32 / 255.0 * 0.15 + 0.92;
    [
        (((c >> 24) & 255) as f32 * s).min(255.0) as u8,
        (((c >> 16) & 255) as f32 * s).min(255.0) as u8,
        (((c >> 8) & 255) as f32 * s).min(255.0) as u8,
        255,
    ]
}
