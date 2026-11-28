//! World map: procedural generation, tiles, objects, pathfinding.
use crate::data::*;
use crate::rng::Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub const W: i32 = 240;
/// the land runs further south than east: below `NORTH_H` lie the tropics
pub const LAND_H: i32 = 320;
/// the height of the northern lands, as they were before the tropics were added
pub const NORTH_H: i32 = 240;
/// the tiles of the land, row by row: those of the Ethereal Void come after
pub const LAND_N: usize = (W * LAND_H) as usize;
/// Below the land, beyond a gulf of nothing that no eye on the land can see
/// across, lies the Ethereal Void (`world/void.rs`): from `VOID_Y0` to `H`.
pub const VOID_Y0: i32 = 360;
/// the whole map, the land and the Void
pub const H: i32 = 440;
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
/// the Ethereal Void: nothing underfoot but the stars (`world/void.rs`)
pub const T_VOID: u8 = 9;
/// the pale stone of the Void's isles and starways
pub const T_ASTRAL: u8 = 10;

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
pub const O_MUSHROOM: u8 = 18;
pub const O_BARREL: u8 = 19;
pub const O_CRATE: u8 = 20;
pub const O_BONES: u8 = 21;
pub const O_BED: u8 = 22;
pub const O_TABLE: u8 = 23;
pub const O_CHAIR: u8 = 24;
pub const O_SHELF: u8 = 25;
pub const O_HEARTH: u8 = 26;
pub const O_CHEST: u8 = 27;
pub const O_SACKS: u8 = 28;
pub const O_SKULLS: u8 = 29;
pub const O_STATUE: u8 = 30;
pub const O_WELL: u8 = 31;
/// a tile covered by the larger object in front of it (drawn there, blocks here)
pub const O_PART: u8 = 32;
pub const O_LONGTABLE: u8 = 33;
pub const O_CART: u8 = 34;
pub const O_THRONE: u8 = 35;
/// a flight of stairs between storeys of a building (`STAIR_UP`/`STAIR_DOWN`
/// in its style say which way it leads from the floor it stands on)
pub const O_STAIRS: u8 = 36;
pub const STAIR_UP: u8 = 1;
pub const STAIR_DOWN: u8 = 2;
/// a mill's water wheel, standing in the river beside the mill
pub const O_WHEEL: u8 = 37;
/// the village's great tree, under which its parties are held
pub const O_BIGTREE: u8 = 38;
/// a white ship with a swan's prow, moored at the elven haven
pub const O_SHIP: u8 = 39;
/// a market stall: a counter under a striped awning; its style is what it
/// sells (an index into `data::STALL_WARES`)
pub const O_STALL: u8 = 40;
// house furniture and decorations
pub const O_WARDROBE: u8 = 41;
/// a kitchen dresser: cupboard below, plates and jugs on a rack above
pub const O_DRESSER: u8 = 42;
/// a weapon rack: swords, a spear and an axe
pub const O_RACK: u8 = 43;
/// a suit of mail on a stand
pub const O_ARMOUR: u8 = 44;
/// an iron cauldron on embers
pub const O_CAULDRON: u8 = 45;
/// a potted plant (two kinds, by the tile)
pub const O_PLANT: u8 = 46;
/// an iron candle-stand, lit
pub const O_CANDLES: u8 = 47;
/// storage jars and a basket
pub const O_JARS: u8 = 48;
pub const O_SPINWHEEL: u8 = 49;
/// a writing desk with pigeonholes
pub const O_DESK: u8 = 50;
/// an armillary sphere on its stand
pub const O_ORRERY: u8 = 51;
pub const O_STOOL: u8 = 52;
/// a rack of herbs hung to dry
pub const O_HERBS: u8 = 53;
/// a cyclopean ruin of sunken R'lyeh, risen from the sea (style: which)
pub const O_RUIN: u8 = 54;
/// a moongate: a ring of standing stones about a gate of blue light that
/// joins the land and the Void (`Map::gates`; walked into, not round)
pub const O_MOONGATE: u8 = 55;
/// a cluster of the Void's glowing crystals (style: its colour)
pub const O_CRYSTAL: u8 = 56;
/// a little table in a pub with a board of Lines laid out on it: nine by
/// nine, coloured balls (`game/lines.rs`)
pub const O_GAMEBOARD: u8 = 57;
/// a stand of bamboo in a grove of the tropic hills; the pandas eat it
/// down (its `res`) and it grows again (`game/panda.rs`)
pub const O_BAMBOO: u8 = 58;
/// O_RUIN styles: R'lyeh's monolith, door and jumble (0–2), the idol in
/// drowned Saltgrave, a wrecked boat
pub const RUIN_IDOL: u8 = 3;
pub const RUIN_WRECK: u8 = 4;
/// seconds a door stays open before it swings shut
pub const DOOR_OPEN: f32 = 5.0;

/// how far a road post's guards keep the peace, in tiles (chebyshev)
pub const POST_R: i32 = 6;

/// wall styles of the castle: curtain walls, and the taller towers
pub const WALL_CASTLE: u8 = 5;
pub const WALL_TOWER: u8 = 6;
/// a burrow house dug into a turf mound: earthen walls, a round door
pub const WALL_BURROW: u8 = 7;
/// the pale stone halls of the elves
pub const WALL_ELVEN: u8 = 8;
/// the south's red sandstone: a fort's curtain walls, and its round bastions
pub const WALL_FORT: u8 = 9;
pub const WALL_BASTION: u8 = 10;
/// a haveli: lime-washed walls under a flat roof with domed kiosks
pub const WALL_HAVELI: u8 = 11;
/// a raja's palace: sandstone under white domes
pub const WALL_PALACE: u8 = 12;
/// a temple: carved sandstone under a tall curved tower
pub const WALL_TEMPLE: u8 = 13;
/// the dark granite halls of the dwarves of Karak Dhum
pub const WALL_DWARF: u8 = 14;
/// the broken grey stone of Saltgrave's drowned cottages, and where they
/// have half fallen
pub const WALL_DROWNED: u8 = 15;
pub const WALL_DROWNED_LOW: u8 = 16;
/// great-tree style: the golden trees of the elven wood
pub const GOLDEN: u8 = 1;
/// fence style: a thick hedge (the village's ring)
pub const HEDGE: u8 = 2;

/// An object's footprint in tiles (along y unless mirrored with FACE_S, then
/// along x) and its height in metres, for blocking, drawing and see-through.
pub fn size(o: u8) -> (i32, f32) {
    match o {
        O_LONGTABLE => (2, 0.8),
        O_CART => (2, 1.4),
        O_TREE => (1, 4.0),
        O_STATUE => (1, 2.4),
        O_WELL => (1, 1.8),
        O_WALL => (1, 1.3),
        O_TENT => (1, 1.6),
        O_HEARTH => (1, 1.8),
        O_SHELF => (1, 1.9),
        O_LAMP => (1, 2.0),
        O_THRONE => (1, 1.6),
        O_BIGTREE => (1, 7.0),
        O_WHEEL => (1, 2.0),
        O_SHIP => (1, 3.0),
        O_RUIN => (1, 3.2),
        O_MOONGATE => (1, 3.0),
        O_CRYSTAL => (1, 1.2),
        O_GAMEBOARD => (1, 0.9),
        O_BAMBOO => (1, 3.4),
        O_WARDROBE => (1, 2.0),
        O_DRESSER => (1, 1.95),
        O_RACK => (1, 1.9),
        O_ARMOUR => (1, 1.8),
        O_CANDLES => (1, 1.6),
        O_HERBS => (1, 1.5),
        O_ORRERY => (1, 1.35),
        O_DESK => (1, 1.25),
        O_SPINWHEEL => (1, 1.1),
        O_PLANT => (1, 0.9),
        O_CAULDRON => (1, 0.8),
        O_JARS => (1, 0.8),
        _ => (1, 0.6),
    }
}
/// style bit: furniture faces down-left (mirrored) instead of down-right
pub const FACE_S: u8 = 0x80;

pub struct Building {
    pub x: i32,
    pub y: i32,
    /// footprint in tiles, walls included (shops are 8×6)
    pub w: i32,
    pub h: i32,
    pub vendor: usize,
    pub style: u8,
    /// the storeys above the ground floor, lowest first
    pub floors: Vec<Floor>,
    /// a room built onto another building (by its index): one house with
    /// it, entered through it, its roof lifting with the house's
    pub part_of: Option<usize>,
}

impl Building {
    /// storeys in all, the ground floor included
    pub fn storeys(&self) -> u8 {
        1 + self.floors.len() as u8
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    /// inside its walls, not on them
    pub fn interior(&self, x: i32, y: i32) -> bool {
        x > self.x && y > self.y && x < self.x + self.w - 1 && y < self.y + self.h - 1
    }
    /// Whether its walls are thin, standing on the edges of its outer ring
    /// of tiles rather than filling them (a house); the castle's keep, the
    /// fort's halls and the burrows keep their thick walls.
    pub fn thin(&self) -> bool {
        thin_style(self.style)
    }
    /// The floor its walls enclose: everything but thick walls.
    pub fn room(&self, x: i32, y: i32) -> bool {
        if self.thin() {
            self.contains(x, y)
        } else {
            self.interior(x, y)
        }
    }
}

/// Wall styles drawn as thin walls on the tile edges (see `Building::thin`).
pub fn thin_style(style: u8) -> bool {
    !matches!(
        style & 0x7f,
        WALL_CASTLE | WALL_TOWER | WALL_BURROW | WALL_FORT | WALL_BASTION
    )
}

/// An upper storey: its objects (walls round the edge, furniture, stairs)
/// over the building's footprint, row by row.
pub struct Floor {
    pub o: Vec<u8>,
    pub style: Vec<u8>,
}

/// Who is moving, for where they may go: which storey they are on, whether
/// they open doors (people do, beasts do not) and whether they take stairs.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Walker {
    pub fl: u8,
    pub doors: bool,
    pub stairs: bool,
}
/// townsfolk: they open doors but keep to their own floor
pub const PERSON: Walker = Walker {
    fl: 0,
    doors: true,
    stairs: false,
};
/// creatures of the wild: the ground floor only, and no doors
pub const BEAST: Walker = Walker {
    fl: 0,
    doors: false,
    stairs: false,
};

/// A road as it was carved, tile by tile from `from` to `to` (place ids:
/// "town", "city", or a site id from `sim::SITE_IDS`), for travellers to follow.
pub struct Road {
    pub from: &'static str,
    pub to: &'static str,
    pub tiles: Vec<(i32, i32)>,
}

pub struct Map {
    pub t: Vec<u8>,
    pub o: Vec<u8>,
    pub shade: Vec<u8>,
    pub style: Vec<u8>,
    pub res: Vec<u8>,
    pub regrow: Vec<(usize, f32)>,
    /// ground level per tile (Pagan's height-mapped playfield), 0..=4
    pub lv: Vec<u8>,
    /// a slope: walkable up to the next level on any side
    pub ramp: Vec<bool>,
    pub road: Vec<bool>,
    /// walkable from the town
    pub reach: Vec<bool>,
    pub town: (i32, i32, i32, i32),
    /// the walled city of Duskhold and its castle, north of the town
    pub city: (i32, i32, i32, i32),
    /// the stables east of the town: barn, yard and paddock (guarded ground)
    pub stable: (i32, i32, i32, i32),
    /// the halfling village west of the town, inside its hedge: a peaceful place
    pub village: (i32, i32, i32, i32),
    /// the rivers, each tile by tile from its source to the sea
    pub rivers: Vec<Vec<(i32, i32)>>,
    /// the elven towns: the valley, the wood and the haven (guarded ground)
    pub elves: Vec<(i32, i32, i32, i32)>,
    /// where the stablemaster's horses graze in the paddock
    pub paddock: Vec<(i32, i32)>,
    /// guard posts along the roads: monsters near one are struck down too
    pub posts: Vec<(i32, i32)>,
    pub start: (i32, i32),
    pub npcs: Vec<(usize, i32, i32)>,
    pub spawns: Vec<(&'static str, i32, i32, i32, usize)>,
    pub ankhs: Vec<(i32, i32)>,
    pub buildings: Vec<Building>,
    pub pois: Vec<(&'static str, i32, i32)>,
    pub roads: Vec<Road>,
    /// the Shadow over each tile, 0 (none) to `GLOOM_MAX`; see `game/shadow.rs`
    pub gloom: Vec<u8>,
    /// trees the Shadow has withered (they green again when it lifts)
    pub withered: Vec<bool>,
    /// seconds each door has left open (0: shut); see `DOOR_OPEN`
    pub door: Vec<f32>,
    /// every door tile, for ticking them
    pub doors: Vec<usize>,
    /// townsfolk who live upstairs: (vendor kind, x, y, storey)
    pub guests: Vec<(usize, i32, i32, u8)>,
    /// Duskhold's market square, x0, y0, x1, y1 (exclusive)
    pub market: (i32, i32, i32, i32),
    /// where a shopper stands to look over a stall: the tile before each
    /// stall tile
    pub stalls: Vec<(i32, i32)>,
    /// where the tropics begin, by column: every tile at or south of it is
    /// tropical (see `world/south.rs`)
    pub tropic_line: Vec<i32>,
    /// the southern cities, Suryagarh and Kanakpur: x0, y0, x1, y1 (guarded ground)
    pub raj: Vec<(i32, i32, i32, i32)>,
    /// Karak Dhum, the dwarf hold in the eastern range, with its gate
    /// forecourt and mines: x0, y0, x1, y1 (guarded ground)
    pub hold: (i32, i32, i32, i32),
    /// the building on each tile, plus one (0: none); see `Map::thin_walls`
    pub bld: Vec<u16>,
    /// where sunken R'lyeh lies, in the bay below Kanakpur (`game/deep.rs`)
    pub rlyeh: Option<(i32, i32)>,
    /// the moongates, in pairs that lead to each other: the land's, then
    /// the Void's (`world/void.rs`)
    pub gates: Vec<(i32, i32)>,
    /// the isles of the Ethereal Void, the Isle of the Gate first
    pub isles: Vec<Isle>,
    /// the middles of the bamboo groves in the tropic hills (`world/bamboo.rs`)
    pub groves: Vec<(i32, i32)>,
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

// ---------------------------------------------------------------- the hex cells
// Under the square tiles lies a grid of hex cells, one tile's area each,
// flat-topped and laid square to the screen. The terraces are shaped from
// them: every tile takes the level of its cell, and the ground is drawn
// along the cells' edges where levels change (see `render::terrace`), so a
// ledge runs as an unbroken line however it lies, not in square steps.

/// circumradius of a hex cell, in tiles (its area is one tile)
pub const HEX_R: f32 = 0.620_403_6;
const S2: f32 = std::f32::consts::FRAC_1_SQRT_2;
const SQ3: f32 = 1.732_050_8;

/// The centre of hex cell (q, r), in tiles.
pub fn hex_centre(q: i32, r: i32) -> (f32, f32) {
    let u = HEX_R * 1.5 * q as f32;
    let v = HEX_R * SQ3 * (r as f32 + q as f32 * 0.5);
    (S2 * (u + v), S2 * (v - u))
}
/// Corner `k` (0..6, round from the right) of hex cell (q, r), in tiles.
/// Edge `k` runs from corner k to corner k + 1 and faces `HEX_NB[k]`.
pub fn hex_corner(q: i32, r: i32, k: usize) -> (f32, f32) {
    let (cx, cy) = hex_centre(q, r);
    let a = std::f32::consts::PI / 3.0 * k as f32;
    let (du, dv) = (HEX_R * a.cos(), HEX_R * a.sin());
    (cx + S2 * (du + dv), cy + S2 * (dv - du))
}
/// the neighbour of a cell across each of its edges
pub const HEX_NB: [(i32, i32); 6] = [(1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1), (1, -1)];
/// The hex cell a point (in tiles) lies in.
pub fn hex_of(x: f32, y: f32) -> (i32, i32) {
    let (u, v) = (S2 * (x - y), S2 * (x + y));
    let qf = u / (1.5 * HEX_R);
    let rf = v / (SQ3 * HEX_R) - qf * 0.5;
    let (cx, cz) = (qf, rf);
    let cy = -cx - cz;
    let (mut rx, ry, mut rz) = (cx.round(), cy.round(), cz.round());
    let (dx, dy, dz) = ((rx - cx).abs(), (ry - cy).abs(), (rz - cz).abs());
    if dx > dy && dx > dz {
        rx = -ry - rz;
    } else if dy <= dz {
        rz = -rx - ry;
    }
    (rx as i32, rz as i32)
}
/// The tile whose level a hex cell's ground is drawn at: the one whose
/// centre lies in it (nearest its centre), else the one under its centre.
pub fn hex_tile(q: i32, r: i32) -> (i32, i32) {
    let (cx, cy) = hex_centre(q, r);
    let (tx, ty) = (cx.round() as i32, cy.round() as i32);
    let mut best = ((tx, ty), f32::MAX, false);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (x, y) = (tx + dx, ty + dy);
            if hex_of(x as f32, y as f32) == (q, r) {
                let d = (x as f32 - cx).powi(2) + (y as f32 - cy).powi(2);
                if d < best.1 {
                    best = ((x, y), d, true);
                }
            }
        }
    }
    best.0
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

/// Furniture that may be set back against a wall: one tile big, and not a
/// stair, a door or part of something longer.
fn movable(o: u8) -> bool {
    !matches!(o, O_NONE | O_STAIRS | O_DOOR | O_WALL | O_PART) && size(o).0 == 1
}

pub fn blocks(o: u8) -> bool {
    !matches!(
        o,
        O_NONE
            | O_DOOR
            | O_BUSH
            | O_FLOWERS
            | O_BONES
            | O_SKULLS
            | O_STAIRS
            | O_MOONGATE
            | O_BAMBOO
    )
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
    /// Can a beast stand here? (a shut door keeps it out)
    pub fn passable(&self, x: i32, y: i32) -> bool {
        self.passable_as(x, y, BEAST)
    }
    /// Can this walker stand here, on its storey?
    pub fn passable_as(&self, x: i32, y: i32, w: Walker) -> bool {
        if !inb(x, y) {
            return false;
        }
        let i = idx(x, y);
        if w.fl > 0 {
            let Some(b) = self.building_at(x, y) else {
                return false;
            };
            let Some(f) = self.buildings[b].floors.get(w.fl as usize - 1) else {
                return false;
            };
            let o = f.o[self.local(b, x, y)];
            return !blocks(o) && (w.stairs || o != O_STAIRS);
        }
        let (t, o) = (self.t[i], self.o[i]);
        if o == O_DOOR && self.door[i] <= 0.0 && !w.doors {
            return false;
        }
        if o == O_STAIRS && !w.stairs {
            return false;
        }
        t != T_WATER && t != T_MOUNT && t != T_VOID && !blocks(o)
    }
    pub fn building_at(&self, x: i32, y: i32) -> Option<usize> {
        if !self.bld.is_empty() {
            return if inb(x, y) {
                (self.bld[idx(x, y)] as usize).checked_sub(1)
            } else {
                None
            };
        }
        self.buildings.iter().position(|b| b.contains(x, y))
    }
    /// The house a building is part of: its own index, or the index of the
    /// building it was built onto.
    pub fn house_of(&self, b: usize) -> usize {
        self.buildings[b].part_of.unwrap_or(b)
    }
    /// The thin-walled building on a tile, if any.
    pub fn thin_at(&self, x: i32, y: i32) -> Option<&Building> {
        self.building_at(x, y)
            .map(|b| &self.buildings[b])
            .filter(|b| b.thin())
    }
    /// Is there a thin wall between a tile and its neighbour (dx, dy) along
    /// x or y, on this storey? A house's walls run round the outside of its
    /// footprint, broken on the ground floor only by its doors.
    pub fn wall_between(&self, x: i32, y: i32, dx: i32, dy: i32, fl: u8) -> bool {
        let (nx, ny) = (x + dx, y + dy);
        let (a, b) = (self.building_at(x, y), self.building_at(nx, ny));
        if a == b {
            return false;
        }
        if let (Some(i), Some(j)) = (a, b) {
            if self.house_of(i) == self.house_of(j) {
                return fl > 0 || (self.obj(x, y) != O_DOOR && self.obj(nx, ny) != O_DOOR);
            }
        }
        let side = |bi: Option<usize>, tx: i32, ty: i32| match bi {
            Some(i) if self.buildings[i].thin() => fl > 0 || self.obj(tx, ty) != O_DOOR,
            _ => false,
        };
        side(a, x, y) || side(b, nx, ny)
    }
    /// Can a step from (x, y) by (dx, dy) (one tile, any of 8 ways) be taken
    /// without going through a thin wall? Diagonally, neither way round the
    /// corner may meet one.
    pub fn edges_clear(&self, x: i32, y: i32, dx: i32, dy: i32, fl: u8) -> bool {
        if self.bld.is_empty() {
            return true;
        }
        if dx == 0 || dy == 0 {
            return !self.wall_between(x, y, dx, dy, fl);
        }
        !self.wall_between(x, y, dx, 0, fl)
            && !self.wall_between(x + dx, y, 0, dy, fl)
            && !self.wall_between(x, y, 0, dy, fl)
            && !self.wall_between(x, y + dy, dx, 0, fl)
    }
    fn local(&self, b: usize, x: i32, y: i32) -> usize {
        let b = &self.buildings[b];
        ((y - b.y) * b.w + (x - b.x)) as usize
    }
    /// The object on a tile at a storey (the map's own on the ground floor).
    pub fn obj_on(&self, x: i32, y: i32, fl: u8) -> (u8, u8) {
        if fl == 0 {
            return if inb(x, y) {
                (self.o[idx(x, y)], self.style[idx(x, y)])
            } else {
                (O_NONE, 0)
            };
        }
        match self.building_at(x, y) {
            Some(b) => match self.buildings[b].floors.get(fl as usize - 1) {
                Some(f) => {
                    let k = self.local(b, x, y);
                    (f.o[k], f.style[k])
                }
                None => (O_NONE, 0),
            },
            None => (O_NONE, 0),
        }
    }
    pub fn door_shut(&self, x: i32, y: i32) -> bool {
        inb(x, y) && self.o[idx(x, y)] == O_DOOR && self.door[idx(x, y)] <= 0.0
    }
    /// The leaves of the door at (x, y): it and the door tiles in line with
    /// it (a double door, or a gate a few tiles wide).
    pub fn door_leaves(&self, x: i32, y: i32) -> Vec<(i32, i32)> {
        let mut v = vec![];
        if self.obj(x, y) != O_DOOR {
            return v;
        }
        v.push((x, y));
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for k in 1..=3 {
                let (nx, ny) = (x + dx * k, y + dy * k);
                if self.obj(nx, ny) != O_DOOR {
                    break;
                }
                v.push((nx, ny));
            }
        }
        v
    }
    /// Open a door, and its other leaves.
    pub fn open_door(&mut self, x: i32, y: i32) {
        for (lx, ly) in self.door_leaves(x, y) {
            self.door[idx(lx, ly)] = DOOR_OPEN;
        }
    }
    pub fn shut_door(&mut self, x: i32, y: i32) {
        for (lx, ly) in self.door_leaves(x, y) {
            self.door[idx(lx, ly)] = 0.0;
        }
    }
    pub fn level(&self, x: i32, y: i32) -> u8 {
        if inb(x, y) {
            self.lv[idx(x, y)]
        } else {
            0
        }
    }
    /// Place a (possibly multi-tile) object with its anchor on the front tile;
    /// along x when `mirrored`, otherwise along y.
    pub fn place(&mut self, x: i32, y: i32, o: u8, mirrored: bool) {
        let (len, _) = size(o);
        for k in 1..len {
            let (px, py) = if mirrored { (x - k, y) } else { (x, y - k) };
            if inb(px, py) {
                self.o[idx(px, py)] = O_PART;
            }
        }
        if inb(x, y) {
            let i = idx(x, y);
            self.o[i] = o;
            if mirrored {
                self.style[i] |= FACE_S;
            } else {
                self.style[i] &= !FACE_S;
            }
        }
    }
    pub fn is_ramp(&self, x: i32, y: i32) -> bool {
        inb(x, y) && self.ramp[idx(x, y)]
    }
    /// can walk between two neighbouring tiles given their ground levels
    pub fn height_ok(&self, ax: i32, ay: i32, bx: i32, by: i32) -> bool {
        let d = self.level(bx, by) as i32 - self.level(ax, ay) as i32;
        d == 0 || (d.abs() == 1 && (self.is_ramp(ax, ay) || self.is_ramp(bx, by)))
    }
    /// can step from (x,y) by (dx,dy) without cutting corners or climbing a ledge
    pub fn can_step(&self, x: i32, y: i32, dx: i32, dy: i32) -> bool {
        self.can_step_as(x, y, dx, dy, BEAST)
    }
    /// can this walker step from (x,y) by (dx,dy) on its storey; no cutting
    /// corners, and through a doorway only straight
    pub fn can_step_as(&self, x: i32, y: i32, dx: i32, dy: i32, w: Walker) -> bool {
        let p = |x: i32, y: i32| self.passable_as(x, y, w);
        if !p(x + dx, y + dy) || !self.edges_clear(x, y, dx, dy, w.fl) {
            return false;
        }
        if w.fl > 0 {
            return dx == 0 || dy == 0 || (p(x + dx, y) && p(x, y + dy));
        }
        if !self.height_ok(x, y, x + dx, y + dy) {
            return false;
        }
        if dx != 0 && dy != 0 {
            let door = |x: i32, y: i32| self.obj(x, y) == O_DOOR;
            return !door(x, y)
                && !door(x + dx, y + dy)
                && p(x + dx, y)
                && p(x, y + dy)
                && self.height_ok(x, y, x + dx, y)
                && self.height_ok(x, y, x, y + dy)
                && self.height_ok(x + dx, y, x + dx, y + dy);
        }
        true
    }
    /// ground heights (in levels) at a tile's corners: top, right, bottom, left as drawn.
    /// A slope tile lifts the edge it shares with a higher neighbour.
    pub fn corners(&self, x: i32, y: i32) -> [f32; 4] {
        let l = self.level(x, y) as f32;
        let mut c = [l; 4];
        if self.is_ramp(x, y) {
            for (dx, dy, a, b) in [
                (1, 0, 1usize, 2usize),
                (0, 1, 2, 3),
                (-1, 0, 3, 0),
                (0, -1, 0, 1),
            ] {
                let n = self.level(x + dx, y + dy) as f32;
                let t = self.tile(x + dx, y + dy);
                if n == l + 1.0 && t != T_MOUNT && t != T_WATER {
                    c[a] = n;
                    c[b] = n;
                }
            }
        }
        c
    }
    /// height of the ground at a tile's centre, in levels
    pub fn surface(&self, x: i32, y: i32) -> f32 {
        let c = self.corners(x, y);
        (c[0] + c[1] + c[2] + c[3]) / 4.0
    }
    /// Inside the town or the castle city (with a margin): guarded ground.
    pub fn in_town(&self, x: i32, y: i32) -> bool {
        [self.town, self.city]
            .iter()
            .any(|&(x0, y0, x1, y1)| x >= x0 - 3 && y >= y0 - 3 && x < x1 + 3 && y < y1 + 3)
    }

    /// In one of the elven towns.
    pub fn in_elf(&self, x: i32, y: i32) -> bool {
        self.elves
            .iter()
            .any(|&(x0, y0, x1, y1)| x >= x0 && y >= y0 && x < x1 && y < y1)
    }

    /// Inside the village hedge.
    /// inside Duskhold's market square
    pub fn in_market(&self, x: i32, y: i32) -> bool {
        let (x0, y0, x1, y1) = self.market;
        x >= x0 && y >= y0 && x < x1 && y < y1
    }
    /// what the stall on this tile sells, if there is one
    pub fn stall_kind(&self, x: i32, y: i32) -> Option<usize> {
        (inb(x, y) && self.o[idx(x, y)] == O_STALL)
            .then(|| self.style[idx(x, y)] as usize % STALL_WARES.len())
    }

    pub fn in_village(&self, x: i32, y: i32) -> bool {
        let (x0, y0, x1, y1) = self.village;
        x >= x0 && y >= y0 && x < x1 && y < y1
    }

    /// Open ground in the world, under no roof: where the beasts of the
    /// streets go.
    pub fn outdoors(&self, x: i32, y: i32) -> bool {
        inb(x, y) && self.bld[idx(x, y)] == 0 && self.passable(x, y)
    }

    /// Ground the guards keep: the town, the city, the stables, the village
    /// and around each road post.
    pub fn guarded(&self, x: i32, y: i32) -> bool {
        let (sx0, sy0, sx1, sy1) = self.stable;
        self.in_town(x, y)
            || self.in_village(x, y)
            || self.in_elf(x, y)
            || self.in_raj(x, y)
            || self.in_hold(x, y)
            || (x >= sx0 && y >= sy0 && x < sx1 && y < sy1)
            || self
                .posts
                .iter()
                .any(|&(px, py)| (x - px).abs() <= POST_R && (y - py).abs() <= POST_R)
    }

    /// A* to any passable tile within `range` (chebyshev) of goal, for a beast.
    pub fn find_path(
        &mut self,
        sx: i32,
        sy: i32,
        gx: i32,
        gy: i32,
        range: i32,
    ) -> Option<Vec<(i32, i32)>> {
        self.find_path_as(sx, sy, gx, gy, range, BEAST)
    }
    /// A* for this walker, on its storey. Stairs end a path, never lie along
    /// one: stepping on them takes you to another floor.
    pub fn find_path_as(
        &mut self,
        sx: i32,
        sy: i32,
        gx: i32,
        gy: i32,
        range: i32,
        w: Walker,
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
            if expanded > 40000 {
                return None;
            }
            let g = self.pf_g[i];
            if i != s && w.stairs && self.obj_on(x, y, w.fl).0 == O_STAIRS {
                continue;
            }
            for &(dx, dy) in DIRS.iter() {
                if !self.can_step_as(x, y, dx, dy, w) {
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

    /// Lay a wobbling dirt road (bridging water) and return its centre line.
    fn carve_road(&mut self, a: (i32, i32), b: (i32, i32), seed: u32) -> Vec<(i32, i32)> {
        let mut line = vec![];
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
            if line.last() != Some(&(x, y)) {
                line.push((x, y));
            }
            for (ox, oy) in [(0, 0), (1, 0), (0, 1)] {
                let (px, py) = (x + ox, y + oy);
                if !inb(px, py) {
                    continue;
                }
                let i = idx(px, py);
                self.road[i] = true;
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
        line
    }

    /// A guard post halfway along a road: two guards and a lamp beside it.
    fn road_post(&mut self, road: &[(i32, i32)]) {
        let n = road.len();
        if n < 6 {
            return;
        }
        let (px, py) = road[n / 2];
        let (qx, qy) = road[n / 2 + 2];
        self.npcs.push((V_GUARD, px, py));
        self.npcs.push((V_GUARD, qx, qy));
        for (dx, dy) in [(2, -1), (-2, 1), (-1, 2), (1, -2)] {
            let (lx, ly) = (px + dx, py + dy);
            if inb(lx, ly) {
                let i = idx(lx, ly);
                if !self.road[i] && !matches!(self.t[i], T_WATER | T_MOUNT | T_BRIDGE) {
                    self.o[i] = O_LAMP;
                    if inb(lx, ly + 1)
                        && self.o[idx(lx, ly + 1)] == O_NONE
                        && !self.road[idx(lx, ly + 1)]
                    {
                        self.o[idx(lx, ly + 1)] = O_CRATE;
                    }
                    break;
                }
            }
        }
        self.posts.push((px, py));
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
        self.set_door(bx + 3, by + 5);
        self.set_door(bx + 4, by + 5);
        self.set_o(bx + 4, by + 6, O_SIGN);
        self.set_o(bx + 1, by + 6, O_BARREL);
        self.set_o(bx + 6, by + 6, O_CRATE);
        if vendor == V_PROV {
            self.set_o(bx + 2, by + 6, O_SACKS);
        }
        // furniture: along the back wall it faces the door (down-left),
        // along the left wall it faces into the room (down-right)
        let back = |x: i32| (x, by + 1, true);
        let side = |y: i32| (bx + 1, y, false);
        let inn = vendor == V_PROV && style == 2;
        let items: Vec<((i32, i32, bool), u8)> = match vendor {
            // a temple's hall: the image in its shrine at the back between
            // lamps, flowers and offerings, bells and incense
            _ if style == WALL_TEMPLE => vec![
                (back(bx + 3), O_STATUE),
                (back(bx + 2), O_CANDLES),
                (back(bx + 5), O_CANDLES),
                (back(bx + 1), O_PLANT),
                (back(bx + 6), O_PLANT),
                (side(by + 3), O_JARS),
                ((bx + 6, by + 3, false), O_FLOWERS),
                ((bx + 6, by + 4, false), O_CANDLES),
            ],
            V_BANKER => vec![
                (back(bx + 1), O_CHEST),
                (back(bx + 5), O_CHEST),
                (back(bx + 6), O_CHEST),
                (side(by + 3), O_SHELF),
                ((bx + 6, by + 3, false), O_TABLE),
                // the counting-house: a desk, a candle-stand, a plant by the window
                (side(by + 2), O_DESK),
                (back(bx + 4), O_CANDLES),
                ((bx + 6, by + 4, false), O_PLANT),
            ],
            V_MAGE => vec![
                (back(bx + 1), O_SHELF),
                (back(bx + 5), O_SHELF),
                (back(bx + 6), O_SHELF),
                ((bx + 6, by + 3, false), O_TABLE),
                ((bx + 6, by + 4, true), O_CHAIR),
                // a brew on the boil, the heavens on a stand, candles
                (side(by + 3), O_CAULDRON),
                (side(by + 4), O_ORRERY),
                (back(bx + 4), O_CANDLES),
                (back(bx + 2), O_JARS),
            ],
            V_HEALER => vec![
                (back(bx + 5), O_BED),
                (back(bx + 6), O_BED),
                (side(by + 3), O_TABLE),
                (back(bx + 1), O_CHEST),
                // herbs drying, simples in jars, a pot of something warming
                (back(bx + 2), O_HERBS),
                (side(by + 4), O_JARS),
                ((bx + 6, by + 3, false), O_CAULDRON),
                ((bx + 6, by + 4, false), O_PLANT),
                (back(bx + 4), O_CANDLES),
            ],
            V_SMITH => vec![
                (back(bx + 1), O_CHEST),
                (back(bx + 6), O_BARREL),
                ((bx + 6, by + 2, false), O_CRATE),
                (back(bx + 5), O_CRATE),
                // the smith's work on show
                (side(by + 3), O_RACK),
                (back(bx + 2), O_ARMOUR),
                (side(by + 4), O_ARMOUR),
            ],
            V_BARKEEP => vec![
                // kegs and a hearth along the back, two trestle tables with benches
                (back(bx + 1), O_BARREL),
                (back(bx + 2), O_BARREL),
                (back(bx + 5), O_HEARTH),
                (back(bx + 6), O_SHELF),
                ((bx + 1, by + 4, false), O_LONGTABLE),
                ((bx + 2, by + 4, true), O_CHAIR),
                ((bx + 2, by + 3, true), O_CHAIR),
                ((bx + 5, by + 4, false), O_LONGTABLE),
                ((bx + 6, by + 4, true), O_CHAIR),
                ((bx + 6, by + 3, true), O_CHAIR),
                // behind the bar: crockery and jars of pickles
                (back(bx + 3), O_DRESSER),
                (back(bx + 4), O_JARS),
                // a table with a board of Lines on it, for whoever will play
                ((bx + 3, by + 3, false), O_GAMEBOARD),
            ],
            V_STABLE => vec![
                // feed sacks and water barrels, a tack chest, a cart of hay
                (back(bx + 1), O_SACKS),
                (back(bx + 2), O_SACKS),
                (back(bx + 6), O_BARREL),
                (back(bx + 5), O_CHEST),
                ((bx + 6, by + 3, false), O_BARREL),
                (side(by + 3), O_CRATE),
                (side(by + 4), O_STOOL),
            ],
            V_LIBRARIAN => vec![
                // shelves on every wall that faces the room, a reading table
                (back(bx + 1), O_SHELF),
                (back(bx + 2), O_SHELF),
                (back(bx + 5), O_SHELF),
                (back(bx + 6), O_SHELF),
                (side(by + 3), O_SHELF),
                (side(by + 4), O_SHELF),
                ((bx + 5, by + 4, false), O_LONGTABLE),
                ((bx + 6, by + 4, true), O_CHAIR),
                ((bx + 6, by + 3, true), O_CHAIR),
                // the librarian's desk, a globe of the heavens, light to read by
                (side(by + 2), O_DESK),
                (back(bx + 3), O_ORRERY),
                (back(bx + 4), O_CANDLES),
            ],
            _ if inn => vec![
                (back(bx + 5), O_HEARTH),
                (side(by + 1), O_BED),
                (side(by + 2), O_BED),
                ((bx + 5, by + 4, false), O_LONGTABLE),
                ((bx + 6, by + 4, true), O_CHAIR),
                ((bx + 6, by + 3, true), O_CHAIR),
                (back(bx + 6), O_DRESSER),
                (back(bx + 2), O_WARDROBE),
                (side(by + 3), O_PLANT),
                (side(by + 4), O_STOOL),
            ],
            // the mill: sacks of grain and flour, the miller's home in a corner
            V_VILLAGER => vec![
                (back(bx + 1), O_SACKS),
                (back(bx + 2), O_SACKS),
                (back(bx + 5), O_HEARTH),
                (back(bx + 6), O_DRESSER),
                (side(by + 3), O_SACKS),
                (side(by + 4), O_JARS),
                ((bx + 6, by + 3, false), O_TABLE),
                ((bx + 6, by + 4, true), O_STOOL),
            ],
            _ => vec![
                (back(bx + 1), O_SHELF),
                (back(bx + 6), O_CRATE),
                (back(bx + 5), O_BARREL),
                ((bx + 6, by + 3, false), O_BARREL),
                (side(by + 3), O_CRATE),
                (back(bx + 2), O_JARS),
                (side(by + 4), O_SACKS),
                ((bx + 6, by + 4, false), O_PLANT),
            ],
        };
        for ((x, y, south), o) in items {
            if size(o).0 > 1 {
                self.place(x, y, o, south);
            } else if inb(x, y) {
                let i = idx(x, y);
                self.o[i] = o;
                if south {
                    self.style[i] |= FACE_S;
                }
            }
        }
        self.buildings.push(Building {
            x: bx,
            y: by,
            w: 8,
            h: 6,
            vendor,
            style,
            floors: vec![],
            part_of: None,
        });
        self.npcs.push((vendor, bx + 3, by + 2));
    }

    /// A door, open while the world is being made (so the town can reach
    /// inside) and shut when it is done.
    fn set_door(&mut self, x: i32, y: i32) {
        if inb(x, y) {
            self.set_o(x, y, O_DOOR);
            let i = idx(x, y);
            self.door[i] = f32::INFINITY;
            self.doors.push(i);
        }
    }

    /// Index the buildings by tile, and thin the walls of the houses: the
    /// wall tiles round a house become floor, its walls standing on their
    /// outer edges instead (`wall_between`), and what stood a step in from
    /// the back and left walls is set back against them.
    pub fn thin_walls(&mut self) {
        self.bld = vec![0; N];
        for (k, b) in self.buildings.iter().enumerate() {
            for y in b.y..b.y + b.h {
                for x in b.x..b.x + b.w {
                    if inb(x, y) {
                        self.bld[idx(x, y)] = k as u16 + 1;
                    }
                }
            }
        }
        for k in 0..self.buildings.len() {
            if !self.buildings[k].thin() {
                continue;
            }
            let (bx, by, w, h) = {
                let b = &self.buildings[k];
                (b.x, b.y, b.w, b.h)
            };
            let ring = |x: i32, y: i32| x == 0 || y == 0 || x == w - 1 || y == h - 1;
            for y in 0..h {
                for x in 0..w {
                    let i = idx(bx + x, by + y);
                    if ring(x, y) && self.o[i] == O_WALL {
                        self.o[i] = O_NONE;
                    }
                }
            }
            // ground floor: back row first, then the left side
            let mut moves = vec![];
            for x in 1..w - 1 {
                moves.push(((bx + x, by + 1), (bx + x, by)));
            }
            for y in 1..h - 1 {
                moves.push(((bx + 1, by + y), (bx, by + y)));
            }
            for ((fx, fy), (tx, ty)) in moves {
                let (fi, ti) = (idx(fx, fy), idx(tx, ty));
                if movable(self.o[fi]) && self.o[ti] == O_NONE {
                    self.o[ti] = self.o[fi];
                    self.style[ti] = self.style[fi];
                    self.o[fi] = O_NONE;
                    // whoever stood at it (the lord at his throne) steps back with it
                    let front = (fx * 2 - tx, fy * 2 - ty);
                    for n in self.npcs.iter_mut() {
                        if (n.1, n.2) == front {
                            (n.1, n.2) = (fx, fy);
                        }
                    }
                }
            }
            for f in self.buildings[k].floors.iter_mut() {
                for y in 0..h {
                    for x in 0..w {
                        let i = (y * w + x) as usize;
                        if ring(x, y) && f.o[i] == O_WALL {
                            f.o[i] = O_NONE;
                        }
                    }
                }
                let mut moves = vec![];
                for x in 1..w - 1 {
                    moves.push(((x, 1), (x, 0)));
                }
                for y in 1..h - 1 {
                    moves.push(((1, y), (0, y)));
                }
                for ((fx, fy), (tx, ty)) in moves {
                    let (fi, ti) = ((fy * w + fx) as usize, (ty * w + tx) as usize);
                    if movable(f.o[fi]) && f.o[ti] == O_NONE {
                        f.o[ti] = f.o[fi];
                        f.style[ti] = f.style[fi];
                        f.o[fi] = O_NONE;
                    }
                }
            }
        }
    }

    /// Build rooms onto the back of the building at (bx, by): a rectangle
    /// `w`×`h` with its corner at (ax, ay), standing right behind it
    /// (`ay + h == by`), so the house is an L, a T or a step rather than a
    /// box, each part under its own roof. A door in the wall between leads
    /// through (one small thing in the shop's back row is cleared away from
    /// it if it must be). `split`, a column from the corner, divides the
    /// annex into two rooms with a door between. `back` furnishes the back
    /// wall from the left (O_NONE leaves a gap) and `side` the left wall from
    /// the back; `thin_walls` sets them back against the walls later.
    fn annex(
        &mut self,
        bx: i32,
        by: i32,
        (ax, ay, w, h): (i32, i32, i32, i32),
        split: Option<i32>,
        back: &[u8],
        side: &[u8],
    ) {
        let Some(main) = self.buildings.iter().position(|b| b.x == bx && b.y == by) else {
            return;
        };
        let (mw, style, vendor) = {
            let b = &self.buildings[main];
            (b.w, b.style, b.vendor)
        };
        debug_assert_eq!(ay + h, by, "an annex stands right behind its building");
        for y in ay..ay + h {
            for x in ax..ax + w {
                if !inb(x, y) {
                    continue;
                }
                let i = idx(x, y);
                self.t[i] = T_FLOOR;
                self.o[i] = O_NONE;
                self.style[i] = style;
                self.road[i] = false;
                self.ramp[i] = false;
            }
        }
        // the door through: where the shop's back row is clear, nearest the
        // middle of the annex's first room
        let room_end = split.map(|s| ax + s).unwrap_or(ax + w);
        let mid = (ax + room_end - 1) / 2;
        let mut cands: Vec<i32> = (ax.max(bx) + 1..room_end.min(bx + mw) - 1).collect();
        cands.sort_by_key(|&x| (x - mid).abs());
        let small = |o: u8| size(o).0 == 1 && !matches!(o, O_STAIRS | O_HEARTH | O_PART);
        let Some(lx) = cands
            .iter()
            .copied()
            .find(|&x| self.obj(x, by + 1) == O_NONE)
            .or_else(|| cands.iter().copied().find(|&x| small(self.obj(x, by + 1))))
        else {
            return;
        };
        let i = idx(lx, by + 1);
        self.o[i] = O_NONE;
        self.style[i] = style;
        self.set_door(lx, ay + h - 1);
        // and between the two rooms of a split annex
        let inner = split.map(|s| (ax + s - 1, ay + h / 2));
        if let Some((dx, dy)) = inner {
            self.set_door(dx, dy);
        }
        // furniture along the back and the left wall, clear of the doors
        let clear = |x: i32, y: i32| {
            (x == lx && y >= ay + h - 2)
                || inner.is_some_and(|(dx, dy)| (x - dx).abs() <= 2 && y == dy)
        };
        for (k, &o) in back.iter().enumerate() {
            let (x, y) = (ax + 1 + k as i32, ay + 1);
            let wall = split.is_some_and(|s| x == ax + s || x == ax + s - 1);
            if o != O_NONE && x < ax + w - 1 && !wall && !clear(x, y) {
                self.o[idx(x, y)] = o;
                self.style[idx(x, y)] = style | FACE_S;
            }
        }
        for (k, &o) in side.iter().enumerate() {
            let (x, y) = (ax + 1, ay + 2 + k as i32);
            if o != O_NONE && y < ay + h - 1 && !clear(x, y) {
                self.o[idx(x, y)] = o;
            }
        }
        // anyone who stood where the rooms went up steps out of the way
        for n in self.npcs.iter_mut() {
            if n.1 >= ax && n.1 < ax + w && n.2 >= ay && n.2 < ay + h {
                n.2 = ay - 1;
            }
        }
        let rects = match split {
            Some(s) => vec![(ax, s), (ax + s, w - s)],
            None => vec![(ax, w)],
        };
        for (x, pw) in rects {
            self.buildings.push(Building {
                x,
                y: ay,
                w: pw,
                h,
                vendor,
                style,
                floors: vec![],
                part_of: Some(main),
            });
        }
    }

    /// Raise the shop at (bx, by) by `n` storeys with stairs up from the back
    /// corner, and furnish each; `rooms` gives each storey's furniture as
    /// (x, y, facing south) in tiles from the building's corner.
    fn add_floors(&mut self, bx: i32, by: i32, rooms: &[&[(i32, i32, bool, u8)]]) {
        let Some(b) = self.buildings.iter().position(|b| b.x == bx && b.y == by) else {
            return;
        };
        let (w, h, style) = {
            let b = &self.buildings[b];
            (b.w, b.h, b.style)
        };
        // stairs alternate between the back corners, so each flight lands
        // clear of the next
        let stair = |k: usize| {
            if k.is_multiple_of(2) {
                (w - 2, 1)
            } else {
                (1, 1)
            }
        };
        let (gx, gy) = stair(0);
        self.place(bx + gx, by + gy, O_STAIRS, false);
        self.style[idx(bx + gx, by + gy)] = STAIR_UP;
        for (k, room) in rooms.iter().enumerate() {
            let mut f = Floor {
                o: vec![O_NONE; (w * h) as usize],
                style: vec![style; (w * h) as usize],
            };
            for y in 0..h {
                for x in 0..w {
                    if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                        f.o[(y * w + x) as usize] = O_WALL;
                    }
                }
            }
            for &(x, y, south, o) in room.iter() {
                let i = (y * w + x) as usize;
                f.o[i] = o;
                f.style[i] = if south { FACE_S } else { 0 };
            }
            let (dx, dy) = stair(k);
            f.o[(dy * w + dx) as usize] = O_STAIRS;
            f.style[(dy * w + dx) as usize] = STAIR_DOWN;
            if k + 1 < rooms.len() {
                let (ux, uy) = stair(k + 1);
                f.o[(uy * w + ux) as usize] = O_STAIRS;
                f.style[(uy * w + ux) as usize] = STAIR_UP;
            }
            self.buildings[b].floors.push(f);
        }
    }

    /// A run of castle wall from (ax, ay) to (bx, by), inclusive, along x or y.
    fn castle_wall(&mut self, a: (i32, i32), b: (i32, i32)) {
        for y in a.1.min(b.1)..=a.1.max(b.1) {
            for x in a.0.min(b.0)..=a.0.max(b.0) {
                if inb(x, y) && self.o[idx(x, y)] != O_WALL {
                    let i = idx(x, y);
                    self.t[i] = T_COBBLE;
                    self.o[i] = O_WALL;
                    self.style[i] = WALL_CASTLE;
                }
            }
        }
    }
    /// A square tower of `n`×`n` wall tiles with its corner at (x, y).
    fn tower(&mut self, x: i32, y: i32, n: i32) {
        for ty in y..y + n {
            for tx in x..x + n {
                if inb(tx, ty) {
                    let i = idx(tx, ty);
                    self.t[i] = T_COBBLE;
                    self.o[i] = O_WALL;
                    self.style[i] = WALL_TOWER;
                }
            }
        }
    }
    /// The castle keep: a 12×8 hall of castle stone under one great roof, a
    /// throne at the back facing the double door, the lord standing by it.
    fn keep(&mut self, bx: i32, by: i32, lord: usize) {
        self.keep_styled(bx, by, lord, WALL_CASTLE);
    }
    /// A great hall like the keep in another style of stone (the Raja's palace).
    fn keep_styled(&mut self, bx: i32, by: i32, lord: usize, style: u8) {
        let (w, h) = (12, 8);
        for y in by..by + h {
            for x in bx..bx + w {
                let edge = x == bx || x == bx + w - 1 || y == by || y == by + h - 1;
                let i = idx(x, y);
                self.t[i] = T_FLOOR;
                self.o[i] = if edge { O_WALL } else { O_NONE };
                self.style[i] = style;
            }
        }
        let door = bx + w / 2;
        self.set_door(door - 1, by + h - 1);
        self.set_door(door, by + h - 1);
        let back = |x: i32| (x, by + 1, true);
        let side = |y: i32| (bx + 1, y, false);
        let items: [((i32, i32, bool), u8); 22] = [
            (back(door - 1), O_THRONE),
            (back(door - 3), O_HEARTH),
            (back(door + 2), O_HEARTH),
            (back(bx + 1), O_CHEST),
            (back(bx + 2), O_CHEST),
            (back(bx + w - 2), O_SHELF),
            (side(by + 3), O_SHELF),
            (side(by + 4), O_SHELF),
            // the great hall: two long tables with benches either side of the aisle
            ((door - 3, by + 6, false), O_LONGTABLE),
            ((door - 2, by + 6, true), O_CHAIR),
            ((door - 2, by + 5, true), O_CHAIR),
            ((door + 2, by + 6, false), O_LONGTABLE),
            ((door + 3, by + 6, true), O_CHAIR),
            ((door + 3, by + 5, true), O_CHAIR),
            ((bx + w - 2, by + 6, false), O_BARREL),
            // the lord's arms and armour, candles either side of the throne
            (back(door - 2), O_CANDLES),
            (back(door + 1), O_CANDLES),
            (side(by + 5), O_RACK),
            (back(bx + w - 3), O_ARMOUR),
            ((bx + w - 2, by + 2, false), O_ARMOUR),
            (side(by + 2), O_WARDROBE),
            ((bx + w - 2, by + 3, false), O_DRESSER),
        ];
        for ((x, y, south), o) in items {
            self.place(x, y, o, south);
        }
        self.buildings.push(Building {
            x: bx,
            y: by,
            w,
            h,
            vendor: lord,
            style,
            floors: vec![],
            part_of: None,
        });
        self.npcs.push((lord, door - 1, by + 2));
    }
}

impl Map {
    /// Terrace the land from its elevation, keep ledges one level high, give
    /// roads and some ledge feet slopes, then make sure every walkable area
    /// can be reached from the town.
    fn assign_levels(&mut self, elev: &[f32]) {
        let land = |t: u8| t != T_WATER && t != T_MOUNT;
        for i in 0..LAND_N {
            let t = self.t[i];
            self.lv[i] = if land(t) && t != T_SAND && t != T_BRIDGE {
                (((elev[i] - 0.36) / 0.095).floor().clamp(0.0, 3.0)) as u8
            } else {
                0
            };
        }
        // smooth the terraces: each land tile takes the median level around it,
        // which removes one-tile spurs and notches from the ledges
        for _ in 0..2 {
            let prev = self.lv.clone();
            for y in 1..LAND_H - 1 {
                for x in 1..W - 1 {
                    let i = idx(x, y);
                    if !land(self.t[i]) || self.t[i] == T_SAND || self.t[i] == T_BRIDGE {
                        continue;
                    }
                    let mut v: Vec<u8> = Vec::with_capacity(9);
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let j = idx(x + dx, y + dy);
                            if land(self.t[j]) {
                                v.push(prev[j]);
                            }
                        }
                    }
                    v.sort_unstable();
                    self.lv[i] = v[v.len() / 2];
                }
            }
        }
        // every tile of a hex cell takes the level of the cell's own tile,
        // so the ledges follow the cells' edges
        let prev = self.lv.clone();
        for y in 0..LAND_H {
            for x in 0..W {
                let i = idx(x, y);
                if !land(self.t[i]) || self.t[i] == T_SAND || self.t[i] == T_BRIDGE {
                    continue;
                }
                let (q, r) = hex_of(x as f32, y as f32);
                let (hx, hy) = hex_tile(q, r);
                if inb(hx, hy) {
                    self.lv[i] = prev[idx(hx, hy)];
                }
            }
        }
        // the town, the city, their buildings and the shrines sit on one flat level
        let (vx0, vy0, vx1, vy1) = self.village;
        for y in vy0..vy1 {
            for x in vx0..vx1 {
                if land(self.t[idx(x, y)]) {
                    self.lv[idx(x, y)] = 1;
                }
            }
        }
        for (x0, y0, x1, y1) in [self.town, self.city] {
            for y in y0 - 3..y1 + 3 {
                for x in x0 - 3..x1 + 3 {
                    if inb(x, y) {
                        self.lv[idx(x, y)] = 1;
                    }
                }
            }
        }
        for &(ax, ay) in self.ankhs.clone().iter() {
            for y in ay - 2..=ay + 2 {
                for x in ax - 2..=ax + 2 {
                    if inb(x, y) && land(self.t[idx(x, y)]) {
                        self.lv[idx(x, y)] = self.lv[idx(ax, ay)];
                    }
                }
            }
        }
        // bridges and their banks sit at water level
        for i in 0..LAND_N {
            if self.t[i] == T_BRIDGE {
                self.lv[i] = 0;
            }
        }
        // no ledge taller than one level between walkable neighbours
        for _ in 0..12 {
            let mut changed = false;
            for y in 0..LAND_H {
                for x in 0..W {
                    let i = idx(x, y);
                    if !land(self.t[i]) {
                        continue;
                    }
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if !inb(nx, ny) || !land(self.t[idx(nx, ny)]) {
                            continue;
                        }
                        let n = self.lv[idx(nx, ny)];
                        if self.lv[i] > n + 1 {
                            self.lv[i] = n + 1;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        // mountains stand on the highest ground beside them
        for y in 0..LAND_H {
            for x in 0..W {
                let i = idx(x, y);
                if self.t[i] == T_MOUNT {
                    let mut l = 0;
                    for &(dx, dy) in DIRS.iter() {
                        if inb(x + dx, y + dy) && land(self.t[idx(x + dx, y + dy)]) {
                            l = l.max(self.lv[idx(x + dx, y + dy)]);
                        }
                    }
                    self.lv[i] = l;
                }
            }
        }
        // slopes: along roads, and at a quarter of the ledge feet
        for y in 0..LAND_H {
            for x in 0..W {
                let i = idx(x, y);
                if !land(self.t[i]) {
                    continue;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if !inb(nx, ny)
                        || !land(self.t[idx(nx, ny)])
                        || self.lv[idx(nx, ny)] != self.lv[i] + 1
                    {
                        continue;
                    }
                    if (self.road[i] && self.road[idx(nx, ny)]) || hash(x, y, 0x5107) < 0.22 {
                        self.ramp[i] = true;
                    }
                }
            }
        }
        // join any walkable area the town cannot reach with one more slope
        let start = self.start;
        for _ in 0..400 {
            let mut seen = vec![false; N];
            let mut q = std::collections::VecDeque::new();
            seen[idx(start.0, start.1)] = true;
            q.push_back(start);
            while let Some((x, y)) = q.pop_front() {
                for &(dx, dy) in DIRS.iter() {
                    let (nx, ny) = (x + dx, y + dy);
                    if inb(nx, ny) && !seen[idx(nx, ny)] && self.can_step(x, y, dx, dy) {
                        seen[idx(nx, ny)] = true;
                        q.push_back((nx, ny));
                    }
                }
            }
            let mut fixed = false;
            'outer: for y in 0..LAND_H {
                for x in 0..W {
                    if !seen[idx(x, y)] || !self.passable(x, y) {
                        continue;
                    }
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if !inb(nx, ny) || seen[idx(nx, ny)] || !self.passable(nx, ny) {
                            continue;
                        }
                        let (a, b) = (self.lv[idx(x, y)], self.lv[idx(nx, ny)]);
                        if a.abs_diff(b) == 1 {
                            let lower = if a < b { idx(x, y) } else { idx(nx, ny) };
                            self.ramp[lower] = true;
                            fixed = true;
                            break 'outer;
                        }
                    }
                }
            }
            self.reach = seen;
            if !fixed {
                break;
            }
        }
    }
}

// Upper storeys' furniture: (x, y, faces south, object) from the building's
// corner. Stairs go in the back corners, (6, 1) and (1, 1): keep them clear.
const INN_ROOMS: &[(i32, i32, bool, u8)] = &[
    (1, 1, true, O_CHEST),
    (1, 2, false, O_BED),
    (1, 3, false, O_BED),
    (1, 4, false, O_BED),
    (4, 1, true, O_HEARTH),
    (6, 3, true, O_BED),
    (6, 4, true, O_BED),
    (3, 3, false, O_TABLE),
    (3, 4, true, O_CHAIR),
    (2, 1, true, O_WARDROBE),
    (5, 1, true, O_STOOL),
];
const WARD: &[(i32, i32, bool, u8)] = &[
    (1, 1, true, O_SHELF),
    (1, 2, false, O_BED),
    (1, 3, false, O_BED),
    (1, 4, false, O_BED),
    (3, 1, true, O_BED),
    (4, 1, true, O_BED),
    (5, 4, false, O_TABLE),
    (5, 1, true, O_HERBS),
    (6, 4, false, O_JARS),
    (2, 1, true, O_CANDLES),
];
const GALLERY: &[(i32, i32, bool, u8)] = &[
    (1, 1, true, O_SHELF),
    (2, 1, true, O_SHELF),
    (3, 1, true, O_SHELF),
    (4, 1, true, O_SHELF),
    (1, 2, false, O_SHELF),
    (1, 3, false, O_SHELF),
    (1, 4, false, O_SHELF),
    (4, 3, false, O_TABLE),
    (4, 4, true, O_CHAIR),
    (5, 1, true, O_DESK),
    (6, 4, false, O_ORRERY),
];
const STUDY: &[(i32, i32, bool, u8)] = &[
    (2, 1, true, O_SHELF),
    (3, 1, true, O_SHELF),
    (4, 1, true, O_SHELF),
    (1, 3, false, O_SHELF),
    (1, 4, false, O_SHELF),
    (5, 3, false, O_TABLE),
    (5, 4, true, O_CHAIR),
    (6, 4, false, O_DESK),
    (5, 1, true, O_CANDLES),
];
const TOWER_TOP: &[(i32, i32, bool, u8)] = &[
    (6, 1, true, O_CHEST),
    (5, 1, true, O_SHELF),
    (4, 3, false, O_STATUE),
    (6, 4, true, O_BED),
    (2, 4, false, O_TABLE),
    (3, 1, true, O_WARDROBE),
    (1, 3, false, O_CAULDRON),
    (1, 4, false, O_ORRERY),
];

mod bamboo;
mod drowned;
mod dwarves;
mod south;
mod void;
pub use drowned::SALT;
pub use dwarves::HOLD;
pub use south::{KANAK, SURYA};
pub use void::{isle_ids, Isle, ISLE_KEYS, ISLE_LV};

pub fn generate(seed: u32) -> Map {
    let mut rng = Rng::new(seed as u64 ^ 0xABCD);
    let mut m = Map {
        t: vec![T_GRASS; N],
        o: vec![O_NONE; N],
        shade: vec![0; N],
        style: vec![0; N],
        res: vec![12; N],
        regrow: vec![],
        town: (62, 64, 98, 104),
        stable: (98, 80, 110, 99),
        village: (12, 62, 50, 98),
        rivers: vec![],
        elves: vec![],
        paddock: vec![],
        city: (60, 14, 100, 56),
        posts: vec![],
        start: (80, 79),
        npcs: vec![],
        spawns: vec![],
        ankhs: vec![],
        buildings: vec![],
        pois: vec![],
        roads: vec![],
        gloom: vec![0; N],
        withered: vec![false; N],
        bld: vec![],
        door: vec![0.0; N],
        doors: vec![],
        guests: vec![],
        market: (0, 0, 0, 0),
        stalls: vec![],
        tropic_line: vec![],
        raj: vec![],
        hold: (0, 0, 0, 0),
        rlyeh: None,
        gates: vec![],
        isles: vec![],
        groves: vec![],
        pf_g: vec![0.0; N],
        pf_par: vec![-1; N],
        pf_stamp: vec![0; N],
        pf_gen: 0,
        lv: vec![0; N],
        ramp: vec![false; N],
        road: vec![false; N],
        reach: vec![false; N],
    };
    // below the land, nothing yet: the Void is made last (`world/void.rs`)
    for i in LAND_N..N {
        m.t[i] = T_WATER;
    }
    let town_c = (80, 84);
    let grave = (124, 32);
    let orcs = (32, 124);
    let hills = (34, 34);
    let lair = (128, 128);
    let city_c = (80, 35);
    let stable_c = (104, 89);
    let village_c = (31, 80);
    let pois: [((i32, i32), f32); 10] = [
        (village_c, 24.0),
        (WOOD_C, 32.0),
        (HAVEN_C, 12.0),
        (town_c, 30.0),
        (stable_c, 14.0),
        (city_c, 30.0),
        (grave, 15.0),
        (orcs, 14.0),
        (hills, 15.0),
        (lair, 15.0),
    ];

    let mut elev = vec![0f32; N];
    let mut moist = vec![0f32; N];
    // which tiles were grass before the land ran on south past the old shore
    let mut old_grass = vec![false; N];
    for y in 0..LAND_H {
        for x in 0..W {
            let i = idx(x, y);
            let noise = fbm(x as f32 / 26.0, y as f32 / 26.0, seed);
            // the height of the land, falling away to the sea within 12
            // tiles of the shore `bottom` (the old one or the new)
            let height = |bottom: i32| {
                let mut e = noise;
                let edge = x.min(y).min(W - 1 - x).min(bottom - 1 - y) as f32;
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
                e
            };
            let e = height(LAND_H);
            elev[i] = e;
            if y < NORTH_H {
                let old = if y < NORTH_H - 12 { e } else { height(NORTH_H) };
                old_grass[i] = (0.36..0.69).contains(&old);
            }
            moist[i] = fbm(x as f32 / 18.0 + 50.0, y as f32 / 18.0 + 50.0, seed ^ 0x77);
            m.shade[i] = (hash(x, y, seed ^ 0x55) * 255.0) as u8;
        }
    }
    // the new southern lands draw on their own stream of chance, so the
    // northern ones come out as they always have for a seed: a tile takes
    // from the old stream exactly when it was grass before
    let mut trng = Rng::new(seed as u64 ^ 0x7209);
    for i in 0..LAND_N {
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
        let grass = m.t[i] == T_GRASS;
        let draws = if old_grass[i] {
            let r = rng.f();
            (r, if mo < 0.35 { rng.f() } else { 1.0 })
        } else if grass {
            let r = trng.f();
            (r, if mo < 0.35 { trng.f() } else { 1.0 })
        } else {
            (1.0, 1.0)
        };
        if grass {
            let r = draws.0;
            if mo > 0.56 && r < 0.42 {
                m.o[i] = O_TREE;
            } else if mo > 0.5 && r > 0.965 {
                m.o[i] = O_MUSHROOM;
            } else if r < 0.025 {
                m.o[i] = O_TREE;
            } else if r < 0.045 {
                m.o[i] = O_BUSH;
            } else if r < 0.06 {
                m.o[i] = O_FLOWERS;
            }
        }
        if grass && mo < 0.35 && draws.1 < 0.05 {
            m.t[i] = T_DIRT;
        }
    }

    // --- the mountains: a great range down the east, the valley of the elves
    // hidden in it, and a lower range across the south
    m.range(
        &mut elev,
        &[
            (168, 6),
            (176, 28),
            (180, 50),
            (186, 75),
            (184, 100),
            (178, 124),
            (172, 146),
        ],
        15.0,
        seed ^ 0x3a11,
    );
    m.range(
        &mut elev,
        &[(14, 188), (50, 181), (90, 191), (130, 184), (164, 193)],
        12.0,
        seed ^ 0x3a12,
    );

    m.pois.push(("Eastern range", 174, 40));
    m.pois.push(("Southern range", 90, 189));
    // passes over the ranges: old tracks, so the lands beyond are not cut off
    for (k, (a, b)) in [
        ((50, 168), (50, 206)),
        ((118, 166), (118, 206)),
        ((156, 26), (206, 26)),
        ((158, 128), (204, 126)),
    ]
    .into_iter()
    .enumerate()
    {
        m.carve_road(a, b, seed ^ (0x9a55 + k as u32));
    }

    // --- rivers: one from the northern hills down through the village to the
    // western sea, one from the eastern uplands through the range to the
    // south-eastern shore
    m.carve_river(
        &[
            (52, 18),
            (48, 44),
            (38, 58),
            (32, 68),
            (30, 80),
            (27, 92),
            (16, 102),
            (-2, 108),
        ],
        seed ^ 0x7177,
    );
    m.carve_river(
        &[
            (118, 58),
            (126, 70),
            (134, 84),
            (146, 96),
            (162, 100),
            (176, 116),
            (194, 134),
            (205, 162),
            (222, 172),
            (244, 178),
        ],
        seed ^ 0x7178,
    );

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
    for y in grave.1 - 14..grave.1 + 14 {
        for x in grave.0 - 16..grave.0 + 16 {
            if !inb(x, y) {
                continue;
            }
            let i = idx(x, y);
            if m.o[i] == O_TREE {
                m.style[i] = 1;
            } else if m.o[i] == O_NONE && m.t[i] != T_WATER && m.t[i] != T_MOUNT && rng.f() < 0.05 {
                m.o[i] = O_BONES;
            }
        }
    }
    for (dx, dy) in [(-7, -5), (6, 4), (-4, 5)] {
        m.set_o(grave.0 + dx, grave.1 + dy, O_SKULLS);
    }
    m.set_o(grave.0 + 3, grave.1 - 4, O_STATUE);
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
    for (dx, dy) in [(-2, 6), (3, -2), (-6, -1), (6, 6)] {
        m.set_o(
            orcs.0 + dx,
            orcs.1 + dy,
            if dx > 0 {
                O_BARREL
            } else if dy > 0 {
                O_SKULLS
            } else {
                O_BONES
            },
        );
    }
    for (dx, dy) in [(-5, -3), (4, -5), (5, 3), (-3, 5), (-6, 2)] {
        m.set_o(orcs.0 + dx, orcs.1 + dy, O_TENT);
    }
    m.set_o(orcs.0 + 2, orcs.1 + 2, O_SACKS);
    m.set_o(orcs.0 - 3, orcs.1 - 4, O_SKULLS);
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
                } else if m.o[i] == O_NONE && d > 3.0 && rng.f() < 0.04 {
                    m.o[i] = O_TREE;
                    m.style[i] = 1;
                } else if m.o[i] == O_NONE && rng.f() < 0.03 {
                    m.o[i] = O_BONES;
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
    for _ in 0..14 {
        let (x, y) = (lair.0 + rng.range(-5, 5), lair.1 + rng.range(-5, 5));
        if m.tile(x, y) == T_CAVE && m.obj(x, y) == O_NONE {
            m.set_o(x, y, O_BONES);
        }
    }
    for (dx, dy) in [(-3, 3), (1, 4), (4, 1), (-1, -4)] {
        m.set_o(lair.0 + dx, lair.1 + dy, O_SKULLS);
    }
    m.set_o(lair.0 + 2, lair.1 - 2, O_HOARD);
    m.set_o(lair.0 + 3, lair.1 - 1, O_HOARD);
    m.pois.push(("Dragon Lair", lair.0, lair.1));

    // --- roads from the town gates
    let (x0, y0, x1, y1) = m.town;
    for (a, b, sd, to) in [
        ((x1, y0 + 13), (grave.0 - 2, grave.1 + 8), 1, "graveyard"),
        ((x0, y0 + 13), (orcs.0 + 6, orcs.1 - 8), 2, "orcs"),
        (((x0 + x1) / 2, y0), (hills.0 + 8, hills.1 + 8), 3, "hills"),
        (((x0 + x1) / 2, y1), (lair.0 - 13, lair.1), 4, "lair"),
    ] {
        let tiles = m.carve_road(a, b, seed ^ sd);
        m.roads.push(Road {
            from: "town",
            to,
            tiles,
        });
    }
    let last = m.carve_road((lair.0 - 13, lair.1), (lair.0 - 7, lair.1), seed ^ 5);
    if let Some(r) = m.roads.last_mut() {
        r.tiles.extend(last.into_iter().skip(1));
    }
    // and from the city's gates: south to the town, east to the graveyard, west to the hills
    let (cx0, cy0, cx1, cy1) = m.city;
    // each with a guard post halfway
    for (a, b, sd, to) in [
        (((cx0 + cx1) / 2, cy1), ((x0 + x1) / 2, y0), 6, "town"),
        ((cx1, cy0 + 18), (grave.0 - 12, grave.1), 7, "graveyard"),
        ((cx0 - 1, cy0 + 18), (hills.0 + 12, hills.1 + 4), 8, "hills"),
    ] {
        let road = m.carve_road(a, b, seed ^ sd);
        m.road_post(&road);
        m.roads.push(Road {
            from: "city",
            to,
            tiles: road,
        });
    }

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
    // the inn is a provisioner in a timber house (style 2)
    m.building(x0 + 26, y0 + 16, 2, V_PROV);
    // the south quarter: a pub and a library either side of the south gate
    m.building(x0 + 2, y0 + 30, 2, V_BARKEEP);
    m.building(x0 + 26, y0 + 30, 1, V_LIBRARIAN);
    // upper storeys: rooms over the inn and the pub, a ward over the
    // healer's, a gallery over the library and the mage's tower
    m.add_floors(x0 + 26, y0 + 16, &[INN_ROOMS]);
    m.add_floors(x0 + 2, y0 + 30, &[INN_ROOMS]);
    m.guests.push((V_FOLK, x0 + 2 + 4, y0 + 30 + 3, 1));
    m.add_floors(x0 + 26, y0 + 2, &[WARD]);
    m.add_floors(x0 + 26, y0 + 30, &[GALLERY]);
    m.add_floors(x0 + 14, y0 + 2, &[STUDY, TOWER_TOP]);
    // rooms built on behind: the banker's strongroom, a sickroom and a
    // dispensary behind the healer's, the pub's kitchen, the library's
    // archive; so the houses stand in Ls, Ts and steps
    m.annex(
        x0 + 2,
        y0 + 2,
        (x0 + 1, y0 - 2, 6, 4),
        None,
        &[O_CHEST, O_CHEST, O_SHELF, O_CHEST],
        &[O_CANDLES],
    );
    m.annex(
        x0 + 26,
        y0 + 2,
        (x0 + 28, y0 - 2, 8, 4),
        Some(4),
        &[O_BED, O_HERBS, O_NONE, O_NONE, O_JARS, O_SHELF],
        &[],
    );
    m.annex(
        x0 + 2,
        y0 + 30,
        (x0 + 4, y0 + 26, 6, 4),
        None,
        &[O_HEARTH, O_BARREL, O_JARS, O_SACKS],
        &[O_TABLE],
    );
    m.annex(
        x0 + 26,
        y0 + 30,
        (x0 + 25, y0 + 26, 7, 4),
        None,
        &[O_SHELF, O_SHELF, O_DESK, O_SHELF, O_SHELF],
        &[O_CANDLES],
    );
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
        (12, 33),
        (23, 33),
        (1, 38),
        (34, 38),
    ] {
        m.set_o(x0 + dx, y0 + dy, O_LAMP);
    }
    for dx in [4, 8, 16, 20, 28, 32] {
        m.set_o(x0 + dx, y0 + 9, O_FLOWERS);
    }
    m.set_o(x0 + 10, y0 + 11, O_STATUE);
    m.set_o(x0 + 25, y0 + 11, O_STATUE);
    m.set_o(x0 + 30, y0 + 13, O_WELL);
    m.set_o(x0 + 5, y0 + 13, O_WELL);
    m.place(x0 + 11, y0 + 25, O_CART, true);
    m.place(x0 + 24, y0 + 25, O_CART, false);
    // drinkers' benches outside the pub, a well and a statue by the library
    m.place(x0 + 11, y0 + 31, O_LONGTABLE, false);
    m.place(x0 + 12, y0 + 31, O_CHAIR, true);
    m.place(x0 + 12, y0 + 30, O_CHAIR, true);
    m.set_o(x0 + 16, y0 + 37, O_BARREL);
    m.set_o(x0 + 24, y0 + 31, O_STATUE);
    m.set_o(x0 + 21, y0 + 37, O_WELL);
    m.npcs.push((V_GUARD, x0 + 15, y0 + 11));
    for (dx, dy) in [(8, 12), (27, 14), (20, 26), (4, 27), (14, 35), (22, 29)] {
        m.npcs.push((V_FOLK, x0 + dx, y0 + dy));
    }
    m.npcs.push((V_GUARD, x0 + 1, y0 + 13));
    m.npcs.push((V_GUARD, x1 - 1, y0 + 14));
    m.npcs.push((V_GUARD, x0 + 17, y1 - 2));
    m.start = (x0 + 18, y0 + 15);
    m.pois.push(("Town", town_c.0, town_c.1));
    m.pois.push(("Pub", x0 + 6, y0 + 37));
    m.pois.push(("Library", x0 + 30, y0 + 37));
    stables(&mut m, &mut rng);

    city(&mut m);

    // --- the village of the small folk, west along a lane from the town
    m.village(&mut rng, seed, (x0 - 1, y0 + 13));
    m.name_rivers();

    // --- the elves: a valley in the mountains, a golden wood, a haven
    m.elven_lands(&mut elev, &mut rng, seed, ((x0 + x1) / 2, y1));

    // --- the tropics south of the southern range, and their cities
    m.tropics(&mut elev, &moist, seed);

    // --- Karak Dhum, the dwarf hold in the eastern range
    m.dwarf_hold(&mut elev, seed);

    m.thin_walls();
    // --- spawn regions (monster, x, y, radius, max)
    m.assign_levels(&elev);
    m.spawns = vec![
        ("rabbit", 82, 116, 9, 5),
        ("rabbit", 48, 80, 10, 4),
        ("rabbit", 108, 70, 9, 4),
        ("rat", 104, 100, 8, 3),
        ("rat", 56, 98, 8, 3),
        ("mongbat", 48, 56, 7, 3),
        ("mongbat", 130, 80, 10, 3),
        ("slime", 82, 128, 10, 3),
        ("slime", 110, 52, 8, 2),
        ("wolf", 20, 48, 11, 4),
        // the new lands east and south
        ("wolf", 150, 30, 12, 4),
        ("wolf", 60, 206, 12, 4),
        ("troll", 198, 14, 12, 2),
        ("troll", 120, 176, 12, 2),
        ("mongbat", 210, 100, 10, 3),
        ("rabbit", 130, 160, 10, 5),
        ("rabbit", 30, 220, 10, 4),
        ("horse", 150, 206, 12, 4),
        ("slime", 100, 215, 10, 3),
        ("rat", 140, 120, 9, 3),
        // great spiders in the deep woods
        ("spider", 60, 206, 10, 3),
        ("spider", 150, 30, 10, 3),
        ("spider", 20, 48, 9, 2),
        ("wolf", 112, 110, 11, 3),
        ("horse", 46, 94, 10, 4),
        ("skeleton", grave.0, grave.1, 8, 5),
        ("zombie", grave.0, grave.1, 8, 4),
        ("lich", grave.0, grave.1, 3, 1),
        ("orc", orcs.0, orcs.1, 8, 6),
        ("orclord", orcs.0, orcs.1, 3, 1),
        ("troll", hills.0, hills.1, 10, 2),
        ("ettin", hills.0, hills.1, 10, 2),
        ("dragon", lair.0, lair.1, 2, 1),
        // the tropics: tigers in the jungle, great spiders and slimes in
        // the wet, rabbits in the clearings
        ("tiger", 96, 262, 14, 3),
        ("tiger", 186, 268, 14, 3),
        ("tiger", 70, 296, 10, 2),
        ("spider", 150, 248, 12, 3),
        ("spider", 20, 280, 10, 2),
        ("slime", 112, 302, 10, 3),
        ("rabbit", 96, 224, 10, 4),
        ("rabbit", 200, 300, 10, 4),
    ];
    let beasts = street_beasts(&m, town_c);
    m.spawns.extend(beasts);
    // drowned Saltgrave: its folk in the street, their priest at the idol,
    // great crabs on the shore
    let (sx, sy, sw, sh) = SALT;
    m.spawns.extend([
        ("saltfolk", sx + 14, sy + 10, 11, 7),
        ("deeppriest", sx + 15, sy + 11, 2, 1),
        ("crab", sx + sw - 3, sy + sh / 2, 7, 4),
    ]);
    // every door was left open while the world was made; shut them now
    for &i in &m.doors {
        m.door[i] = 0.0;
    }
    // bamboo in the tropic hills, with its pandas
    m.bamboo_groves(seed);
    // and last, below it all, the Ethereal Void
    m.make_void(seed);
    m
}

/// The beasts of the streets (`game/critters.rs`): flocks of pigeons on the
/// squares, dogs and cats about the town, Duskhold and the village, and in
/// the two cities of the south cows and pigs wandering the streets as well,
/// and troops of monkeys.
fn street_beasts(m: &Map, town_c: (i32, i32)) -> Vec<(&'static str, i32, i32, i32, usize)> {
    let centre = |(x0, y0, x1, y1): (i32, i32, i32, i32)| ((x0 + x1) / 2, (y0 + y1) / 2);
    let mut v = vec![];
    // a few dogs and cats about each of these spots, so they're met with
    // wherever one walks, not thinly spread over the whole place
    let dogs_cats = |v: &mut Vec<_>, at: &[(i32, i32)], dogs: usize, cats: usize| {
        for &(x, y) in at {
            v.push(("dog", x, y, 6, dogs));
            v.push(("cat", x, y, 5, cats));
        }
    };
    // the town: flocks on the square and by the well; dogs and cats by the
    // start, the square, the pub and the library
    let (x0, y0, _, _) = m.town;
    v.push(("pigeon", town_c.0, town_c.1, 4, 6));
    v.push(("pigeon", x0 + 21, y0 + 34, 3, 5));
    let (sx, sy) = m.start;
    dogs_cats(
        &mut v,
        &[(sx, sy + 2), town_c, (x0 + 8, y0 + 33), (x0 + 28, y0 + 33)],
        2,
        2,
    );
    // Duskhold: flocks on the market and in the castle bailey; dogs and cats
    // by the south gate, the market, the pub and the bailey
    let (cx0, cy0, _, cy1) = m.city;
    let (cx, _) = centre(m.city);
    let (mx, my) = centre(m.market);
    v.push(("pigeon", mx, my, 5, 7));
    v.push(("pigeon", cx, cy0 + 12, 4, 5));
    dogs_cats(
        &mut v,
        &[
            (cx, cy1 - 5),
            (mx, my),
            (cx0 + 15, cy0 + 26),
            (cx, cy0 + 16),
        ],
        2,
        2,
    );
    // the village
    let (vx, vy) = centre(m.village);
    let (vx0, _, vx1, _) = m.village;
    dogs_cats(
        &mut v,
        &[(vx, vy), ((vx0 + vx) / 2, vy), ((vx + vx1) / 2, vy)],
        1,
        1,
    );
    // Suryagarh and Kanakpur: their squares and streets
    for &r in &m.raj {
        let (rx, ry) = centre(r);
        let (rx0, ry0, rx1, ry1) = r;
        let (qx, qy) = ((rx1 - rx0) / 4, (ry1 - ry0) / 4);
        v.push(("pigeon", rx, ry, 5, 7));
        let spots = [
            (rx, ry),
            (rx - qx, ry - qy),
            (rx + qx, ry - qy),
            (rx - qx, ry + qy),
            (rx + qx, ry + qy),
        ];
        dogs_cats(&mut v, &spots, 2, 1);
        for &(x, y) in &spots {
            v.push(("cow", x, y, 7, 2));
            v.push(("pig", x, y, 6, 1));
        }
    }
    // troops of monkeys about the temples, Suryagarh's square and
    // Kanakpur's bazaar
    let (sx, sy, sw, sh) = south::SURYA;
    let (kx, ky, kw, _) = south::KANAK;
    let troops = [
        (sx + 36, sy + 11, 5),
        (sx + sw / 2 + 4, sy + sh / 2, 3),
        (kx + kw / 2, ky + 11, 5),
        (kx + 10, ky + 13, 4),
    ];
    if !m.raj.is_empty() {
        for (x, y, n) in troops {
            v.push(("monkey", x, y, 6, n));
        }
    }
    v
}

/// The stables east of the town: a timber barn with the stablemaster, and a
/// paddock fenced with split rails where his horses graze.
fn stables(m: &mut Map, rng: &mut Rng) {
    let (x0, y0, x1, y1) = m.stable;
    for y in y0..y1 {
        for x in x0..x1 {
            let t = if rng.f() < 0.35 { T_DIRT } else { T_GRASS };
            m.set(x, y, t, O_NONE);
        }
    }
    let (bx, by) = (x0 + 2, y0 + 2);
    m.building(bx, by, 2, V_STABLE);
    // paddock: a ring of rails with a gate facing the barn door
    let (px0, py0, px1, py1) = (x0 + 1, y0 + 10, x0 + 10, y0 + 17);
    for y in py0..=py1 {
        for x in px0..=px1 {
            let edge = x == px0 || x == px1 || y == py0 || y == py1;
            let gate = y == py0 && (x == bx + 3 || x == bx + 4);
            if edge && !gate {
                m.set(x, y, T_GRASS, O_FENCE);
                m.style[idx(x, y)] = 1;
            }
        }
    }
    m.set_o(px1 - 1, py1 - 1, O_SACKS);
    m.set_o(px0 + 1, py1 - 1, O_BARREL);
    m.place(x1 - 1, by + 3, O_CART, false);
    m.paddock = vec![(px0 + 2, py0 + 3), (px0 + 6, py0 + 2), (px0 + 4, py0 + 5)];
    m.pois.push(("Stables", bx + 4, by + 6));
}

/// Duskhold: a walled city north of the town. Its castle fills the northern
/// half (a bailey with corner towers around the keep), houses and a market
/// square the southern half; gates open south, west and east.
fn city(m: &mut Map) {
    let (x0, y0, x1, y1) = m.city;
    let (xe, ye) = (x1 - 1, y1 - 1);
    // the south gate and the main street
    let mid = (x0 + x1) / 2;
    // a cleared field around the walls
    for y in y0 - 3..y1 + 3 {
        for x in x0 - 3..x1 + 3 {
            if inb(x, y) && matches!(m.o[idx(x, y)], O_TREE | O_BUSH | O_MUSHROOM | O_BOULDER) {
                m.o[idx(x, y)] = O_NONE;
            }
        }
    }
    for y in y0..y1 {
        for x in x0..x1 {
            m.set(x, y, T_COBBLE, O_NONE);
            m.style[idx(x, y)] = 0;
        }
    }
    // the city wall, with three-tile gates south, west and east
    let gy = y0 + 17; // west and east gates span gy..gy+2
    m.castle_wall((x0, y0), (xe, y0));
    m.castle_wall((x0, ye), (mid - 2, ye));
    m.castle_wall((mid + 2, ye), (xe, ye));
    m.castle_wall((x0, y0), (x0, gy - 1));
    m.castle_wall((x0, gy + 3), (x0, ye));
    m.castle_wall((xe, y0), (xe, gy - 1));
    m.castle_wall((xe, gy + 3), (xe, ye));
    for (tx, ty) in [(x0, y0), (xe - 2, y0), (x0, ye - 2), (xe - 2, ye - 2)] {
        m.tower(tx, ty, 3);
    }
    // gatehouse towers either side of each gate
    m.tower(mid - 3, ye - 1, 2);
    m.tower(mid + 2, ye - 1, 2);
    m.tower(x0, gy - 2, 2);
    m.tower(x0, gy + 3, 2);
    m.tower(xe - 1, gy - 2, 2);
    m.tower(xe - 1, gy + 3, 2);

    // the castle bailey against the north wall, its gate on the main street
    let (bx0, bx1, by1) = (x0 + 8, x0 + 31, y0 + 16);
    m.castle_wall((bx0, y0), (bx0, by1));
    m.castle_wall((bx1, y0), (bx1, by1));
    m.castle_wall((bx0, by1), (mid - 2, by1));
    m.castle_wall((mid + 1, by1), (bx1, by1));
    for (tx, ty) in [(bx0, y0), (bx1 - 2, y0), (bx0, by1 - 2), (bx1 - 2, by1 - 2)] {
        m.tower(tx, ty, 3);
    }
    m.tower(mid - 3, by1 - 1, 2);
    m.tower(mid + 1, by1 - 1, 2);
    m.keep(mid - 6, y0 + 3, V_LORD);
    // the courtyard
    m.set_o(bx0 + 4, y0 + 13, O_WELL);
    for (dx, dy) in [(13, 11), (26, 11)] {
        m.set_o(x0 + dx, y0 + dy, O_LAMP);
    }
    for (dx, dy, o) in [
        (11, 4, O_BARREL),
        (11, 5, O_BARREL),
        (12, 4, O_CRATE),
        (28, 4, O_CRATE),
        (28, 5, O_SACKS),
        (27, 4, O_BARREL),
        (27, 13, O_STATUE),
    ] {
        m.set_o(x0 + dx, y0 + dy, o);
    }
    m.place(x0 + 24, y0 + 14, O_CART, true);
    // the yards between the bailey and the city wall: a soldiers' camp and stores
    m.set_o(x0 + 3, y0 + 5, O_TENT);
    m.set_o(x0 + 5, y0 + 9, O_TENT);
    m.set_o(x0 + 3, y0 + 12, O_TENT);
    m.set_o(x0 + 5, y0 + 6, O_FIRE);
    for (dx, dy, o) in [
        (34, 5, O_CRATE),
        (35, 5, O_CRATE),
        (36, 5, O_BARREL),
        (36, 6, O_SACKS),
        (34, 12, O_BARREL),
        (36, 12, O_CRATE),
    ] {
        m.set_o(x0 + dx, y0 + dy, o);
    }
    m.npcs.push((V_FOLK, x0 + 4, y0 + 8));
    m.npcs.push((V_GUARD, mid - 1, by1 - 2));
    m.npcs.push((V_GUARD, mid + 2, y0 + 12));
    m.npcs.push((V_GUARD, bx0 + 3, y0 + 8));

    // the first row of houses faces the castle across the street, the pub
    // right beside its gate; a second row and the market square lie below
    let hy = y0 + 21;
    m.building(x0 + 2, hy, 4, V_SMITH);
    m.building(x0 + 11, hy, 3, V_BARKEEP);
    m.building(x0 + 22, hy, 2, V_PROV);
    m.building(x0 + 31, hy, 1, V_MAGE);
    m.add_floors(x0 + 11, hy, &[INN_ROOMS]);
    m.guests.push((V_FOLK, x0 + 11 + 4, hy + 3, 1));
    m.add_floors(x0 + 31, hy, &[STUDY, TOWER_TOP]);
    // behind the smithy a store, and behind the inn two bedrooms
    m.annex(
        x0 + 2,
        hy,
        (x0 + 2, hy - 4, 6, 4),
        None,
        &[O_CRATE, O_BARREL, O_SACKS, O_CRATE],
        &[O_RACK],
    );
    m.annex(
        x0 + 22,
        hy,
        (x0 + 22, hy - 4, 8, 4),
        Some(4),
        &[O_BED, O_WARDROBE, O_NONE, O_NONE, O_BED, O_DRESSER],
        &[],
    );
    // drinkers' benches out in the street, under the castle walls
    m.place(x0 + 13, y0 + 19, O_LONGTABLE, false);
    m.place(x0 + 14, y0 + 19, O_CHAIR, true);
    m.place(x0 + 14, y0 + 18, O_CHAIR, true);
    m.set_o(x0 + 16, y0 + 19, O_BARREL);
    let hy2 = y0 + 30;
    m.building(x0 + 2, hy2, 3, V_HEALER);
    m.building(x0 + 31, hy2, 4, V_PROV);
    m.set_o(x0 + 11, y0 + 31, O_FORGE);
    m.set_o(x0 + 13, y0 + 31, O_ANVIL);
    // the market square: a fountain, lamps and a few stalls
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        m.set_o(mid - 1 + dx, y0 + 34 + dy, O_FOUNTAIN);
    }
    for (dx, dy) in [(3, 18), (36, 18), (10, 28), (30, 28), (12, 39), (27, 39)] {
        m.set_o(x0 + dx, y0 + dy, O_LAMP);
    }
    // the market: six stalls of two tiles each round the fountain, each
    // keeper at the end of their counter, and a crowd of shoppers going
    // between them
    m.market = (x0 + 11, y0 + 29, x0 + 30, y0 + 41);
    for (k, (dx, dy)) in [(16, 31), (23, 31), (12, 37), (16, 37), (23, 37), (27, 37)]
        .into_iter()
        .enumerate()
    {
        for sx in [dx, dx + 1] {
            m.set_o(x0 + sx, y0 + dy, O_STALL);
            m.style[idx(x0 + sx, y0 + dy)] = k as u8;
            m.stalls.push((x0 + sx, y0 + dy + 1));
        }
        m.npcs.push((V_STALL, x0 + dx - 1, y0 + dy));
    }
    for (dx, dy, o) in [
        (18, 30, O_BARREL),
        (25, 30, O_CRATE),
        (14, 36, O_SACKS),
        (29, 36, O_CRATE),
        (29, 37, O_BARREL),
    ] {
        m.set_o(x0 + dx, y0 + dy, o);
    }
    m.place(x0 + 15, y0 + 40, O_CART, true);
    for (dx, dy) in [
        (13, 32),
        (15, 33),
        (18, 33),
        (21, 32),
        (25, 33),
        (28, 32),
        (14, 34),
        (23, 35),
        (26, 35),
        (12, 38),
        (15, 39),
        (18, 38),
        (21, 39),
        (24, 38),
        (27, 39),
        (29, 34),
    ] {
        m.npcs.push((V_FOLK, x0 + dx, y0 + dy));
    }
    m.pois.push(("Market", mid, y0 + 33));
    m.npcs.push((V_GUARD, mid + 2, ye - 2));
    m.npcs.push((V_GUARD, x0 + 2, gy + 1));
    m.npcs.push((V_GUARD, xe - 2, gy + 1));
    for (dx, dy) in [(8, 19), (30, 18), (16, 33), (24, 36), (10, 38), (33, 38)] {
        m.npcs.push((V_FOLK, x0 + dx, y0 + dy));
    }
    m.pois.push(("Duskhold", mid, y0 + 30));
    m.pois.push(("Duskhold Pub", x0 + 15, y0 + 28));
    m.pois.push(("Castle", mid, y0 + 12));
}

/// Minimap colour for a tile, RGBA bytes.
/// the deepest Shadow
pub const GLOOM_MAX: u8 = 6;

pub fn mini_color(m: &Map, i: usize) -> [u8; 4] {
    let c: u32 = match (m.t[i], m.o[i]) {
        (_, O_WALL) => 0x5a4a3aff,
        (_, O_CART) | (_, O_PART) => 0x5a4028ff,
        (_, O_STALL) => 0xb05a30ff,
        (_, O_TREE) => 0x2e5a22ff,
        (_, O_BAMBOO) => 0x6a8a2aff,
        (T_WATER, _) => 0x2a5582ff,
        (T_SAND, _) => 0xcdb882ff,
        (T_GRASS, _) => 0x5b8a38ff,
        (T_DIRT, _) => 0x86684aff,
        (T_COBBLE, _) => 0x9a9488ff,
        (T_FLOOR, _) => 0x8f6a42ff,
        (T_MOUNT, _) => 0x6e675cff,
        (T_BRIDGE, _) => 0x7a5432ff,
        (T_CAVE, _) => 0x40382fff,
        (T_ASTRAL, _) => 0x8a80a8ff,
        _ => 0x000000ff,
    };
    let s = (m.shade[i] as f32 / 255.0 * 0.15 + 0.86 + m.lv[i] as f32 * 0.06)
        * (1.0 - m.gloom[i] as f32 / GLOOM_MAX as f32 * 0.55);
    [
        (((c >> 24) & 255) as f32 * s).min(255.0) as u8,
        (((c >> 16) & 255) as f32 * s).min(255.0) as u8,
        (((c >> 8) & 255) as f32 * s).min(255.0) as u8,
        255,
    ]
}

// ---------------------------------------------------------------- rivers and the village
impl Map {
    /// A river along `pts`, wobbling between them: two tiles wide, three on
    /// its lower course. Towns and the city are left dry.
    fn carve_river(&mut self, pts: &[(i32, i32)], seed: u32) {
        let mut line: Vec<(i32, i32)> = vec![];
        let legs = (pts.len() - 1) as f32;
        for (k, w) in pts.windows(2).enumerate() {
            let (ax, ay) = (w[0].0 as f32, w[0].1 as f32);
            let (bx, by) = (w[1].0 as f32, w[1].1 as f32);
            let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt().max(1.0);
            let (nx, ny) = (-(by - ay) / len, (bx - ax) / len);
            let steps = (len * 3.0) as i32;
            for s in 0..steps {
                let t = s as f32 / steps as f32;
                let wob = (vnoise(t * 3.0 + k as f32 * 7.0, 1.5, seed) - 0.5)
                    * 6.0
                    * (t * std::f32::consts::PI).sin();
                let x = (ax + (bx - ax) * t + nx * wob).round() as i32;
                let y = (ay + (by - ay) * t + ny * wob).round() as i32;
                if line.last() != Some(&(x, y)) {
                    line.push((x, y));
                }
                let wide = ((k as f32 + t) / legs > 0.65) as i32;
                for oy in 0..=1 + wide {
                    for ox in 0..=1 + wide {
                        let (px, py) = (x + ox, y + oy);
                        if !inb(px, py) || self.in_town(px, py) {
                            continue;
                        }
                        let i = idx(px, py);
                        self.t[i] = T_WATER;
                        self.o[i] = O_NONE;
                        self.road[i] = false;
                    }
                }
            }
        }
        // reeds and a little sand along the banks
        for &(x, y) in &line {
            for (dx, dy) in [(-1, 0), (3, 0), (0, -1), (0, 3), (-1, 2), (3, 1)] {
                let (px, py) = (x + dx, y + dy);
                if !inb(px, py) || self.in_town(px, py) {
                    continue;
                }
                let i = idx(px, py);
                if self.t[i] == T_GRASS && self.o[i] == O_NONE {
                    let h = hash(px, py, seed ^ 0x5eed);
                    if h < 0.12 {
                        self.o[i] = O_BUSH;
                    } else if h > 0.93 {
                        self.t[i] = T_SAND;
                    }
                }
            }
        }
        // fords: short bridges here and there, so the land beyond is not cut off
        let n = line.len();
        for f in [0.22f32, 0.5, 0.8] {
            let k = (n as f32 * f) as usize;
            if k + 3 >= n {
                continue;
            }
            let (ax, ay) = line[k];
            let (bx, by) = line[k + 3];
            let (dx, dy) = ((bx - ax) as f32, (by - ay) as f32);
            let l = (dx * dx + dy * dy).sqrt().max(1.0);
            // across the flow, along whichever axis is nearer
            let (px, py) = if (dy / l).abs() > (dx / l).abs() {
                (1, 0)
            } else {
                (0, 1)
            };
            if self.in_town(ax, ay) || self.in_village(ax, ay) {
                continue;
            }
            for side in [-1, 1] {
                for s in 0..6 {
                    for w in 0..2 {
                        let (x, y) = (ax + px * s * side + py * w, ay + py * s * side + px * w);
                        if !inb(x, y) {
                            continue;
                        }
                        let i = idx(x, y);
                        if self.t[i] == T_WATER {
                            self.t[i] = T_BRIDGE;
                            self.road[i] = true;
                        } else if self.t[i] != T_BRIDGE {
                            // the bank where it lands takes a slope
                            self.road[i] = true;
                            if matches!(self.o[i], O_TREE | O_BUSH | O_BOULDER) {
                                self.o[i] = O_NONE;
                            }
                        }
                    }
                }
            }
        }
        self.rivers.push(line);
    }

    /// A label for each river on the map of places, midway along it.
    fn name_rivers(&mut self) {
        for (k, name) in ["West river", "East river"].into_iter().enumerate() {
            if let Some(r) = self.rivers.get(k) {
                let (x, y) = r[r.len() / 3];
                self.pois.push((name, x, y));
            }
        }
    }

    /// Whether a w×h footprint at (bx, by), with a tile of room round it,
    /// lies on clear dry ground inside the village hedge, off its lanes.
    fn village_room(&self, bx: i32, by: i32, w: i32, h: i32) -> bool {
        let (x0, y0, x1, y1) = self.village;
        if bx - 1 <= x0 || by - 1 <= y0 || bx + w >= x1 - 1 || by + h + 1 >= y1 - 1 {
            return false;
        }
        for y in by - 1..=by + h + 1 {
            for x in bx - 1..=bx + w {
                let i = idx(x, y);
                if !matches!(self.t[i], T_GRASS | T_DIRT | T_SAND)
                    || self.road[i]
                    || !matches!(self.o[i], O_NONE | O_TREE | O_BUSH | O_FLOWERS | O_MUSHROOM)
                {
                    return false;
                }
            }
        }
        true
    }

    /// A burrow house dug into a turf mound: earthen walls round a snug
    /// room, a round door in the middle of its front, a garden path.
    /// Clear trees and shrubs from round a footprint, so its door is free.
    fn clear_round(&mut self, bx: i32, by: i32, w: i32, h: i32) {
        for y in by - 1..=by + h + 1 {
            for x in bx - 1..=bx + w {
                if inb(x, y) && matches!(self.o[idx(x, y)], O_TREE | O_BUSH | O_MUSHROOM) {
                    self.o[idx(x, y)] = O_NONE;
                }
            }
        }
    }

    fn burrow(&mut self, bx: i32, by: i32, w: i32, h: i32, vendor: usize) {
        self.clear_round(bx, by, w, h);
        for y in by..by + h {
            for x in bx..bx + w {
                let edge = x == bx || x == bx + w - 1 || y == by || y == by + h - 1;
                let i = idx(x, y);
                self.t[i] = T_FLOOR;
                self.o[i] = if edge { O_WALL } else { O_NONE };
                self.style[i] = WALL_BURROW;
            }
        }
        let dx = bx + w / 2;
        self.set_door(dx, by + h - 1);
        // a path to the door, flowers either side of it
        let i = idx(dx, by + h);
        self.t[i] = T_DIRT;
        self.o[i] = O_NONE;
        for fx in [dx - 2, dx + 2] {
            if inb(fx, by + h) && self.o[idx(fx, by + h)] == O_NONE {
                self.o[idx(fx, by + h)] = O_FLOWERS;
            }
        }
        let back = |x: i32| (x, by + 1, true);
        let mut items: Vec<((i32, i32, bool), u8)> = if vendor == V_BARKEEP {
            vec![
                (back(bx + 1), O_BARREL),
                (back(bx + 2), O_BARREL),
                (back(bx + w - 3), O_HEARTH),
                (back(bx + w - 2), O_SHELF),
                ((bx + 2, by + h - 2, false), O_LONGTABLE),
                ((bx + 3, by + h - 2, true), O_CHAIR),
                ((bx + 3, by + h - 3, true), O_CHAIR),
                ((bx + w - 2, by + h - 2, true), O_CHAIR),
                // a dresser of crockery, a stool by the fire, flowers in a pot
                (back(bx + w - 4), O_DRESSER),
                ((bx + w - 2, by + 2, true), O_STOOL),
                ((bx + w - 2, by + 3, true), O_PLANT),
            ]
        } else {
            vec![
                (back(bx + 1), O_SHELF),
                (back(bx + w - 2), O_HEARTH),
                ((bx + 1, by + h - 2, false), O_BED),
                ((bx + w - 2, by + h - 2, true), O_BARREL),
                // a spinning wheel, plates on the dresser, a plant by the door
                ((bx + 1, by + 2, false), O_SPINWHEEL),
                ((bx + w - 2, by + 2, true), O_PLANT),
            ]
        };
        if w >= 7 {
            items.push((back(bx + 2), O_TABLE));
        }
        if vendor != V_BARKEEP && w >= 7 {
            // each home keeps its crockery or its clothes by the back wall
            let o = if (bx + by) % 2 == 0 {
                O_DRESSER
            } else {
                O_WARDROBE
            };
            items.push((back(bx + w - 3), o));
        }
        for ((x, y, south), o) in items {
            if size(o).0 > 1 {
                self.place(x, y, o, south);
            } else if inb(x, y) {
                let i = idx(x, y);
                self.o[i] = o;
                if south {
                    self.style[i] |= FACE_S;
                }
            }
        }
        self.buildings.push(Building {
            x: bx,
            y: by,
            w,
            h,
            vendor,
            style: WALL_BURROW,
            floors: vec![],
            part_of: None,
        });
        self.npcs.push((vendor, dx, by + 2));
    }

    /// The village of the small folk west of the town: burrow houses in a
    /// ring of hedge, a mill on the river, an inn, and a great tree on the
    /// green. Nothing wild comes in: the hedge keeps beasts out, its gates
    /// are shut, and a hayward keeps the gate.
    fn village(&mut self, rng: &mut Rng, seed: u32, from: (i32, i32)) {
        let (x0, y0, x1, y1) = self.village;
        let (cx, cy) = ((x0 + x1) / 2, (y0 + y1) / 2);
        // a gentle green: no rock, few trees
        for y in y0..y1 {
            for x in x0..x1 {
                let i = idx(x, y);
                match self.t[i] {
                    T_WATER => continue,
                    T_MOUNT | T_SAND => self.t[i] = T_GRASS,
                    _ => {}
                }
                if self.o[i] == O_TREE && rng.f() < 0.85 {
                    self.o[i] = O_NONE;
                }
                if matches!(self.o[i], O_BOULDER | O_BONES | O_SKULLS) {
                    self.o[i] = O_NONE;
                }
            }
        }
        // the lane from the town, through the east gate and over the river
        let road = self.carve_road(from, (x1 + 1, cy), seed ^ 31);
        self.roads.push(Road {
            from: "town",
            to: "village",
            tiles: road,
        });
        self.carve_road((x1, cy), (x0 + 4, cy + 2), seed ^ 32);
        // the hedge all round, with gates where the lane runs through it
        for y in y0..y1 {
            for x in x0..x1 {
                if x != x0 && x != x1 - 1 && y != y0 && y != y1 - 1 {
                    continue;
                }
                let i = idx(x, y);
                if self.t[i] == T_WATER {
                    continue;
                }
                if self.road[i] {
                    self.set_door(x, y);
                    self.style[i] = HEDGE;
                } else {
                    self.t[i] = T_GRASS;
                    self.o[i] = O_FENCE;
                    self.style[i] = HEDGE;
                }
            }
        }
        // the mill: on the bank, its wheel turning in the river beside it
        // the wheel on the river side: east of the mill if it can be, where
        // it shows from the lane, else west
        'mill: for east in [true, false] {
            for by in y0 + 3..y1 - 9 {
                for bx in x0 + 3..x1 - 10 {
                    let wet = |x: i32, y: i32| self.t[idx(x, y)] == T_WATER;
                    let wheel = if !east && wet(bx - 1, by + 2) && wet(bx - 1, by + 3) {
                        (bx - 1, by + 2)
                    } else if east && wet(bx + 8, by + 2) && wet(bx + 8, by + 3) {
                        (bx + 8, by + 2)
                    } else {
                        continue;
                    };
                    // the mill and its yard in front must stand on dry,
                    // clear ground; the river may run close by
                    let (vx0, vy0, vx1, vy1) = self.village;
                    let room = bx > vx0 + 1
                        && by > vy0 + 1
                        && bx + 8 < vx1 - 1
                        && by + 7 < vy1 - 1
                        && (by..=by + 6).all(|y| {
                            (bx..=bx + 7).all(|x| {
                                let i = idx(x, y);
                                matches!(self.t[i], T_GRASS | T_DIRT | T_SAND)
                                    && !self.road[i]
                                    && matches!(
                                        self.o[i],
                                        O_NONE | O_TREE | O_BUSH | O_FLOWERS | O_MUSHROOM
                                    )
                            })
                        });
                    if room {
                        self.clear_round(bx, by, 8, 6);
                        self.building(bx, by, 2, V_VILLAGER);
                        self.o[idx(wheel.0, wheel.1)] = O_WHEEL;
                        self.pois.push(("Mill", bx + 4, by + 3));
                        break 'mill;
                    }
                }
            }
        }
        // the great tree on the green, near the middle, with tables beneath it
        let mut best: Option<(i32, (i32, i32))> = None;
        for ty in y0 + 4..y1 - 4 {
            for tx in x0 + 4..x1 - 4 {
                let d = (tx - cx - 5).pow(2) + (ty - cy).pow(2);
                if best.is_none_or(|b| d < b.0) && self.village_room(tx - 2, ty - 2, 6, 4) {
                    best = Some((d, (tx, ty)));
                }
            }
        }
        if let Some((_, (tx, ty))) = best {
            self.set_o(tx, ty, O_BIGTREE);
            self.place(tx + 2, ty + 2, O_LONGTABLE, false);
            self.place(tx + 3, ty + 2, O_CHAIR, true);
            self.place(tx + 3, ty + 1, O_CHAIR, true);
            self.set_o(tx - 2, ty - 2, O_LAMP);
            self.set_o(tx + 3, ty - 2, O_LAMP);
            self.pois.push(("Party tree", tx, ty));
        }
        // the inn near the lane, then the houses wherever there is room,
        // nearest the middle first
        let mut spots: Vec<(i32, i32)> = (y0 + 2..y1 - 7)
            .flat_map(|y| (x0 + 2..x1 - 8).map(move |x| (x, y)))
            .collect();
        spots.sort_by_key(|&(x, y)| (x + 4 - cx).pow(2) + (y + 3 - cy).pow(2));
        if let Some(&(bx, by)) = spots
            .iter()
            .find(|&&(bx, by)| self.village_room(bx, by, 9, 6))
        {
            self.burrow(bx, by, 9, 6, V_BARKEEP);
            self.pois.push(("Village inn", bx + 4, by + 3));
        }
        let mut homes = 0;
        for (bx, by) in spots {
            if homes < 8 && self.village_room(bx, by, 7, 5) {
                self.burrow(bx, by, 7, 5, V_VILLAGER);
                homes += 1;
            }
        }
        // gardens: flowers and shrubs on the open green
        for y in y0 + 1..y1 - 1 {
            for x in x0 + 1..x1 - 1 {
                let i = idx(x, y);
                if self.o[i] != O_NONE || self.road[i] || self.t[i] != T_GRASS {
                    continue;
                }
                let r = rng.f();
                if r < 0.07 {
                    self.o[i] = O_FLOWERS;
                } else if r < 0.1 {
                    self.o[i] = O_BUSH;
                }
            }
        }
        // the hayward at the east gate, and a few folk about the lanes
        self.npcs.push((V_GUARD, x1 - 3, cy - 2));
        let lanes: Vec<(i32, i32)> = (y0 + 2..y1 - 2)
            .flat_map(|y| (x0 + 2..x1 - 2).map(move |x| (x, y)))
            .filter(|&(x, y)| self.road[idx(x, y)] && self.t[idx(x, y)] == T_DIRT)
            .collect();
        for k in 0..3 {
            if let Some(&(x, y)) = lanes.get(lanes.len() * (k + 1) / 4) {
                self.npcs.push((V_VILLAGER, x, y));
            }
        }
        self.pois.push(("Village", cx, cy));
    }
}

// ---------------------------------------------------------------- mountains and the elves
/// where the elven wood and the haven stand
pub const WOOD_C: (i32, i32) = (200, 198);
pub const HAVEN_C: (i32, i32) = (214, 148);
/// the hidden valley in the eastern range
pub const VALLEY_C: (i32, i32) = (184, 75);

fn seg_dist(px: f32, py: f32, a: (i32, i32), b: (i32, i32)) -> f32 {
    let (ax, ay, bx, by) = (a.0 as f32, a.1 as f32, b.0 as f32, b.1 as f32);
    let (dx, dy) = (bx - ax, by - ay);
    let l = dx * dx + dy * dy;
    let t = if l > 0.0 {
        (((px - ax) * dx + (py - ay) * dy) / l).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((px - ax - t * dx).powi(2) + (py - ay - t * dy).powi(2)).sqrt()
}

impl Map {
    /// A mountain range along `pts`: a jagged core of rock `width` wide,
    /// with high foothills stepping down on either side.
    fn range(&mut self, elev: &mut [f32], pts: &[(i32, i32)], width: f32, seed: u32) {
        let (mut x0, mut y0, mut x1, mut y1) = (W, LAND_H, 0, 0);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let pad = width as i32 * 2;
        for y in (y0 - pad).max(0)..(y1 + pad).min(LAND_H) {
            for x in (x0 - pad).max(0)..(x1 + pad).min(W) {
                if self.in_town(x, y) || self.in_village(x, y) {
                    continue;
                }
                let d = pts
                    .windows(2)
                    .map(|w| seg_dist(x as f32, y as f32, w[0], w[1]))
                    .fold(f32::MAX, f32::min);
                let jag = (vnoise(x as f32 / 6.0, y as f32 / 6.0, seed) - 0.5) * width * 0.7
                    + (vnoise(x as f32 / 2.5, y as f32 / 2.5, seed ^ 9) - 0.5) * 3.0;
                let i = idx(x, y);
                if self.t[i] == T_WATER && d > width * 0.3 {
                    continue;
                }
                if d < width * 0.5 + jag {
                    self.t[i] = T_MOUNT;
                    self.o[i] = O_NONE;
                    elev[i] = elev[i].max(0.7);
                } else if d < width * 1.3 + jag {
                    // foothills: high, rocky, few trees
                    let f = 1.0 - (d - width * 0.5) / (width * 0.8);
                    elev[i] = elev[i].max(0.5 + 0.16 * f.clamp(0.0, 1.0));
                    if self.t[i] == T_WATER || self.t[i] == T_SAND {
                        self.t[i] = T_GRASS;
                    }
                    let h = hash(x, y, seed ^ 0xb0);
                    if h < 0.05 {
                        self.o[i] = O_BOULDER;
                    } else if self.o[i] == O_TREE && h < 0.5 {
                        self.o[i] = O_NONE;
                    }
                }
            }
        }
    }

    /// A straight paved way, two tiles wide.
    fn pave(&mut self, a: (i32, i32), b: (i32, i32), t: u8) {
        let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
        for k in 0..=n {
            let x = a.0 + (b.0 - a.0) * k / n;
            let y = a.1 + (b.1 - a.1) * k / n;
            for (ox, oy) in [(0, 0), (1, 0), (0, 1)] {
                let (px, py) = (x + ox, y + oy);
                if inb(px, py) {
                    let i = idx(px, py);
                    self.t[i] = if self.t[i] == T_WATER { T_BRIDGE } else { t };
                    self.road[i] = true;
                    if !matches!(self.o[i], O_WALL | O_DOOR) {
                        self.o[i] = O_NONE;
                    }
                }
            }
        }
    }

    /// A pale stone hall of the elves, with the ground before it kept clear.
    fn elf_hall(&mut self, bx: i32, by: i32, vendor: usize) {
        self.clear_round(bx, by, 8, 6);
        self.building(bx, by, WALL_ELVEN, vendor);
    }

    /// Clear ground for an elven town: grass, level, nothing standing.
    fn clear_flat(&mut self, elev: &mut [f32], x: i32, y: i32) {
        if !inb(x, y) {
            return;
        }
        let i = idx(x, y);
        self.t[i] = T_GRASS;
        self.o[i] = O_NONE;
        self.road[i] = false;
        elev[i] = 0.5;
    }

    /// The elven lands: a hidden valley in the eastern range, a town under
    /// golden trees in a deep wood to the south-east, and a haven on the
    /// eastern shore with a white ship at its quay. Roads with guard posts
    /// run to each; all three are guarded, peaceful ground.
    fn elven_lands(&mut self, elev: &mut [f32], rng: &mut Rng, seed: u32, south_gate: (i32, i32)) {
        // ---- the valley: an oval of green walled in by the mountains
        let (vx, vy) = VALLEY_C;
        let (rx, ry) = (17.0f32, 13.0f32);
        for y in vy - 13..=vy + 13 {
            for x in vx - 17..=vx + 17 {
                let d = ((x - vx) as f32 / rx).powi(2) + ((y - vy) as f32 / ry).powi(2);
                if d < 1.0 {
                    self.clear_flat(elev, x, y);
                    if d > 0.75 && hash(x, y, seed ^ 0x77) < 0.35 {
                        self.o[idx(x, y)] = O_TREE;
                    }
                }
            }
        }
        // roads in from the stables and on east to the haven, through passes
        let (sx1, sy0, _, sy1) = (self.stable.2, self.stable.1, 0, self.stable.3);
        let west = self.carve_road((sx1 + 1, (sy0 + sy1) / 2), (vx - 18, vy), seed ^ 41);
        self.road_post(&west);
        self.roads.push(Road {
            from: "town",
            to: "valley",
            tiles: west,
        });
        let east = self.carve_road((vx + 18, vy), (HAVEN_C.0, HAVEN_C.1 - 11), seed ^ 42);
        self.road_post(&east);
        self.pave((vx - 18, vy), (vx + 18, vy), T_COBBLE);
        self.elf_hall(vx - 11, vy - 7, V_HEALER);
        self.elf_hall(vx + 1, vy - 7, V_MAGE);
        self.elf_hall(vx - 11, vy + 3, V_LIBRARIAN);
        self.elf_hall(vx + 1, vy + 3, V_BARKEEP);
        for (dx, dy) in [(-14, -2), (14, -2), (-14, 3), (14, 3), (0, -2), (0, 3)] {
            self.set_o(vx + dx, vy + dy, O_LAMP);
        }
        for (dx, dy) in [(-6, -1), (6, 2), (-2, 2), (10, -1)] {
            self.npcs.push((V_ELF, vx + dx, vy + dy));
        }
        self.npcs.push((V_GUARD, vx - 16, vy - 1));
        self.npcs.push((V_GUARD, vx + 16, vy + 2));
        self.elves.push((vx - 17, vy - 13, vx + 18, vy + 14));
        self.pois.push(("Elf valley", vx, vy));

        // ---- the wood: a deep forest ring round a clearing of golden trees
        let (wx, wy) = WOOD_C;
        for y in wy - 34..=wy + 34 {
            for x in wx - 34..=wx + 34 {
                if !inb(x, y) {
                    continue;
                }
                let d = (((x - wx).pow(2) + (y - wy).pow(2)) as f32).sqrt();
                let i = idx(x, y);
                if d < 16.0 {
                    self.clear_flat(elev, x, y);
                } else if d < 34.0 && self.t[i] == T_GRASS && self.o[i] == O_NONE {
                    let deep = 1.0 - (d - 16.0) / 18.0;
                    if rng.f() < 0.35 + 0.45 * deep {
                        self.o[i] = O_TREE;
                    }
                }
            }
        }
        let gate = (wx, wy - 34);
        let north = self.carve_road(south_gate, gate, seed ^ 43);
        self.road_post(&north);
        self.roads.push(Road {
            from: "town",
            to: "wood",
            tiles: north,
        });
        self.carve_road(gate, (wx, wy - 15), seed ^ 44);
        self.pave((wx, wy - 15), (wx, wy + 12), T_DIRT);
        self.pave((wx - 13, wy - 1), (wx + 13, wy - 1), T_DIRT);
        self.elf_hall(wx - 12, wy - 9, V_PROV);
        self.elf_hall(wx + 3, wy - 9, V_HEALER);
        self.elf_hall(wx - 12, wy + 3, V_MAGE);
        for (dx, dy) in [
            (0, 0),
            (-7, 9),
            (8, 9),
            (-13, 5),
            (13, 5),
            (6, -12),
            (-6, -12),
        ] {
            let (tx, ty) = (wx + dx + 2 * (dx == 0) as i32, wy + dy);
            if self.o[idx(tx, ty)] == O_NONE && !self.road[idx(tx, ty)] {
                self.o[idx(tx, ty)] = O_BIGTREE;
                self.style[idx(tx, ty)] = GOLDEN;
            }
        }
        for (dx, dy) in [(-3, -4), (4, 4), (-4, 7), (9, 1)] {
            self.npcs.push((V_ELF, wx + dx, wy + dy));
        }
        self.npcs.push((V_GUARD, wx - 1, wy - 15));
        self.npcs.push((V_GUARD, wx + 2, wy - 15));
        self.elves.push((wx - 16, wy - 16, wx + 17, wy + 17));
        self.pois.push(("Elf wood", wx, wy));

        // ---- the haven: halls above a quay on the eastern shore
        let (hx, hy) = HAVEN_C;
        for y in hy - 10..=hy + 10 {
            for x in hx - 10..=hx + 10 {
                self.clear_flat(elev, x, y);
            }
        }
        // the open sea east of the quay
        for y in hy - 14..=hy + 14 {
            for x in hx + 12..W {
                if inb(x, y) {
                    let i = idx(x, y);
                    self.t[i] = T_WATER;
                    self.o[i] = O_NONE;
                    self.road[i] = false;
                }
            }
        }
        for x in hx + 11..hx + 12 {
            for y in hy - 10..=hy + 10 {
                let i = idx(x, y);
                self.t[i] = T_SAND;
                self.o[i] = O_NONE;
            }
        }
        self.pave((hx, hy - 11), (hx, hy), T_COBBLE);
        self.pave((hx - 9, hy), (hx + 18, hy), T_COBBLE);
        let wood_road = self.carve_road((wx + 6, wy - 30), (hx - 4, hy + 11), seed ^ 45);
        self.road_post(&wood_road);
        self.pave((hx - 4, hy + 11), (hx - 4, hy), T_COBBLE);
        self.elf_hall(hx - 9, hy - 8, V_PROV);
        self.elf_hall(hx + 2, hy - 8, V_BANKER);
        self.elf_hall(hx - 9, hy + 3, V_MAGE);
        self.set_o(hx + 15, hy - 2, O_SHIP);
        for (dx, dy) in [(-7, 9), (7, 9), (8, -1), (-3, -1)] {
            self.set_o(hx + dx, hy + dy, O_LAMP);
        }
        for (dx, dy) in [(5, 2), (12, 1), (3, -2), (-1, 6)] {
            self.npcs.push((V_ELF, hx + dx, hy + dy));
        }
        self.npcs.push((V_GUARD, hx + 1, hy - 10));
        self.elves.push((hx - 10, hy - 10, hx + 19, hy + 11));
        self.pois.push(("Elf haven", hx, hy));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every open tile of a storey is reached from `from` (4-way steps); with
    /// `ring` 1, the outer ring of tiles too (a thin-walled house's).
    fn all_reached(
        w: i32,
        h: i32,
        ring: i32,
        open: &dyn Fn(i32, i32) -> bool,
        from: &[(i32, i32)],
    ) -> Option<(i32, i32)> {
        let mut seen = vec![false; (w * h) as usize];
        let mut todo: Vec<(i32, i32)> = from.to_vec();
        for &(x, y) in from {
            seen[(y * w + x) as usize] = true;
        }
        while let Some((x, y)) = todo.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 1 - ring && ny >= 1 - ring && nx < w - 1 + ring && ny < h - 1 + ring {
                    let i = (ny * w + nx) as usize;
                    if !seen[i] && open(nx, ny) {
                        seen[i] = true;
                        todo.push((nx, ny));
                    }
                }
            }
        }
        (1 - ring..h - 1 + ring)
            .flat_map(|y| (1 - ring..w - 1 + ring).map(move |x| (x, y)))
            .find(|&(x, y)| open(x, y) && !seen[(y * w + x) as usize])
    }

    /// The tropics lie south of the southern range, and their two cities
    /// can be walked to from the town, the Raja on his throne in his palace.
    #[test]
    fn the_tropics_and_their_cities() {
        for seed in [1409u32, 7, 99] {
            let m = generate(seed);
            assert!(
                !m.tropic(80, 84) && !m.tropic(80, 44),
                "the north is not tropical"
            );
            assert!(m.tropic(96, 280) && m.tropic(20, 300), "the far south is");
            assert_eq!(m.raj.len(), 2);
            for (name, (x0, y0, w, h)) in [("Suryagarh", SURYA), ("Kanakpur", KANAK)] {
                assert!(m.pois.iter().any(|p| p.0 == name));
                // every haveli, temple and palace door opens onto ground
                // that the town can reach
                let inside: Vec<&Building> = m
                    .buildings
                    .iter()
                    .filter(|b| b.x >= x0 && b.y >= y0 && b.x < x0 + w && b.y < y0 + h)
                    .collect();
                assert!(inside.len() >= 5, "{name} has its buildings");
                for b in inside {
                    let ok = (1..b.w - 1).any(|x| {
                        let (dx, dy) = (b.x + x, b.y + b.h);
                        m.obj(dx, dy - 1) == O_DOOR && m.reach[idx(dx, dy)]
                    });
                    assert!(
                        ok,
                        "seed {seed}: {name} building at {},{} cut off",
                        b.x, b.y
                    );
                }
            }
            let (_, rx, ry) = *m.npcs.iter().find(|n| n.0 == V_RAJA).expect("raja");
            assert_eq!(m.obj(rx, ry - 1), O_THRONE);
            assert!(m.in_raj(rx, ry) && m.guarded(rx, ry));
        }
    }

    /// Furniture never walls off part of a room, a shopkeeper or a stair,
    /// and every room built on behind a house is reached through it.
    #[test]
    fn furnished_rooms_stay_walkable() {
        for seed in [1409u32, 7, 99] {
            let m = generate(seed);
            let mut annexed = 0;
            for (bi, b) in m.buildings.iter().enumerate() {
                if b.part_of.is_some() {
                    continue;
                }
                // the house: this building and the rooms built onto it
                let parts: Vec<&Building> = std::iter::once(b)
                    .chain(m.buildings.iter().filter(|o| o.part_of == Some(bi)))
                    .collect();
                annexed += parts.len() - 1;
                let open =
                    |x: i32, y: i32| parts.iter().any(|p| p.room(x, y)) && !blocks(m.obj(x, y));
                let (bx, by) = (b.x, b.y);
                // in from the door(s) in the front wall
                let fy = if b.thin() { by + b.h - 1 } else { by + b.h - 2 };
                let mut todo: Vec<(i32, i32)> = (1..b.w - 1)
                    .filter(|&x| m.obj(bx + x, by + b.h - 1) == O_DOOR)
                    .map(|x| (bx + x, fy))
                    .filter(|&(x, y)| open(x, y))
                    .collect();
                assert!(
                    !todo.is_empty(),
                    "building at {bx},{by} has its door blocked"
                );
                let mut seen: std::collections::HashSet<(i32, i32)> =
                    todo.iter().copied().collect();
                while let Some((x, y)) = todo.pop() {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let n = (x + dx, y + dy);
                        if open(n.0, n.1) && !seen.contains(&n) && !m.wall_between(x, y, dx, dy, 0)
                        {
                            seen.insert(n);
                            todo.push(n);
                        }
                    }
                }
                for p in &parts {
                    for y in p.y..p.y + p.h {
                        for x in p.x..p.x + p.w {
                            assert!(
                                !open(x, y) || seen.contains(&(x, y)),
                                "seed {seed}: building at {bx},{by} (vendor {}): tile {x},{y} walled off",
                                b.vendor
                            );
                        }
                    }
                }
                for (k, f) in b.floors.iter().enumerate() {
                    let open = |x: i32, y: i32| !blocks(f.o[(y * b.w + x) as usize]);
                    let stairs: Vec<(i32, i32)> = (0..b.h)
                        .flat_map(|y| (0..b.w).map(move |x| (x, y)))
                        .filter(|&(x, y)| f.o[(y * b.w + x) as usize] == O_STAIRS)
                        .collect();
                    assert!(!stairs.is_empty());
                    if let Some((x, y)) =
                        all_reached(b.w, b.h, b.thin() as i32, &open, &stairs[..1])
                    {
                        panic!(
                            "building at {bx},{by} storey {}: tile {x},{y} walled off",
                            k + 1
                        );
                    }
                }
            }
            assert!(annexed >= 6, "seed {seed}: {annexed} rooms built on");
        }
    }
}
