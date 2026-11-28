//! Isometric renderer. Produces a compact command buffer that the JS host
//! replays onto a 2D canvas (no wasm-bindgen needed).
use crate::bake::{self, Anim, Look, Weapon};
use crate::data::*;
use crate::game::*;
use crate::tex::*;
use crate::world::*;
use std::collections::{HashMap, VecDeque};

mod south;
mod terrace;
mod walls;
use south::southern;

/// The texture of a wall in this style.
fn wall_tex(style: u8) -> u32 {
    match style {
        2 => TEX_WOODWALL,
        3 => TEX_PLASTER,
        WALL_CASTLE | WALL_TOWER | WALL_ELVEN | WALL_DWARF | WALL_DROWNED | WALL_DROWNED_LOW => {
            TEX_ASHLAR
        }
        WALL_FORT | WALL_BASTION | WALL_PALACE | WALL_TEMPLE => TEX_SANDSTONE,
        WALL_HAVELI => TEX_LIMEWASH,
        _ => TEX_BRICK,
    }
}

/// A baked atlas the host has (or is about to get).
pub struct SpriteInfo {
    pub id: u32,
    pub last: f32,
    pub fw: usize,
    pub fh: usize,
    pub ax: f32,
    pub ay: f32,
}

/// Baked character atlases: baked on first sight, handed to the host once,
/// dropped after a while off screen.
pub struct SpriteCache {
    pub ids: HashMap<u64, SpriteInfo>,
    pub pending: VecDeque<(u32, bake::Atlas)>,
    pub current: Option<(u32, bake::Atlas)>,
    pub evicted: VecDeque<u32>,
    pub next: u32,
    /// frames baked per game frame, shared by all jobs in progress
    pub budget: usize,
    /// and the milliseconds a game frame may spend baking them
    pub bake_ms: f64,
    pub jobs: Vec<(u64, Look, bake::Atlas, usize)>,
    /// the shared atlas of baked props (furniture, trees, mushrooms...)
    pub props: Option<u32>,
}
impl Default for SpriteCache {
    fn default() -> Self {
        SpriteCache {
            ids: HashMap::new(),
            pending: VecDeque::new(),
            current: None,
            evicted: VecDeque::new(),
            next: 1,
            budget: 24,
            bake_ms: 5.0,
            jobs: vec![],
            props: None,
        }
    }
}

/// The ground (terrain, borders, rock faces) is the same every frame, but
/// zoomed out it is thousands of textured polygons, which the host's canvas
/// fills slowly. So it is drawn once into square chunks of art pixels that the
/// host keeps as canvases, and each frame only puts those down. Water is left
/// as holes in the chunks and drawn live underneath, so it still flows.
pub struct GroundCache {
    pub enabled: bool,
    /// (zoom, pix) the chunks were drawn at; any change redraws them all
    key: (u32, u32),
    /// chunk (column, row) -> (host id, frame last on screen)
    chunks: HashMap<(i32, i32), (u32, u32)>,
    frame: u32,
    /// frame the zoom last changed on
    zoomed: u32,
    /// tiles whose ground changed: their chunks are drawn again
    stale: Vec<(i32, i32)>,
}
impl GroundCache {
    /// Mark the chunks over these tiles to be drawn again.
    pub fn stale_tiles(&mut self, tiles: &[(i32, i32)]) {
        self.stale.extend_from_slice(tiles);
    }
}
impl Default for GroundCache {
    fn default() -> Self {
        GroundCache {
            enabled: true,
            key: (0, 0),
            chunks: HashMap::new(),
            frame: 0,
            zoomed: 0,
            stale: vec![],
        }
    }
}
/// side of a ground chunk, in art pixels
const CHUNK: i32 = 256;
/// chunks a frame may draw ahead of need, just outside the screen
const CHUNK_AHEAD: usize = 1;
/// chunks a frame may draw while it still draws the ground the slow way
const CHUNK_FILL: usize = 1;
/// frames the zoom must hold still before chunks are drawn at it: while a
/// pinch is going on they'd be thrown away every frame
const CHUNK_SETTLE: u32 = 10;
/// Host canvas ids for chunks, unique even across a new game.
fn chunk_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
const BG: u32 = 0x1d3a57ff;
/// the Shadow's colour over the land: a bruised violet-black
const GLOOM: u32 = 0x160b20ff;

fn weapon_of(id: Option<&str>) -> Weapon {
    match id {
        Some("dagger") | Some("katana") | Some("broadsword") | Some("longsword") => Weapon::Sword,
        Some("hatchet") => Weapon::Axe,
        Some("mace") | Some("warhammer") => Weapon::Mace,
        Some("pickaxe") => Weapon::Pick,
        _ => Weapon::None,
    }
}

#[derive(Default)]
pub struct Canvas {
    pub cmd: Vec<u32>,
    pub strs: Vec<u8>,
    /// width of ink borders in CSS px (one "art pixel")
    pub ink: f32,
    /// see-through things are drawn without ink lines, or they leave a wireframe
    pub mute_ink: bool,
}

impl Canvas {
    pub fn clear(&mut self) {
        self.cmd.clear();
        self.strs.clear();
    }
    #[inline]
    fn f(&mut self, v: f32) {
        self.cmd.push(v.to_bits());
    }
    pub fn fill(&mut self, c: u32) {
        self.cmd.push(1);
        self.cmd.push(c);
    }
    pub fn stroke(&mut self, c: u32, w: f32) {
        self.cmd.push(2);
        self.cmd.push(c);
        self.f(w);
    }
    pub fn poly(&mut self, p: &[(f32, f32)]) {
        self.cmd.push(3);
        self.cmd.push(p.len() as u32);
        for &(x, y) in p {
            self.f(x);
            self.f(y);
        }
    }
    pub fn poly_s(&mut self, p: &[(f32, f32)]) {
        self.cmd.push(4);
        self.cmd.push(p.len() as u32);
        for &(x, y) in p {
            self.f(x);
            self.f(y);
        }
    }
    pub fn ell(&mut self, x: f32, y: f32, rx: f32, ry: f32) {
        self.cmd.push(5);
        self.f(x);
        self.f(y);
        self.f(rx.max(0.1));
        self.f(ry.max(0.1));
    }
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.cmd.push(6);
        self.f(x);
        self.f(y);
        self.f(w);
        self.f(h);
    }
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.cmd.push(7);
        self.f(x1);
        self.f(y1);
        self.f(x2);
        self.f(y2);
    }
    /// align: 0 centre, 1 left
    pub fn text(&mut self, x: f32, y: f32, size: f32, align: u32, s: &str, outline: u32) {
        // the world's pixel font has printable ASCII only
        let s = crate::pack::ascii_fold(s);
        let off = self.strs.len() as u32;
        self.strs.extend_from_slice(s.as_bytes());
        self.cmd.push(8);
        self.f(x);
        self.f(y);
        self.f(size);
        self.cmd.push(align);
        self.cmd.push(off);
        self.cmd.push(s.len() as u32);
        self.cmd.push(outline);
    }
    /// a line with a dark ink border, for limbs and weapons
    pub fn ink_line(&mut self, a: (f32, f32), b: (f32, f32), col: u32, w: f32) {
        self.stroke(INK, w + 2.0 * self.ink.max(1.0));
        self.line(a.0, a.1, b.0, b.1);
        self.stroke(col, w);
        self.line(a.0, a.1, b.0, b.1);
    }
    pub fn alpha(&mut self, a: f32) {
        self.cmd.push(9);
        self.f(a);
    }
    pub fn dark(&mut self, x: f32, y: f32, r0: f32, r1: f32, a: f32, c: u32) {
        self.cmd.push(10);
        self.f(x);
        self.f(y);
        self.f(r0);
        self.f(r1);
        self.f(a);
        self.cmd.push(c);
    }
    pub fn ell_s(&mut self, x: f32, y: f32, rx: f32, ry: f32) {
        self.cmd.push(11);
        self.f(x);
        self.f(y);
        self.f(rx.max(0.1));
        self.f(ry.max(0.1));
    }
    /// While width > 0, every fill (poly/ellipse/rect) also gets an ink outline.
    pub fn outline(&mut self, c: u32, w: f32) {
        self.cmd.push(13);
        self.cmd.push(if self.mute_ink { 0 } else { c });
        self.f(if self.mute_ink { 0.0 } else { w });
    }
    /// a frame from a baked sprite atlas, anchored at the figure's feet
    #[allow(clippy::too_many_arguments)]
    pub fn sprite(
        &mut self,
        id: u32,
        frame: usize,
        fw: usize,
        fh: usize,
        ax: f32,
        ay: f32,
        x: f32,
        y: f32,
        scale: f32,
        flags: u32,
    ) {
        self.cmd.push(15);
        self.cmd.push(id);
        self.cmd.push(frame as u32);
        self.cmd.push(fw as u32);
        self.cmd.push(fh as u32);
        self.f(ax);
        self.f(ay);
        self.f(x);
        self.f(y);
        self.f(scale);
        self.cmd.push(flags);
    }
    /// fill with texture `tex`, mapped by the affine matrix (a b c d e f)
    #[allow(clippy::too_many_arguments)]
    pub fn pattern(&mut self, tex: u32, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        self.cmd.push(14);
        self.cmd.push(tex);
        for v in [a, b, c, d, e, f] {
            self.f(v);
        }
    }
    /// start drawing into the host's chunk canvas `id` (w x h art pixels);
    /// coordinates are CSS pixels from its top-left corner
    pub fn chunk_begin(&mut self, id: u32, w: u32, h: u32) {
        self.cmd.extend_from_slice(&[16, id, w, h]);
    }
    pub fn chunk_end(&mut self) {
        self.cmd.push(17);
    }
    /// put chunk `id` down with its top-left at x,y
    pub fn chunk_draw(&mut self, id: u32, x: f32, y: f32) {
        self.cmd.push(18);
        self.cmd.push(id);
        self.f(x);
        self.f(y);
    }
    pub fn chunk_free(&mut self, id: u32) {
        self.cmd.push(19);
        self.cmd.push(id);
    }
    /// while on, fills cut holes instead of painting
    pub fn erase(&mut self, on: bool) {
        self.cmd.push(20);
        self.cmd.push(on as u32);
    }
    pub fn glow(&mut self, x: f32, y: f32, r: f32, c: u32) {
        self.cmd.push(12);
        self.f(x);
        self.f(y);
        self.f(r);
        self.cmd.push(c);
    }
}

pub fn shade(c: u32, f: f32) -> u32 {
    let r = (((c >> 24) & 255) as f32 * f).clamp(0.0, 255.0) as u32;
    let g = (((c >> 16) & 255) as f32 * f).clamp(0.0, 255.0) as u32;
    let b = (((c >> 8) & 255) as f32 * f).clamp(0.0, 255.0) as u32;
    (r << 24) | (g << 16) | (b << 8) | (c & 255)
}
fn fi32(i: i32) -> f32 {
    i as f32
}
pub fn mix(a: u32, b: u32, k: f32) -> u32 {
    let ch = |c: u32, sh: u32| ((c >> sh) & 255) as f32;
    let m = |sh: u32| ((ch(a, sh) * (1.0 - k) + ch(b, sh) * k).clamp(0.0, 255.0) as u32) << sh;
    m(24) | m(16) | m(8) | (a & 255)
}
pub fn h2(x: i32, y: i32, k: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
        ^ (y as u32).wrapping_mul(0x1656_67b1)
        ^ (k as u32).wrapping_mul(0x9e37_79b9);
    h = (h ^ (h >> 15)).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    (h & 0xffff) as f32 / 65536.0
}
pub fn with_a(c: u32, a: f32) -> u32 {
    (c & 0xffffff00) | ((a.clamp(0.0, 1.0) * 255.0) as u32)
}

impl Game {
    /// The ground's texture on a tile: the tropics have their own.
    fn ground_tex(&self, x: i32, y: i32, t: u8) -> u32 {
        if self.map.tropic(x, y) {
            match t {
                T_GRASS => return TEX_JUNGLE,
                T_DIRT => return TEX_LATERITE,
                T_COBBLE if self.map.in_raj(x, y) => return TEX_REDFLAG,
                _ => {}
            }
        }
        tex_of(t)
    }
    /// which ground spills over which at borders: jungle over plain grass
    fn prio_at(&self, x: i32, y: i32) -> i32 {
        let t = self.map.tile(x, y);
        if t == T_GRASS && self.map.tropic(x, y) {
            5
        } else {
            prio(t)
        }
    }
}

fn tex_of(t: u8) -> u32 {
    match t {
        T_WATER => TEX_WATER,
        T_SAND => TEX_SAND,
        T_DIRT => TEX_DIRT,
        T_COBBLE => TEX_FLAG,
        T_FLOOR => TEX_FLOOR,
        T_MOUNT => TEX_ROCK,
        T_BRIDGE => TEX_BRIDGE,
        T_CAVE => TEX_CAVE,
        _ => TEX_GRASS,
    }
}
/// which terrain spills over which at borders
fn prio(t: u8) -> i32 {
    match t {
        T_WATER => 0,
        T_CAVE => 1,
        T_SAND => 2,
        T_DIRT => 3,
        T_GRASS => 4,
        _ => 10,
    }
}

#[allow(dead_code)]
fn tile_color(t: u8) -> u32 {
    match t {
        T_WATER => 0x101a3cff,
        T_SAND => 0x7a6a4cff,
        T_GRASS => 0x434826ff,
        T_DIRT => 0x58402eff,
        T_COBBLE => 0x5a5856ff,
        T_FLOOR => 0x583a22ff,
        T_MOUNT => 0x5e5650ff,
        T_BRIDGE => 0x4a321eff,
        T_CAVE => 0x221a16ff,
        _ => 0x000000ff,
    }
}

const INK: u32 = 0x120c08ff;
/// screen pixels per ground level at zoom 1
pub const LH: f32 = 20.0;
/// Heights drawn by hand, scaled to the baked sprites' steeper camera
/// (0.8 / ASPECT per metre instead of the plain 2:1 view's 0.866).
pub const ZS: f32 = 0.77;
/// a storey of a house, in pixels at zoom 1 (a house wall's height)
pub const STOREY_PX: f32 = 38.0 * ZS;
/// a storey, in ground levels (`LH`)
pub const STOREY: f32 = STOREY_PX / LH;

/// Small helper for drawing in "model units" around a screen anchor.
struct P {
    x: f32,
    y: f32,
    s: f32,
    f: f32,
}
impl P {
    #[inline]
    fn at(&self, dx: f32, dy: f32) -> (f32, f32) {
        (self.x + dx * self.f * self.s, self.y + dy * self.s)
    }
}

struct Hum {
    skin: u32,
    body: u32,
    legs: u32,
    robe: bool,
    hair: Option<u32>,
    long_hair: bool,
    helm: Option<u32>,
    hat: Option<u32>,
    weapon: u8,
    shield: Option<u32>,
    heads: u8,
    skull: bool,
    eyes: Option<u32>,
}
const HUM: Hum = Hum {
    skin: 0xe8b890ff,
    body: 0xb09070ff,
    legs: 0x4a3a2aff,
    robe: false,
    hair: None,
    long_hair: false,
    helm: None,
    hat: None,
    weapon: 0,
    shield: None,
    heads: 1,
    skull: false,
    eyes: None,
};

impl Game {
    pub fn render(&mut self) {
        self.prebake();
        let mut c = std::mem::take(&mut self.cmds);
        c.clear();
        c.ink = self.pix.max(1.0);
        // the view moves in whole art pixels, so the ground chunks fit together
        // exactly; the camera itself keeps its smooth position
        let (camx, camy) = (self.camx, self.camy);
        let a = self.zoom / self.pix;
        let (hw, hh) = (self.w / (2.0 * self.pix), self.h / (2.0 * self.pix));
        self.camx = ((self.camx * a - hw).round() + hw) / a;
        self.camy = ((self.camy * a - hh).round() + hh) / a;
        let chunks = self.ground_chunks(&mut c);
        self.draw(&mut c, chunks.as_deref());
        self.camx = camx;
        self.camy = camy;
        self.cmds = c;
    }

    /// Bring the ground chunks on screen up to date, drawing any that are
    /// missing, and say where each goes. None when the cache is off.
    fn ground_chunks(&mut self, c: &mut Canvas) -> Option<Vec<(u32, f32, f32)>> {
        self.ground.frame = self.ground.frame.wrapping_add(1);
        let frame = self.ground.frame;
        let key = (self.zoom.to_bits(), self.pix.to_bits());
        if !self.ground.enabled || key != self.ground.key {
            for (_, (id, _)) in self.ground.chunks.drain() {
                c.chunk_free(id);
            }
            self.ground.key = key;
            self.ground.zoomed = frame;
        }
        let (pix, a) = (self.pix, self.zoom / self.pix);
        // chunks under changed ground (the Shadow moving) are drawn again;
        // a tile's art reaches a diamond's half-width aside and up to four
        // levels of raised ground above
        if !self.ground.stale.is_empty() {
            let k = CHUNK as f32;
            let mut keys = std::collections::HashSet::new();
            for (x, y) in std::mem::take(&mut self.ground.stale) {
                let (ix, iy) = (((x - y) * 32) as f32 * a, ((x + y) * 16) as f32 * a);
                let (rx, up) = (36.0 * a, (4.0 * LH + 20.0) * a);
                for i in ((ix - rx) / k).floor() as i32..=((ix + rx) / k).floor() as i32 {
                    for j in ((iy - up) / k).floor() as i32..=((iy + 20.0 * a) / k).floor() as i32 {
                        keys.insert((i, j));
                    }
                }
            }
            for ij in keys {
                if let Some((id, _)) = self.ground.chunks.remove(&ij) {
                    c.chunk_free(id);
                }
            }
        }
        if !self.ground.enabled || frame.wrapping_sub(self.ground.zoomed) < CHUNK_SETTLE {
            return None;
        }
        // art-pixel position of the screen's top-left corner, a whole number
        let ox = (self.camx * a - self.w / (2.0 * pix)).round();
        let oy = (self.camy * a - self.h / (2.0 * pix)).round();
        let k = CHUNK as f32;
        let (i0, i1) = (
            (ox / k).floor() as i32,
            ((ox + self.w / pix) / k).floor() as i32,
        );
        let (j0, j1) = (
            (oy / k).floor() as i32,
            ((oy + self.h / pix) / k).floor() as i32,
        );
        // Too many missing (just zoomed, or a long way at once)? Draw a few
        // and the ground the slow way this frame, rather than stall.
        let missing = (j0..=j1)
            .flat_map(|j| (i0..=i1).map(move |i| (i, j)))
            .filter(|ij| !self.ground.chunks.contains_key(ij))
            .collect::<Vec<_>>();
        if missing.len() > CHUNK_FILL {
            for &(i, j) in missing.iter().take(CHUNK_FILL) {
                self.build_chunk(c, i, j);
            }
            for j in j0..=j1 {
                for i in i0..=i1 {
                    if let Some(e) = self.ground.chunks.get_mut(&(i, j)) {
                        e.1 = frame;
                    }
                }
            }
            return None;
        }
        let mut out = vec![];
        for j in j0..=j1 {
            for i in i0..=i1 {
                let id = match self.ground.chunks.get_mut(&(i, j)) {
                    Some(e) => {
                        e.1 = frame;
                        e.0
                    }
                    None => self.build_chunk(c, i, j),
                };
                out.push((id, (i as f32 * k - ox) * pix, (j as f32 * k - oy) * pix));
            }
        }
        // the ring just off screen, a few a frame, so walking rarely waits
        let mut ahead = 0;
        'ring: for j in j0 - 1..=j1 + 1 {
            for i in i0 - 1..=i1 + 1 {
                if ahead >= CHUNK_AHEAD {
                    break 'ring;
                }
                if let Some(e) = self.ground.chunks.get_mut(&(i, j)) {
                    e.1 = frame;
                } else {
                    self.build_chunk(c, i, j);
                    ahead += 1;
                }
            }
        }
        // forget chunks well away from the screen
        let mut gone = vec![];
        self.ground.chunks.retain(|&(i, j), &mut (id, last)| {
            let keep = frame.wrapping_sub(last) < 90
                || (i >= i0 - 2 && i <= i1 + 2 && j >= j0 - 2 && j <= j1 + 2);
            if !keep {
                gone.push(id);
            }
            keep
        });
        for id in gone {
            c.chunk_free(id);
        }
        Some(out)
    }

    /// Draw ground chunk (i, j) into a new host canvas.
    fn build_chunk(&mut self, c: &mut Canvas, i: i32, j: i32) -> u32 {
        let id = chunk_id();
        self.ground.chunks.insert((i, j), (id, self.ground.frame));
        let a = self.zoom / self.pix;
        let (gx, gy) = ((i * CHUNK) as f32, (j * CHUNK) as f32);
        let saved = (self.camx, self.camy, self.w, self.h);
        // a camera that puts the chunk's corner at the origin
        self.camx = gx / a;
        self.camy = gy / a;
        self.w = 0.0;
        self.h = 0.0;
        c.chunk_begin(id, CHUNK as u32, CHUNK as u32);
        let side = CHUNK as f32 * self.pix;
        c.fill(BG);
        c.rect(0.0, 0.0, side, side);
        let k = CHUNK as f32 / a;
        let (d0, d1) = (
            (self.camx / 32.0).floor() as i32 - 2,
            ((self.camx + k) / 32.0).ceil() as i32 + 2,
        );
        // raised ground reaches up from rows further down; rock faces hang
        // down from rows above
        let (s0, s1) = (
            (self.camy / 16.0).floor() as i32 - 5,
            ((self.camy + k) / 16.0).ceil() as i32 + 6,
        );
        self.ground_pass(c, s0, s1, d0, d1, true);
        c.chunk_end();
        (self.camx, self.camy, self.w, self.h) = saved;
        id
    }

    /// A woven rug laid in the middle of a building's ground floor.
    fn rug(&self, c: &mut Canvas, b: &crate::world::Building, z: f32) {
        let (ix0, iy0, ix1, iy1) = (b.x + 1, b.y + 1, b.x + b.w - 2, b.y + b.h - 2);
        if ix1 - ix0 < 2 || iy1 - iy0 < 2 || b.style == 4 {
            return;
        }
        // no rug across a wall that splits the room
        if (iy0..=iy1).any(|y| (ix0..=ix1).any(|x| self.map.obj(x, y) == O_WALL)) {
            return;
        }
        let hv = h2(b.x, b.y, 81);
        let (field, border, motif) = match b.style {
            WALL_CASTLE | WALL_TOWER => (0x6a1a1aff, 0x2a1a30ff, 0xc8a040ff),
            WALL_ELVEN => (0x2a4a6aff, 0xc8d0d8ff, 0xe8e8f0ff),
            // the dwarves': madder red bordered in soot black, knotwork in gold
            WALL_DWARF => (0x6a2418ff, 0x1e1c1cff, 0xc89a3aff),
            WALL_BURROW => (0x8a5a2aff, 0x3a5a2aff, 0xe8c060ff),
            // the south's cotton dhurries: indigo, madder and saffron
            WALL_HAVELI | WALL_PALACE | WALL_TEMPLE => [
                (0x2a3a7aff, 0xa83a1aff, 0xe8b040ff),
                (0xa83a1aff, 0x2a3a6aff, 0xe8d8b0ff),
                (0xd87a1aff, 0x6a1a1aff, 0x2a3a7aff),
            ][(hv * 3.0) as usize % 3],
            _ => [
                (0x7a2a20ff, 0x2a2a3aff, 0xd8b060ff),
                (0x2a4a3aff, 0x6a3020ff, 0xd8c890ff),
                (0x5a3a6aff, 0x2a2018ff, 0xc89050ff),
            ][(hv * 3.0) as usize % 3],
        };
        let lift = self.map.level(ix0, iy0) as f32 * LH * z;
        let (xa, xb, ya, yb) = (
            ix0 as f32 + 0.15,
            ix1 as f32 - 0.15,
            iy0 as f32 + 0.15,
            iy1 as f32 - 0.15,
        );
        let sp = |x: f32, y: f32| {
            let (a, b) = self.to_screen(x, y);
            (a, b - lift)
        };
        let quad = |c: &mut Canvas, x0: f32, x1: f32, y0: f32, y1: f32| {
            c.poly(&[sp(x0, y0), sp(x1, y0), sp(x1, y1), sp(x0, y1)]);
        };
        let ink = self.pix.max(z);
        // a shadow, the border, the field
        c.outline(0, 0.0);
        c.fill(0x00000040);
        quad(c, xa + 0.05, xb + 0.08, ya + 0.05, yb + 0.08);
        c.fill(border);
        quad(c, xa, xb, ya, yb);
        c.fill(field);
        quad(c, xa + 0.18, xb - 0.18, ya + 0.18, yb - 0.18);
        // a thin bright line inside the border
        c.stroke(with_a(motif, 0.7), ink);
        let r = [
            sp(xa + 0.1, ya + 0.1),
            sp(xb - 0.1, ya + 0.1),
            sp(xb - 0.1, yb - 0.1),
            sp(xa + 0.1, yb - 0.1),
        ];
        for i in 0..4 {
            let (p, q) = (r[i], r[(i + 1) % 4]);
            c.line(p.0, p.1, q.0, q.1);
        }
        // the medallion
        let (mx, my) = ((xa + xb) / 2.0, (ya + yb) / 2.0);
        let (rx, ry) = (((xb - xa) * 0.28).min(1.4), ((yb - ya) * 0.28).min(1.4));
        c.fill(motif);
        c.poly(&[
            sp(mx - rx, my),
            sp(mx, my - ry),
            sp(mx + rx, my),
            sp(mx, my + ry),
        ]);
        c.fill(border);
        c.poly(&[
            sp(mx - rx * 0.6, my),
            sp(mx, my - ry * 0.6),
            sp(mx + rx * 0.6, my),
            sp(mx, my + ry * 0.6),
        ]);
        c.fill(field);
        c.poly(&[
            sp(mx - rx * 0.25, my),
            sp(mx, my - ry * 0.25),
            sp(mx + rx * 0.25, my),
            sp(mx, my + ry * 0.25),
        ]);
        // small diamonds marching down the border
        c.fill(motif);
        let n = ((xb - xa) * 2.0) as i32;
        for i in 0..n {
            let u = xa + 0.25 + (xb - xa - 0.5) * (i as f32 + 0.5) / n as f32;
            for v in [ya + 0.09, yb - 0.09] {
                let (cx, cy) = sp(u, v);
                c.poly(&[
                    (cx - 2.0 * z, cy),
                    (cx, cy - 1.0 * z),
                    (cx + 2.0 * z, cy),
                    (cx, cy + 1.0 * z),
                ]);
            }
        }
        // the fringe at both ends
        c.stroke(0xd8c8a0c0, ink);
        let m = ((yb - ya) * 5.0) as i32;
        for i in 0..=m {
            let v = ya + (yb - ya) * i as f32 / m as f32;
            for (u0, u1) in [(xa, xa - 0.1), (xb, xb + 0.1)] {
                let (p, q) = (sp(u0, v), sp(u1, v));
                c.line(p.0, p.1, q.0, q.1);
            }
        }
        c.outline(INK, ink);
    }

    /// Terrain, rock faces and ragged borders for rows s0..=s1, d0..=d1.
    /// With `holes`, water is cut out rather than drawn (it is drawn live).
    #[allow(clippy::too_many_arguments)]
    fn ground_pass(&self, c: &mut Canvas, s0: i32, s1: i32, d0: i32, d1: i32, holes: bool) {
        let z = self.zoom;
        let t = self.clock;
        let scr = |x: i32, y: i32| self.to_screen(x as f32, y as f32);
        // ---- flat ground; terraces are gathered and drawn on hex cells
        let mut erasing = false;
        let mut terraced = vec![];
        for s in s0..=s1 {
            for d in d0..=d1 {
                if (s + d) & 1 != 0 {
                    continue;
                }
                let (x, y) = ((s + d) / 2, (s - d) / 2);
                let (sx, sy) = scr(x, y);
                let tl = self.map.tile(x, y);
                if holes && tl == T_WATER {
                    if !erasing {
                        c.erase(true);
                        c.fill(0xffffffff);
                        erasing = true;
                    }
                    let cn = self.map.corners(x, y);
                    c.poly(&self.diamond(sx, sy, &cn));
                    if self.has_ledges(x, y, &cn) {
                        c.erase(false);
                        erasing = false;
                        self.ledges(c, x, y, sx, sy, &cn, z);
                    }
                    continue;
                }
                if erasing {
                    c.erase(false);
                    erasing = false;
                }
                if self.terraced(x, y) {
                    // its own ground under the cells, for any sliver they miss
                    let lv = self.map.level(x, y) as f32;
                    self.pat(c, self.ground_tex(x, y, tl), lv * LH * z);
                    c.poly(&self.diamond(sx, sy, &[lv; 4]));
                    terraced.push((x, y));
                    continue;
                }
                let sh = if inb(x, y) {
                    self.map.shade[idx(x, y)]
                } else {
                    ((x * 31 + y * 17) & 255) as u8
                };
                self.ground(c, x, y, sx, sy, tl, sh, z, t);
            }
        }
        if erasing {
            c.erase(false);
        }
        self.terraces(c, &terraced, holes);
        // ---- ragged terrain borders
        for s in s0..=s1 {
            for d in d0..=d1 {
                if (s + d) & 1 != 0 {
                    continue;
                }
                let (x, y) = ((s + d) / 2, (s - d) / 2);
                let tl = self.map.tile(x, y);
                let pr = self.prio_at(x, y);
                if !(1..10).contains(&pr) {
                    continue;
                }
                let (sx, sy) = scr(x, y);
                if self.map.is_ramp(x, y) {
                    continue;
                }
                let lvl = self.map.level(x, y);
                for dir in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                    let (nx, ny) = (x + dir.0, y + dir.1);
                    if self.prio_at(nx, ny) < pr
                        && self.map.level(nx, ny) == lvl
                        && !self.map.is_ramp(nx, ny)
                        && !self.terraced(x, y)
                        && !self.terraced(nx, ny)
                    {
                        self.fringe(c, x, y, sx, sy, tl, dir, z);
                    }
                }
            }
        }
        // ---- rugs on the ground floors of houses and halls
        for b in &self.map.buildings {
            let (bs0, bs1) = (b.x + b.y, b.x + b.y + b.w + b.h);
            let (bd0, bd1) = (b.x - b.y - b.h, b.x - b.y + b.w);
            if bs1 < s0 || bs0 > s1 || bd1 < d0 || bd0 > d1 {
                continue;
            }
            self.rug(c, b, z);
        }
        // ---- foam where the water meets the land, on the water's own edge: a
        // pale line and a fainter one a little way out
        for s in s0..=s1 {
            for d in d0..=d1 {
                if (s + d) & 1 != 0 {
                    continue;
                }
                let (x, y) = ((s + d) / 2, (s - d) / 2);
                if !inb(x, y) || self.map.tile(x, y) != T_WATER {
                    continue;
                }
                let (gx, gy) = self.to_screen(x as f32, y as f32);
                let dm = self.diamond(gx, gy, &self.map.corners(x, y));
                // corners: 0 top, 1 right, 2 bottom, 3 left; the edge toward each
                // land neighbour, and the way back in over the water
                let wc = self.map.corners(x, y);
                for (dx, dy, a, b, la, lb) in [
                    (1i32, 0i32, 1usize, 2usize, 0usize, 3usize),
                    (0, 1, 2, 3, 1, 0),
                    (-1, 0, 3, 0, 2, 1),
                    (0, -1, 0, 1, 3, 2),
                ] {
                    let (nx, ny) = (x + dx, y + dy);
                    let t = self.map.tile(nx, ny);
                    if t == T_WATER || t == T_BRIDGE || !inb(nx, ny) {
                        continue;
                    }
                    // a terraced bank has its foam on its hex edges
                    if self.terraced(nx, ny) {
                        continue;
                    }
                    // a bank at the water's level spills its ragged edge over
                    // the water, with its own foam (see `fringe`)
                    if self.map.level(nx, ny) == self.map.level(x, y)
                        && !self.map.is_ramp(nx, ny)
                        && !self.map.is_ramp(x, y)
                        && (1..10).contains(&self.prio_at(nx, ny))
                        && self.prio_at(nx, ny) > self.prio_at(x, y)
                    {
                        continue;
                    }
                    // where the bank stands higher, its edge hides the water's:
                    // the foam goes on whichever edge is seen
                    let lc = self.map.corners(nx, ny);
                    let (lx, ly) = self.to_screen(nx as f32, ny as f32);
                    let ld = self.diamond(lx, ly, &lc);
                    let p = if lc[la] > wc[a] { ld[la] } else { dm[a] };
                    let q = if lc[lb] > wc[b] { ld[lb] } else { dm[b] };
                    let (ox, oy) = (-(dx - dy) as f32, -(dx + dy) as f32 * 0.5);
                    let wob = (h2(x * 3 + dx, y * 5 + dy, 17) - 0.5) * 2.0 * z;
                    c.stroke(0xd8e4e070, 1.5 * self.pix.max(z));
                    c.line(p.0 + ox * z, p.1 + oy * z, q.0 + ox * z, q.1 + oy * z);
                    let (fx, fy) = (ox * 5.0 * z, oy * 5.0 * z + wob);
                    c.stroke(0xc8d8dc30, self.pix.max(z));
                    c.line(p.0 + fx, p.1 + fy, q.0 + fx, q.1 + fy);
                }
            }
        }
        // ---- the Shadow, laid over the land it holds (baked into the chunks,
        // which are drawn again where it moves)
        for s in s0..=s1 {
            for d in d0..=d1 {
                if (s + d) & 1 != 0 {
                    continue;
                }
                let (x, y) = ((s + d) / 2, (s - d) / 2);
                if !inb(x, y) {
                    continue;
                }
                let g = self.map.gloom[idx(x, y)];
                if g == 0 || (holes && self.map.tile(x, y) == T_WATER) {
                    continue;
                }
                let (sx, sy) = scr(x, y);
                let cn = self.map.corners(x, y);
                c.fill(with_a(GLOOM, g as f32 / GLOOM_MAX as f32 * 0.46));
                c.poly(&self.diamond(sx, sy, &cn));
            }
        }
    }

    /// How big an entity stands, for tapping and for what floats over it:
    /// a rider sits as high as a horse's head.
    pub fn ent_size(&self, e: &Ent) -> f32 {
        if e.k == EK::Player && self.pl.riding {
            1.35
        } else {
            e.mon().map(|m| m.size).unwrap_or(1.0)
        }
    }

    /// The baked look of an entity, if it is drawn as a baked figure.
    pub fn look_for(&self, i: usize) -> Option<Look> {
        self.look_for_ent(&self.ents[i])
    }
    pub fn look_for_ent(&self, e: &Ent) -> Option<Look> {
        let skin = 0xe0b090ff;
        match e.k {
            EK::Player => {
                let pl = &self.pl;
                if pl.ghost {
                    let mut l = Look::human(0xd8e4f0ff, 0xc8d4e8ff, 0xc8d4e8ff);
                    l.role = 0;
                    l.robe = true;
                    l.female = pl.female;
                    return Some(l);
                }
                let arm = pl.armor.as_ref().map(|a| a.def().id);
                let body = pl
                    .armor
                    .as_ref()
                    .map(|a| a.def().color)
                    .unwrap_or(0x8a6a4aff);
                let mut l = Look::human(skin, body, 0x3a3040ff);
                l.role = 0;
                l.robe = arm == Some("robe");
                l.metal_body = matches!(
                    arm,
                    Some("ringmail") | Some("chainmail") | Some("platemail")
                );
                l.skirt = l.metal_body;
                l.helm = if arm == Some("platemail") {
                    0xc8ced6ff
                } else {
                    0
                };
                l.hair = pl.hair;
                l.female = pl.female;
                l.long_hair = pl.female;
                l.weapon = if pl.war {
                    weapon_of(pl.weapon.as_ref().map(|w| w.def().id))
                } else {
                    Weapon::None
                };
                if let Action::Gather { skill, .. } = pl.action {
                    l.weapon = match skill {
                        MINING => Weapon::Pick,
                        LUMBER => Weapon::Axe,
                        _ => Weapon::Pole,
                    };
                }
                l.shield = pl.shield.as_ref().map(|s| s.def().color).unwrap_or(0);
                if pl.riding {
                    if let Some(h) = &pl.horse {
                        let (coat, mane) = HORSE_COATS[h.coat as usize % HORSE_COATS.len()];
                        l.mount = coat;
                        l.mane = mane;
                    }
                }
                Some(l)
            }
            EK::Npc(v) => {
                let vd = &VENDORS[v];
                // every townsperson dressed a little differently, from their look seed
                let r = |k: u32| (e.look.wrapping_mul(0x9e37_79b9).rotate_left(k * 5) >> 8) % 1000;
                const SKINS: [u32; 4] = [0xe0b090ff, 0xd09a74ff, 0xb07850ff, 0xe8c4a8ff];
                const CLOTH: [u32; 8] = [
                    0x5a4a38ff, 0x3e4a3aff, 0x6a3a2aff, 0x4a4652ff, 0x7a6a4aff, 0x3a3a44ff,
                    0x5a2a30ff, 0x6a5e50ff,
                ];
                const LEGS: [u32; 4] = [0x3a3028ff, 0x2a2a30ff, 0x4a3a28ff, 0x3a3a2aff];
                let mut l = Look::human(
                    SKINS[(r(1) % 4) as usize],
                    vd.robe,
                    LEGS[(r(2) % 4) as usize],
                );
                l.role = 1;
                l.hair = HAIR[(r(3) % 5) as usize];
                l.female = e.look % 2 == 0;
                l.long_hair = l.female && r(4) % 3 != 0;
                l.boots = [0x3a2a1aff, 0x2a1e14ff, 0x4a3a2aff][(r(5) % 3) as usize];
                l.bulk = (r(6) % 18) as u8;
                l.scale = 94 + (r(7) % 10) as u16;
                match v {
                    V_GUARD => {
                        l.metal_body = true;
                        l.skirt = true;
                        l.helm = 0xb0b8c0ff;
                        l.weapon = Weapon::Halberd;
                        l.shield = 0x3050a0ff;
                        l.legs = 0x5a5e66ff;
                        l.hair = 0;
                        l.female = false;
                        l.long_hair = false;
                        l.scale = 102;
                    }
                    V_MAGE => {
                        l.robe = true;
                        l.hat = vd.robe;
                        l.weapon = Weapon::Staff;
                        l.hair = 0xd0d0d0ff;
                        l.female = false;
                        l.long_hair = false;
                    }
                    V_HEALER => l.robe = true,
                    V_LIBRARIAN => {
                        // a scholar's dark robe and grey hair
                        l.robe = true;
                        l.legs = vd.robe;
                        l.hair = 0xb0aca4ff;
                        l.hunch = 8;
                    }
                    V_BARKEEP => {
                        l.bulk = 17;
                        l.legs = 0x3a3028ff;
                    }
                    V_LORD => {
                        // a long crimson robe, a gilt helm and a sword at hand
                        l.robe = true;
                        l.legs = vd.robe;
                        l.helm = 0xc8a040ff;
                        l.weapon = Weapon::Sword;
                        l.long_hair = false;
                        l.scale = 104;
                    }
                    V_SMITH => {
                        l.weapon = Weapon::Mace;
                        l.skin = 0xd8a070ff;
                        l.bulk = 18;
                    }
                    V_VILLAGER => {
                        // the small folk: short and stout, curly-haired,
                        // barefoot, in waistcoats and aprons of garden colours
                        const COATS: [u32; 6] = [
                            0x5a6a32ff, 0x7a5a2aff, 0x3a5a3aff, 0x8a6a3aff, 0x6a3a2aff, 0x4a5a6aff,
                        ];
                        const CURLS: [u32; 4] = [0x5a3a1aff, 0x7a5228ff, 0x3a2614ff, 0x9a6a30ff];
                        l.scale = 70 + (r(7) % 6) as u16;
                        l.bulk = 15 + (r(6) % 4) as u8;
                        l.body = COATS[(r(8) % 6) as usize];
                        l.legs = [0x6a5a3aff, 0x4a4a32ff, 0x5a4630ff][(r(2) % 3) as usize];
                        l.boots = l.skin;
                        l.hair = CURLS[(r(3) % 4) as usize];
                        l.long_hair = l.female && r(4) % 2 == 0;
                        if l.female && r(9) % 2 == 0 {
                            l.robe = true;
                            l.legs = l.body;
                        }
                    }
                    V_FOLK => {
                        l.body = CLOTH[(r(8) % 8) as usize];
                        // shoppers at the market carry baskets
                        l.basket = self.map.in_market(e.home.0, e.home.1) && r(10) % 3 != 0;
                        match r(9) % 6 {
                            // a long dress or a monk's habit
                            0 | 1 => {
                                l.robe = true;
                                l.legs = l.body;
                            }
                            // a hooded traveller with a walking stick
                            2 => {
                                l.hood = shade(l.body, 0.8);
                                l.weapon = Weapon::Staff;
                            }
                            // an old one: grey and bent
                            3 => {
                                l.hair = 0xb0aca4ff;
                                l.hunch = 18;
                                l.weapon = Weapon::Staff;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                // the southern cities' people: brown-skinned and black-haired,
                // the men in turbans, kurtas and white dhotis, the women in
                // saris with the end over the shoulder
                if self.map.in_raj(e.home.0, e.home.1) || v == V_RAJA {
                    const SKIN: [u32; 5] =
                        [0x8a5a3aff, 0x9a6a44ff, 0x7a4a2eff, 0xa87a52ff, 0x6a4028ff];
                    const BRIGHT: [u32; 8] = [
                        0xd8801aff, 0xa81a2aff, 0xb02a7aff, 0x1a8a8aff, 0x2a7a3aff, 0x2a3a8aff,
                        0xc8a030ff, 0xe8c0a0ff,
                    ];
                    const PALE: [u32; 5] =
                        [0xe8e0d0ff, 0xd8d0b8ff, 0xc8d8e0ff, 0xe0c8c0ff, 0xd8c8a0ff];
                    let pick = |k: u32, a: &[u32]| a[(r(k) % a.len() as u32) as usize];
                    l.skin = pick(1, &SKIN);
                    l.hair = 0x1a1410ff;
                    l.hood = 0;
                    l.hat = 0;
                    l.helm = 0;
                    l.furs = 0;
                    l.metal_body = false;
                    l.skirt = false;
                    l.boots = shade(l.skin, 0.8);
                    if v == V_GUARD {
                        // the sipahi: a red coat, a saffron turban, talwar and a brass shield
                        l.female = false;
                        l.body = 0xa83a1aff;
                        l.legs = 0xe0d8c8ff;
                        l.turban = 0xe8901aff;
                        l.sash = 0xe8d8a0ff;
                        l.weapon = Weapon::Sword;
                        l.shield = 0xc8a040ff;
                        l.boots = 0x4a2a18ff;
                    } else if v == V_RAJA {
                        l.female = false;
                        l.robe = true;
                        l.body = 0xc8a040ff;
                        l.legs = l.body;
                        l.turban = 0xf0e8d8ff;
                        l.sash = 0x9a1a2aff;
                        l.weapon = Weapon::Sword;
                        l.scale = 104;
                    } else if v == V_HEALER {
                        // the priest: saffron, a white thread, a bare head
                        l.female = false;
                        l.robe = true;
                        l.body = 0xe08a2aff;
                        l.legs = l.body;
                        l.hair = 0x2a2420ff;
                        l.long_hair = false;
                        l.sash = 0xf0e8d8ff;
                    } else if l.female {
                        l.robe = true;
                        l.body = pick(8, &BRIGHT);
                        l.legs = l.body;
                        l.sash = shade(pick(9, &BRIGHT), 1.0);
                        l.long_hair = true;
                        if r(10) % 3 == 0 {
                            // the sari's end drawn over the head
                            l.hood = l.sash;
                        }
                    } else {
                        l.robe = false;
                        l.body = pick(8, &PALE);
                        l.legs = 0xe8e0d0ff;
                        l.turban = pick(11, &BRIGHT);
                        if r(12) % 2 == 0 {
                            l.sash = pick(9, &BRIGHT);
                        }
                        if v == V_SMITH {
                            l.body = l.skin;
                            l.turban = 0x8a2a1aff;
                            l.sash = 0;
                        }
                    }
                    if v == V_MAGE {
                        l.robe = true;
                        l.body = 0x2a2a6aff;
                        l.legs = l.body;
                        l.turban = 0xe8e0d0ff;
                        l.weapon = Weapon::Staff;
                    }
                    if l.turban != 0 {
                        l.hair = 0;
                        l.long_hair = false;
                    }
                    if l.hood != 0 {
                        l.long_hair = false;
                    }
                }
                // the elves: tall and slender, long-haired, in grey, green and
                // silver; their wardens unhelmed, cloaked, sword at hand
                if self.map.in_elf(e.home.0, e.home.1) && v != V_VILLAGER {
                    const ROBES: [u32; 5] =
                        [0x5a6a5aff, 0x6a7a8aff, 0x3a5a4aff, 0x8a8a9aff, 0x4a5a6aff];
                    const HAIRS: [u32; 4] = [0xe8e0c8ff, 0xd8c080ff, 0x1a1a22ff, 0xc8ccd8ff];
                    l.scale = 106 + (r(7) % 6) as u16;
                    l.bulk = (r(6) % 4) as u8;
                    l.hair = HAIRS[(r(3) % 4) as usize];
                    l.long_hair = true;
                    l.ears = true;
                    l.skin = 0xf0d8c0ff;
                    l.hunch = 0;
                    l.helm = 0;
                    l.metal_body = false;
                    l.skirt = false;
                    l.hat = 0;
                    l.hood = 0;
                    l.body = ROBES[(r(8) % 5) as usize];
                    l.boots = 0x4a4a3aff;
                    if v == V_GUARD {
                        l.weapon = Weapon::Sword;
                        l.shield = 0;
                        l.legs = 0x3a4a3aff;
                        l.body = 0x3a5a3aff;
                        l.hood = 0x2a4a2aff;
                    } else {
                        l.robe = true;
                        l.legs = l.body;
                        l.weapon = if v == V_MAGE {
                            Weapon::Staff
                        } else {
                            Weapon::None
                        };
                    }
                }
                // the dwarves of Karak Dhum: short and broad, ruddy and
                // bearded, in leather, wool and mail of earth colours
                if self.map.in_hold(e.home.0, e.home.1) || v == V_THANE {
                    dwarf_look(&mut l, v, &r, e.home.0 >= HOLD.0 + HOLD.2);
                }
                Some(l)
            }
            EK::Mon(m) => {
                let d = &MONS[m];
                let mut l = Look::human(d.color, d.color, d.color);
                match d.id {
                    "skeleton" => {
                        l.skeleton = true;
                        l.weapon = Weapon::Sword;
                        l.shield = 0x6a5a40ff;
                        l.eyes = 0x200808ff;
                        l.boots = d.color;
                    }
                    "zombie" => {
                        l.body = 0x4a3a2eff;
                        l.legs = 0x3a3a30ff;
                        l.hair = 0x2a2a20ff;
                        l.zombie = true;
                        l.eyes = 0xc0e060ff;
                    }
                    "orc" | "orclord" => {
                        l.body = 0x6a4a2aff;
                        l.legs = 0x3a2a1aff;
                        l.weapon = Weapon::Axe;
                        l.eyes = 0xff3020ff;
                        l.bulk = 20;
                        l.scale = 105;
                        l.tusks = true;
                        l.hair = 0x141210ff;
                        l.long_hair = true;
                        l.furs = 0x4a3a2aff;
                        if d.id == "orclord" {
                            l.body = 0x8a8f96ff;
                            l.metal_body = true;
                            l.skirt = true;
                            l.helm = 0x6a6058ff;
                            l.shield = 0x5a3a20ff;
                            l.scale = 112;
                            l.bulk = 25;
                        }
                    }
                    "troll" => {
                        l.body = 0x5a4a30ff;
                        l.weapon = Weapon::Club;
                        l.eyes = 0xffe040ff;
                        l.tusks = true;
                        l.furs = 0x5a5040ff;
                        l.bulk = 45;
                        l.hunch = 30;
                        l.scale = 125;
                        l.skirt = true;
                    }
                    "ettin" => {
                        l.body = 0x6a5038ff;
                        l.weapon = Weapon::Club;
                        l.heads = 2;
                        l.bulk = 50;
                        l.hunch = 15;
                        l.scale = 140;
                        l.skirt = true;
                        l.eyes = 0x200808ff;
                    }
                    "lich" => {
                        l.skin = 0xe8e2cfff;
                        l.skeleton = true;
                        l.robe = true;
                        l.body = 0x5a3c80ff;
                        l.hat = 0x3a2a5aff;
                        l.weapon = Weapon::Staff;
                        l.eyes = 0x60ff60ff;
                    }
                    "rabbit" => return Some(Look::creature(bake::Body::Rabbit, 0x8a6a48ff, 140)),
                    "rat" => return Some(Look::creature(bake::Body::Rat, 0x5e4e42ff, 130)),
                    "wolf" => return Some(Look::creature(bake::Body::Wolf, 0x8a8a86ff, 100)),
                    "tiger" => {
                        let mut t = Look::creature(bake::Body::Wolf, d.color, 128);
                        t.stripes = 0x1a1008ff;
                        return Some(t);
                    }
                    "pigeon" | "dog" | "cat" | "cow" | "pig" | "monkey" => {
                        return street_look(d.id, e.look)
                    }
                    "cthulhu" => return Some(Look::creature(bake::Body::Cthulhu, d.color, 220)),
                    "crab" => return Some(Look::creature(bake::Body::Crab, d.color, 140)),
                    "deeppriest" => {
                        // a deep one in a weed-dark robe and hood, with a staff
                        l.skin = 0x5e7e6cff;
                        l.eyes = 0xd8e8a0ff;
                        l.ears = true;
                        l.hunch = 22;
                        l.robe = true;
                        l.body = 0x22302aff;
                        l.legs = 0x22302aff;
                        l.hood = 0x1a2620ff;
                        l.weapon = Weapon::Staff;
                        l.sash = 0x6a8a4aff;
                    }
                    "deepone" => deep_one(&mut l),
                    // Saltgrave's folk pass for fishermen, until the town
                    // turns and their faces change
                    "saltfolk" if self.salt.turned > 0.0 => deep_one(&mut l),
                    "saltfolk" => {
                        let r = e.look;
                        const COATS: [u32; 5] =
                            [0x5a5a3aff, 0x3a4a5aff, 0x5a5650ff, 0x4a3a2aff, 0x40483eff];
                        l.skin = 0xa8b0a0ff;
                        l.eyes = 0xe8ecd0ff;
                        l.body = COATS[r as usize % 5];
                        l.legs = 0x2e2a26ff;
                        l.boots = 0x24221eff;
                        l.hunch = 10 + (r >> 4) as u8 % 14;
                        l.bulk = (r >> 8) as u8 % 16;
                        l.female = r & 0x100 != 0;
                        l.skirt = l.female;
                        l.hair = if (r >> 10) % 3 == 0 { 0x8a8a80ff } else { 0 };
                        l.long_hair = l.female && l.hair != 0;
                        l.hat = if (r >> 12) % 3 == 0 { 0x6a6a3aff } else { 0 };
                        l.hood = if l.female && l.hat == 0 {
                            0x3a3a34ff
                        } else {
                            0
                        };
                    }
                    "mongbat" => return Some(Look::creature(bake::Body::Bat, 0x6a4e6aff, 100)),
                    "slime" => return Some(Look::creature(bake::Body::Slime, 0x5aa83aff, 100)),
                    "spider" => return Some(Look::creature(bake::Body::Spider, d.color, 135)),
                    "dragon" => return Some(Look::creature(bake::Body::Dragon, 0x9a2e24ff, 160)),
                    "horse" => {
                        let (coat, mane) = HORSE_COATS[e.coat() as usize % HORSE_COATS.len()];
                        let mut h = Look::creature(bake::Body::Horse, coat, 100);
                        h.mane = mane;
                        // the player's horse is saddled, the stable's wear halters
                        h.tack = match e.tame() {
                            1 => 2,
                            2 => 1,
                            _ => 0,
                        };
                        return Some(h);
                    }
                    _ => return None,
                }
                Some(l)
            }
        }
    }

    /// Bake atlases for figures on screen that have none yet (a few per frame).
    fn prebake(&mut self) {
        let mut want: Vec<(f32, Look)> = vec![];
        let onscreen = |g: &Game, x: f32, y: f32| {
            let (sx, sy) = g.to_screen(x, y);
            sx > -200.0 && sx < g.w + 200.0 && sy > -200.0 && sy < g.h + 300.0
        };
        for i in 0..self.ents.len() {
            let e = &self.ents[i];
            if e.dead || !onscreen(self, e.x, e.y) {
                continue;
            }
            if let Some(l) = self.look_for(i) {
                want.push((e.dist(self.ents[0].x, self.ents[0].y), l));
            }
        }
        for c in self.corpses.iter() {
            if let Some(l) = &c.look {
                if onscreen(self, c.x as f32, c.y as f32) {
                    let (dx, dy) = (c.x as f32 - self.ents[0].x, c.y as f32 - self.ents[0].y);
                    want.push(((dx * dx + dy * dy).sqrt(), l.clone()));
                }
            }
        }
        if self.sprites.props.is_none() {
            let atlas = bake::bake_props();
            let id = self.sprites.next;
            self.sprites.next += 1;
            self.sprites.props = Some(id);
            self.sprites.pending.push_back((id, atlas));
        }
        // the nearest first: they're the ones looked at
        want.sort_by(|a, b| a.0.total_cmp(&b.0));
        let now = self.clock;
        for (_, l) in want {
            let k = l.key();
            if let Some(info) = self.sprites.ids.get_mut(&k) {
                info.last = now;
                continue;
            }
            if self.sprites.jobs.iter().any(|j| j.0 == k) || self.sprites.jobs.len() >= 3 {
                continue;
            }
            let atlas = bake::new_atlas(&l);
            self.sprites.jobs.push((k, l, atlas, 0));
        }
        // bake a slice of frames per game frame so play never stalls: so many
        // frames, and no longer than so long (zoomed out, many new figures
        // come into view at once and a slow device would stutter for seconds)
        let mut left = self.sprites.budget;
        let t0 = crate::now_ms();
        while left > 0
            && !self.sprites.jobs.is_empty()
            && (left == self.sprites.budget || crate::now_ms() - t0 < self.sprites.bake_ms)
        {
            let job = &mut self.sprites.jobs[0];
            let cost = if job.1.scale > 130 {
                4
            } else if job.1.equine() {
                2
            } else {
                1
            };
            bake::bake_frame(&job.1, &mut job.2, job.3);
            job.3 += 1;
            left = left.saturating_sub(cost);
            if job.3 >= bake::total_frames(&job.1) {
                let (k, l, atlas, _) = self.sprites.jobs.remove(0);
                let id = self.sprites.next;
                self.sprites.next += 1;
                let (ax, ay) = l.anchor();
                self.sprites.ids.insert(
                    k,
                    SpriteInfo {
                        id,
                        last: now,
                        fw: atlas.fw,
                        fh: atlas.fh,
                        ax,
                        ay,
                    },
                );
                self.sprites.pending.push_back((id, atlas));
            }
        }
        let mut gone = vec![];
        self.sprites.ids.retain(|_, v| {
            if now - v.last > 45.0 {
                gone.push(v.id);
                false
            } else {
                true
            }
        });
        self.sprites.evicted.extend(gone);
    }

    /// Which animation an entity is showing right now, and how far into it (0..1).
    fn anim_of(&self, e: &Ent) -> (Anim, f32) {
        let tau = std::f32::consts::TAU;
        let cyc = |rate: f32| (self.clock * rate + e.id as f32 * 0.37).fract();
        if e.down_t > 0.0 {
            // fall (death frames forwards), lie still, then get up (the same frames backwards)
            let t = e.down_total - e.down_t;
            let (fall, rise) = (0.45, e.down_total * 0.4);
            let ph = if t < fall {
                t / fall
            } else if e.down_t > rise {
                0.99
            } else {
                e.down_t / rise
            };
            return (Anim::Die, ph);
        }
        if e.anim.kick > 0.0 {
            return (Anim::Kick, 1.0 - e.anim.kick);
        }
        let is_player = e.k == EK::Player;
        if is_player {
            if self.pl.blocking {
                return (Anim::Block, cyc(1.0));
            }
            if let Action::Cast { t, .. } = self.pl.action {
                return (Anim::Cast, 1.0 - (t / 1.2).min(1.0));
            }
            if matches!(self.pl.action, Action::Gather { .. }) && e.path.is_empty() {
                return (Anim::Work, (self.clock * 1.5).fract());
            }
        }
        if e.anim.atk > 0.0 {
            return (Anim::Attack, 1.0 - e.anim.atk);
        }
        if e.anim.ready > 0.0 {
            return (Anim::Ready, 1.0 - e.anim.ready);
        }
        if e.anim.hurt > 0.45 {
            return (Anim::Hit, if e.anim.hurt > 0.75 { 0.0 } else { 0.6 });
        }
        // a bird on the wing
        if e.anim.air > 0.0 {
            return (Anim::Run, cyc(3.2));
        }
        let fighting = match e.k {
            EK::Player => self.pl.war,
            EK::Mon(_) => e.combat.target != 0,
            EK::Npc(_) => false,
        };
        if !e.path.is_empty() {
            let ph = (e.anim.walk.rem_euclid(tau)) / tau;
            let near_foe = fighting
                && self
                    .find(e.combat.target)
                    .map(|i| self.ents[i].dist(e.x, e.y) < 2.6)
                    .unwrap_or(false);
            if near_foe {
                return (Anim::Advance, ph);
            }
            if e.running || (matches!(e.k, EK::Mon(_)) && e.combat.target != 0) {
                return (Anim::Run, ph);
            }
            return (Anim::Walk, ph);
        }
        if e.say.t > 0.0 && matches!(e.k, EK::Npc(_)) {
            return (Anim::Talk, cyc(1.2));
        }
        if fighting {
            return (Anim::Stance, cyc(0.75));
        }
        // the beasts of the streets spend long spells grazing, rooting,
        // sniffing, pecking or (a cat) sitting: (how often, how long, how fast)
        if let EK::Mon(m) = e.k {
            let spell = match MONS[m].id {
                "pigeon" => Some((0.2, 0.45, 1.5)),
                "cat" => Some((0.035, 0.6, 0.4)),
                "cow" => Some((0.04, 0.6, 0.6)),
                "pig" => Some((0.08, 0.5, 1.0)),
                "dog" => Some((0.07, 0.3, 1.2)),
                // a monkey with something to eat does nothing else
                "monkey" if e.stolen.is_some() => Some((1.0, 1.0, 0.9)),
                "monkey" => Some((0.09, 0.45, 0.9)),
                _ => None,
            };
            if let Some((rate, frac, speed)) = spell {
                let w = (self.clock * rate + e.id as f32 * 0.61).fract();
                return if w < frac {
                    (Anim::Look, cyc(speed))
                } else {
                    (Anim::Idle, cyc(0.6))
                };
            }
        }
        // now and then a figure glances left and right
        let w = (self.clock * 0.14 + e.id as f32 * 0.61).fract();
        if w < 0.16 {
            return (Anim::Look, w / 0.16);
        }
        (Anim::Idle, cyc(0.6))
    }

    /// Draw an entity as its baked figure; false if it has none (yet).
    fn draw_sprite(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, z: f32) -> bool {
        let Some(look) = self.look_for_ent(e) else {
            return false;
        };
        let Some(info) = self.sprites.ids.get(&look.key()) else {
            return false;
        };
        let (a, ph) = self.anim_of(e);
        let (frame, flip) = bake::frame_index(&look, a, ph, e.anim.vdir);
        let ghost = e.k == EK::Player && self.pl.ghost;
        let lift = if ghost {
            4.0 * z + (self.clock * 2.0).sin() * 2.0 * z
        } else {
            // a flying bird, eased up and down
            let a = e.anim.air;
            a * a * (3.0 - 2.0 * a) * crate::game::critters::FLIGHT * LH * z
        };
        c.sprite(
            info.id,
            frame,
            info.fw,
            info.fh,
            info.ax,
            info.ay,
            sx,
            sy - lift,
            z,
            flip as u32 | if ghost { 2 } else { 0 },
        );
        true
    }

    fn darkness(&self) -> f32 {
        let ph = (self.time / 480.0).fract();
        // 0..0.55 day, 0.55..0.65 dusk, 0.65..0.92 night, 0.92..1 dawn
        if ph < 0.55 {
            0.0
        } else if ph < 0.65 {
            (ph - 0.55) / 0.1 * 0.62
        } else if ph < 0.92 {
            0.62
        } else {
            (1.0 - (ph - 0.92) / 0.08) * 0.62
        }
    }
    pub fn is_night(&self) -> bool {
        self.darkness() > 0.3
    }
    /// Pagan's six four-hour periods of the day
    pub fn period(&self) -> &'static str {
        const P: [&str; 6] = [
            "Daytide",
            "Threemoons",
            "Lastebb",
            "Eventide",
            "Bloodwatch",
            "Firstebb",
        ];
        P[(((self.time / 480.0).fract() * 6.0) as usize).min(5)]
    }

    fn draw(&self, c: &mut Canvas, chunks: Option<&[(u32, f32, f32)]>) {
        let z = self.zoom;
        let t = self.clock;
        c.fill(BG);
        c.rect(0.0, 0.0, self.w, self.h);
        let cd = self.camx / 32.0;
        let cs = self.camy / 16.0;
        let dr = (self.w * 0.5 / (32.0 * z)).ceil() as i32 + 2;
        let sr = (self.h * 0.5 / (16.0 * z)).ceil() as i32 + 2;
        let (s0, s1) = (cs.floor() as i32 - sr, cs.ceil() as i32 + sr + 6);
        let (d0, d1) = (cd.floor() as i32 - dr, cd.ceil() as i32 + dr);
        let p = &self.ents[0];
        let (ps, pd) = (p.x + p.y, p.x - p.y);

        // ---- the ground: live water under the cached chunks, or all of it
        match chunks {
            Some(list) => {
                self.pat(c, TEX_WATER, 0.0);
                c.rect(0.0, 0.0, self.w, self.h);
                for &(id, x, y) in list {
                    c.chunk_draw(id, x, y);
                }
            }
            None => self.ground_pass(c, s0, s1, d0, d1, false),
        }
        if let Some((dx, dy)) = self.dest {
            let (sx, sy) = self.scr_h(dx as f32, dy as f32, self.map.surface(dx, dy));
            let k = 0.6 + (t * 6.0).sin() * 0.15;
            c.stroke(0xfff0a0cc, 1.5);
            c.poly_s(&[
                (sx, sy - 10.0 * z * k),
                (sx + 20.0 * z * k, sy),
                (sx, sy + 10.0 * z * k),
                (sx - 20.0 * z * k, sy),
            ]);
        }

        // ---- blood and gore on the ground
        for d in self.decals.iter() {
            let (dx, dy) = self.scr_h(
                d.x,
                d.y,
                self.map.surface(d.x.round() as i32, d.y.round() as i32),
            );
            if dx < -60.0 || dx > self.w + 60.0 || dy < -60.0 || dy > self.h + 60.0 {
                continue;
            }
            let k = if d.t < 0.0 {
                ((d.t + 1.5) / 1.5).clamp(0.15, 1.0)
            } else {
                1.0
            };
            let a = ((90.0 - d.t) / 25.0).clamp(0.0, 1.0) * 0.85;
            let (rx, ry) = (d.r * 32.0 * z * k, d.r * 16.0 * z * k);
            c.fill(with_a(d.color, a));
            c.ell(dx, dy, rx, ry);
            c.fill(with_a(shade(d.color, 0.6), a));
            c.ell(dx - rx * 0.2, dy + ry * 0.1, rx * 0.55, ry * 0.5);
            let j = (d.x * 13.0).sin();
            c.fill(with_a(d.color, a));
            c.ell(
                dx + rx * (1.1 + j * 0.2),
                dy - ry * 0.4,
                rx * 0.18,
                ry * 0.18,
            );
            c.ell(
                dx - rx * 1.05,
                dy + ry * (0.3 + j * 0.2),
                rx * 0.12,
                ry * 0.12,
            );
        }

        // ---- entity buckets
        let mut order: Vec<(i32, usize)> = self
            .ents
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.dead && self.ent_visible(e))
            .map(|(i, e)| ((e.x + e.y).round() as i32, i))
            .collect();
        order.sort_by_key(|o| o.0);
        let mut oi = 0;
        let mut lights: Vec<(f32, f32, f32, u32)> = vec![];

        // the building and storey the player is up in, if any: there the view
        // is cut away above their floor
        let upstairs = self.floor_view().filter(|&(_, f)| f > 0);
        // ---- pass 2: objects + entities in depth order
        for s in s0..=s1 {
            for d in d0..=d1 {
                if (s + d) & 1 != 0 {
                    continue;
                }
                let (x, y) = ((s + d) / 2, (s - d) / 2);
                if !inb(x, y) {
                    continue;
                }
                let i = idx(x, y);
                let (sx, sy) = self.scr_h(x as f32, y as f32, self.map.surface(x, y));
                let tl = self.map.t[i];
                let near_front =
                    (s as f32) > ps + 0.3 && (s as f32 - ps) < 6.0 && ((d as f32) - pd).abs() < 4.5;
                if tl == T_MOUNT {
                    let a = if near_front && (d as f32 - pd).abs() < 2.5 && (s as f32 - ps) < 3.5 {
                        0.45
                    } else {
                        1.0
                    };
                    if a < 1.0 {
                        c.alpha(a);
                    }
                    self.mountain(c, x, y, sx, sy, self.map.shade[i], z);
                    if a < 1.0 {
                        c.alpha(1.0);
                    }
                }
                // the front walls of a house just behind this tile
                self.thin_walls_at(c, x, y, near_front, false, 0, z, t, &mut lights);
                if let Some((b, f)) = upstairs {
                    if self.map.buildings[b].room(x, y) {
                        self.upper_tile(c, x, y, f, z, t, &mut lights);
                        self.thin_walls_at(c, x, y, false, true, f, z, t, &mut lights);
                        continue;
                    }
                }
                self.thin_walls_at(c, x, y, false, true, 0, z, t, &mut lights);
                let o = self.map.o[i];
                if o == O_NONE {
                    self.clutter(c, x, y, sx, sy, tl, z);
                }
                if o != O_NONE {
                    // anything tall enough to hide you turns see-through
                    let tall = size(o).1;
                    let occl = match o {
                        O_WALL => near_front,
                        O_PART => false,
                        _ if tall >= 1.4 => {
                            near_front
                                && (d as f32 - pd).abs() < 1.0 + tall * 0.4
                                && (s as f32 - ps) < 1.0 + tall * 0.75
                        }
                        _ => false,
                    };
                    if occl {
                        c.alpha(0.35);
                        c.mute_ink = true;
                    }
                    c.outline(INK, self.pix.max(z));
                    self.object(
                        c,
                        o,
                        x,
                        y,
                        sx,
                        sy,
                        self.map.shade[i],
                        self.map.style[i],
                        z,
                        t,
                        &mut lights,
                    );
                    c.outline(0, 0.0);
                    // the storeys above a doorway are wall
                    if o == O_DOOR {
                        self.over_door(c, x, y, sx, sy, z, t, &mut lights);
                    }
                    if occl {
                        c.alpha(1.0);
                        c.mute_ink = false;
                    }
                }
            }
            for cp in self.corpses.iter().filter(|cp| cp.x + cp.y == s) {
                let (sx, sy) = self.scr_h(cp.x as f32, cp.y as f32, self.map.surface(cp.x, cp.y));
                self.corpse(c, cp, sx, sy, z);
            }
            while oi < order.len() && order[oi].0 <= s {
                let e = &self.ents[order[oi].1];
                let (sx, sy) = self.scr_h(e.x, e.y, e.anim.vz);
                if sx > -150.0 && sx < self.w + 150.0 && sy > -150.0 && sy < self.h + 250.0 {
                    c.outline(INK, self.pix.max(z));
                    self.entity(c, e, sx, sy, z, t);
                    c.outline(0, 0.0);
                }
                oi += 1;
            }
            // roofs close over everything inside once their front corner is reached
            for b in self.map.buildings.iter() {
                if b.x + b.w - 1 + b.y + b.h - 1 == s {
                    self.roof(c, b, z);
                }
            }
        }
        while oi < order.len() {
            oi += 1;
        }

        // ---- effects
        for f in self.fx.iter() {
            self.effect(c, f, z, t);
        }

        // ---- Pagan-style perpetual twilight: violet haze and a dark vignette
        c.outline(0, 0.0);
        // drifting fog banks, thick and dark in the Shadow
        let gv = self.gloom_view;
        let fog = if gv > 0.02 {
            with_a(0x4a3a58ff, (10.0 + gv * 26.0) / 255.0)
        } else {
            0xa8a0900a
        };
        for i in 0..6 + (gv * 6.0) as i32 {
            let fi = i as f32;
            let fx = (h2(i, 3, 1) * (self.w + 500.0) + t * (5.0 + fi)) % (self.w + 500.0) - 250.0;
            let fy = h2(i, 5, 2) * self.h;
            c.fill(fog);
            c.ell(fx, fy, 260.0 * z, 70.0 * z);
            c.ell(fx + 90.0 * z, fy + 20.0 * z, 180.0 * z, 50.0 * z);
        }
        c.fill(0x2a120a16);
        c.rect(-10.0, -10.0, self.w + 20.0, self.h + 20.0);
        if gv > 0.02 {
            c.fill(with_a(GLOOM, gv * 0.3));
            c.rect(-10.0, -10.0, self.w + 20.0, self.h + 20.0);
        }
        c.dark(
            self.w / 2.0,
            self.h / 2.0,
            self.w.min(self.h) * (0.34 - gv * 0.1),
            self.w.max(self.h) * 0.75,
            0.38 + gv * 0.3,
            0x050302ff,
        );
        // falling ash and the odd ember
        let px1 = self.pix.max(1.0);
        for i in 0..44 + (gv * 90.0) as i32 {
            let (a, b) = (h2(i, 11, 3), h2(i, 13, 4));
            let ax = (a * self.w + t * (6.0 + b * 10.0) + (t * 0.7 + fi32(i)).sin() * 12.0)
                .rem_euclid(self.w);
            let ay = (b * self.h + t * (10.0 + a * 14.0)).rem_euclid(self.h);
            c.fill(if i % 9 == 0 {
                with_a(0xff7a30ff, 0.5 + (t * 6.0 + fi32(i)).sin() * 0.3)
            } else {
                0x9a948870
            });
            c.rect(ax, ay, 1.5 * px1, 1.5 * px1);
        }

        // the hour, the weather and the forest's shade
        self.draw_sky(c, z, t, &lights);

        // ---- overhead texts, bars, floats (above everything)
        let tid = p.combat.target;
        for e in self.ents.iter() {
            if e.dead || !self.ent_visible(e) {
                continue;
            }
            let (sx, sy) = self.scr_h(e.x, e.y, e.anim.vz);
            if sx < -100.0 || sx > self.w + 100.0 || sy < -100.0 || sy > self.h + 200.0 {
                continue;
            }
            let size = self.ent_size(e);
            let top = sy - (44.0 * size * ZS + 18.0) * z;
            let ts = (12.0 * z.max(0.85)).min(16.0);
            let targeted = e.id == tid && tid != 0;
            let show_bar = targeted
                || (e.hp < e.mhp && e.k != EK::Player && matches!(e.k, EK::Mon(_)))
                || (e.k == EK::Player && e.hp < e.mhp && !self.pl.ghost);
            if show_bar {
                let bw = 30.0 * z;
                c.fill(0x000000b0);
                c.rect(
                    sx - bw / 2.0 - 1.0,
                    top - 1.0,
                    bw + 2.0,
                    5.0 * z.max(0.8) + 2.0,
                );
                c.fill(0xc02020ff);
                c.rect(sx - bw / 2.0, top, bw, 5.0 * z.max(0.8));
                c.fill(if e.k == EK::Player {
                    0x3070ffff
                } else {
                    0x30d040ff
                });
                c.rect(
                    sx - bw / 2.0,
                    top,
                    bw * (e.hp.max(0) as f32 / e.mhp.max(1) as f32),
                    5.0 * z.max(0.8),
                );
            }
            if targeted {
                let col = match e.k {
                    EK::Mon(m) if MONS[m].karma < 0 => 0xff5050ff,
                    EK::Mon(_) => 0xc8c8c8ff,
                    _ => 0x60a0ffff,
                };
                c.text(sx, top - 6.0 * z, ts, 0, &e.name, 0x000000ff);
                let _ = col;
                c.stroke(with_a(col, 0.9), 2.0);
                c.ell_s(sx, sy, 16.0 * z * size.max(0.7), 7.0 * z * size.max(0.7));
            }
            if e.say.t > 0.0 && !e.say.text.is_empty() {
                let a = (e.say.t * 2.0).min(1.0);
                c.alpha(a);
                c.fill(e.say.col);
                c.text(
                    sx,
                    top - if targeted { 22.0 } else { 8.0 } * z,
                    ts,
                    0,
                    &e.say.text,
                    0x000000ff,
                );
                c.alpha(1.0);
            }
        }
        for cp in self.corpses.iter().filter(|cp| cp.player) {
            let (sx, sy) = self.scr_h(cp.x as f32, cp.y as f32, self.map.surface(cp.x, cp.y));
            c.fill(0xffd070ff);
            c.text(
                sx,
                sy - 16.0 * z,
                (11.0 * z).max(10.0),
                0,
                "your corpse",
                0x000000ff,
            );
        }
        for f in self.floats.iter() {
            let (sx, sy) = self.scr_h(
                f.x,
                f.y,
                self.map.surface(f.x.round() as i32, f.y.round() as i32),
            );
            c.alpha((1.2 - f.t).min(1.0));
            c.fill(f.color);
            c.text(
                sx + 10.0 * z,
                sy - (50.0 + f.t * 40.0) * z,
                (14.0 * z).max(12.0),
                0,
                &f.text,
                0x000000ff,
            );
            c.alpha(1.0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ground(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        tl: u8,
        _sh: u8,
        z: f32,
        _t: f32,
    ) {
        let hw = 32.0 * z + 0.7;
        let hh = 16.0 * z + 0.7;
        let lz = LH * z;
        let cn = self.map.corners(x, y);
        let base = self.map.level(x, y) as f32;
        if tl != T_MOUNT {
            let pts = self.diamond(sx, sy, &cn);
            self.pat(c, self.ground_tex(x, y, tl), base * lz);
            c.poly(&pts);
            if tl == T_BRIDGE {
                let sy = sy - base * lz;
                c.fill(0x00000050);
                let (a, b) = (sx - hw * 0.5, sy - hh * 0.5);
                c.poly(&[
                    (a, b),
                    (a + 3.0 * z, b - 1.5 * z),
                    (a + hw + 3.0 * z, b + hh - 1.5 * z),
                    (a + hw, b + hh),
                ]);
            }
        }
        self.ledges(c, x, y, sx, sy, &cn, z);
    }

    /// A tile's top, its corners raised to their levels.
    fn diamond(&self, sx: f32, sy: f32, cn: &[f32; 4]) -> [(f32, f32); 4] {
        let z = self.zoom;
        let (hw, hh, lz) = (32.0 * z + 0.7, 16.0 * z + 0.7, LH * z);
        [
            (sx, sy - hh - cn[0] * lz),
            (sx + hw, sy - cn[1] * lz),
            (sx, sy + hh - cn[2] * lz),
            (sx - hw, sy - cn[3] * lz),
        ]
    }
    /// whether `ledges` would draw anything for this tile
    fn has_ledges(&self, x: i32, y: i32, cn: &[f32; 4]) -> bool {
        let n1 = self.map.corners(x + 1, y);
        let n2 = self.map.corners(x, y + 1);
        cn[1] > n1[0] + 0.01 || cn[2] > n1[3] + 0.01 || cn[3] > n2[0] + 0.01 || cn[2] > n2[1] + 0.01
    }

    /// Rock faces where this tile stands above its front neighbours.
    #[allow(clippy::too_many_arguments)]
    fn ledges(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, cn: &[f32; 4], z: f32) {
        let hw = 32.0 * z + 0.7;
        let hh = 16.0 * z + 0.7;
        let lz = LH * z;
        let n1 = self.map.corners(x + 1, y);
        if cn[1] > n1[0] + 0.01 || cn[2] > n1[3] + 0.01 {
            let face = [
                (sx + hw, sy - cn[1] * lz),
                (sx, sy + hh - cn[2] * lz),
                (sx, sy + hh - n1[3] * lz),
                (sx + hw, sy - n1[0] * lz),
            ];
            // a slope's side is an earth bank, a full step is rock
            if cn[1] <= n1[0] + 0.01 || cn[2] <= n1[3] + 0.01 {
                self.pat(
                    c,
                    self.ground_tex(x, y, self.map.tile(x, y)),
                    self.map.level(x, y) as f32 * lz,
                );
            } else {
                self.face_pat(c, TEX_CLIFF, true);
            }
            c.poly(&face);
            c.fill(0x00000030);
            c.poly(&face);
            self.ledge_lip(c, x, y, &face, 1, z);
        }
        let n2 = self.map.corners(x, y + 1);
        if cn[3] > n2[0] + 0.01 || cn[2] > n2[1] + 0.01 {
            let face = [
                (sx - hw, sy - cn[3] * lz),
                (sx, sy + hh - cn[2] * lz),
                (sx, sy + hh - n2[1] * lz),
                (sx - hw, sy - n2[0] * lz),
            ];
            // a slope's side is an earth bank, a full step is rock
            if cn[3] <= n2[0] + 0.01 || cn[2] <= n2[1] + 0.01 {
                self.pat(
                    c,
                    self.ground_tex(x, y, self.map.tile(x, y)),
                    self.map.level(x, y) as f32 * lz,
                );
            } else {
                self.face_pat(c, TEX_CLIFF, false);
            }
            c.poly(&face);
            c.fill(0x00000070);
            c.poly(&face);
            self.ledge_lip(c, x, y, &face, 2, z);
        }
    }

    /// The top of a ledge's face: the ground above curling over its edge in
    /// a ragged lip, a lit rim, and the foot of the face in shadow, so a
    /// step of rock reads as the edge of a terrace.
    fn ledge_lip(&self, c: &mut Canvas, x: i32, y: i32, face: &[(f32, f32); 4], k: i32, z: f32) {
        let ink = self.pix.max(z);
        let tl = self.map.tile(x, y);
        let drop = |i: usize| (face[3 - i].1 - face[i].1).max(0.0);
        // the foot, darker where it meets the ground below
        let foot = |f: f32| {
            [
                (face[3].0, face[3].1 - drop(0) * f),
                (face[2].0, face[2].1 - drop(1) * f),
                face[2],
                face[3],
            ]
        };
        c.fill(0x0000002c);
        c.poly(&foot(0.45));
        c.fill(0x00000030);
        c.poly(&foot(0.2));
        if tl != T_MOUNT && tl != T_WATER && drop(0) > 4.0 * z && drop(1) > 4.0 * z {
            let n = 8;
            let mut pts = Vec::with_capacity(2 * n + 2);
            let lerp = |a: (f32, f32), b: (f32, f32), t: f32| {
                (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
            };
            for i in 0..=n {
                pts.push(lerp(face[0], face[1], i as f32 / n as f32));
            }
            for i in (0..=n).rev() {
                let t = i as f32 / n as f32;
                let (px, py) = lerp(face[0], face[1], t);
                let d = (1.2 + h2(x * 2 + k, y * 2 - k, i as i32) * 3.2) * z;
                pts.push((px, py + d.min(drop(0).min(drop(1)) * 0.5)));
            }
            self.pat(
                c,
                self.ground_tex(x, y, tl),
                self.map.level(x, y) as f32 * LH * z,
            );
            c.poly(&pts);
            c.fill(0x00000030);
            c.poly(&pts);
            // the rim catches the light
            c.stroke(0xd8d0a03a, ink);
            c.line(face[0].0, face[0].1 + ink, face[1].0, face[1].1 + ink);
        }
        c.stroke(INK, ink);
        c.line(face[0].0, face[0].1, face[1].0, face[1].1);
    }

    /// Ragged border of a tile's texture spilling into a lower-priority neighbour,
    /// so terrain types blend instead of meeting on the diamond grid.
    #[allow(clippy::too_many_arguments)]
    fn fringe(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        tl: u8,
        dir: (i32, i32),
        z: f32,
    ) {
        let lift = self.map.level(x, y) as f32 * LH * z;
        let sy = sy - lift;
        let pt = |u: f32, v: f32| (sx + (u - v) * 32.0 * z, sy - 16.0 * z + (u + v) * 16.0 * z);
        let edge = |s: f32, d: f32| match dir {
            (1, 0) => (1.0 + d, s),
            (-1, 0) => (-d, s),
            (0, 1) => (s, 1.0 + d),
            _ => (s, -d),
        };
        let (kx, ky) = (x * 2 + dir.0, y * 2 + dir.1);
        // a smooth, wandering line rather than a row of spikes: random
        // depths at a few knots, eased between them
        let n = 12;
        let knots = 4;
        let depth = |k: usize| -> f32 {
            if k == 0 || k == n {
                return 0.0;
            }
            let f = k as f32 / n as f32 * knots as f32;
            let (i, u) = (f.floor() as i32, f.fract());
            let u = u * u * (3.0 - 2.0 * u);
            let a = 0.08 + h2(kx, ky, i) * 0.26;
            let b = 0.08 + h2(kx, ky, i + 1) * 0.26;
            // taper to nothing at the corners, so neighbouring edges meet
            let taper = (k.min(n - k) as f32 / 2.0).min(1.0);
            (a + (b - a) * u) * taper + (h2(kx, ky, k as i32 + 40) - 0.5) * 0.05 * taper
        };
        let mut pts = Vec::with_capacity(2 * n + 2);
        for k in 0..=n {
            let (u, v) = edge(k as f32 / n as f32, -0.02);
            pts.push(pt(u, v));
        }
        for k in (0..=n).rev() {
            let (u, v) = edge(k as f32 / n as f32, depth(k));
            pts.push(pt(u, v));
        }
        self.pat(c, self.ground_tex(x, y, tl), lift);
        c.poly(&pts);
        if self.map.tile(x + dir.0, y + dir.1) == T_WATER {
            // foam along the shore's own ragged line, and a fainter line a
            // little way out on the water
            let ink = self.pix.max(z);
            c.stroke(0xc8d4d880, 1.4 * ink);
            let foam: Vec<(f32, f32)> = pts[n + 1..].to_vec();
            c.poly_s(&foam);
            let out: Vec<(f32, f32)> = (0..=n)
                .rev()
                .filter(|&k| k > 0 && k < n)
                .map(|k| {
                    let w = (h2(kx, ky, k as i32 + 80) - 0.5) * 0.03;
                    let (u, v) = edge(k as f32 / n as f32, depth(k) + 0.1 + w);
                    pt(u, v)
                })
                .collect();
            c.stroke(0xb8c8d038, ink);
            c.poly_s(&out);
        }
    }

    /// set a world-anchored texture pattern as the fill
    fn pat(&self, c: &mut Canvas, tex: u32, lift: f32) {
        let z = self.zoom;
        let mut e = -self.camx * z + self.w / 2.0;
        let f = -self.camy * z + self.h / 2.0 - lift;
        if tex == TEX_WATER {
            e += (self.clock * 4.0).rem_euclid(128.0) * z;
        }
        c.pattern(tex, z, 0.0, 0.0, z, e, f);
    }
    /// textured vertical face; `right` faces slope up to the right
    fn face_pat(&self, c: &mut Canvas, tex: u32, right: bool) {
        let z = self.zoom;
        let e = -self.camx * z + self.w / 2.0;
        let f = -self.camy * z + self.h / 2.0;
        c.pattern(tex, z, if right { -0.5 * z } else { 0.5 * z }, 0.0, z, e, f);
    }

    #[allow(clippy::too_many_arguments)]
    fn mountain(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, sh: u8, z: f32) {
        let hw = 32.0 * z + 0.6;
        let hh = 16.0 * z + 0.6;
        let mut n = 0;
        for &(dx, dy) in DIRS.iter() {
            if self.map.tile(x + dx, y + dy) == T_MOUNT {
                n += 1;
            }
        }
        if let Some(pid) = self.sprites.props {
            let tall = n >= 6;
            let k = bake::P_CRAG + (sh as usize % 3) + if tall { 3 } else { 0 };
            let (ax, ay) = bake::PROP_ANCHOR;
            c.outline(0, 0.0);
            c.sprite(
                pid,
                k,
                bake::PROP_FW,
                bake::PROP_FH,
                ax,
                ay,
                sx,
                sy,
                z,
                (sh & 8 != 0) as u32,
            );
            c.outline(INK, self.pix.max(z));
            return;
        }
        let hgt = (10.0 + n as f32 * 4.0 + (sh % 7) as f32) * z;
        let ink = self.pix.max(z);
        let left = [
            (sx - hw, sy),
            (sx, sy + hh),
            (sx, sy + hh - hgt),
            (sx - hw, sy - hgt),
        ];
        let right = [
            (sx, sy + hh),
            (sx + hw, sy),
            (sx + hw, sy - hgt),
            (sx, sy + hh - hgt),
        ];
        let top = [
            (sx, sy - hh - hgt),
            (sx + hw, sy - hgt),
            (sx, sy + hh - hgt),
            (sx - hw, sy - hgt),
        ];
        c.outline(INK, ink);
        self.face_pat(c, TEX_CLIFF, false);
        c.poly(&left);
        self.face_pat(c, TEX_CLIFF, true);
        c.poly(&right);
        c.outline(0, 0.0);
        c.fill(0x00000070);
        c.poly(&left);
        c.fill(0x00000030);
        c.poly(&right);
        self.pat(c, TEX_ROCK, hgt);
        c.poly(&top);
        // ink only where the plateau meets lower ground
        c.stroke(INK, ink);
        if self.map.tile(x - 1, y) != T_MOUNT {
            c.line(top[3].0, top[3].1, top[0].0, top[0].1);
        }
        if self.map.tile(x, y - 1) != T_MOUNT {
            c.line(top[0].0, top[0].1, top[1].0, top[1].1);
        }
        if self.map.tile(x + 1, y) != T_MOUNT {
            c.line(top[1].0, top[1].1, top[2].0, top[2].1);
        }
        if self.map.tile(x, y + 1) != T_MOUNT {
            c.line(top[2].0, top[2].1, top[3].0, top[3].1);
        }
        if self.map.res[idx(x, y)] == 0 {
            c.fill(0x00000050);
            c.poly(&top);
        }
    }

    /// Grass tufts, ferns, pebbles and twigs scattered over open ground, as
    /// Pagan's wilds are littered with small things.
    #[allow(clippy::too_many_arguments)]
    fn clutter(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, tl: u8, z: f32) {
        let Some(pid) = self.sprites.props else {
            return;
        };
        if self.map.in_town(x, y) && tl != T_GRASS {
            return;
        }
        let h = ((x as u32).wrapping_mul(0x9e37_79b9) ^ (y as u32).wrapping_mul(0x85eb_ca6b))
            .wrapping_mul(0xc2b2_ae35);
        let r = (h >> 24) as usize; // 0..255
        let k = match tl {
            T_GRASS if r < 70 => bake::P_TUFT + r % 3,
            T_GRASS if r < 84 => bake::P_FERN,
            T_GRASS if r < 92 => bake::P_PEBBLES + r % 2,
            T_DIRT if r < 40 => bake::P_PEBBLES + r % 2,
            T_DIRT if r < 60 => bake::P_TWIGS,
            T_DIRT if r < 75 => bake::P_TUFT + r % 3,
            T_SAND if r < 30 => bake::P_PEBBLES + r % 2,
            _ => return,
        };
        // somewhere inside the tile, not always its middle
        let (ox, oy) = (
            ((h >> 4) & 15) as f32 / 15.0 - 0.5,
            ((h >> 8) & 15) as f32 / 15.0 - 0.5,
        );
        let (px, py) = (sx + (ox - oy) * 22.0 * z, sy + (ox + oy) * 11.0 * z);
        let (ax, ay) = bake::PROP_ANCHOR;
        c.outline(0, 0.0);
        c.sprite(
            pid,
            k,
            bake::PROP_FW,
            bake::PROP_FH,
            ax,
            ay,
            px,
            py,
            z,
            (h >> 12) & 1,
        );
    }

    /// What the shop sign at a tile says.
    fn sign_label(&self, x: i32, y: i32) -> Option<&'static str> {
        self.map
            .buildings
            .iter()
            .find(|b| b.part_of.is_none() && b.x + 4 == x && b.y + 6 == y)
            .map(|b| match (b.vendor, b.style) {
                (V_SMITH, _) => "SMITH",
                (V_MAGE, _) => "MAGE",
                (V_PROV, 2) => "INN",
                (V_PROV, _) => "GOODS",
                (V_HEALER, _) => "HEALER",
                (V_BANKER, _) => "BANK",
                (V_BARKEEP, _) => "PUB",
                (V_LIBRARIAN, _) => "BOOKS",
                _ => "",
            })
    }

    /// Torches and banners hung on the outside faces of a wall.
    #[allow(clippy::too_many_arguments)]
    /// Merlons along every edge of a castle wall's walk that faces open
    /// ground or lower wall, drawn back to front.
    #[allow(clippy::too_many_arguments)]
    fn battlements(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        style: u8,
        hgt: f32,
        z: f32,
    ) {
        let hw = 32.0 * z;
        let hh = 16.0 * z;
        // a point on the wall top, in tile-local units (u along x, v along y)
        let at = |u: f32, v: f32, h: f32| (sx + (u - v) * hw, sy + (u + v) * hh - hgt - h);
        let open = |nx: i32, ny: i32| {
            self.map.obj(nx, ny) != O_WALL || {
                let ns = self.map.style[idx(nx, ny)];
                wall_h(ns) < wall_h(style)
            }
        };
        let mh = 9.0 * z;
        let ink = self.pix.max(z);
        let fort = matches!(style, WALL_FORT | WALL_BASTION);
        let stone = if fort { 0xa85a40ff } else { 0x6a6660ff };
        // back edges first (towards -y, -x), then the front ones
        let edges = [
            (x, y - 1, false, -0.36f32),
            (x - 1, y, true, -0.36),
            (x + 1, y, true, 0.36),
            (x, y + 1, false, 0.36),
        ];
        for (nx, ny, along_v, off) in edges {
            if !open(nx, ny) {
                continue;
            }
            for k in [-0.25f32, 0.25] {
                // centre of this merlon, and its half sizes along u and v
                let (cu, cv) = if along_v { (off, k) } else { (k, off) };
                let (a, b) = if along_v { (0.1, 0.13) } else { (0.13, 0.1) };
                let right = [
                    at(cu + a, cv - b, 0.0),
                    at(cu + a, cv + b, 0.0),
                    at(cu + a, cv + b, mh),
                    at(cu + a, cv - b, mh),
                ];
                let left = [
                    at(cu - a, cv + b, 0.0),
                    at(cu + a, cv + b, 0.0),
                    at(cu + a, cv + b, mh),
                    at(cu - a, cv + b, mh),
                ];
                let top = [
                    at(cu - a, cv - b, mh),
                    at(cu + a, cv - b, mh),
                    at(cu + a, cv + b, mh),
                    at(cu - a, cv + b, mh),
                ];
                c.outline(INK, ink);
                c.fill(shade(stone, 0.55));
                c.poly(&left);
                c.fill(shade(stone, 0.75));
                c.poly(&right);
                c.fill(shade(stone, 1.0));
                c.poly(&top);
                if fort {
                    // the south's merlons end in rounded caps
                    let (tx, ty) = at(cu, cv, mh);
                    c.ell(tx, ty - 1.5 * z, 4.0 * z, 3.0 * z);
                }
            }
        }
        c.outline(INK, ink);
    }

    #[allow(clippy::too_many_arguments)]
    /// Half-timbering on a plastered wall: posts, rails and a brace on each
    /// storey of both faces.
    #[allow(clippy::too_many_arguments)]
    fn timbers(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, storeys: u8, z: f32) {
        self.timbers_on(c, x, y, sx, sy, storeys, z, 3, 0.0);
    }
    /// Timbers on the faces in `mask` (1: the left face, 2: the right),
    /// from `from` storeys up.
    #[allow(clippy::too_many_arguments)]
    fn timbers_on(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        storeys: u8,
        z: f32,
        mask: u8,
        from: f32,
    ) {
        let (hw, hh) = (32.0 * z, 16.0 * z);
        let st = STOREY_PX * z;
        let w = 2.2 * self.pix.max(z);
        c.outline(0, 0.0);
        c.stroke(0x2e1c10ff, w);
        // (bottom-left corner, bottom-right corner) of each face
        let faces = [
            ((sx - hw, sy), (sx, sy + hh), 0),
            ((sx, sy + hh), (sx + hw, sy), 1),
        ];
        for k in 0..storeys {
            let up = k as f32 * st;
            if (k as f32) < from - 0.01 {
                continue;
            }
            for (a, b, side) in faces {
                if mask & (1 << side) == 0 {
                    continue;
                }
                let (a, b) = ((a.0, a.1 - up), (b.0, b.1 - up));
                let top = st - 1.5 * z;
                // the rail at the foot and the one at the top
                c.line(a.0, a.1 - 2.0 * z, b.0, b.1 - 2.0 * z);
                c.line(a.0, a.1 - top, b.0, b.1 - top);
                // a post at the near end
                c.line(b.0, b.1, b.0, b.1 - top);
                // a mid rail, and a brace on the faces without a window
                let window = side == 0 && (x + y) % 3 == 0;
                if !window {
                    if (x * 3 + y + k as i32 + side) % 2 == 0 {
                        c.line(a.0, a.1 - 2.0 * z, b.0, b.1 - top);
                    } else {
                        c.line(a.0, a.1 - top, b.0, b.1 - 2.0 * z);
                    }
                }
            }
        }
        c.outline(INK, self.pix.max(z));
    }

    /// A window on the left face of a wall, storey `k`: frame, panes lit
    /// warm after dark, shutters, and on a house's ground floor a box of
    /// flowers.
    #[allow(clippy::too_many_arguments)]
    fn house_window(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        p: &P,
        k: u8,
        style: u8,
        z: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        let up = k as f32 * STOREY_PX;
        // a point on the face: along it from its left end, and up from the ground
        let at = |u: f32, h: f32| p.at(u - 32.0, u * 0.5 - h - up);
        let quad = |c: &mut Canvas, u0: f32, u1: f32, h0: f32, h1: f32| {
            c.poly(&[at(u0, h0), at(u1, h0), at(u1, h1), at(u0, h1)]);
        };
        let ink = self.pix.max(z);
        let house = style == 2 || style == 3;
        let lit = self.daylight().0 > 0.18 && (x * 5 + y * 3 + k as i32) % 4 != 0;
        let h2v = h2(x, y, 71);
        c.outline(INK, ink);
        // shutters, folded back
        if house {
            c.fill([0x34502eff, 0x6a2a1eff, 0x2e3e5eff][(h2v * 3.0) as usize % 3]);
            quad(c, 4.0, 9.0, 10.0, 27.0);
            quad(c, 23.0, 28.0, 10.0, 27.0);
        }
        // frame and glass
        c.fill(0x2a1a10ff);
        quad(c, 9.0, 23.0, 9.0, 28.0);
        c.outline(0, 0.0);
        c.fill(if lit { 0xe8a040ff } else { 0x141a22ff });
        quad(c, 10.5, 21.5, 10.5, 26.5);
        if lit {
            c.fill(0xffd080ff);
            quad(c, 12.0, 16.0, 12.0, 18.0);
        } else {
            // the sky caught in the glass
            c.fill(0x4a5a6a90);
            c.poly(&[
                at(11.0, 20.0),
                at(14.0, 26.0),
                at(16.0, 26.0),
                at(12.0, 18.0),
            ]);
        }
        // the cross of the mullions
        c.stroke(0x2a1a10ff, ink);
        let (a, b) = (at(16.0, 10.0), at(16.0, 27.0));
        c.line(a.0, a.1, b.0, b.1);
        let (a, b) = (at(10.0, 18.5), at(22.0, 18.5));
        c.line(a.0, a.1, b.0, b.1);
        if house {
            {
                c.stroke(0x00000040, ink);
                for h in [14.0, 18.0, 22.0] {
                    for (u0, u1) in [(4.5, 8.5), (23.5, 27.5)] {
                        let (a, b) = (at(u0, h), at(u1, h));
                        c.line(a.0, a.1, b.0, b.1);
                    }
                }
            }
            if k == 0 && h2v > 0.35 {
                // a window box, spilling flowers
                c.outline(INK, ink);
                c.fill(0x4a2e18ff);
                quad(c, 8.0, 24.0, 5.0, 9.0);
                c.outline(0, 0.0);
                for i in 0..7 {
                    let u = 9.0 + i as f32 * 2.2;
                    let hh = 9.5 + h2(x + i, y, 73) * 2.5;
                    let (fx, fy) = at(u, hh);
                    c.fill(0x3a6a2aff);
                    c.ell(fx, fy + 1.0 * z, 1.6 * z, 1.3 * z);
                    c.fill(
                        [0xc83a3aff, 0xe8d060ff, 0xe8e0e8ff, 0xc860a0ff]
                            [(i as usize + (h2v * 4.0) as usize) % 4],
                    );
                    c.ell(fx, fy, 1.0 * z, 1.0 * z);
                }
            }
        }
        if lit {
            let (gx, gy) = at(16.0, 18.0);
            c.alpha(0.3);
            c.glow(gx, gy, 10.0 * z, 0xffa040ff);
            c.alpha(1.0);
            lights.push((gx, gy, 20.0, 0xffa050ff));
        }
        c.outline(INK, ink);
    }

    fn wall_deco(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        style: u8,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        self.wall_deco_on(c, x, y, sx, sy, style, z, t, lights, 3);
    }
    /// Torches and banners on the outward faces in `mask` (1: the right
    /// face, toward x+1; 2: the left, toward y+1).
    #[allow(clippy::too_many_arguments)]
    fn wall_deco_on(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        style: u8,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
        mask: u8,
    ) {
        let Some(pid) = self.sprites.props else {
            return;
        };
        if style == 4 && (x + y) % 2 == 0 {
            return;
        }
        let hw = 32.0 * z;
        let hh = 16.0 * z;
        // right face looks toward x+1, left face toward y+1 (drawn mirrored)
        for (nx, ny, flip, fx) in [(x + 1, y, false, 0.5f32), (x, y + 1, true, -0.5)] {
            if mask & if flip { 2 } else { 1 } == 0 {
                continue;
            }
            let nt = self.map.tile(nx, ny);
            let no = self.map.obj(nx, ny);
            if nt == T_FLOOR || no == O_WALL || no == O_DOOR || no == O_SIGN {
                continue;
            }
            if flip && (x + y) % 3 == 0 {
                continue; // a window already
            }
            let k = ((x * 7 + y * 13) as u32).wrapping_mul(2654435761) >> 28;
            let (cx, cy) = (sx + hw * fx, sy + hh * 0.5);
            let (ax, ay) = bake::PROP_ANCHOR;
            if k < 3 {
                let (tx, ty) = (cx, cy - 14.0 * z);
                c.sprite(
                    pid,
                    bake::P_TORCH,
                    bake::PROP_FW,
                    bake::PROP_FH,
                    ax,
                    ay,
                    tx,
                    ty,
                    z,
                    flip as u32,
                );
                let fl = 0.85
                    + (t * 11.0 + (x * 3 + y) as f32).sin() * 0.1
                    + (t * 23.0 + x as f32).sin() * 0.05;
                let fxo = if flip { -4.0 } else { 4.0 } * z;
                c.outline(0, 0.0);
                c.alpha(0.5);
                c.glow(tx + fxo, ty - 11.0 * z, 16.0 * z * fl, 0xff8a30ff);
                c.alpha(1.0);
                c.outline(INK, self.pix.max(z));
                lights.push((tx + fxo, ty - 11.0 * z, 110.0, 0xff9040ff));
            } else if k < 5 && style != 4 && !southern(style) {
                let v = bake::P_BANNER + ((x + y) as usize & 1);
                c.sprite(
                    pid,
                    v,
                    bake::PROP_FW,
                    bake::PROP_FH,
                    ax,
                    ay,
                    cx,
                    cy - 4.0 * z,
                    z,
                    flip as u32,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn object(
        &self,
        c: &mut Canvas,
        o: u8,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        sh: u8,
        style: u8,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        // a house's door is drawn in its thin wall, and swung in when open
        if o == O_DOOR && self.map.thin_at(x, y).is_some() {
            if !self.map.door_shut(x, y) {
                self.open_door(c, x, y, z);
            }
            return;
        }
        let hw = 32.0 * z + 0.6;
        let hh = 16.0 * z + 0.6;
        let p = P {
            x: sx,
            y: sy,
            s: z,
            f: 1.0,
        };
        if let Some(pid) = self.sprites.props {
            let flip_v = sh & 4 != 0;
            let anim4 = ((t * 9.0) as usize + (x * 3 + y) as usize) % 4;
            let pick = match o {
                O_TREE if self.map.tropic(x, y) => {
                    // palms, mostly; a banyan or a stand of bamboo in the jungle
                    let k = if style & 0x7f == 1 {
                        bake::P_DEAD + (sh as usize % 2)
                    } else if self.map.res[idx(x, y)] == 0 {
                        bake::P_STUMP
                    } else if self.map.tile(x, y) == T_SAND || sh % 5 < 3 {
                        bake::P_PALM + (sh as usize / 5 % 2)
                    } else if sh % 5 == 3 {
                        bake::P_BANYAN
                    } else {
                        bake::P_BAMBOO
                    };
                    Some((k, flip_v))
                }
                O_TREE => {
                    let k = if style & 0x7f == 1 {
                        bake::P_DEAD + (sh as usize % 2)
                    } else if self.map.res[idx(x, y)] == 0 {
                        bake::P_STUMP
                    } else if sh % 3 == 0 {
                        bake::P_PINE + (sh as usize / 3 % 2)
                    } else if sh % 7 == 1 {
                        bake::P_BIRCH + (sh as usize / 7 % 2)
                    } else {
                        // the oaks follow the season: amber in autumn, bare in winter
                        match self.sim.season(crate::sim::Sim::day(self.time)) {
                            "autumn" => bake::P_AUTUMN + (sh as usize % 2),
                            "winter" if sh % 3 == 1 => bake::P_DEAD + (sh as usize % 2),
                            _ => bake::P_OAK + (sh as usize % 3),
                        }
                    };
                    Some((k, flip_v))
                }
                O_MUSHROOM => Some((bake::P_SHROOM + (sh as usize % 2), flip_v)),
                O_BARREL => Some((bake::P_BARREL, false)),
                O_CRATE => Some((bake::P_CRATE, flip_v)),
                O_BED => Some((bake::P_BED, style & FACE_S != 0)),
                O_TABLE => Some((bake::P_TABLE, style & FACE_S != 0)),
                O_CHAIR => Some((bake::P_CHAIR, style & FACE_S != 0)),
                O_THRONE => Some((bake::P_THRONE, style & FACE_S != 0)),
                O_SHELF => Some((bake::P_SHELF, style & FACE_S != 0)),
                O_HEARTH => Some((bake::P_HEARTH, style & FACE_S != 0)),
                O_CHEST => Some((bake::P_CHEST, style & FACE_S != 0)),
                O_SACKS => Some((bake::P_SACKS, flip_v)),
                O_SKULLS => Some((bake::P_SKULLS, flip_v)),
                O_STATUE => Some((bake::P_STATUE, false)),
                O_WELL => Some((bake::P_WELL, false)),
                O_LONGTABLE => Some((bake::P_LONGTABLE, style & FACE_S != 0)),
                O_CART => Some((bake::P_CART, style & FACE_S != 0)),
                O_WARDROBE => Some((bake::P_WARDROBE, style & FACE_S != 0)),
                O_DRESSER => Some((bake::P_DRESSER, style & FACE_S != 0)),
                O_RACK => Some((bake::P_RACK, style & FACE_S != 0)),
                O_ARMOUR => Some((bake::P_ARMOUR, style & FACE_S != 0)),
                O_CAULDRON => Some((bake::P_CAULDRON, flip_v)),
                O_PLANT => Some((bake::P_PLANT + (sh as usize % 2), flip_v)),
                O_CANDLES => Some((bake::P_CANDLES, style & FACE_S != 0)),
                O_JARS => Some((bake::P_JARS, flip_v)),
                O_SPINWHEEL => Some((bake::P_SPINWHEEL, style & FACE_S != 0)),
                O_DESK => Some((bake::P_DESK, style & FACE_S != 0)),
                O_ORRERY => Some((bake::P_ORRERY, false)),
                O_STOOL => Some((bake::P_STOOL, flip_v)),
                O_HERBS => Some((bake::P_HERBS, style & FACE_S != 0)),
                O_RUIN => Some((bake::P_RUIN + (style as usize % 5), flip_v)),
                O_DOOR if self.map.door_shut(x, y) && !self.odd_door(x, y) => Some((
                    bake::P_DOOR + (self.map.obj(x - 1, y) != O_WALL) as usize,
                    true,
                )),
                O_FENCE if style & 0x7f != HEDGE => Some((
                    if style & 0x7f == 1 {
                        bake::P_RAIL
                    } else {
                        bake::P_FENCE
                    },
                    self.map.obj(x + 1, y) == O_FENCE || self.map.obj(x - 1, y) == O_FENCE,
                )),
                O_GRAVE => Some((bake::P_GRAVE + (sh as usize % 2), flip_v)),
                O_TENT => Some((bake::P_TENT, flip_v)),
                O_FIRE => Some((bake::P_FIRE + anim4, false)),
                O_ANKH => Some((bake::P_ANKH, false)),
                O_LAMP => Some((bake::P_LAMP + anim4, false)),
                O_BONES => Some((bake::P_BONES, flip_v)),
                O_BUSH => Some((bake::P_BUSH + (sh as usize % 2), flip_v)),
                O_BOULDER => Some((bake::P_BOULDER + (sh as usize % 2), flip_v)),
                O_FLOWERS => Some((bake::P_FLOWERS + (sh as usize % 2), flip_v)),
                O_HOARD => Some((bake::P_HOARD, flip_v)),
                O_SIGN => Some((bake::P_SIGN, true)),
                O_ANVIL => Some((bake::P_ANVIL, false)),
                O_FORGE => Some((bake::P_FORGE + anim4, false)),
                O_FOUNTAIN => {
                    // one basin over four tiles, drawn from the front one
                    if self.map.obj(x + 1, y) == O_FOUNTAIN || self.map.obj(x, y + 1) == O_FOUNTAIN
                    {
                        return;
                    }
                    Some((bake::P_FOUNTAIN, false))
                }
                _ => None,
            };
            if let Some((k, flip)) = pick {
                // Pagan draws no drop shadows; light comes from fires and magic
                let fl = 0.85 + (t * 11.0 + (x * 3 + y) as f32).sin() * 0.1;
                let glow = |c: &mut Canvas, gy: f32, r: f32, col: u32, a: f32| {
                    c.outline(0, 0.0);
                    c.alpha(a);
                    c.glow(sx, sy - gy * z, r * z * fl, col);
                    c.alpha(1.0);
                    c.outline(INK, self.pix.max(z));
                };
                match o {
                    O_HEARTH => lights.push((sx, sy - 10.0 * z, 130.0, 0xff9040ff)),
                    O_FIRE => {
                        glow(c, 8.0, 26.0, 0xff8030ff, 0.5);
                        lights.push((sx, sy - 6.0 * z, 140.0, 0xff9040ff));
                    }
                    O_LAMP => {
                        glow(c, 31.0, 22.0, 0xff8030ff, 0.5);
                        lights.push((sx, sy - 31.0 * z, 150.0, 0xffa050ff));
                    }
                    O_FORGE => {
                        glow(c, 16.0, 24.0, 0xff7020ff, 0.45);
                        lights.push((sx, sy - 14.0 * z, 90.0, 0xff9040ff));
                    }
                    O_ANKH => {
                        let gl = 0.5 + (t * 2.0).sin() * 0.2;
                        glow(c, 34.0, 30.0, 0xffe090ff, gl * 0.6);
                        lights.push((sx, sy - 30.0 * z, 110.0, 0xffe090ff));
                    }
                    O_HOARD => lights.push((sx, sy - 6.0 * z, 70.0, 0xffd060ff)),
                    O_CANDLES => {
                        glow(c, 36.0, 12.0, 0xffb050ff, 0.35);
                        lights.push((sx, sy - 34.0 * z, 90.0, 0xffb060ff));
                    }
                    O_CAULDRON => {
                        glow(c, 20.0, 12.0, 0x80e050ff, 0.3);
                        lights.push((sx, sy - 6.0 * z, 60.0, 0xff9040ff));
                    }
                    _ => {}
                }
                let (ax, ay) = bake::PROP_ANCHOR;
                // larger objects are drawn from their front tile at the middle of their footprint
                let (len, _) = size(o);
                let back = (len - 1) as f32 * 0.5;
                let (sx, sy) = if o == O_FOUNTAIN {
                    (sx, sy - 16.0 * z)
                } else if style & FACE_S != 0 {
                    (sx - back * 32.0 * z, sy - back * 16.0 * z)
                } else {
                    (sx + back * 32.0 * z, sy - back * 16.0 * z)
                };
                c.sprite(
                    pid,
                    k,
                    bake::PROP_FW,
                    bake::PROP_FH,
                    ax,
                    ay,
                    sx,
                    sy,
                    z,
                    flip as u32,
                );
                if o == O_SIGN {
                    if let Some(l) = self.sign_label(x, y) {
                        c.fill(0xf0e0b0ff);
                        c.text(sx + 2.0 * z, sy - 23.0 * z, 6.5 * z, 0, l, 0x00000000);
                    }
                }
                if o == O_HOARD && (t * 3.0 + x as f32).sin() > 0.6 {
                    c.outline(0, 0.0);
                    c.fill(0xffffffff);
                    c.ell(sx + 5.0 * z, sy - 12.0 * z, 1.5 * z, 4.0 * z);
                    c.ell(sx + 5.0 * z, sy - 12.0 * z, 4.0 * z, 1.5 * z);
                    c.outline(INK, self.pix.max(z));
                }
                return;
            }
        }
        match o {
            O_TREE => {
                let depleted = self.map.res[idx(x, y)] == 0;
                c.fill(0x00000040);
                c.ell(sx, sy, 16.0 * z, 7.0 * z);
                c.fill(0x3e2a18ff);
                c.poly(&[
                    p.at(-4.0, 0.0),
                    p.at(4.0, 0.0),
                    p.at(2.5, -26.0),
                    p.at(-2.5, -26.0),
                ]);
                if style == 1 {
                    // dead, gnarled tree
                    c.outline(0, 0.0);
                    c.stroke(INK, 4.5 * z);
                    let br = [
                        (-2.0, -18.0, -14.0, -34.0),
                        (1.0, -24.0, 12.0, -40.0),
                        (0.0, -26.0, -3.0, -48.0),
                        (-8.0, -28.0, -16.0, -30.0),
                        (6.0, -32.0, 15.0, -30.0),
                    ];
                    for &(ax, ay, bx, by) in br.iter() {
                        let (a, b) = (p.at(ax, ay), p.at(bx, by));
                        c.line(a.0, a.1, b.0, b.1);
                    }
                    c.stroke(0x4a3826ff, 2.5 * z);
                    for &(ax, ay, bx, by) in br.iter() {
                        let (a, b) = (p.at(ax, ay), p.at(bx, by));
                        c.line(a.0, a.1, b.0, b.1);
                    }
                    c.outline(INK, self.pix.max(z));
                    return;
                }
                let g = if depleted { 0x4e5028ff } else { 0x263c18ff };
                let f = 0.85 + sh as f32 / 255.0 * 0.3;
                if sh % 3 == 0 {
                    for k in 0..3 {
                        let kf = k as f32;
                        c.fill(shade(g, f * (0.8 + kf * 0.12)));
                        c.poly(&[
                            p.at(-15.0 + kf * 4.0, -16.0 - kf * 12.0),
                            p.at(15.0 - kf * 4.0, -16.0 - kf * 12.0),
                            p.at(0.0, -38.0 - kf * 12.0),
                        ]);
                    }
                } else {
                    c.fill(shade(g, f * 0.75));
                    c.ell(sx, sy - 34.0 * z, 21.0 * z, 15.0 * z);
                    c.fill(shade(g, f));
                    c.ell(sx - 7.0 * z, sy - 41.0 * z, 13.0 * z, 10.0 * z);
                    c.fill(shade(g, f * 1.25));
                    c.ell(sx + 5.0 * z, sy - 46.0 * z, 9.0 * z, 7.0 * z);
                    c.outline(0, 0.0);
                    for k in 0..5 {
                        let (u, v) = (h2(x, y, k) * 34.0 - 17.0, h2(x, y, k + 9) * 22.0 - 48.0);
                        c.fill(shade(g, if k % 2 == 0 { 0.6 } else { 1.45 }));
                        c.rect(
                            sx + u * z,
                            sy + v * z,
                            2.0 * self.pix.max(z),
                            self.pix.max(z),
                        );
                    }
                    c.outline(INK, self.pix.max(z));
                }
            }
            O_WALL if style == WALL_BURROW => self.burrow_wall(c, x, y, sx, sy, z),
            O_WALL => {
                // the dwarves' gate walls are battlemented; their halls are not
                let castle = matches!(style, WALL_CASTLE | WALL_TOWER | WALL_FORT | WALL_BASTION)
                    || (style == WALL_DWARF && self.map.tile(x, y) != T_FLOOR);
                let storeys = self.wall_storeys(x, y);
                let hgt = 38.0 * ZS * z * wall_h(style) * storeys as f32;
                let tex = wall_tex(style);
                let left = [
                    (sx - hw, sy),
                    (sx, sy + hh),
                    (sx, sy + hh - hgt),
                    (sx - hw, sy - hgt),
                ];
                let right = [
                    (sx, sy + hh),
                    (sx + hw, sy),
                    (sx + hw, sy - hgt),
                    (sx, sy + hh - hgt),
                ];
                let top = [
                    (sx, sy - hh - hgt),
                    (sx + hw, sy - hgt),
                    (sx, sy + hh - hgt),
                    (sx - hw, sy - hgt),
                ];
                if style == WALL_HAVELI {
                    // lime-wash runs on unbroken from tile to tile
                    c.outline(0x12100c30, self.pix.max(z));
                }
                self.face_pat(c, tex, false);
                c.poly(&left);
                self.face_pat(c, tex, true);
                c.poly(&right);
                c.outline(0, 0.0);
                if style == WALL_ELVEN {
                    // pale, nearly white stone
                    c.fill(0xe8ecf070);
                    c.poly(&left);
                    c.fill(0xf0f4f890);
                    c.poly(&right);
                    c.fill(0x2a3a5020);
                    c.poly(&left);
                } else if style == WALL_DWARF {
                    // dark granite, with a band of bronze knotwork high on
                    // each face where the old runes are cut
                    c.fill(0x10141aa8);
                    c.poly(&left);
                    c.fill(0x10141a70);
                    c.poly(&right);
                    let band = hgt * 0.8;
                    c.stroke(0x8a6a2aff, 2.0 * self.pix.max(z));
                    c.line(sx - hw, sy - band, sx, sy + hh - band);
                    c.line(sx, sy + hh - band, sx + hw, sy - band);
                    c.stroke(0xd8b050c0, self.pix.max(z));
                    c.line(sx - hw, sy - band - z, sx, sy + hh - band - z);
                    c.line(sx, sy + hh - band - z, sx + hw, sy - band - z);
                } else {
                    c.fill(if style == 4 { 0x000000a0 } else { 0x00000068 });
                    c.poly(&left);
                    c.fill(if style == 4 { 0x00000070 } else { 0x00000028 });
                    c.poly(&right);
                }
                // soot towards the ground
                c.fill(0x0a060430);
                c.poly(&[
                    (sx - hw, sy),
                    (sx, sy + hh),
                    (sx, sy + hh - hgt * 0.3),
                    (sx - hw, sy - hgt * 0.3),
                ]);
                c.poly(&[
                    (sx, sy + hh),
                    (sx + hw, sy),
                    (sx + hw, sy - hgt * 0.3),
                    (sx, sy + hh - hgt * 0.3),
                ]);
                c.outline(INK, self.pix.max(z));
                self.pat(c, tex, hgt);
                c.poly(&top);
                c.outline(0, 0.0);
                c.fill(0x0c080660);
                c.poly(&top);
                c.outline(INK, self.pix.max(z));
                // battlements on the open walls; the keep's lie under its roof
                if castle && self.map.tile(x, y) != T_FLOOR {
                    self.battlements(c, x, y, sx, sy, style, hgt, z);
                } else if style != 4 {
                    if style == 3 {
                        self.timbers(c, x, y, sx, sy, storeys, z);
                    }
                    let out = self.map.tile(x, y + 1) != T_FLOOR
                        && !matches!(self.map.obj(x, y + 1), O_WALL | O_DOOR);
                    if (x + y) % 3 == 0 && out && style != WALL_TEMPLE {
                        for k in 0..storeys {
                            if southern(style) {
                                self.jharokha(c, x, y, &p, k, style, z, lights);
                            } else {
                                self.house_window(c, x, y, &p, k, style, z, lights);
                            }
                        }
                    }
                }
                // the shade under the eaves
                if !castle
                    && style != 4
                    && style != WALL_ELVEN
                    && self.map.building_at(x, y).is_some()
                {
                    c.outline(0, 0.0);
                    let e = 5.0 * z;
                    c.fill(0x00000050);
                    c.poly(&[
                        (sx - hw, sy - hgt),
                        (sx, sy + hh - hgt),
                        (sx, sy + hh - hgt + e),
                        (sx - hw, sy - hgt + e),
                    ]);
                    c.poly(&[
                        (sx, sy + hh - hgt),
                        (sx + hw, sy - hgt),
                        (sx + hw, sy - hgt + e),
                        (sx, sy + hh - hgt + e),
                    ]);
                    c.outline(INK, self.pix.max(z));
                }
                // a timber beam where each storey meets the next
                if storeys > 1 {
                    c.stroke(0x2a1a10ff, 1.5 * self.pix.max(z));
                    for k in 1..storeys {
                        let up = k as f32 * STOREY_PX * z;
                        c.line(sx - hw, sy - up, sx, sy + hh - up);
                        c.line(sx, sy + hh - up, sx + hw, sy - up);
                    }
                }
                self.wall_deco(c, x, y, sx, sy, style, z, t, lights);
            }
            O_DOOR if self.odd_door(x, y) => self.odd_door_draw(c, x, y, sx, sy, z),
            O_FENCE if style & 0x7f == HEDGE => self.hedge(c, x, y, sx, sy, z),
            O_WHEEL => self.water_wheel(c, x, y, z, t),
            O_STALL => self.stall(c, x, y, sx, sy, style, z, t, lights),
            O_SHIP => self.ship(c, x, y, z, t),
            O_BIGTREE => self.great_tree(c, x, y, sx, sy, z, t, lights),
            O_DOOR if self.map.door_shut(x, y) => {
                c.fill(0x4a2e18ff);
                c.poly(&[
                    p.at(-30.0, -1.0),
                    p.at(-20.0, 4.0),
                    p.at(-20.0, -26.0),
                    p.at(-30.0, -31.0),
                ]);
            }
            O_DOOR => self.open_door(c, x, y, z),
            O_STAIRS => {
                let base = self.map.level(x, y) as f32;
                self.stairs(c, x, y, base, z);
            }
            O_ANVIL => {
                c.fill(0x00000050);
                c.ell(sx, sy, 14.0 * z, 6.0 * z);
                c.fill(0x404448ff);
                c.rect(sx - 4.0 * z, sy - 12.0 * z, 8.0 * z, 12.0 * z);
                c.fill(0x5a6066ff);
                c.poly(&[
                    p.at(-14.0, -12.0),
                    p.at(10.0, -12.0),
                    p.at(16.0, -16.0),
                    p.at(-10.0, -18.0),
                ]);
            }
            O_FORGE => {
                c.fill(0x6a3a2aff);
                c.poly(&[
                    p.at(-16.0, 0.0),
                    p.at(0.0, 8.0),
                    p.at(16.0, 0.0),
                    p.at(16.0, -18.0),
                    p.at(0.0, -10.0),
                    p.at(-16.0, -18.0),
                ]);
                c.fill(0x8a4a32ff);
                c.poly(&[
                    p.at(-16.0, -18.0),
                    p.at(0.0, -26.0),
                    p.at(16.0, -18.0),
                    p.at(0.0, -10.0),
                ]);
                let fl = 0.8 + (t * 9.0 + x as f32).sin() * 0.2;
                c.fill(0xff8a20ff);
                c.ell(sx, sy - 18.0 * z, 9.0 * z * fl, 4.0 * z);
                c.fill(0xffe060ff);
                c.ell(sx, sy - 19.0 * z, 4.0 * z * fl, 2.0 * z);
                lights.push((sx, sy - 18.0 * z, 90.0, 0xff9040ff));
            }
            O_GRAVE => {
                c.fill(0x00000040);
                c.ell(sx, sy, 10.0 * z, 4.0 * z);
                c.fill(0x8a8a88ff);
                c.rect(sx - 6.0 * z, sy - 16.0 * z, 12.0 * z, 16.0 * z);
                c.ell(sx, sy - 16.0 * z, 6.0 * z, 4.0 * z);
                c.stroke(0x505050ff, 1.2 * z);
                c.line(sx, sy - 16.0 * z, sx, sy - 6.0 * z);
                c.line(sx - 3.0 * z, sy - 12.0 * z, sx + 3.0 * z, sy - 12.0 * z);
                c.fill(0x6a5a40ff);
                c.poly(&[
                    p.at(-8.0, 4.0),
                    p.at(10.0, 10.0),
                    p.at(16.0, 6.0),
                    p.at(-2.0, 0.0),
                ]);
            }
            O_TENT => {
                c.fill(0x00000040);
                c.ell(sx, sy, 24.0 * z, 10.0 * z);
                c.fill(0x8a7050ff);
                c.poly(&[p.at(-24.0, 0.0), p.at(0.0, 10.0), p.at(0.0, -30.0)]);
                c.fill(0xa88a62ff);
                c.poly(&[p.at(0.0, 10.0), p.at(24.0, 0.0), p.at(0.0, -30.0)]);
                c.fill(0x2a2018ff);
                c.poly(&[p.at(-4.0, 8.0), p.at(4.0, 8.0), p.at(0.0, -8.0)]);
            }
            O_FIRE => {
                c.fill(0x4a3020ff);
                c.rect(sx - 10.0 * z, sy - 3.0 * z, 20.0 * z, 4.0 * z);
                for k in 0..3 {
                    let kf = k as f32;
                    let fl = (t * 10.0 + kf * 2.1).sin() * 3.0 * z;
                    c.fill(if k == 2 { 0xffe070ff } else { 0xff7a20e0 });
                    c.ell(
                        sx + (kf - 1.0) * 4.0 * z,
                        sy - 8.0 * z - fl.abs(),
                        (6.0 - kf) * z,
                        (10.0 - kf * 2.0) * z + fl,
                    );
                }
                lights.push((sx, sy - 8.0 * z, 140.0, 0xff9040ff));
            }
            O_ANKH => {
                c.fill(0x8a8478ff);
                c.poly(&[
                    p.at(-12.0, 0.0),
                    p.at(0.0, 6.0),
                    p.at(12.0, 0.0),
                    p.at(0.0, -6.0),
                ]);
                c.stroke(0xe8c040ff, 3.5 * z);
                c.ell_s(sx, sy - 40.0 * z, 5.0 * z, 7.0 * z);
                c.line(sx, sy - 33.0 * z, sx, sy - 4.0 * z);
                c.line(sx - 9.0 * z, sy - 30.0 * z, sx + 9.0 * z, sy - 30.0 * z);
                let gl = 0.5 + (t * 2.0).sin() * 0.2;
                c.fill(with_a(0xfff0a0ff, gl * 0.25));
                c.ell(sx, sy - 25.0 * z, 16.0 * z, 26.0 * z);
                lights.push((sx, sy - 25.0 * z, 110.0, 0xffe090ff));
            }
            O_LAMP => {
                // stone pedestal with an iron fire bowl
                c.fill(0x5a5650ff);
                c.poly(&[
                    p.at(-6.0, 0.0),
                    p.at(6.0, 0.0),
                    p.at(4.0, -20.0),
                    p.at(-4.0, -20.0),
                ]);
                c.fill(0x2a2420ff);
                c.ell(sx, sy - 22.0 * z, 10.0 * z, 4.5 * z);
                c.outline(0, 0.0);
                for k in 0..3 {
                    let kf = k as f32;
                    let fl = (t * 11.0 + kf * 2.3 + x as f32).sin() * 2.5 * z;
                    c.fill(if k == 2 { 0xffe070ff } else { 0xff6a18e0 });
                    c.ell(
                        sx + (kf - 1.0) * 3.5 * z,
                        sy - 27.0 * z - fl.abs(),
                        (5.0 - kf) * z,
                        (8.0 - kf * 1.5) * z + fl,
                    );
                }
                c.outline(INK, self.pix.max(z));
                lights.push((sx, sy - 26.0 * z, 150.0, 0xffa050ff));
            }
            O_MUSHROOM => {
                let big = 0.8 + h2(x, y, 5) * 0.6;
                let cap = if sh % 2 == 0 { 0x8a3a2aff } else { 0x6a5a8aff };
                c.fill(0xc8bca0ff);
                c.poly(&[
                    p.at(-4.0 * big, 0.0),
                    p.at(4.0 * big, 0.0),
                    p.at(3.0 * big, -26.0 * big),
                    p.at(-3.0 * big, -26.0 * big),
                ]);
                c.fill(cap);
                c.ell(sx, sy - 28.0 * z * big, 18.0 * z * big, 9.0 * z * big);
                c.fill(shade(cap, 1.3));
                c.ell(
                    sx - 4.0 * z * big,
                    sy - 31.0 * z * big,
                    8.0 * z * big,
                    4.0 * z * big,
                );
                c.outline(0, 0.0);
                c.fill(0xe8dcc0ff);
                for k in 0..4 {
                    let (u, v) = (h2(x, y, k) * 24.0 - 12.0, h2(x, y, k + 7) * 6.0 - 32.0);
                    c.rect(sx + u * z * big, sy + v * z * big, 2.0 * z, 1.5 * z);
                }
                c.outline(INK, self.pix.max(z));
            }
            O_BARREL => {
                c.fill(0x6a4424ff);
                c.rect(sx - 7.0 * z, sy - 18.0 * z, 14.0 * z, 16.0 * z);
                c.ell(sx, sy - 2.0 * z, 7.0 * z, 3.0 * z);
                c.fill(0x8a5a30ff);
                c.ell(sx, sy - 18.0 * z, 7.0 * z, 3.0 * z);
                c.outline(0, 0.0);
                c.stroke(0x2a2420ff, 1.5 * z);
                c.line(sx - 7.0 * z, sy - 7.0 * z, sx + 7.0 * z, sy - 7.0 * z);
                c.line(sx - 7.0 * z, sy - 14.0 * z, sx + 7.0 * z, sy - 14.0 * z);
                c.outline(INK, self.pix.max(z));
            }
            O_CRATE => {
                let hgt = 14.0 * z;
                let (w2, h2_) = (13.0 * z, 6.5 * z);
                c.fill(0x5a4026ff);
                c.poly(&[
                    (sx - w2, sy),
                    (sx, sy + h2_),
                    (sx, sy + h2_ - hgt),
                    (sx - w2, sy - hgt),
                ]);
                c.fill(0x7a5632ff);
                c.poly(&[
                    (sx, sy + h2_),
                    (sx + w2, sy),
                    (sx + w2, sy - hgt),
                    (sx, sy + h2_ - hgt),
                ]);
                c.fill(0x92683cff);
                c.poly(&[
                    (sx, sy - h2_ - hgt),
                    (sx + w2, sy - hgt),
                    (sx, sy + h2_ - hgt),
                    (sx - w2, sy - hgt),
                ]);
                c.outline(0, 0.0);
                c.stroke(0x3a2814ff, self.pix.max(z));
                c.line(sx - w2, sy - hgt, sx, sy + h2_);
                c.line(sx, sy + h2_ - hgt, sx + w2, sy);
                c.outline(INK, self.pix.max(z));
            }
            O_BONES => {
                c.outline(0, 0.0);
                c.stroke(0xd8d0b8ff, 2.0 * z);
                let a = h2(x, y, 1) * 6.0;
                c.line(sx - 8.0 * z, sy + a * z * 0.3, sx + 6.0 * z, sy - 3.0 * z);
                c.line(
                    sx - 3.0 * z,
                    sy + 4.0 * z,
                    sx + 2.0 * z,
                    sy - 4.0 * z + a * 0.3,
                );
                c.fill(0xe0d8c0ff);
                c.ell(sx + 9.0 * z, sy + 1.0 * z, 4.0 * z, 3.5 * z);
                c.fill(0x1a1410ff);
                c.rect(
                    sx + 7.5 * z,
                    sy,
                    1.2 * self.pix.max(z),
                    1.2 * self.pix.max(z),
                );
                c.rect(
                    sx + 10.0 * z,
                    sy,
                    1.2 * self.pix.max(z),
                    1.2 * self.pix.max(z),
                );
                c.outline(INK, self.pix.max(z));
            }
            O_FOUNTAIN => {
                c.fill(0x9a9a98ff);
                c.poly(&[(sx, sy - hh), (sx + hw, sy), (sx, sy + hh), (sx - hw, sy)]);
                c.fill(0x7a7a78ff);
                c.poly(&[
                    (sx - hw, sy),
                    (sx, sy + hh),
                    (sx, sy + hh - 6.0 * z),
                    (sx - hw, sy - 6.0 * z),
                ]);
                c.poly(&[
                    (sx, sy + hh),
                    (sx + hw, sy),
                    (sx + hw, sy - 6.0 * z),
                    (sx, sy + hh - 6.0 * z),
                ]);
                let w = shade(0x3a7ab0ff, 1.0 + (t * 2.0 + x as f32).sin() * 0.08);
                c.fill(w);
                c.poly(&[
                    (sx, sy - hh * 0.7 - 6.0 * z),
                    (sx + hw * 0.75, sy - 6.0 * z),
                    (sx, sy + hh * 0.7 - 6.0 * z),
                    (sx - hw * 0.75, sy - 6.0 * z),
                ]);
                if (x + y) % 2 == 0 {
                    c.fill(0xd0e8ffc0);
                    let j = (t * 8.0).sin() * 2.0 * z;
                    c.ell(sx, sy - 14.0 * z + j, 2.0 * z, 8.0 * z);
                }
            }
            O_BUSH => {
                c.fill(0x00000030);
                c.ell(sx, sy, 10.0 * z, 4.0 * z);
                c.fill(shade(0x3d7a2aff, 0.9 + sh as f32 / 900.0));
                c.ell(sx - 4.0 * z, sy - 6.0 * z, 7.0 * z, 6.0 * z);
                c.ell(sx + 4.0 * z, sy - 7.0 * z, 7.0 * z, 6.0 * z);
                c.fill(0x4f9036ff);
                c.ell(sx, sy - 10.0 * z, 6.0 * z, 5.0 * z);
            }
            O_BOULDER => {
                c.fill(0x00000040);
                c.ell(sx, sy, 14.0 * z, 6.0 * z);
                c.fill(0x7a766cff);
                c.ell(sx, sy - 8.0 * z, 13.0 * z, 10.0 * z);
                c.fill(0x9a968aff);
                c.ell(sx - 3.0 * z, sy - 12.0 * z, 6.0 * z, 4.0 * z);
            }
            O_FENCE => {
                c.stroke(0x3a3a3aff, 2.0 * z);
                let horiz = self.map.obj(x + 1, y) == O_FENCE || self.map.obj(x - 1, y) == O_FENCE;
                let (a, b) = if horiz {
                    (p.at(-16.0, -8.0), p.at(16.0, 8.0))
                } else {
                    (p.at(16.0, -8.0), p.at(-16.0, 8.0))
                };
                for k in [0.0f32, 0.5, 1.0] {
                    let (px, py) = (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
                    c.line(px, py, px, py - 16.0 * z);
                }
                c.line(a.0, a.1 - 12.0 * z, b.0, b.1 - 12.0 * z);
                c.line(a.0, a.1 - 6.0 * z, b.0, b.1 - 6.0 * z);
            }
            O_FLOWERS => {
                let cols = [0xe04060ff, 0xf0d040ff, 0xffffffff, 0x8060e0ff];
                for k in 0..5 {
                    let kf = k as f32;
                    let ox = ((sh as f32 + kf * 47.0) % 24.0 - 12.0) * z;
                    let oy = ((sh as f32 * 0.7 + kf * 31.0) % 10.0 - 5.0) * z;
                    c.fill(cols[(k + sh as usize) % 4]);
                    c.ell(sx + ox, sy + oy - 2.0 * z, 2.0 * z, 1.6 * z);
                }
            }
            O_HOARD => {
                c.fill(0xc89a28ff);
                c.ell(sx, sy - 4.0 * z, 20.0 * z, 9.0 * z);
                c.fill(0xf0c840ff);
                c.ell(sx - 3.0 * z, sy - 8.0 * z, 12.0 * z, 6.0 * z);
                let sp = (t * 3.0 + x as f32).sin();
                if sp > 0.6 {
                    c.fill(0xffffffff);
                    c.ell(sx + 5.0 * z, sy - 10.0 * z, 1.5 * z, 4.0 * z);
                    c.ell(sx + 5.0 * z, sy - 10.0 * z, 4.0 * z, 1.5 * z);
                }
                lights.push((sx, sy - 6.0 * z, 70.0, 0xffd060ff));
            }
            O_SIGN => {
                c.stroke(0x4a3020ff, 2.0 * z);
                c.line(sx, sy, sx, sy - 22.0 * z);
                c.fill(0x8a6238ff);
                c.rect(sx - 10.0 * z, sy - 30.0 * z, 20.0 * z, 10.0 * z);
                if let Some(l) = self.sign_label(x, y) {
                    c.fill(0xf0e0b0ff);
                    c.text(sx, sy - 22.0 * z, 6.5 * z, 0, l, 0x00000000);
                }
            }
            _ => {}
        }
    }

    /// A hipped roof over a building: lifted away while the player is inside,
    /// see-through while the player is just behind it.
    fn roof(&self, c: &mut Canvas, b: &Building, z: f32) {
        let (px, py) = self.ents[0].tile();
        let (bw, bh) = (b.w - 1, b.h - 1);
        if px >= b.x && px <= b.x + bw && py >= b.y && py <= b.y + bh {
            return;
        }
        // a house built of several parts opens up all together
        let bs = &self.map.buildings;
        let house = |i: usize| bs[i].part_of.unwrap_or(i);
        if let (Some(me), Some(at)) = (
            bs.iter().position(|o| std::ptr::eq(o, b)),
            self.map.building_at(px, py),
        ) {
            if bs[at].room(px, py) && house(me) == house(at) {
                return;
            }
        }
        if b.style == WALL_BURROW {
            return self.mound(c, b, z);
        }
        // taller walls hide more of what is behind them
        let reach = (wall_h(b.style) * b.storeys() as f32 * 5.0).round() as i32;
        let behind = px >= b.x - 2 && px <= b.x + bw + 2 && py >= b.y - reach && py < b.y;
        let side = py >= b.y - 2 && py <= b.y + bh + 1 && px >= b.x - reach && px < b.x;
        // a temple's tower hides still more
        let tall = if b.style == WALL_TEMPLE { 6 } else { 0 };
        let behind =
            behind || (px >= b.x - 2 && px <= b.x + bw + 2 && py >= b.y - reach - tall && py < b.y);
        if self.south_roof(c, b, z, if behind || side { 0.3 } else { 1.0 }) {
            return;
        }
        let (x0, y0, x1, y1) = (
            b.x as f32 - 0.8,
            b.y as f32 - 0.8,
            (b.x + bw) as f32 + 0.8,
            (b.y + bh) as f32 + 0.8,
        );
        let ym = (y0 + y1) / 2.0;
        let hw = (y1 - y0) / 2.0;
        let (rx0, rx1) = (x0 + hw, x1 - hw);
        let eave = 38.0 * ZS * z * wall_h(b.style) * b.storeys() as f32;
        let ridge = eave + 34.0 * z * (b.h as f32 / 6.0);
        let floor = self.map.level(b.x + 1, b.y + 1) as f32 * LH * z;
        let sp = |x: f32, y: f32, h: f32| {
            let (a, b) = self.to_screen(x, y);
            (a, b - h - floor)
        };
        let inn = b.vendor == V_PROV && b.style == 2;
        let base = match b.style {
            1 => 0x4a505eff,
            2 => 0x5a3a24ff,
            // plastered houses: tiles of red clay, some older and browner
            3 => [0x7a3a28ff, 0x8a4a2cff, 0x6a3a2eff][(b.x * 7 + b.y * 3).rem_euclid(3) as usize],
            WALL_CASTLE => 0x34383fff,
            WALL_ELVEN => 0x6a8098ff,
            // heavy slabs of dark slate
            WALL_DWARF => 0x2a2e34ff,
            _ => 0x3a3a40ff,
        };
        let faces: [(Vec<(f32, f32)>, f32); 4] = [
            (
                vec![
                    sp(x0, y0, eave),
                    sp(x1, y0, eave),
                    sp(rx1, ym, ridge),
                    sp(rx0, ym, ridge),
                ],
                0.6,
            ),
            (
                vec![sp(x0, y0, eave), sp(rx0, ym, ridge), sp(x0, y1, eave)],
                0.95,
            ),
            (
                vec![sp(x1, y0, eave), sp(x1, y1, eave), sp(rx1, ym, ridge)],
                0.7,
            ),
            (
                vec![
                    sp(x0, y1, eave),
                    sp(rx0, ym, ridge),
                    sp(rx1, ym, ridge),
                    sp(x1, y1, eave),
                ],
                1.0,
            ),
        ];
        let a = if behind || side { 0.3 } else { 1.0 };
        if a < 1.0 {
            c.alpha(a);
        }
        let ink = self.pix.max(z);
        for (pts, k) in faces.iter() {
            c.outline(INK, ink);
            c.fill(shade(base, *k));
            c.poly(pts);
            c.outline(0, 0.0);
            // courses of shingles running parallel to the eaves: course r runs
            // from l0 to l1, r = 0 at the eave and n at the ridge
            let n = 7;
            let course = |r: i32| {
                let f = r as f32 / n as f32;
                let lerp =
                    |p: (f32, f32), q: (f32, f32)| (p.0 + (q.0 - p.0) * f, p.1 + (q.1 - p.1) * f);
                if pts.len() == 4 {
                    // trapezoid: eave edge pts[0]-pts[1] (north) or pts[0]-pts[3] (south)
                    if *k < 0.65 {
                        (lerp(pts[0], pts[3]), lerp(pts[1], pts[2]))
                    } else {
                        (lerp(pts[0], pts[1]), lerp(pts[3], pts[2]))
                    }
                } else {
                    let apex = if *k > 0.9 { pts[1] } else { pts[2] };
                    let (e0, e1) = if *k > 0.9 {
                        (pts[0], pts[2])
                    } else {
                        (pts[0], pts[1])
                    };
                    (lerp(e0, apex), lerp(e1, apex))
                }
            };
            let tiled = b.style != WALL_ELVEN;
            for r in 0..n {
                let (l0, l1) = course(r);
                let (m0, m1) = course(r + 1);
                if r > 0 {
                    c.stroke(shade(base, k * 0.62), ink);
                    c.line(l0.0, l0.1, l1.0, l1.1);
                }
                // a lit lip along the lower edge of each course
                c.stroke(with_a(shade(base, k * 1.25), 0.5), ink);
                let (o0, o1) = (
                    (l0.0 + (m0.0 - l0.0) * 0.18, l0.1 + (m0.1 - l0.1) * 0.18),
                    (l1.0 + (m1.0 - l1.0) * 0.18, l1.1 + (m1.1 - l1.1) * 0.18),
                );
                c.line(o0.0, o0.1, o1.0, o1.1);
                if !tiled {
                    continue;
                }
                // the joints between tiles, staggered course by course
                let len = ((l1.0 - l0.0).powi(2) + (l1.1 - l0.1).powi(2)).sqrt();
                let m = (len / (9.0 * z)).floor() as i32;
                c.stroke(shade(base, k * 0.55), ink);
                for j in 0..m {
                    let f = (j as f32 + if r % 2 == 0 { 0.5 } else { 1.0 }) / (m as f32 + 0.5);
                    if f >= 1.0 {
                        continue;
                    }
                    let a = (l0.0 + (l1.0 - l0.0) * f, l0.1 + (l1.1 - l0.1) * f);
                    let bq = (m0.0 + (m1.0 - m0.0) * f, m0.1 + (m1.1 - m0.1) * f);
                    let e = (a.0 + (bq.0 - a.0) * 0.8, a.1 + (bq.1 - a.1) * 0.8);
                    c.line(a.0, a.1, e.0, e.1);
                    // the odd tile darker, or green with moss
                    let hv = h2(b.x * 31 + r, b.y * 17 + j, (k * 10.0) as i32);
                    if hv < 0.12 || (hv > 0.94 && b.style != 1) {
                        let g = (j as f32 + 0.5 + if r % 2 == 0 { 0.5 } else { 1.0 })
                            / (m as f32 + 0.5);
                        if g < 1.0 {
                            let q = (l0.0 + (l1.0 - l0.0) * g, l0.1 + (l1.1 - l0.1) * g);
                            let q2 = (m0.0 + (m1.0 - m0.0) * g, m0.1 + (m1.1 - m0.1) * g);
                            let (cx, cy) = (
                                (a.0 + q.0 + bq.0 + q2.0) / 4.0,
                                (a.1 + q.1 + bq.1 + q2.1) / 4.0,
                            );
                            c.fill(if hv < 0.12 { 0x00000038 } else { 0x4a6a2a70 });
                            c.ell(cx, cy, 3.5 * z, 2.2 * z);
                        }
                    }
                }
            }
            // the eave board
            let (e0, e1) = course(0);
            c.stroke(shade(base, k * 0.4), 1.5 * ink);
            c.line(e0.0, e0.1, e1.0, e1.1);
        }
        // ridge cap
        c.stroke(shade(base, 0.45), 2.0 * ink);
        let (r0, r1) = (sp(rx0, ym, ridge), sp(rx1, ym, ridge));
        c.line(r0.0, r0.1, r1.0, r1.1);
        if b.style == WALL_DWARF {
            // a bronze ridge cap, catching what light reaches the hold
            c.stroke(0xa8823aff, ink);
            c.line(r0.0, r0.1 - 0.5 * z, r1.0, r1.1 - 0.5 * z);
        }
        if b.style == WALL_ELVEN {
            // slender silver spires at the ridge's ends, each with a star
            for (sx, sy) in [r0, r1] {
                c.stroke(0xd8e0e8ff, 1.5 * ink);
                c.line(sx, sy, sx, sy - 22.0 * z);
                c.fill(0xf0f4ffff);
                c.ell(sx, sy - 23.0 * z, 2.0 * z, 2.0 * z);
                c.alpha(0.5);
                c.glow(sx, sy - 23.0 * z, 9.0 * z, 0xc0d8ffff);
                c.alpha(1.0);
            }
        }
        // every hall under the mountain keeps a fire going
        if inn
            || b.vendor == V_SMITH
            || b.vendor == V_BARKEEP
            || b.vendor == V_LORD
            || b.style == WALL_DWARF
        {
            // a stone chimney with a wisp of smoke
            let (cx, cy) = sp(rx1 - 0.6, ym - 0.4, ridge - 6.0 * z);
            c.outline(INK, ink);
            c.fill(0x5a5650ff);
            c.rect(cx - 5.0 * z, cy - 22.0 * z, 10.0 * z, 26.0 * z);
            c.fill(0x2a2624ff);
            c.rect(cx - 6.0 * z, cy - 24.0 * z, 12.0 * z, 4.0 * z);
            c.outline(0, 0.0);
            for k in 0..4 {
                let kf = k as f32;
                let t = (self.clock * 0.4 + kf * 0.25).fract();
                c.fill(with_a(0x8a8680ff, 0.35 * (1.0 - t)));
                c.ell(
                    cx + (t * 9.0 + kf).sin() * 6.0 * z,
                    cy - 28.0 * z - t * 40.0 * z,
                    (4.0 + t * 8.0) * z,
                    (3.0 + t * 6.0) * z,
                );
            }
        }
        if a < 1.0 {
            c.alpha(1.0);
        }
    }

    fn corpse(&self, c: &mut Canvas, cp: &Corpse, sx: f32, sy: f32, z: f32) {
        if let Some(l) = &cp.look {
            if let Some(info) = self.sprites.ids.get(&l.key()) {
                let (frame, flip) =
                    bake::frame_index(l, Anim::Die, (cp.age / 0.6).min(1.0), cp.dir8);
                c.sprite(
                    info.id,
                    frame,
                    info.fw,
                    info.fh,
                    info.ax,
                    info.ay,
                    sx,
                    sy,
                    z,
                    flip as u32,
                );
                if cp.player || !cp.items.is_empty() || cp.gold > 0 {
                    c.outline(0, 0.0);
                    c.fill(0xffd84060);
                    c.ell(sx, sy + 2.0 * z, 3.0 * z, 1.5 * z);
                }
                return;
            }
        }
        let s = z * cp.size.clamp(0.5, 1.8);
        c.fill(0x00000050);
        c.ell(sx, sy, 16.0 * s, 6.0 * s);
        c.fill(shade(cp.color, 0.6));
        c.ell(sx, sy - 3.0 * s, 13.0 * s, 5.0 * s);
        c.fill(0xe0d8c0ff);
        c.ell(sx + 9.0 * s, sy - 5.0 * s, 3.5 * s, 3.0 * s);
        c.stroke(0xe0d8c0ff, 1.5 * s);
        c.line(sx - 8.0 * s, sy - 2.0 * s, sx - 2.0 * s, sy - 6.0 * s);
        if cp.player || !cp.items.is_empty() || cp.gold > 0 {
            c.fill(0xffd84080);
            c.ell(sx, sy - 3.0 * s, 3.0 * s, 1.5 * s);
        }
    }

    fn entity(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, z: f32, t: f32) {
        if self.draw_sprite(c, e, sx, sy, z) {
            return;
        }
        let moving = !e.path.is_empty();
        let wk = if moving { e.anim.walk.sin() } else { 0.0 };
        let flash = e.anim.hurt > 0.5;
        match e.k {
            EK::Player => {
                let pl = &self.pl;
                let body = pl
                    .armor
                    .as_ref()
                    .map(|a| a.def().color)
                    .unwrap_or(0x8a6a4aff);
                let robe = pl
                    .armor
                    .as_ref()
                    .map(|a| a.def().id == "robe")
                    .unwrap_or(false);
                let weapon = match pl.weapon.as_ref().map(|w| w.def().id) {
                    None => 0,
                    Some("hatchet") => 2,
                    Some("mace") | Some("warhammer") | Some("pickaxe") => 3,
                    Some(_) => 1,
                };
                let h = Hum {
                    body,
                    robe,
                    legs: if robe { body } else { 0x3a3040ff },
                    hair: Some(pl.hair),
                    long_hair: pl.female,
                    weapon: if pl.war || matches!(pl.action, Action::Gather { .. }) {
                        weapon
                    } else {
                        0
                    },
                    shield: pl.shield.as_ref().map(|s| s.def().color),
                    helm: if pl
                        .armor
                        .as_ref()
                        .map(|a| a.def().id == "platemail")
                        .unwrap_or(false)
                    {
                        Some(0xc8ced6ff)
                    } else {
                        None
                    },
                    ..HUM
                };
                if pl.ghost {
                    c.alpha(0.45);
                    let g = Hum {
                        skin: 0xe8f0ffff,
                        body: 0xe8f0ffff,
                        legs: 0xe8f0ffff,
                        robe: true,
                        hair: None,
                        weapon: 0,
                        shield: None,
                        helm: None,
                        ..HUM
                    };
                    self.humanoid(
                        c,
                        e,
                        sx,
                        sy - 4.0 * z - (t * 2.0).sin() * 2.0 * z,
                        z,
                        1.0,
                        &g,
                        wk,
                        false,
                    );
                    c.alpha(1.0);
                } else {
                    if pl.war {
                        c.stroke(0xd0302080, 1.5);
                        c.ell_s(sx, sy, 14.0 * z, 6.0 * z);
                    }
                    self.humanoid(c, e, sx, sy, z, 1.0, &h, wk, flash);
                }
                if let Action::Cast { .. } = pl.action {
                    c.fill(with_a(0x80b0ffff, 0.4 + (t * 12.0).sin() * 0.2));
                    let (hx, hy) = (sx + e.anim.face * 10.0 * z, sy - 30.0 * z);
                    c.ell(hx, hy, 5.0 * z, 5.0 * z);
                }
            }
            EK::Npc(v) => {
                let vd = &VENDORS[v];
                let hair = HAIR[(e.look as usize) % HAIR.len()];
                let h = match v {
                    V_GUARD => Hum {
                        body: 0xc8ced6ff,
                        legs: 0x7a8088ff,
                        helm: Some(0xb0b8c0ff),
                        weapon: 5,
                        shield: Some(0x3050a0ff),
                        ..HUM
                    },
                    V_MAGE => Hum {
                        body: vd.robe,
                        legs: vd.robe,
                        robe: true,
                        hat: Some(vd.robe),
                        hair: Some(0xd0d0d0ff),
                        weapon: 4,
                        ..HUM
                    },
                    V_HEALER => Hum {
                        body: vd.robe,
                        legs: vd.robe,
                        robe: true,
                        hair: Some(hair),
                        long_hair: e.look % 2 == 0,
                        ..HUM
                    },
                    V_BANKER => Hum {
                        body: vd.robe,
                        legs: 0x2a2030ff,
                        hair: Some(hair),
                        ..HUM
                    },
                    V_LIBRARIAN => Hum {
                        body: vd.robe,
                        legs: vd.robe,
                        robe: true,
                        hair: Some(0xb0aca4ff),
                        ..HUM
                    },
                    V_LORD => Hum {
                        body: vd.robe,
                        legs: vd.robe,
                        robe: true,
                        hair: Some(hair),
                        ..HUM
                    },
                    V_SMITH => Hum {
                        body: vd.robe,
                        legs: 0x3a3028ff,
                        hair: Some(hair),
                        weapon: 3,
                        skin: 0xd8a070ff,
                        ..HUM
                    },
                    _ => Hum {
                        body: vd.robe,
                        legs: 0x4a3a2aff,
                        hair: Some(hair),
                        long_hair: e.look % 3 == 0,
                        ..HUM
                    },
                };
                self.humanoid(c, e, sx, sy, z, 1.0, &h, wk, false);
            }
            EK::Mon(m) => {
                let d = &MONS[m];
                let s = d.size;
                let col = if flash {
                    mix(d.color, 0xc02010ff, 0.5)
                } else {
                    d.color
                };
                match d.id {
                    "rabbit" | "rat" | "wolf" | "horse" => {
                        self.quad(c, e, sx, sy, z * s, col, d.id, wk, t)
                    }
                    // until their figures are baked
                    "tiger" | "dog" | "cow" | "pig" => {
                        self.quad(c, e, sx, sy, z * s, col, "wolf", wk, t)
                    }
                    "cat" | "pigeon" | "monkey" => {
                        self.quad(c, e, sx, sy, z * s, col, "rabbit", wk, t)
                    }
                    "crab" => self.slime(c, e, sx, sy, z * s, col, t),
                    "mongbat" => self.mongbat(c, e, sx, sy, z * s, col, t),
                    "slime" | "spider" => self.slime(c, e, sx, sy, z * s, col, t),
                    "dragon" => self.dragon(c, e, sx, sy, z * s, col, wk, t),
                    "skeleton" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: col,
                            legs: col,
                            skull: true,
                            weapon: 1,
                            shield: Some(0x6a5a40ff),
                            eyes: Some(0x000000ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "zombie" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x5a4a3aff,
                            legs: 0x3a3a30ff,
                            hair: Some(0x2a2a20ff),
                            eyes: Some(0xd0ff60ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "orc" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x6a4a2aff,
                            legs: 0x3a2a1aff,
                            weapon: 2,
                            eyes: Some(0xff3020ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "orclord" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x8a8f96ff,
                            legs: 0x3a2a1aff,
                            weapon: 2,
                            helm: Some(0x6a6058ff),
                            shield: Some(0x5a3a20ff),
                            eyes: Some(0xff3020ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "troll" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x5a4a30ff,
                            legs: col,
                            weapon: 6,
                            eyes: Some(0xffe040ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "ettin" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x6a5038ff,
                            legs: col,
                            weapon: 6,
                            heads: 2,
                            eyes: Some(0x000000ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "lich" => {
                        c.fill(with_a(0x80ff80ff, 0.12 + (t * 3.0).sin() * 0.05));
                        c.ell(sx, sy - 24.0 * z * s, 22.0 * z * s, 30.0 * z * s);
                        self.humanoid(
                            c,
                            e,
                            sx,
                            sy - 3.0 * z,
                            z,
                            s,
                            &Hum {
                                skin: 0xe8e2cfff,
                                body: col,
                                legs: col,
                                robe: true,
                                skull: true,
                                weapon: 4,
                                eyes: Some(0x60ff60ff),
                                hat: Some(0x3a2a5aff),
                                ..HUM
                            },
                            wk,
                            flash,
                        )
                    }
                    _ => self.humanoid(c, e, sx, sy, z, s, &Hum { body: col, ..HUM }, wk, flash),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn humanoid(
        &self,
        c: &mut Canvas,
        e: &Ent,
        sx: f32,
        sy: f32,
        z: f32,
        scale: f32,
        h: &Hum,
        wk: f32,
        flash: bool,
    ) {
        let t = self.clock;
        let ink = self.pix.max(z);
        let moving = !e.path.is_empty();
        let recoil = e.anim.hurt * e.anim.hurt * 3.0;
        let sx = sx - e.anim.face * recoil * z;
        let breathe = if moving {
            0.0
        } else {
            (t * 2.2 + e.id as f32 * 0.7).sin() * 0.7
        };
        let bob = if moving {
            -(e.anim.walk.sin()).abs() * 1.8
        } else {
            0.0
        };
        let up = bob + breathe * 0.5;
        let p = P {
            x: sx,
            y: sy,
            s: z * scale * 1.12,
            f: e.anim.face,
        };
        let s = p.s;
        let tint = |col: u32| {
            if flash {
                mix(col, 0xc02010ff, 0.5)
            } else {
                col
            }
        };
        let casting = e.k == EK::Player && matches!(self.pl.action, Action::Cast { .. });

        // ---- arm & weapon kinematics (model units, facing right)
        let shoulder = (6.5, -27.0 + up);
        let rest = (8.5 - wk * 3.0, -17.0 + up);
        let windup = (1.5, -39.0 + up);
        let strike = (15.0, -12.0 + up);
        let lerp =
            |a: (f32, f32), b: (f32, f32), k: f32| (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
        let hand_at = |ph: f32| {
            if ph < 0.3 {
                lerp(rest, windup, ph / 0.3)
            } else if ph < 0.62 {
                lerp(windup, strike, (ph - 0.3) / 0.32)
            } else {
                lerp(strike, rest, (ph - 0.62) / 0.38)
            }
        };
        let blade_len = match h.weapon {
            4 => 22.0,
            5 => 28.0,
            _ => 15.0,
        };
        let tip_of = |hand: (f32, f32)| {
            let (dx, dy) = (
                (hand.0 - shoulder.0) * 0.6 + 0.8,
                (hand.1 - shoulder.1) * 0.6 - 0.6,
            );
            let l = (dx * dx + dy * dy).sqrt().max(0.01);
            (hand.0 + dx / l * blade_len, hand.1 + dy / l * blade_len)
        };
        let attacking = e.anim.atk > 0.0 && !casting;
        let phase = 1.0 - e.anim.atk;
        let hand = if casting {
            (7.0, -41.0 + up)
        } else if attacking {
            hand_at(phase)
        } else {
            rest
        };
        let back_hand = if casting {
            (-7.0, -41.0 + up)
        } else {
            (-8.0 + wk * 3.0, -16.0 + up)
        };

        let draw_weapon_arm = |c: &mut Canvas| {
            let (a, b) = (p.at(shoulder.0, shoulder.1), p.at(hand.0, hand.1));
            c.ink_line(a, b, tint(h.skin), 2.7 * s);
            if h.weapon > 0 && !casting {
                let tip = tip_of(hand);
                let wcol = match h.weapon {
                    4 | 6 => 0x5a3e22ff,
                    5 => 0x7a7e84ff,
                    _ => 0xc8ccd0ff,
                };
                let (a, b) = (p.at(hand.0, hand.1), p.at(tip.0, tip.1));
                c.ink_line(a, b, wcol, if h.weapon == 6 { 4.0 * s } else { 2.0 * s });
                match h.weapon {
                    2 | 5 => {
                        c.fill(0x9aa0a8ff);
                        c.ell(b.0, b.1, 4.0 * s, 3.0 * s);
                    }
                    3 => {
                        c.fill(0x4a5058ff);
                        c.rect(b.0 - 3.5 * s, b.1 - 3.0 * s, 7.0 * s, 6.0 * s);
                    }
                    4 => {
                        c.fill(0x60a0e0ff);
                        c.ell(b.0, b.1, 2.8 * s, 2.8 * s);
                    }
                    6 => {
                        c.fill(0x4a3018ff);
                        c.ell(b.0, b.1, 4.0 * s, 4.0 * s);
                    }
                    _ => {}
                }
            }
            if casting {
                for hp in [hand, back_hand] {
                    let (x, y) = p.at(hp.0, hp.1);
                    c.fill(with_a(0x80b0ffff, 0.5 + (t * 14.0).sin() * 0.25));
                    c.ell(x, y, 4.0 * s, 4.0 * s);
                }
            }
        };

        // shadow (no ink)
        c.outline(0, 0.0);
        c.fill(0x00000030);
        c.ell(sx, sy, 9.0 * s, 3.5 * s);
        // slash trail
        if attacking && h.weapon > 0 && phase > 0.3 && phase < 0.8 {
            let mut pts = vec![];
            let end = phase.min(0.62);
            for k in 0..7 {
                let ph = 0.3 + (end - 0.3) * k as f32 / 6.0;
                let tp = tip_of(hand_at(ph));
                pts.push(p.at(tp.0, tp.1));
            }
            c.stroke(
                with_a(0xe8e0d0ff, 0.45 * (1.0 - (phase - 0.3) / 0.5)),
                3.0 * s,
            );
            c.poly_s(&pts);
        }
        c.outline(INK, ink);

        if e.anim.back {
            draw_weapon_arm(c);
        }
        // legs / robe
        if h.robe {
            c.fill(tint(shade(h.body, 0.8)));
            c.poly(&[
                p.at(-7.0, -27.0 + up),
                p.at(7.0, -27.0 + up),
                p.at(9.5 + wk, -1.0),
                p.at(-9.5 + wk, -1.0),
            ]);
        } else {
            let (a, b) = (p.at(-3.0, -14.0 + up), p.at(-3.0 + wk * 4.5, -0.5));
            c.ink_line(a, b, tint(h.legs), 3.4 * s);
            let (a, b) = (p.at(3.0, -14.0 + up), p.at(3.0 - wk * 4.5, -0.5));
            c.ink_line(a, b, tint(h.legs), 3.4 * s);
        }
        // torso, lit from the upper left
        let torso = [
            p.at(-7.0, -13.0 + up),
            p.at(7.0, -13.0 + up),
            p.at(6.5, -29.0 + up),
            p.at(-6.5, -29.0 + up),
        ];
        c.fill(tint(h.body));
        c.poly(&torso);
        c.outline(0, 0.0);
        c.fill(0x00000048);
        c.poly(&[
            (sx + 1.0 * s, sy + (-13.0 + up) * s),
            (sx + 7.0 * s, sy + (-13.0 + up) * s),
            (sx + 6.5 * s, sy + (-29.0 + up) * s),
            (sx + 1.5 * s, sy + (-29.0 + up) * s),
        ]);
        if h.skull {
            c.stroke(0x00000080, 1.0 * s);
            for k in 0..3 {
                let yy = -25.0 + up + k as f32 * 4.0;
                let (a, b) = (p.at(-5.0, yy), p.at(5.0, yy));
                c.line(a.0, a.1, b.0, b.1);
            }
        }
        c.fill(shade(h.body, 0.5));
        let (a, b) = (p.at(-7.0, -15.0 + up), p.at(7.0, -15.0 + up));
        c.poly(&[a, b, (b.0, b.1 + 2.0 * s), (a.0, a.1 + 2.0 * s)]);
        c.outline(INK, ink);
        // shield arm
        let sh = p.at(-6.5, -27.0 + up);
        c.ink_line(sh, p.at(back_hand.0, back_hand.1), tint(h.skin), 2.7 * s);
        if let Some(scol) = h.shield {
            if !casting {
                c.fill(scol);
                let (x, y) = p.at(-9.0, -19.0 + up);
                c.ell(x, y, 4.5 * s, 7.0 * s);
                c.fill(shade(scol, 1.4));
                c.ell(x, y, 1.5 * s, 2.0 * s);
            }
        }
        if !e.anim.back {
            draw_weapon_arm(c);
        }
        // heads
        let offs: &[f32] = if h.heads == 2 { &[-4.5, 4.5] } else { &[0.0] };
        for &ox in offs {
            let (hx, hy) = p.at(ox, -34.0 + up);
            if h.long_hair {
                if let Some(hc) = h.hair {
                    c.fill(hc);
                    c.poly(&[
                        p.at(ox - 5.5, -36.0 + up),
                        p.at(ox + 5.5, -36.0 + up),
                        p.at(ox + 5.0, -26.0 + up),
                        p.at(ox - 6.0, -25.0 + up),
                    ]);
                }
            }
            c.fill(tint(h.skin));
            c.ell(hx, hy, 5.2 * s, 5.4 * s);
            c.outline(0, 0.0);
            c.fill(0x00000040);
            c.ell(hx + 2.0 * s, hy + 1.0 * s, 3.0 * s, 4.0 * s);
            if !e.anim.back {
                if let Some(ec) = h.eyes {
                    c.fill(ec);
                    let (ex, ey) = p.at(ox + 2.5, -34.5 + up);
                    c.ell(ex, ey, 1.1 * s, 1.1 * s);
                    let (ex, ey) = p.at(ox - 0.5, -34.5 + up);
                    c.ell(ex, ey, 1.1 * s, 1.1 * s);
                }
            }
            c.outline(INK, ink);
            if let Some(hc) = h.hair {
                c.fill(hc);
                if e.anim.back {
                    c.ell(hx, hy - 0.5 * s, 5.5 * s, 5.2 * s);
                } else {
                    let (x, y) = p.at(ox - 0.5, -37.5 + up);
                    c.ell(x, y, 5.6 * s, 3.2 * s);
                }
            }
            if let Some(hc) = h.helm {
                c.fill(hc);
                let (x, y) = p.at(ox, -36.5 + up);
                c.ell(x, y, 6.0 * s, 4.5 * s);
                if !e.anim.back {
                    c.stroke(shade(hc, 0.5), 1.2 * s);
                    let (a, b) = (p.at(ox + 2.5, -37.0 + up), p.at(ox + 2.5, -32.0 + up));
                    c.line(a.0, a.1, b.0, b.1);
                }
            }
            if let Some(hc) = h.hat {
                c.fill(hc);
                c.poly(&[
                    p.at(ox - 8.0, -37.0 + up),
                    p.at(ox + 8.0, -37.0 + up),
                    p.at(ox - 2.0, -52.0 + up),
                ]);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn quad(
        &self,
        c: &mut Canvas,
        e: &Ent,
        sx: f32,
        sy: f32,
        s: f32,
        col: u32,
        kind: &str,
        wk: f32,
        _t: f32,
    ) {
        let p = P {
            x: sx,
            y: sy,
            s,
            f: e.anim.face,
        };
        c.fill(0x00000050);
        c.ell(sx, sy, 13.0 * s, 5.0 * s);
        let leg = shade(col, 0.7);
        c.stroke(INK, 3.6 * s);
        for (lx, ph) in [(-7.0, 1.0), (-4.0, -1.0), (5.0, -1.0), (8.0, 1.0)] {
            let (a, b) = (p.at(lx, -6.0), p.at(lx + wk * ph * 3.0, 0.0));
            c.line(a.0, a.1, b.0, b.1);
        }
        c.stroke(leg, 2.2 * s);
        for (lx, ph) in [(-7.0, 1.0), (-4.0, -1.0), (5.0, -1.0), (8.0, 1.0)] {
            let (a, b) = (p.at(lx, -6.0), p.at(lx + wk * ph * 3.0, 0.0));
            c.line(a.0, a.1, b.0, b.1);
        }
        let bob = if kind == "rabbit" {
            wk.abs() * -3.0
        } else {
            0.0
        };
        // tail
        c.stroke(col, if kind == "rat" { 1.4 * s } else { 2.6 * s });
        let (a, b) = (
            p.at(-10.0, -9.0 + bob),
            p.at(
                if kind == "rat" { -20.0 } else { -16.0 },
                if kind == "wolf" { -13.0 } else { -5.0 },
            ),
        );
        if kind == "rabbit" {
            c.fill(0xf0f0f0ff);
            c.ell(a.0, a.1, 3.0 * s, 3.0 * s);
        } else {
            c.line(a.0, a.1, b.0, b.1);
        }
        c.fill(col);
        let (bx, by) = p.at(0.0, -9.0 + bob);
        c.ell(bx, by, 11.0 * s, 5.5 * s);
        c.fill(shade(col, 1.15));
        let (hx, hy) = p.at(10.0, -12.0 + bob);
        c.ell(hx, hy, 4.5 * s, 4.0 * s);
        let (nx, ny) = p.at(14.0, -11.0 + bob);
        c.ell(nx, ny, 3.0 * s, 2.0 * s);
        c.fill(0x101010ff);
        let (ex, ey) = p.at(11.5, -13.5 + bob);
        c.ell(ex, ey, 0.9 * s, 0.9 * s);
        c.fill(shade(col, 0.9));
        match kind {
            "rabbit" => {
                let (ax, ay) = p.at(8.5, -19.0 + bob);
                c.ell(ax, ay, 1.5 * s, 5.0 * s);
                let (ax, ay) = p.at(10.5, -19.5 + bob);
                c.ell(ax, ay, 1.5 * s, 5.0 * s);
            }
            "wolf" => {
                c.poly(&[
                    p.at(7.0, -15.0 + bob),
                    p.at(9.0, -20.0 + bob),
                    p.at(11.0, -15.0 + bob),
                ]);
            }
            _ => {
                let (ax, ay) = p.at(9.0, -16.0 + bob);
                c.ell(ax, ay, 2.0 * s, 2.0 * s);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn mongbat(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, s: f32, col: u32, t: f32) {
        let p = P {
            x: sx,
            y: sy,
            s,
            f: e.anim.face,
        };
        let bob = (t * 5.0 + e.id as f32).sin() * 3.0;
        let flap = (t * 16.0 + e.id as f32).sin();
        c.fill(0x00000040);
        c.ell(sx, sy, 9.0 * s, 3.5 * s);
        c.fill(shade(col, 0.8));
        c.poly(&[
            p.at(-2.0, -24.0 + bob),
            p.at(-20.0, -32.0 + bob + flap * 9.0),
            p.at(-12.0, -20.0 + bob),
            p.at(-6.0, -18.0 + bob),
        ]);
        c.poly(&[
            p.at(2.0, -24.0 + bob),
            p.at(20.0, -32.0 + bob + flap * 9.0),
            p.at(12.0, -20.0 + bob),
            p.at(6.0, -18.0 + bob),
        ]);
        c.fill(col);
        let (x, y) = p.at(0.0, -22.0 + bob);
        c.ell(x, y, 6.0 * s, 7.0 * s);
        let (x, y) = p.at(2.0, -30.0 + bob);
        c.ell(x, y, 4.5 * s, 4.0 * s);
        c.poly(&[
            p.at(-1.0, -32.0 + bob),
            p.at(0.0, -38.0 + bob),
            p.at(2.0, -33.0 + bob),
        ]);
        c.poly(&[
            p.at(3.0, -33.0 + bob),
            p.at(5.0, -38.0 + bob),
            p.at(6.0, -32.0 + bob),
        ]);
        c.fill(0xffe040ff);
        let (x, y) = p.at(4.0, -30.0 + bob);
        c.ell(x, y, 1.0 * s, 1.0 * s);
    }

    #[allow(clippy::too_many_arguments)]
    fn slime(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, s: f32, col: u32, t: f32) {
        let q = (t * 4.0 + e.id as f32).sin() * 0.12;
        c.fill(0x00000040);
        c.ell(sx, sy, 12.0 * s, 4.5 * s);
        c.fill(with_a(col, 0.9));
        c.ell(
            sx,
            sy - 6.0 * s * (1.0 - q),
            11.0 * s * (1.0 + q),
            8.0 * s * (1.0 - q),
        );
        c.fill(0xffffff90);
        c.ell(sx - 4.0 * s, sy - 10.0 * s, 3.0 * s, 2.0 * s);
        c.fill(0x102010ff);
        c.ell(sx + e.anim.face * 3.0 * s, sy - 8.0 * s, 1.2 * s, 1.6 * s);
        c.ell(sx + e.anim.face * 6.5 * s, sy - 8.0 * s, 1.2 * s, 1.6 * s);
    }

    #[allow(clippy::too_many_arguments)]
    fn dragon(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, s: f32, col: u32, wk: f32, t: f32) {
        let p = P {
            x: sx,
            y: sy,
            s,
            f: e.anim.face,
        };
        let flap = (t * 2.5).sin();
        c.fill(0x00000060);
        c.ell(sx, sy, 22.0 * s, 8.0 * s);
        // far wing
        c.fill(shade(col, 0.6));
        c.poly(&[
            p.at(-2.0, -22.0),
            p.at(-12.0, -46.0 - flap * 6.0),
            p.at(-26.0, -40.0 - flap * 4.0),
            p.at(-14.0, -20.0),
        ]);
        // tail
        c.fill(shade(col, 0.85));
        c.poly(&[
            p.at(-10.0, -16.0),
            p.at(-28.0, -6.0),
            p.at(-34.0, -10.0),
            p.at(-10.0, -10.0),
        ]);
        // legs
        c.stroke(shade(col, 0.7), 4.0 * s);
        for (lx, ph) in [(-8.0, 1.0), (-4.0, -1.0), (6.0, -1.0), (9.0, 1.0)] {
            let (a, b) = (p.at(lx, -10.0), p.at(lx + wk * ph * 2.0, 0.0));
            c.line(a.0, a.1, b.0, b.1);
        }
        // body
        c.fill(col);
        let (bx, by) = p.at(0.0, -15.0);
        c.ell(bx, by, 14.0 * s, 8.0 * s);
        c.fill(0xe0a060ff);
        let (bx, by) = p.at(2.0, -11.0);
        c.ell(bx, by, 9.0 * s, 3.5 * s);
        // neck & head
        c.fill(col);
        c.poly(&[
            p.at(6.0, -20.0),
            p.at(10.0, -12.0),
            p.at(16.0, -28.0),
            p.at(12.0, -32.0),
        ]);
        let (hx, hy) = p.at(16.0, -32.0);
        c.ell(hx, hy, 6.0 * s, 4.0 * s);
        let (hx, hy) = p.at(21.0, -31.0);
        c.ell(hx, hy, 4.0 * s, 2.5 * s);
        c.stroke(0xe8e0c8ff, 1.5 * s);
        let (a, b) = (p.at(14.0, -35.0), p.at(10.0, -41.0));
        c.line(a.0, a.1, b.0, b.1);
        let (a, b) = (p.at(17.0, -35.5), p.at(15.0, -42.0));
        c.line(a.0, a.1, b.0, b.1);
        c.fill(0xffe040ff);
        let (ex, ey) = p.at(17.5, -33.0);
        c.ell(ex, ey, 1.2 * s, 1.0 * s);
        // near wing
        c.fill(shade(col, 0.9));
        c.poly(&[
            p.at(0.0, -22.0),
            p.at(4.0, -50.0 - flap * 7.0),
            p.at(-16.0, -44.0 - flap * 5.0),
            p.at(-8.0, -20.0),
        ]);
        c.stroke(shade(col, 0.5), 1.0 * s);
        let (a, b) = (p.at(0.0, -22.0), p.at(-6.0, -46.0 - flap * 6.0));
        c.line(a.0, a.1, b.0, b.1);
    }

    fn effect(&self, c: &mut Canvas, f: &Fx, z: f32, t: f32) {
        let k = (f.t / f.dur).clamp(0.0, 1.0);
        let (x0, y0) = self.scr_h(
            f.x0,
            f.y0,
            self.map.surface(f.x0.round() as i32, f.y0.round() as i32),
        );
        let (x1, y1) = self.scr_h(
            f.x1,
            f.y1,
            self.map.surface(f.x1.round() as i32, f.y1.round() as i32),
        );
        let (y0, y1) = (y0 - 26.0 * ZS * z, y1 - 24.0 * ZS * z);
        let lerp = |k: f32| (x0 + (x1 - x0) * k, y0 + (y1 - y0) * k);
        // baked frame animations for the big spells
        if let Some(pid) = self.sprites.props {
            let (ax, ay) = bake::PROP_ANCHOR;
            let fr = ((k * bake::FX_FRAMES as f32) as usize).min(bake::FX_FRAMES - 1);
            let pick = match f.kind {
                FX_EXPLOSION => Some((bake::P_FX_EXPLO + fr, x1, y1 + 24.0 * ZS * z, 0xff8030ff)),
                FX_FLAMESTRIKE => Some((bake::P_FX_FLAME + fr, x1, y1 + 24.0 * ZS * z, 0xff6020ff)),
                FX_HEAL | FX_RES => {
                    Some((bake::P_FX_HEAL + fr, x0, y0 + 26.0 * ZS * z, 0xffe090ff))
                }
                FX_FIREBALL => {
                    let (x, y) = lerp(k);
                    Some((
                        bake::P_FX_BALL + ((t * 14.0) as usize % 4),
                        x,
                        y + 12.0 * z,
                        0xff8030ff,
                    ))
                }
                _ => None,
            };
            if let Some((frame, x, y, glow)) = pick {
                c.outline(0, 0.0);
                c.alpha(0.45 * (1.0 - k * 0.7));
                c.glow(x, y - 20.0 * z, 34.0 * z, glow);
                c.alpha(if k > 0.85 { (1.0 - k) / 0.15 } else { 1.0 });
                c.sprite(pid, frame, bake::PROP_FW, bake::PROP_FH, ax, ay, x, y, z, 0);
                c.alpha(1.0);
                if f.kind == FX_RES {
                    c.fill(with_a(0xfff0b0ff, 0.3 * (1.0 - k)));
                    c.rect(x0 - 14.0 * z, y0 - 200.0 * z, 28.0 * z, 230.0 * z);
                }
                return;
            }
        }
        match f.kind {
            FX_ARROW => {
                let (a, b) = (lerp((k - 0.15).max(0.0)), lerp(k));
                c.stroke(0xff4040ff, 3.0 * z);
                c.line(a.0, a.1, b.0, b.1);
                c.fill(0xffc0c0ff);
                c.ell(b.0, b.1, 2.5 * z, 2.5 * z);
            }
            FX_FIREBALL | FX_BREATH => {
                let n = if f.kind == FX_BREATH { 6 } else { 3 };
                for i in 0..n {
                    let kk = (k - i as f32 * 0.07).max(0.0);
                    let (x, y) = lerp(kk);
                    let j = if f.kind == FX_BREATH {
                        ((i as f32 * 2.3 + t * 9.0).sin()) * 8.0 * z
                    } else {
                        0.0
                    };
                    let r = (8.0 - i as f32 * 1.5).max(2.0) * z;
                    c.fill(0xff7020d0);
                    c.ell(x + j, y + j * 0.5, r, r);
                    c.fill(0xffe070ff);
                    c.ell(x + j, y + j * 0.5, r * 0.5, r * 0.5);
                }
            }
            FX_EBOLT => {
                let (x, y) = lerp(k);
                let (a, _) = (lerp((k - 0.2).max(0.0)), 0);
                c.stroke(0x6080ffa0, 6.0 * z);
                c.line(a.0, a.1, x, y);
                c.fill(0xa0c0ffff);
                c.ell(x, y, 7.0 * z, 7.0 * z);
                c.fill(0xffffffff);
                c.ell(x, y, 3.5 * z, 3.5 * z);
            }
            FX_LIGHTNING => {
                let mut pts = vec![];
                let top = y1 - 260.0 * z;
                for i in 0..=8 {
                    let yy = top + (y1 - top) * i as f32 / 8.0;
                    let j = if i == 0 || i == 8 {
                        0.0
                    } else {
                        ((i as f32 * 12.9898 + f.x1 * 7.0 + (t * 30.0).floor()).sin() * 43758.5)
                            .fract()
                            - 0.5
                    };
                    pts.push((x1 + j * 30.0 * z, yy));
                }
                c.stroke(0x8ab0ffc0, 6.0 * z);
                c.poly_s(&pts);
                c.stroke(0xffffffff, 2.0 * z);
                c.poly_s(&pts);
                c.fill(with_a(0xffffffff, 0.4 * (1.0 - k)));
                c.ell(x1, y1 + 20.0 * z, 20.0 * z, 9.0 * z);
            }
            FX_EXPLOSION => {
                let r = (10.0 + k * 34.0) * z;
                c.fill(with_a(0xff8030ff, 0.85 * (1.0 - k)));
                c.ell(x1, y1, r, r * 0.8);
                c.fill(with_a(0xffe080ff, 0.9 * (1.0 - k)));
                c.ell(x1, y1, r * 0.5, r * 0.4);
            }
            FX_FLAMESTRIKE => {
                for i in 0..6 {
                    let fi = i as f32;
                    let yy = y1 + 20.0 * z - fi * 12.0 * z * (0.4 + k);
                    let w = (12.0 - fi) * z * (1.0 - k * 0.5);
                    c.fill(if i % 2 == 0 {
                        with_a(0xff6020ff, 0.8 * (1.0 - k))
                    } else {
                        with_a(0xffd060ff, 0.8 * (1.0 - k))
                    });
                    c.ell(x1 + (fi * 1.7 + t * 10.0).sin() * 3.0 * z, yy, w, w * 0.9);
                }
            }
            FX_HARM => {
                c.fill(with_a(0xa040e0ff, 0.6 * (1.0 - k)));
                c.ell(x1, y1, (8.0 + k * 18.0) * z, (6.0 + k * 12.0) * z);
            }
            FX_HEAL | FX_RES | FX_SPARK => {
                let col = if f.kind == FX_RES {
                    0xffe080ff
                } else {
                    0x80ffb0ff
                };
                for i in 0..8 {
                    let a = i as f32 * 0.785 + k * 3.0;
                    let r = 12.0 * z;
                    let yy = y0 + 20.0 * z - k * 40.0 * z + (i as f32 * 1.3).sin() * 6.0 * z;
                    c.fill(with_a(col, 1.0 - k));
                    c.ell(x0 + a.cos() * r, yy, 2.2 * z, 2.2 * z);
                }
                if f.kind == FX_RES {
                    c.fill(with_a(0xfff0b0ff, 0.35 * (1.0 - k)));
                    c.rect(x0 - 14.0 * z, y0 - 200.0 * z, 28.0 * z, 230.0 * z);
                }
            }
            FX_BLOOD => {
                let (bx, by) = self.scr_h(
                    f.x0,
                    f.y0,
                    self.map.surface(f.x0.round() as i32, f.y0.round() as i32),
                );
                let col = f.tgt;
                for j in 0..8 {
                    let jf = j as f32;
                    let ang = jf * 0.8 + f.x0 * 3.0;
                    let (vx, vy) = (
                        ang.cos() * (10.0 + jf * 2.0),
                        -14.0 - (jf * 1.7).sin().abs() * 10.0,
                    );
                    let x = bx + vx * k * z;
                    let y = by - 22.0 * z + (vy * k + 46.0 * k * k) * z;
                    c.fill(with_a(col, 1.0 - k * 0.6));
                    let r = (2.2 - k) * z.max(0.8);
                    c.rect(x, y, r, r);
                }
            }
            FX_FIZZLE => {
                for i in 0..5 {
                    let fi = i as f32;
                    c.fill(with_a(0x909090ff, 0.6 * (1.0 - k)));
                    c.ell(
                        x0 + (fi - 2.0) * 5.0 * z,
                        y0 - k * 20.0 * z - fi * 2.0 * z,
                        (3.0 + k * 4.0) * z,
                        (3.0 + k * 4.0) * z,
                    );
                }
            }
            _ => {}
        }
    }
}

/// How tall a wall of this style stands, relative to a house wall.
fn wall_h(style: u8) -> f32 {
    match style & !FACE_S {
        WALL_CASTLE => 1.45,
        WALL_TOWER => 1.9,
        WALL_BURROW => 0.95,
        WALL_ELVEN => 1.15,
        WALL_FORT => 1.6,
        WALL_BASTION => 2.05,
        WALL_PALACE => 1.35,
        WALL_TEMPLE => 1.1,
        WALL_DWARF => 1.25,
        WALL_DROWNED => 0.7,
        WALL_DROWNED_LOW => 0.35,
        _ => 1.0,
    }
}

// ---------------------------------------------------------------- storeys and doors
impl Game {
    /// How many storeys of wall to draw on this tile: all of a building's
    /// from outside, up to the player's own floor from within.
    fn wall_storeys(&self, x: i32, y: i32) -> u8 {
        let Some(b) = self.map.building_at(x, y) else {
            return 1;
        };
        match self.floor_view() {
            Some((pb, f)) if pb == b => f + 1,
            _ => self.map.buildings[b].storeys(),
        }
    }

    /// A shut-able wall face from `base` up `hgt` pixels on this tile: the
    /// storeys over a doorway.
    #[allow(clippy::too_many_arguments)]
    fn wall_box(&self, c: &mut Canvas, sx: f32, sy: f32, hgt: f32, style: u8, z: f32) {
        let (hw, hh) = (32.0 * z + 0.6, 16.0 * z + 0.6);
        let tex = wall_tex(style);
        let left = [
            (sx - hw, sy),
            (sx, sy + hh),
            (sx, sy + hh - hgt),
            (sx - hw, sy - hgt),
        ];
        let right = [
            (sx, sy + hh),
            (sx + hw, sy),
            (sx + hw, sy - hgt),
            (sx, sy + hh - hgt),
        ];
        let top = [
            (sx, sy - hh - hgt),
            (sx + hw, sy - hgt),
            (sx, sy + hh - hgt),
            (sx - hw, sy - hgt),
        ];
        c.outline(INK, self.pix.max(z));
        self.face_pat(c, tex, false);
        c.poly(&left);
        self.face_pat(c, tex, true);
        c.poly(&right);
        c.outline(0, 0.0);
        c.fill(0x00000068);
        c.poly(&left);
        c.fill(0x00000028);
        c.poly(&right);
        c.outline(INK, self.pix.max(z));
        self.pat(c, tex, hgt);
        c.poly(&top);
        c.outline(0, 0.0);
        c.fill(0x0c080660);
        c.poly(&top);
        // the beam the upper storey rests on
        c.stroke(0x2a1a10ff, 1.5 * self.pix.max(z));
        c.line(sx - hw, sy, sx, sy + hh);
        c.line(sx, sy + hh, sx + hw, sy);
    }

    /// The upper storeys of wall over a doorway, when they are drawn.
    #[allow(clippy::too_many_arguments)]
    fn over_door(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        z: f32,
        _t: f32,
        _lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        let st = self.wall_storeys(x, y);
        if st <= 1 || self.map.thin_at(x, y).is_some() {
            return;
        }
        let Some(b) = self.map.building_at(x, y) else {
            return;
        };
        let style = self.map.buildings[b].style;
        let base = STOREY_PX * z;
        self.wall_box(c, sx, sy - base, base * (st - 1) as f32, style, z);
        c.outline(0, 0.0);
    }

    /// An open door: its leaf swung in against the jamb.
    fn open_door(&self, c: &mut Canvas, x: i32, y: i32, z: f32) {
        let lz = LH * z;
        let base = self.map.level(x, y) as f32 * lz;
        let hinge = if self.map.obj(x - 1, y) != O_DOOR {
            x as f32 - 0.45
        } else {
            x as f32 + 0.45
        };
        let sp = |wx: f32, wy: f32, h: f32| {
            let (a, b) = self.to_screen(wx, wy);
            (a, b - base - h)
        };
        let (y0, y1) = (y as f32 + 0.35, y as f32 - 0.55);
        let h = 26.0 * z;
        c.outline(INK, self.pix.max(z));
        c.fill(0x3a2412ff);
        c.poly(&[
            sp(hinge, y0, 0.0),
            sp(hinge, y1, 0.0),
            sp(hinge, y1, h),
            sp(hinge, y0, h),
        ]);
        c.outline(0, 0.0);
        // planks
        c.stroke(0x24160aff, self.pix.max(z));
        for k in 1..3 {
            let f = k as f32 / 3.0;
            let yy = y0 + (y1 - y0) * f;
            let (a, b) = (sp(hinge, yy, 0.0), sp(hinge, yy, h));
            c.line(a.0, a.1, b.0, b.1);
        }
    }

    /// A flight of wooden stairs on a tile, from `base` (in levels) up a storey
    /// toward the back wall.
    fn stairs(&self, c: &mut Canvas, x: i32, y: i32, base: f32, z: f32) {
        let lz = LH * z;
        let sp = |wx: f32, wy: f32, h: f32| {
            let (a, b) = self.to_screen(wx, wy);
            (a, b - h * lz)
        };
        let n = 6;
        let (x0, x1) = (x as f32 - 0.42, x as f32 + 0.42);
        let ink = self.pix.max(z);
        // back to front, so each step hides the one behind it
        for k in (0..n).rev() {
            let ya = y as f32 + 0.5 - k as f32 / n as f32;
            let yb = ya - 1.0 / n as f32;
            let top = base + STOREY * (k + 1) as f32 / n as f32;
            c.outline(INK, ink);
            // its side, toward x+1
            c.fill(0x3a2616ff);
            c.poly(&[
                sp(x1, ya, base),
                sp(x1, yb, base),
                sp(x1, yb, top),
                sp(x1, ya, top),
            ]);
            // its riser, toward y+1
            c.fill(0x4e3420ff);
            c.poly(&[
                sp(x0, ya, base),
                sp(x1, ya, base),
                sp(x1, ya, top),
                sp(x0, ya, top),
            ]);
            // its tread
            c.fill(0x7a5634ff);
            c.poly(&[
                sp(x0, ya, top),
                sp(x1, ya, top),
                sp(x1, yb, top),
                sp(x0, yb, top),
            ]);
        }
        // a handrail up the open side
        c.outline(0, 0.0);
        c.stroke(0x2a1a0eff, 1.5 * ink);
        let (a, b) = (
            sp(x1, y as f32 + 0.5, base + 0.9),
            sp(x1, y as f32 - 0.5, base + STOREY + 0.9),
        );
        c.line(a.0, a.1, b.0, b.1);
    }

    /// An inside tile of a building the player is upstairs in: the storeys
    /// below as a solid block, then the floor and what stands on it.
    #[allow(clippy::too_many_arguments)]
    fn upper_tile(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        f: u8,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        let lz = LH * z;
        let (hw, hh) = (32.0 * z + 0.6, 16.0 * z + 0.6);
        let lvl = self.map.level(x, y) as f32;
        let e = lvl + f as f32 * STOREY;
        let (gx, gy) = self.to_screen(x as f32, y as f32);
        let (uo, us) = self.map.obj_on(x, y, f);
        let hole = uo == O_STAIRS && us & STAIR_DOWN != 0;
        let solid = if hole { e - STOREY } else { e };
        if solid > lvl {
            c.outline(0, 0.0);
            c.fill(0x23170fff);
            c.poly(&[
                (gx - hw, gy - lvl * lz),
                (gx, gy + hh - lvl * lz),
                (gx, gy + hh - solid * lz),
                (gx - hw, gy - solid * lz),
            ]);
            c.fill(0x2e1f14ff);
            c.poly(&[
                (gx, gy + hh - lvl * lz),
                (gx + hw, gy - lvl * lz),
                (gx + hw, gy - solid * lz),
                (gx, gy + hh - solid * lz),
            ]);
        }
        if hole {
            // the stairwell: the flight up from the floor below
            self.stairs(c, x, y, e - STOREY, z);
            return;
        }
        self.pat(c, tex_of(T_FLOOR), e * lz);
        c.poly(&self.diamond(gx, gy, &[e; 4]));
        c.fill(0x0000002a);
        c.poly(&self.diamond(gx, gy, &[e; 4]));
        if uo == O_STAIRS {
            self.stairs(c, x, y, e, z);
        } else if uo != O_NONE && uo != O_WALL && uo != O_PART {
            c.outline(INK, self.pix.max(z));
            let sh = self.map.shade[idx(x, y)];
            self.object(c, uo, x, y, gx, gy - e * lz, sh, us, z, t, lights);
            c.outline(0, 0.0);
        }
    }
}

// ---------------------------------------------------------------- the village
impl Game {
    /// A door drawn its own way: a gate in the village hedge, or the round
    /// door of a burrow house.
    fn odd_door(&self, x: i32, y: i32) -> bool {
        match self.map.building_at(x, y) {
            None => true,
            Some(b) => self.map.buildings[b].style == WALL_BURROW,
        }
    }
    fn odd_door_draw(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, z: f32) {
        if self.map.building_at(x, y).is_some() {
            self.burrow_face(c, x, y, sx, sy, z, true);
        } else {
            self.gate(c, x, y, sx, sy, z);
        }
    }

    /// One tile of a burrow's earthen front: a bank of earth faced with
    /// timber, turf on top; a round window now and then, or the round door.
    fn burrow_wall(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, z: f32) {
        self.burrow_face(c, x, y, sx, sy, z, false);
    }
    #[allow(clippy::too_many_arguments)]
    fn burrow_face(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, z: f32, door: bool) {
        let (hw, hh) = (32.0 * z + 0.6, 16.0 * z + 0.6);
        let hgt = STOREY_PX * z * wall_h(WALL_BURROW);
        let ink = self.pix.max(z);
        let left = [
            (sx - hw, sy),
            (sx, sy + hh),
            (sx, sy + hh - hgt),
            (sx - hw, sy - hgt),
        ];
        let right = [
            (sx, sy + hh),
            (sx + hw, sy),
            (sx + hw, sy - hgt),
            (sx, sy + hh - hgt),
        ];
        let top = [
            (sx, sy - hh - hgt),
            (sx + hw, sy - hgt),
            (sx, sy + hh - hgt),
            (sx - hw, sy - hgt),
        ];
        c.outline(INK, ink);
        c.fill(0x5a4430ff);
        c.poly(&left);
        c.fill(0x6a5038ff);
        c.poly(&right);
        // turf over the top
        c.fill(0x4e6e2cff);
        c.poly(&top);
        c.outline(0, 0.0);
        // a course of stones at the foot
        c.fill(0x6a645aff);
        c.poly(&[
            (sx - hw, sy),
            (sx, sy + hh),
            (sx, sy + hh - hgt * 0.22),
            (sx - hw, sy - hgt * 0.22),
        ]);
        // the face toward y+1 is the front of the house
        let front = self.map.obj(x, y + 1) != O_WALL
            && !self
                .map
                .buildings
                .iter()
                .any(|b| b.style == WALL_BURROW && b.interior(x, y + 1));
        let (fx, fy) = (sx - hw * 0.5, sy + hh * 0.5);
        if door {
            let r = 13.5 * z;
            let (cx, cy) = (fx, fy - r - 1.0 * z);
            c.outline(INK, ink);
            c.fill(0x8a6a3aff);
            c.ell(cx, cy, r + 2.0 * z, r + 2.0 * z);
            if self.map.door_shut(x, y) {
                const GREENS: [u32; 3] = [0x2f6a2aff, 0x3a5a7aff, 0x7a3a24ff];
                c.fill(GREENS[(x as usize + y as usize) % 3]);
                c.ell(cx, cy, r, r);
                c.outline(0, 0.0);
                c.fill(0xe0c050ff);
                c.ell(cx, cy, 2.0 * z, 2.0 * z);
            } else {
                c.fill(0x140c06ff);
                c.ell(cx, cy, r, r);
            }
            c.outline(0, 0.0);
        } else if front && (x + y) % 2 == 0 {
            let (cx, cy) = (fx, fy - hgt * 0.55);
            c.outline(INK, ink);
            c.fill(0x8a6a3aff);
            c.ell(cx, cy, 7.5 * z, 7.0 * z);
            c.fill(0x1a1410ff);
            c.ell(cx, cy, 5.5 * z, 5.0 * z);
            c.outline(0, 0.0);
            c.stroke(0x8a6a3aff, ink);
            c.line(cx - 5.5 * z, cy, cx + 5.5 * z, cy);
            c.line(cx, cy - 5.0 * z, cx, cy + 5.0 * z);
            // a warm glow within
            c.fill(0xffb04830);
            c.ell(cx, cy, 4.0 * z, 3.5 * z);
        }
    }

    /// A burrow's roof: a low turf mound grown over it, with a chimney pot.
    fn mound(&self, c: &mut Canvas, b: &Building, z: f32) {
        let (bw, bh) = (b.w - 1, b.h - 1);
        let floor = self.map.level(b.x + 1, b.y + 1) as f32 * LH * z;
        let eave = STOREY_PX * z * wall_h(WALL_BURROW);
        let rise = 26.0 * z + 5.0 * z * b.h as f32;
        let (x0, y0, x1, y1) = (
            b.x as f32 - 0.3,
            b.y as f32 - 0.3,
            (b.x + bw) as f32 + 0.3,
            (b.y + bh) as f32 + 0.3,
        );
        let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let sp = |x: f32, y: f32, h: f32| {
            let (a, b) = self.to_screen(x, y);
            (a, b - h - floor)
        };
        // rings from the eaves to the crown, each a little smaller and higher,
        // so the mound swells like a hill
        let ink = self.pix.max(z);
        let n = 9;
        let mut rings: Vec<[(f32, f32); 8]> = Vec::new();
        for k in 0..n {
            let f = k as f32 / n as f32;
            // a dome's profile: steep at the eaves, flat on the crown
            let s = (1.0 - f * f).sqrt() * 0.92 + 0.08 * (1.0 - f);
            let h = eave + rise * f;
            let (ax, ay, bx2, by2) = (
                mx + (x0 - mx) * s,
                my + (y0 - my) * s,
                mx + (x1 - mx) * s,
                my + (y1 - my) * s,
            );
            // rounded corners: an octagon round the rectangle
            let cut = 0.35 * s;
            let pts = [
                sp(ax + cut, ay, h),
                sp(bx2 - cut, ay, h),
                sp(bx2, ay + cut, h),
                sp(bx2, by2 - cut, h),
                sp(bx2 - cut, by2, h),
                sp(ax + cut, by2, h),
                sp(ax, by2 - cut, h),
                sp(ax, ay + cut, h),
            ];
            if k == 0 {
                c.outline(INK, ink);
            } else {
                c.outline(0, 0.0);
            }
            c.fill(shade(0x456a28ff, 0.7 + f * 0.55));
            c.poly(&pts);
            rings.push(pts);
        }
        c.outline(0, 0.0);
        // the light: the slope toward x+1 catches it, the one toward y+1 is
        // in shade, as on every wall
        for k in 0..n - 1 {
            let (r, q) = (&rings[k], &rings[k + 1]);
            let fade = 1.0 - k as f32 / n as f32;
            c.fill(with_a(0x0a1408ff, 0.3 * fade));
            c.poly(&[r[4], r[5], r[6], q[6], q[5], q[4]]);
            c.fill(with_a(0xe0f0a0ff, 0.12 * fade));
            c.poly(&[r[1], r[2], r[3], q[3], q[2], q[1]]);
        }
        // a bright crown
        let (cx, cy) = sp(mx + 0.3, my - 0.3, eave + rise);
        c.fill(0xc0e08030);
        c.ell(cx, cy, 26.0 * z, 12.0 * z);
        // hummocks of lighter and darker turf
        for k in 0..((b.w * b.h) / 2) {
            let hx = h2(k, b.x * 3 + b.y, 47);
            let hy = h2(k, b.y * 3 + b.x, 49);
            let (tx, ty) = (
                x0 + 0.6 + hx * (x1 - x0 - 1.2),
                y0 + 0.6 + hy * (y1 - y0 - 1.2),
            );
            let d =
                (((tx - mx) / (x1 - x0)).powi(2) + ((ty - my) / (y1 - y0)).powi(2)).sqrt() * 2.0;
            let h = eave + rise * (1.0 - d.min(1.0).powi(2)).max(0.0).sqrt();
            let (a, bb) = sp(tx, ty, h);
            c.fill(if k % 2 == 0 { 0x5e8236a0 } else { 0x34521c90 });
            c.ell(a, bb, 7.0 * z, 3.5 * z);
        }
        // tufts and flowers in the turf
        for k in 0..((b.w * b.h) as u32) {
            let hx = h2(k as i32, b.x * 7 + b.y, 41);
            let hy = h2(k as i32, b.y * 5 + b.x, 43);
            let (tx, ty) = (
                x0 + 0.5 + hx * (x1 - x0 - 1.0),
                y0 + 0.5 + hy * (y1 - y0 - 1.0),
            );
            let d =
                (((tx - mx) / (x1 - x0)).powi(2) + ((ty - my) / (y1 - y0)).powi(2)).sqrt() * 2.0;
            let h = eave + rise * (1.0 - d.min(1.0).powi(2)).max(0.0).sqrt();
            let (a, bb) = sp(tx, ty, h);
            if k % 5 == 0 {
                c.fill([0xe8d070ff, 0xd8a0c0ff, 0xf0f0e0ff][(k / 5 % 3) as usize]);
                c.ell(a, bb, 1.5 * z, 1.2 * z);
            } else {
                c.stroke(0x3a5a20ff, ink);
                c.line(a, bb, a + 1.0 * z, bb - 3.0 * z);
            }
        }
        // the chimney pot, smoking
        let (cx, cy) = sp(mx + 0.8, my - 0.6, eave + rise * 0.9);
        c.outline(INK, ink);
        c.fill(0x7a5a44ff);
        c.rect(cx - 3.0 * z, cy - 12.0 * z, 6.0 * z, 12.0 * z);
        c.outline(0, 0.0);
        let t = self.clock;
        for k in 0..3 {
            let f = (t * 0.4 + k as f32 / 3.0) % 1.0;
            c.fill(with_a(0xb0aaa0ff, 0.35 * (1.0 - f)));
            c.ell(
                cx + f * 8.0 * z,
                cy - 14.0 * z - f * 26.0 * z,
                (3.0 + f * 5.0) * z,
                (2.5 + f * 4.0) * z,
            );
        }
    }

    /// One tile of a market stall: a counter with a cloth skirt and its
    /// wares on top, poles at the stall's ends and a striped awning with a
    /// scalloped valance. After dark the wares are covered and a lantern is lit.
    #[allow(clippy::too_many_arguments)]
    fn stall(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        style: u8,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        let kind = style as usize % 6;
        let ink = self.pix.max(z);
        // a point on the tile: u along x, v along y (both -0.5..0.5), h up
        let at =
            |u: f32, v: f32, h: f32| (sx + (u - v) * 32.0 * z, sy + (u + v) * 16.0 * z - h * z);
        let quad = |c: &mut Canvas, p: [(f32, f32); 4]| c.poly(&p);
        let left_end = self.map.obj(x - 1, y) != O_STALL;
        let right_end = self.map.obj(x + 1, y) != O_STALL;
        let (awn_a, awn_b, skirt) = [
            (0x3a6a2aff, 0xe0d8b8ff, 0x2e4a22ff),
            (0xa02a22ff, 0xe8dcc0ff, 0x6a2018ff),
            (0x2a5a8aff, 0xe8ecf0ff, 0x203a5aff),
            (0x5a2a6aff, 0xd8b048ff, 0x3a1a48ff),
            (0xb07a2aff, 0x5a3a1eff, 0x6a4a1aff),
            (0x2a7a72ff, 0xe8f0e8ff, 0x1a4a46ff),
        ][kind];
        let open = self.market_open();
        let (v0, v1, top) = (-0.12f32, 0.3f32, 13.0f32);
        let (u0, u1) = (
            if left_end { -0.46 } else { -0.5 },
            if right_end { 0.46 } else { 0.5 },
        );
        // the back poles, behind the counter
        c.outline(INK, ink);
        let pole = |c: &mut Canvas, u: f32, v: f32, h: f32| {
            let (a, b) = (at(u, v, 0.0), at(u, v, h));
            c.fill(0x4a3220ff);
            c.rect(a.0 - 1.5 * z, b.1, 3.0 * z, a.1 - b.1);
        };
        if left_end {
            pole(c, -0.46, -0.42, 46.0);
        }
        if right_end {
            pole(c, 0.46, -0.42, 46.0);
        }
        // the counter
        c.fill(0x5a3c22ff);
        quad(
            c,
            [
                at(u0, v1, 0.0),
                at(u1, v1, 0.0),
                at(u1, v1, top),
                at(u0, v1, top),
            ],
        );
        if right_end {
            c.fill(0x6e4c2cff);
            quad(
                c,
                [
                    at(u1, v1, 0.0),
                    at(u1, v0, 0.0),
                    at(u1, v0, top),
                    at(u1, v1, top),
                ],
            );
        }
        c.fill(0x8a6a42ff);
        quad(
            c,
            [
                at(u0, v0, top),
                at(u1, v0, top),
                at(u1, v1, top),
                at(u0, v1, top),
            ],
        );
        // the cloth hung over the front, in the stall's colour
        c.outline(0, 0.0);
        c.fill(skirt);
        quad(
            c,
            [
                at(u0, v1 + 0.01, top),
                at(u1, v1 + 0.01, top),
                at(u1, v1 + 0.01, top - 8.0),
                at(u0, v1 + 0.01, top - 8.0),
            ],
        );
        c.fill(with_a(awn_b, 0.5));
        for k in 0..3 {
            let u = u0 + (u1 - u0) * (k as f32 + 0.5) / 3.0;
            let (a, b) = (
                at(u - 0.05, v1 + 0.01, top - 8.0),
                at(u + 0.05, v1 + 0.01, top - 8.0),
            );
            c.poly(&[a, b, ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0 + 3.0 * z)]);
        }
        // the wares
        let hv = |k: i32, s: i32| h2(x * 7 + k, y * 3 + k * 5, s);
        if !open {
            // covered for the night
            c.outline(INK, ink);
            c.fill(0x5a5650ff);
            quad(
                c,
                [
                    at(u0 + 0.04, v0 + 0.02, top),
                    at(u1 - 0.04, v0 + 0.02, top),
                    at(u1 - 0.04, v1 - 0.02, top + 4.0),
                    at(u0 + 0.04, v1 - 0.02, top + 4.0),
                ],
            );
            c.outline(0, 0.0);
        } else {
            match kind {
                0 => {
                    // bundles of herbs, bulbs of garlic, gnarled roots
                    for k in 0..7 {
                        let (u, v) = (-0.38 + k as f32 * 0.12, -0.02 + hv(k, 1) * 0.22);
                        let p = at(u, v, top + 2.0);
                        match k % 3 {
                            0 => {
                                c.fill(0x3a6a2aff);
                                c.ell(p.0, p.1 - 1.0 * z, 3.5 * z, 2.5 * z);
                                c.fill(0x5a8a3aff);
                                c.ell(p.0 - 1.0 * z, p.1 - 2.0 * z, 2.0 * z, 1.5 * z);
                            }
                            1 => {
                                c.fill(0xe8e0d0ff);
                                c.ell(p.0, p.1, 2.4 * z, 2.0 * z);
                                c.fill(0xffffffff);
                                c.ell(p.0 - 0.8 * z, p.1 - 0.8 * z, 0.8 * z, 0.8 * z);
                            }
                            _ => {
                                c.fill(0x8a5a32ff);
                                c.ell(p.0, p.1, 3.0 * z, 1.4 * z);
                            }
                        }
                    }
                }
                1 => {
                    // a basket of loaves
                    for k in 0..5 {
                        let (u, v) = (-0.34 + k as f32 * 0.17, 0.02 + hv(k, 2) * 0.16);
                        let p = at(u, v, top + 2.5);
                        c.fill(0x9a6024ff);
                        c.ell(p.0, p.1, 4.5 * z, 2.6 * z);
                        c.fill(0xd09a4cff);
                        c.ell(p.0 - 0.5 * z, p.1 - 1.0 * z, 3.4 * z, 1.6 * z);
                        c.stroke(0x7a4a1aff, ink);
                        c.line(p.0 - 1.5 * z, p.1 - 1.5 * z, p.0 + 0.5 * z, p.1 - 0.2 * z);
                    }
                }
                2 => {
                    // fish laid out on straw
                    c.fill(0xc8b060ff);
                    quad(
                        c,
                        [
                            at(u0 + 0.06, v0 + 0.04, top + 0.5),
                            at(u1 - 0.06, v0 + 0.04, top + 0.5),
                            at(u1 - 0.06, v1 - 0.04, top + 0.5),
                            at(u0 + 0.06, v1 - 0.04, top + 0.5),
                        ],
                    );
                    for k in 0..6 {
                        let (u, v) = (-0.36 + k as f32 * 0.14, 0.0 + hv(k, 3) * 0.2);
                        let p = at(u, v, top + 1.5);
                        c.fill(0x6a8a9aff);
                        c.ell(p.0, p.1, 4.0 * z, 1.4 * z);
                        c.fill(0xc0d0d8ff);
                        c.ell(p.0 - 0.5 * z, p.1 - 0.5 * z, 2.5 * z, 0.6 * z);
                        c.fill(0x4a6a7aff);
                        c.poly(&[
                            (p.0 + 3.5 * z, p.1),
                            (p.0 + 6.0 * z, p.1 - 1.6 * z),
                            (p.0 + 6.0 * z, p.1 + 1.6 * z),
                        ]);
                    }
                }
                3 => {
                    // bolts of cloth and a folded hide
                    for k in 0..4 {
                        let (u, v) = (-0.33 + k as f32 * 0.22, 0.08);
                        let p = at(u, v, top + 3.0);
                        let col =
                            [0x8a2a2aff, 0x2a4a7aff, 0x4a6a3aff, 0xb09040ff][(k + x) as usize % 4];
                        c.outline(INK, ink);
                        c.fill(col);
                        c.rect(p.0 - 4.0 * z, p.1 - 3.0 * z, 8.0 * z, 6.0 * z);
                        c.outline(0, 0.0);
                        c.fill(shade(col, 1.3));
                        c.rect(p.0 - 4.0 * z, p.1 - 3.0 * z, 8.0 * z, 1.5 * z);
                    }
                }
                4 => {
                    // a keg on its cradle and jugs of ale
                    let p = at(-0.2, 0.05, top + 5.0);
                    c.outline(INK, ink);
                    c.fill(0x6a4424ff);
                    c.ell(p.0, p.1, 7.0 * z, 5.0 * z);
                    c.outline(0, 0.0);
                    c.stroke(0x2a2a2aff, ink);
                    c.line(p.0 - 3.0 * z, p.1 - 4.5 * z, p.0 - 3.0 * z, p.1 + 4.5 * z);
                    c.line(p.0 + 3.0 * z, p.1 - 4.5 * z, p.0 + 3.0 * z, p.1 + 4.5 * z);
                    for k in 0..3 {
                        let q = at(0.12 + k as f32 * 0.11, 0.1 + hv(k, 4) * 0.1, top + 3.0);
                        c.outline(INK, ink);
                        c.fill(0xa07a50ff);
                        c.rect(q.0 - 2.0 * z, q.1 - 3.0 * z, 4.0 * z, 5.0 * z);
                        c.outline(0, 0.0);
                        c.fill(0xe8d8a8ff);
                        c.ell(q.0, q.1 - 3.0 * z, 2.0 * z, 0.8 * z);
                    }
                }
                _ => {
                    // rows of little bottles, red, blue and green, catching the light
                    for k in 0..8 {
                        let (u, v) = (-0.38 + (k % 4) as f32 * 0.25, 0.0 + (k / 4) as f32 * 0.16);
                        let p = at(u, v, top + 3.0);
                        let col =
                            [0xb02a2aff, 0x2a4ab0ff, 0x3a9a3aff][(k as usize + x as usize) % 3];
                        c.fill(col);
                        c.ell(p.0, p.1, 1.8 * z, 2.4 * z);
                        c.fill(0xd8d0c0ff);
                        c.rect(p.0 - 0.6 * z, p.1 - 4.5 * z, 1.2 * z, 2.0 * z);
                        c.fill(0xffffffc0);
                        c.rect(p.0 - 1.0 * z, p.1 - 1.5 * z, 0.8 * z, 0.8 * z);
                    }
                }
            }
        }
        // the front poles
        c.outline(INK, ink);
        if left_end {
            pole(c, -0.46, 0.46, 37.0);
        }
        if right_end {
            pole(c, 0.46, 0.46, 37.0);
        }
        // the awning, striped along the stall, sloping down to the front
        let (av0, av1, ah0, ah1) = (-0.5f32, 0.58f32, 47.0f32, 35.0f32);
        let n = 4;
        for j in 0..n {
            let (ua, ub) = (-0.5 + j as f32 / n as f32, -0.5 + (j + 1) as f32 / n as f32);
            c.outline(0, 0.0);
            c.fill(if (x * n + j) % 2 == 0 { awn_a } else { awn_b });
            quad(
                c,
                [
                    at(ua, av0, ah0),
                    at(ub, av0, ah0),
                    at(ub, av1, ah1),
                    at(ua, av1, ah1),
                ],
            );
        }
        // shade on the awning's far half, light on its near
        c.fill(0x00000020);
        quad(
            c,
            [
                at(-0.5, av0, ah0),
                at(0.5, av0, ah0),
                at(0.5, 0.0, (ah0 + ah1) / 2.0),
                at(-0.5, 0.0, (ah0 + ah1) / 2.0),
            ],
        );
        // the scalloped valance hanging from the front edge
        for j in 0..(n * 2) {
            let u = -0.5 + (j as f32 + 0.5) / (n * 2) as f32;
            let p = at(u, av1, ah1);
            c.fill(if (x * n + j / 2) % 2 == 0 {
                awn_a
            } else {
                awn_b
            });
            c.ell(p.0, p.1 + 1.0 * z, 4.0 * z, 3.0 * z);
        }
        c.stroke(INK, ink);
        let (a, b) = (at(-0.5, av1, ah1), at(0.5, av1, ah1));
        c.line(a.0, a.1, b.0, b.1);
        let (a, b) = (at(-0.5, av0, ah0), at(0.5, av0, ah0));
        c.line(a.0, a.1, b.0, b.1);
        if left_end {
            let (a, b) = (at(-0.5, av0, ah0), at(-0.5, av1, ah1));
            c.line(a.0, a.1, b.0, b.1);
        }
        if right_end {
            let (a, b) = (at(0.5, av0, ah0), at(0.5, av1, ah1));
            c.line(a.0, a.1, b.0, b.1);
        }
        // after dark, a lantern on the front pole
        if !open && left_end {
            let p = at(-0.46, 0.46, 30.0);
            let fl = 0.9 + (t * 9.0 + x as f32).sin() * 0.1;
            c.outline(INK, ink);
            c.fill(0x2a2a2aff);
            c.rect(p.0 - 2.5 * z, p.1 - 3.0 * z, 5.0 * z, 6.0 * z);
            c.outline(0, 0.0);
            c.fill(0xffc060ff);
            c.rect(p.0 - 1.5 * z, p.1 - 2.0 * z, 3.0 * z, 4.0 * z);
            c.alpha(0.45);
            c.glow(p.0, p.1, 14.0 * z * fl, 0xffa040ff);
            c.alpha(1.0);
            lights.push((p.0, p.1, 55.0, 0xffa050ff));
        }
        c.outline(INK, ink);
    }

    /// A tile of the village's thick hedge: a bank of leafy clumps, uneven
    /// along the top, darker toward the roots, lit on its upper side.
    fn hedge(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, z: f32) {
        let (hw, hh) = (28.0 * z, 14.0 * z);
        let hgt = STOREY_PX * z * (0.8 + h2(x, y, 57) * 0.25);
        let ink = self.pix.max(z);
        // the body, in shade
        c.outline(INK, ink);
        c.fill(0x1e3616ff);
        c.poly(&[
            (sx - hw, sy),
            (sx, sy + hh),
            (sx + hw, sy),
            (sx + hw, sy - hgt * 0.8),
            (sx, sy + hh - hgt * 0.8),
            (sx - hw, sy - hgt * 0.8),
        ]);
        c.outline(0, 0.0);
        // clumps, low to high, so the top reads as leaves
        for k in 0..16 {
            let a = h2(x * 13 + k, y * 7, 51) * 2.0 - 1.0;
            let b = (k as f32 + h2(x * 5, y * 11 + k, 53)) / 16.0;
            let up = b * hgt;
            let (lx, ly) = (sx + a * hw * 0.85, sy + hh * 0.4 * (1.0 - a.abs()) - up);
            let r = (7.0 + h2(x + k, y, 55) * 5.0) * z;
            let light = b > 0.6;
            c.fill(match (light, k % 3) {
                (true, 0) => 0x5e8a36ff,
                (true, _) => 0x4a7430ff,
                (false, 0) => 0x2a4a1eff,
                _ => 0x345a24ff,
            });
            c.ell(lx, ly, r, r * 0.75);
        }
        // the odd pale bloom
        if h2(x, y, 59) < 0.3 {
            let (lx, ly) = (sx + (h2(x, y, 60) - 0.5) * hw, sy - hgt * 0.7);
            c.fill(0xe8dcc8ff);
            c.ell(lx, ly, 1.5 * z, 1.5 * z);
        }
    }

    /// A gate in the hedge: two posts and, when shut, a barred wooden gate.
    fn gate(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, z: f32) {
        let along_x = matches!(self.map.obj(x - 1, y), O_FENCE | O_DOOR)
            || matches!(self.map.obj(x + 1, y), O_FENCE | O_DOOR);
        let p = P {
            x: sx,
            y: sy,
            s: z,
            f: 1.0,
        };
        let (a, b) = if along_x {
            (p.at(-16.0, -8.0), p.at(16.0, 8.0))
        } else {
            (p.at(16.0, -8.0), p.at(-16.0, 8.0))
        };
        let ink = self.pix.max(z);
        if self.map.door_shut(x, y) {
            c.stroke(0x6a4a2aff, 3.0 * z);
            for k in [6.0f32, 13.0, 20.0] {
                c.line(a.0, a.1 - k * z, b.0, b.1 - k * z);
            }
            c.stroke(0x4a3018ff, 2.0 * z);
            c.line(a.0, a.1 - 6.0 * z, b.0, b.1 - 20.0 * z);
        }
        c.outline(INK, ink);
        c.fill(0x4a3018ff);
        for (px, py) in [a, b] {
            c.rect(px - 2.0 * z, py - 24.0 * z, 4.0 * z, 24.0 * z);
        }
        c.outline(0, 0.0);
    }

    /// The mill's water wheel, turning in the river beside it.
    fn water_wheel(&self, c: &mut Canvas, x: i32, y: i32, z: f32, t: f32) {
        let r = 0.95f32;
        let lz = 32.0 * z;
        let base = self.map.level(x, y) as f32 * LH * z;
        let pt = |a: f32, rr: f32, dx: f32| {
            let (sx, sy) = self.to_screen(x as f32 + dx, y as f32 + 0.5 + a.cos() * rr);
            (sx, sy - base - (0.45 + a.sin()) * rr * lz)
        };
        let spin = -t * 0.9;
        let ink = self.pix.max(z);
        // the far rim, the paddles, then the near rim
        for side in [-0.18f32, 0.18] {
            let rim: Vec<(f32, f32)> = (0..20)
                .map(|k| pt(k as f32 / 20.0 * std::f32::consts::TAU, r, side))
                .collect();
            c.stroke(if side < 0.0 { 0x3a2614ff } else { 0x5a3e22ff }, 2.5 * z);
            c.poly_s(&rim);
            if side < 0.0 {
                for k in 0..10 {
                    let a = spin + k as f32 / 10.0 * std::f32::consts::TAU;
                    let (p0, p1) = (pt(a, r, -0.18), pt(a, r, 0.18));
                    c.outline(INK, ink);
                    c.fill(0x6a4a2aff);
                    let (q0, q1) = (pt(a, r * 0.78, -0.18), pt(a, r * 0.78, 0.18));
                    c.poly(&[p0, p1, q1, q0]);
                    c.outline(0, 0.0);
                }
            }
        }
        c.stroke(0x4a3018ff, 1.5 * z);
        for k in 0..5 {
            let a = spin + k as f32 / 5.0 * std::f32::consts::TAU;
            let (p0, p1) = (pt(a, r, 0.18), pt(a + std::f32::consts::PI, r, 0.18));
            c.line(p0.0, p0.1, p1.0, p1.1);
        }
        // foam where the paddles meet the water
        let (fx, fy) = self.to_screen(x as f32, y as f32 + 0.5);
        c.fill(0xe0e8f060);
        c.ell(fx, fy - base, 18.0 * z, 5.0 * z);
    }

    /// The village's great tree, lanterns hung in its boughs.
    #[allow(clippy::too_many_arguments)]
    fn great_tree(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        let ink = self.pix.max(z);
        let golden = self.map.style[idx(x, y)] & 0x7f == GOLDEN;
        c.outline(INK, ink);
        // roots and trunk (silver-grey on the elves' trees)
        c.fill(if golden { 0x9a9a92ff } else { 0x4a3220ff });
        c.poly(&[
            (sx - 16.0 * z, sy + 4.0 * z),
            (sx + 16.0 * z, sy + 4.0 * z),
            (sx + 8.0 * z, sy - 50.0 * z),
            (sx - 8.0 * z, sy - 50.0 * z),
        ]);
        c.fill(if golden { 0xc0c0b8ff } else { 0x5a3e26ff });
        c.poly(&[
            (sx - 2.0 * z, sy + 4.0 * z),
            (sx + 16.0 * z, sy + 4.0 * z),
            (sx + 8.0 * z, sy - 50.0 * z),
            (sx + 1.0 * z, sy - 50.0 * z),
        ]);
        // the crown, in layers from dark to light
        let blobs: [(f32, f32, f32, f32, u32); 7] = [
            (0.0, -86.0, 70.0, 44.0, 0x22401cff),
            (-34.0, -78.0, 38.0, 28.0, 0x2a4a20ff),
            (34.0, -80.0, 40.0, 28.0, 0x2a4a20ff),
            (-12.0, -104.0, 44.0, 28.0, 0x33582aff),
            (20.0, -98.0, 36.0, 24.0, 0x3a6230ff),
            (-22.0, -92.0, 24.0, 16.0, 0x46723aff),
            (8.0, -116.0, 26.0, 16.0, 0x46723aff),
        ];
        const GOLD: [u32; 7] = [
            0x8a6a1aff, 0x9a7a22ff, 0x9a7a22ff, 0xb89430ff, 0xc8a438ff, 0xe0c050ff, 0xe8cc60ff,
        ];
        for (k, &(dx, dy, rx, ry, col)) in blobs.iter().enumerate() {
            if k > 0 {
                c.outline(0, 0.0);
            }
            c.fill(if golden { GOLD[k] } else { col });
            c.ell(sx + dx * z, sy + dy * z, rx * z, ry * z);
        }
        c.outline(0, 0.0);
        // lanterns on strings through the lower boughs
        for k in 0..7 {
            let a = k as f32 / 7.0 * std::f32::consts::TAU + 0.3;
            let (lx, ly) = (sx + a.cos() * 56.0 * z, sy - 70.0 * z + a.sin() * 18.0 * z);
            let fl = 0.8 + (t * 5.0 + k as f32).sin() * 0.15;
            c.alpha(0.5);
            c.glow(lx, ly, 10.0 * z * fl, 0xffb050ff);
            c.alpha(1.0);
            c.fill(if golden {
                [0xe0f0ffff, 0xc0d8ffff, 0xffffffff][k % 3]
            } else {
                [0xffd070ff, 0xff9050ff, 0xfff0a0ff][k % 3]
            });
            c.ell(lx, ly, 2.5 * z, 3.0 * z);
            if k % 2 == 0 {
                lights.push((lx, ly, 60.0, if golden { 0xc0d8ffff } else { 0xffb050ff }));
            }
        }
        let _ = (x, y);
    }
}

impl Game {
    /// The elves' white ship at the haven quay: a long pale hull with a
    /// swan's neck for a prow, one mast and a grey sail, riding the swell.
    fn ship(&self, c: &mut Canvas, x: i32, y: i32, z: f32, t: f32) {
        let lz = LH * z;
        let bob = (t * 1.3).sin() * 1.5 * z;
        let sp = |wx: f32, wy: f32, h: f32| {
            let (a, b) = self.to_screen(wx, wy);
            (a, b - h * lz + bob)
        };
        let ink = self.pix.max(z);
        // the hull runs along x, from stern (x-1.6) to prow (x+1.5)
        let (y0, y1) = (y as f32 - 0.45, y as f32 + 0.45);
        let (xa, xb) = (x as f32 - 1.6, x as f32 + 1.5);
        c.outline(INK, ink);
        c.fill(0xc8ccd0ff);
        c.poly(&[
            sp(xa, y1, 0.4),
            sp(xb, y1, 0.4),
            sp(xb + 0.4, y as f32, 1.0),
            sp(xb, y1, 1.0),
            sp(xa - 0.2, y1, 1.1),
        ]);
        c.fill(0xe8ecf0ff);
        c.poly(&[
            sp(xa - 0.2, y1, 1.1),
            sp(xb, y1, 1.0),
            sp(xb, y0, 1.0),
            sp(xa - 0.2, y0, 1.1),
        ]);
        // the swan's neck and head at the prow
        c.outline(0, 0.0);
        c.stroke(0xf4f6f8ff, 3.0 * z);
        let n0 = sp(xb + 0.3, y as f32, 1.0);
        let n1 = sp(xb + 0.5, y as f32, 2.2);
        let n2 = sp(xb + 0.2, y as f32, 2.9);
        c.line(n0.0, n0.1, n1.0, n1.1);
        c.line(n1.0, n1.1, n2.0, n2.1);
        c.fill(0xe8c040ff);
        c.ell(n2.0 + 3.0 * z, n2.1 + 1.0 * z, 2.5 * z, 1.2 * z);
        // mast and sail
        let m0 = sp(x as f32, y as f32, 1.0);
        let m1 = sp(x as f32, y as f32, 6.0);
        c.stroke(0x6a5a44ff, 2.0 * z);
        c.line(m0.0, m0.1, m1.0, m1.1);
        c.outline(INK, ink);
        c.fill(0xd8dce0e0);
        c.poly(&[
            sp(x as f32 - 0.05, y as f32 + 0.6, 5.6),
            sp(x as f32 - 0.05, y as f32 - 0.6, 5.6),
            sp(x as f32 - 0.05, y as f32 - 0.7, 2.0),
            sp(x as f32 - 0.05, y as f32 + 0.7, 2.0),
        ]);
        c.outline(0, 0.0);
        // a lantern at the stern
        let (lx, ly) = sp(xa, y as f32, 2.0);
        c.alpha(0.5);
        c.glow(lx, ly, 12.0 * z, 0xc0d8ffff);
        c.alpha(1.0);
        c.fill(0xf0f8ffff);
        c.ell(lx, ly, 2.0 * z, 2.5 * z);
    }
}

impl Game {
    /// The light of the hour, the weather, and the gloom under trees, laid
    /// over the world: see `game/sky.rs`.
    fn draw_sky(&self, c: &mut Canvas, z: f32, t: f32, lights: &[(f32, f32, f32, u32)]) {
        use crate::game::sky::Weather;
        let (px, py) = self.ents[0].tile();
        let indoors = self.map.building_at(px, py).is_some();
        let (w, k) = self.weather();
        let (dark, tint) = self.daylight();
        let fv = self.forest_view;
        let full = |c: &mut Canvas| c.rect(-10.0, -10.0, self.w + 20.0, self.h + 20.0);
        c.outline(0, 0.0);
        // under the trees: green gloom, light coming through in patches
        if fv > 0.02 {
            c.fill(with_a(0x0a1a08ff, 0.42 * fv));
            full(c);
            for i in 0..9 {
                let (a, b) = (h2(i, 21, 5), h2(i, 23, 6));
                let x = a * self.w + (t * 0.15 + i as f32).sin() * 18.0 * z;
                let y = b * self.h + (t * 0.11 + i as f32 * 2.0).cos() * 10.0 * z;
                c.alpha(0.22 * fv * (1.0 - dark));
                c.glow(x, y, (34.0 + 30.0 * a) * z, 0xe8f0a0ff);
            }
            c.alpha(1.0);
        }
        // grey skies
        let grey = match w {
            Weather::Rain => 0.14,
            Weather::Storm => 0.3,
            Weather::Snow => 0.06,
            Weather::Fog => 0.1,
            Weather::Clear => 0.0,
        } * k;
        if grey > 0.0 {
            c.fill(with_a(0x1a2028ff, grey));
            full(c);
        }
        if w == Weather::Fog && k > 0.0 {
            for i in 0..14 {
                let fi = i as f32;
                let fx = (h2(i, 31, 7) * (self.w + 600.0) + t * (4.0 + fi * 0.5))
                    % (self.w + 600.0)
                    - 300.0;
                let fy = h2(i, 37, 8) * self.h;
                c.fill(with_a(0xb4b4acff, 0.12 * k));
                c.ell(fx, fy, 300.0 * z, 90.0 * z);
            }
            c.fill(with_a(0x9a9a96ff, 0.18 * k));
            full(c);
        }
        // night: dark, the lamps and fires shining through it
        if dark > 0.01 {
            if tint != 0 {
                c.fill(with_a(tint, dark * 0.35));
                full(c);
            }
            c.fill(with_a(0x04060eff, dark));
            full(c);
            for &(lx, ly, r, col) in lights {
                c.alpha((dark * 1.1).min(0.65));
                c.glow(lx, ly, r * z * 0.75, col);
            }
            // enough to see one's own feet by
            let (sx, sy) = self.scr_h(self.ents[0].x, self.ents[0].y, self.ents[0].anim.vz);
            c.alpha(dark * 0.55);
            c.glow(sx, sy - 16.0 * z, 80.0 * z, 0x9aa4c0ff);
            c.alpha(1.0);
        }
        if indoors || k <= 0.0 {
            return;
        }
        // what falls from the sky (thinner under the trees)
        let under = 1.0 - 0.6 * fv;
        match w {
            Weather::Rain | Weather::Storm => {
                let n = ((if w == Weather::Storm { 170.0 } else { 90.0 }) * k * under) as i32;
                c.stroke(0x9ab0c8a8, self.pix.max(z));
                for i in 0..n {
                    let (a, b) = (h2(i, 41, 9), h2(i, 43, 10));
                    let sp = 520.0 + b * 200.0;
                    let x = (a * (self.w + 60.0) - t * 90.0).rem_euclid(self.w + 60.0);
                    let y = (b * self.h + t * sp).rem_euclid(self.h + 40.0) - 20.0;
                    c.line(x, y, x - 4.0 * z, y + 13.0 * z);
                }
                if w == Weather::Storm {
                    // now and then the lightning
                    let beat = (t * 0.31).floor() as i32;
                    let ph = (t * 0.31).fract();
                    if h2(beat, 47, 11) < 0.45 && ph < 0.05 {
                        let a = if ph < 0.015 || (0.03..0.04).contains(&ph) {
                            0.55
                        } else {
                            0.0
                        };
                        if a > 0.0 {
                            c.fill(with_a(0xe8f0ffff, a * k));
                            full(c);
                        }
                    }
                }
            }
            Weather::Snow => {
                let n = (140.0 * k * under) as i32;
                c.fill(0xf4f8ffe0);
                for i in 0..n {
                    let (a, b) = (h2(i, 51, 12), h2(i, 53, 13));
                    let x = (a * self.w + (t * 0.8 + i as f32).sin() * 14.0 * z - t * 12.0)
                        .rem_euclid(self.w);
                    let y = (b * self.h + t * (26.0 + a * 20.0)).rem_euclid(self.h);
                    let r = (1.0 + a * 1.2) * self.pix.max(1.0);
                    c.rect(x, y, r, r);
                }
            }
            _ => {}
        }
    }
}

/// Dress a dwarf of Karak Dhum: `r` draws from their look seed, and `mine`
/// says they work down in the mines.
fn dwarf_look(l: &mut Look, v: usize, r: &dyn Fn(u32) -> u32, mine: bool) {
    const SKIN: [u32; 4] = [0xd8a080ff, 0xc89070ff, 0xe0b090ff, 0xb87a58ff];
    const BEARD: [u32; 7] = [
        0x6a3a1aff, 0x3a2a1aff, 0x8a8680ff, 0xa8501eff, 0x1a1612ff, 0x7a5a2aff, 0xc8c4bcff,
    ];
    const WOOL: [u32; 6] = [
        0x5a3a2aff, 0x4a4a52ff, 0x6a2a20ff, 0x3a4a5aff, 0x5a5030ff, 0x2e3a2eff,
    ];
    let pick = |k: u32, a: &[u32]| a[(r(k) % a.len() as u32) as usize];
    l.scale = 76 + (r(7) % 7) as u16;
    l.bulk = 22 + (r(6) % 6) as u8;
    l.hunch = 0;
    l.skin = pick(1, &SKIN);
    l.hair = pick(3, &BEARD);
    l.beard = if l.female { 0 } else { l.hair };
    l.long_hair = l.female;
    l.robe = false;
    l.ears = false;
    l.turban = 0;
    l.sash = 0;
    l.basket = false;
    l.hat = 0;
    l.hood = 0;
    l.helm = 0;
    l.furs = 0;
    l.metal_body = false;
    l.skirt = false;
    l.body = pick(8, &WOOL);
    l.legs = shade(pick(2, &WOOL), 0.7);
    l.boots = [0x2a1e14ff, 0x3a2a1aff, 0x1e1a18ff][(r(5) % 3) as usize];
    l.weapon = Weapon::None;
    l.shield = 0;
    match v {
        V_GUARD => {
            // the gatewardens: mail to the knee, a steel cap, axe and a
            // round shield bossed in bronze
            l.female = false;
            l.long_hair = false;
            l.beard = l.hair;
            l.metal_body = true;
            l.skirt = true;
            l.body = 0x7a8088ff;
            l.helm = 0x9aa0a8ff;
            l.legs = 0x5a5e66ff;
            l.weapon = Weapon::Axe;
            l.shield = 0x8a5a2aff;
            l.scale = 82;
        }
        V_THANE => {
            // the King: a long crimson robe, a gilt crown-helm, a white
            // beard to the belt and his war-hammer at hand
            l.female = false;
            l.long_hair = false;
            l.hair = 0xd8d4ccff;
            l.beard = l.hair;
            l.robe = true;
            l.body = 0x7a1a1aff;
            l.legs = l.body;
            l.helm = 0xd8b040ff;
            l.furs = 0x4a3a2aff;
            l.weapon = Weapon::Mace;
            l.scale = 84;
        }
        V_SMITH => {
            // bare-armed in a leather apron, hammer in hand
            l.body = 0x4a3020ff;
            l.bulk = 28;
            l.weapon = Weapon::Mace;
        }
        V_MAGE => {
            // the runecaster: a hooded robe of deep blue and a staff
            l.robe = true;
            l.body = 0x24305aff;
            l.legs = l.body;
            l.hood = 0x1e2648ff;
            l.weapon = Weapon::Staff;
        }
        V_HEALER => {
            l.robe = true;
            l.body = 0x6a6a5aff;
            l.legs = l.body;
            l.hair = 0xb0aca4ff;
            l.beard = if l.female { 0 } else { l.hair };
        }
        V_BARKEEP => {
            l.bulk = 30;
            l.body = 0x7a5a3aff;
        }
        _ => {
            // miners with picks and hoods, smiths at the forge with
            // hammers, the rest about their business
            if mine || r(9).is_multiple_of(4) {
                l.weapon = Weapon::Pick;
                l.hood = shade(l.body, 0.75);
                l.long_hair = false;
            } else if r(9) % 4 == 1 {
                l.weapon = Weapon::Mace;
                l.body = 0x4a3020ff;
            } else if r(9) % 4 == 2 && !l.female {
                // an elder, grey to the belt
                l.hair = 0xc8c4bcff;
                l.beard = l.hair;
                l.hunch = 10;
            }
        }
    }
}

/// The beasts of the streets, each in one of a few coats picked by `seed`
/// (a monkey's: 0 and 1 are macaques, 2 and 3 langurs).
pub fn street_look(id: &str, seed: u32) -> Option<Look> {
    use bake::Body;
    let k = seed as usize;
    let l = match id {
        "pigeon" => {
            // body, wings, neck: blue-bar grey, purple-necked, white, brown
            const COATS: [(u32, u32, u32); 4] = [
                (0x86889aff, 0xa2a4b0ff, 0x4e7a6eff),
                (0x7e8092ff, 0x9a9caaff, 0x6e5a86ff),
                (0xe2e0daff, 0xeceae4ff, 0xd0d4c8ff),
                (0x7a5e50ff, 0x957666ff, 0x5a6a5aff),
            ];
            let (b, w, n) = COATS[k % 4];
            let mut l = Look::creature(Body::Bird, b, 150);
            l.body = w;
            l.legs = n;
            l
        }
        "dog" => {
            // coat, markings, pricked ears: the tan pariah, black and tan,
            // brown and white, grey
            const COATS: [(u32, u32, bool); 4] = [
                (0xb4844eff, 0xe6d8bcff, true),
                (0x2e2a28ff, 0xb88a58ff, true),
                (0x6a4a30ff, 0xe0d8c8ff, false),
                (0x8e8880ff, 0, false),
            ];
            let (c, p, ears) = COATS[k % 4];
            let mut l = Look::creature(Body::Dog, c, 108);
            l.patch = p;
            l.ears = ears;
            l
        }
        "cat" => {
            // ginger tabby, grey tabby, black, cream
            const COATS: [(u32, u32); 4] = [
                (0xc4783aff, 0x8a4a1cff),
                (0x8a8884ff, 0x3e3c3aff),
                (0x1e1c1cff, 0),
                (0xe0d4bcff, 0),
            ];
            let (c, st) = COATS[k % 4];
            let mut l = Look::creature(Body::Cat, c, 125);
            l.stripes = st;
            l
        }
        "cow" => {
            // white, grey and red zebu; one with its horns painted
            const COATS: [(u32, u32); 4] = [
                (0xe2ddd2ff, 0),
                (0xa8a298ff, 0),
                (0x8e5430ff, 0),
                (0xe2ddd2ff, 0xa83020ff),
            ];
            let (c, h) = COATS[k % 4];
            let mut l = Look::creature(Body::Cow, c, 100);
            l.horns = h;
            l
        }
        "pig" => {
            // pink, black, pink with black spots
            const COATS: [(u32, u32); 3] =
                [(0xd8a090ff, 0), (0x34302eff, 0), (0xd8a090ff, 0x34302eff)];
            let (c, p) = COATS[k % 3];
            let mut l = Look::creature(Body::Pig, c, 100);
            l.patch = p;
            l
        }
        "monkey" => {
            // fur, belly, face, hands and feet, long tail: rhesus macaques
            // brown and greyer, Hanuman langurs buff and grey
            const COATS: [(u32, u32, u32, u32, bool); 4] = [
                (0x8e6a44ff, 0xb89870ff, 0xd88878ff, 0x6a5048ff, false),
                (0x8a7c62ff, 0xb0a488ff, 0xc87a6cff, 0x5e504aff, false),
                (0xc4baa6ff, 0xe0d8c8ff, 0x24201eff, 0x2a2624ff, true),
                (0xa29c92ff, 0xc8c2b6ff, 0x1e1a1aff, 0x242020ff, true),
            ];
            let (c, b, f, h, long) = COATS[k % 4];
            let mut l = Look::creature(Body::Monkey, c, 120);
            l.body = b;
            l.patch = f;
            l.legs = h;
            l.long_hair = long;
            l
        }
        _ => return None,
    };
    Some(l)
}

/// A deep one's look: hunched, scaled and grey-green, with gill-fins for
/// ears, pale bulging eyes and a spear.
fn deep_one(l: &mut Look) {
    l.skin = 0x5e7e6cff;
    l.body = 0x4a6a5aff;
    l.legs = 0x4a6a5aff;
    l.boots = 0x3e5a4cff;
    l.eyes = 0xd8e8a0ff;
    l.ears = true;
    l.hunch = 28;
    l.bulk = 12;
    l.weapon = Weapon::Halberd;
    l.skirt = true;
    l.furs = 0x3a4a30ff;
    l.hair = 0;
    l.hat = 0;
    l.hood = 0;
    l.female = false;
}
