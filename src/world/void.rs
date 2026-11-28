//! The Ethereal Void: islands of pale stone adrift among the stars, below
//! the land and beyond a gulf of nothing.
//!
//! It lies in its own band of the map (`VOID_Y0..H`), so everything that
//! works on the land works there too: walking, fighting, spawning, saving.
//! Nothing joins it to the land but the two moongates (`Map::gates`): a ring
//! of standing stones east of the town whose gate opens at night, and the
//! gate on the Isle of the Gate in the Void, which never closes
//! (`game/void.rs` carries the traveller across).
//!
//! The isles (`ISLES`) hang at one height with sheer rock below them, and
//! are joined by narrow starways from the Isle of the Gate. The Gate isle
//! has an ankh, for anyone who dies out here. The others are bare but for
//! crystals, boulders and the statues of whoever was here before, and the
//! things of the Void: starbats, star-stalkers and voidwraiths.
//!
//! A world pack can rename an isle and settle a people on it:
//!
//! ```toml
//! [void.echoes]
//! name = "the Isle of the Choir"
//! folk = "orc"        # any monster id
//! count = 6
//! ```

use super::*;

/// the level the isles and starways stand at; the Void itself is 0
pub const ISLE_LV: u8 = 3;

/// An island of the Void.
#[derive(Clone, Debug)]
pub struct Isle {
    pub id: &'static str,
    pub name: &'static str,
    pub x: i32,
    pub y: i32,
    pub r: i32,
}

/// id, name, centre and radius; the first is the Isle of the Gate
const ISLES: [(&str, &str, i32, i32, i32); 7] = [
    ("gate", "the Isle of the Gate", 120, 400, 9),
    ("echoes", "the Isle of Echoes", 66, 384, 11),
    ("glass", "the Glass Shoals", 176, 380, 10),
    ("pillars", "the Hanging Pillars", 40, 420, 9),
    ("ember", "the Ember Reach", 202, 422, 10),
    ("hollow", "the Hollow Star", 120, 370, 7),
    ("far", "the Farthest Isle", 226, 372, 6),
];

/// the isles' ids, as a world pack names them (`[void.<id>]`)
pub fn isle_ids() -> Vec<&'static str> {
    ISLES.iter().map(|i| i.0).collect()
}
/// the keys a pack's `[void.<id>]` may set
pub const ISLE_KEYS: [&str; 3] = ["name", "folk", "count"];

/// What lives on each isle: (isle, monster, how many)
const DWELLERS: [(&str, &str, usize); 8] = [
    ("echoes", "voidwraith", 2),
    ("echoes", "starbat", 2),
    ("glass", "stalker", 3),
    ("pillars", "starbat", 4),
    ("ember", "stalker", 2),
    ("ember", "voidwraith", 1),
    ("hollow", "starbat", 3),
    ("far", "voidwraith", 3),
];

/// where the gate to the Void stands on the land: the search for open
/// ground starts here, east of the town between it and the city
const LAND_GATE_NEAR: (i32, i32) = (112, 72);

impl Map {
    /// Is (x, y) in the Ethereal Void's band of the map?
    pub fn in_void(&self, x: i32, y: i32) -> bool {
        inb(x, y) && y >= VOID_Y0 - 20
    }

    /// The isle (x, y) is on or near, if any.
    pub fn isle_at(&self, x: i32, y: i32) -> Option<usize> {
        self.isles.iter().position(|s| {
            let d = (((x - s.x).pow(2) + (y - s.y).pow(2)) as f32).sqrt();
            d <= s.r as f32 + 2.0
        })
    }

    /// Make the Void: the isles, the starways between them, the gate, and
    /// the spawners of what lives there. It draws on its own stream of
    /// chance, after the land is made, so the land is as it always was.
    pub(super) fn make_void(&mut self, seed: u32) {
        let mut rng = Rng::new(seed as u64 ^ 0x5741_5253);
        for i in LAND_N..N {
            self.t[i] = T_VOID;
            self.o[i] = O_NONE;
            self.lv[i] = 0;
        }
        let pack = crate::pack::get();
        for &(id, name, x, y, r) in &ISLES {
            let name = pack
                .str(&format!("void.{id}"), "name")
                .map(|s| crate::pack::leak(s.to_string()))
                .unwrap_or(name);
            self.isles.push(Isle { id, name, x, y, r });
        }
        // the isles: ragged blobs of stone
        let isles = self.isles.clone();
        for s in &isles {
            for y in s.y - s.r - 3..=s.y + s.r + 3 {
                for x in s.x - s.r - 3..=s.x + s.r + 3 {
                    if !inb(x, y) || y < VOID_Y0 {
                        continue;
                    }
                    let d = (((x - s.x).pow(2) + (y - s.y).pow(2)) as f32).sqrt();
                    let edge =
                        s.r as f32 * (0.8 + 0.4 * fbm(x as f32 / 5.0, y as f32 / 5.0, seed ^ 0x51));
                    if d <= edge {
                        self.ground_void(x, y);
                    }
                }
            }
        }
        // the starways: two tiles wide, bowing a little, from the Gate isle
        // to each of the others
        let (gx, gy) = (isles[0].x, isles[0].y);
        for (k, s) in isles.iter().enumerate().skip(1) {
            let (dx, dy) = ((s.x - gx) as f32, (s.y - gy) as f32);
            let len = (dx * dx + dy * dy).sqrt();
            let bow = if k % 2 == 0 { 4.0 } else { -4.0 };
            let steps = (len * 2.0) as i32;
            for j in 0..=steps {
                let t = j as f32 / steps as f32;
                let b = (t * std::f32::consts::PI).sin() * bow;
                let x = gx as f32 + dx * t - dy / len * b;
                let y = gy as f32 + dy * t + dx / len * b;
                let (x, y) = (x.round() as i32, y.round() as i32);
                for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    if inb(x + ox, y + oy) && y + oy >= VOID_Y0 {
                        self.ground_void(x + ox, y + oy);
                    }
                }
            }
        }
        // the Gate isle: the gate in the middle, an ankh beside it
        let gate = (gx, gy);
        self.clear_void(gx, gy, 2);
        self.o[idx(gx, gy)] = O_MOONGATE;
        let ankh = (gx + 3, gy - 2);
        self.clear_void(ankh.0, ankh.1, 0);
        self.o[idx(ankh.0, ankh.1)] = O_ANKH;
        self.ankhs.push(ankh);
        // crystals, boulders and old statues about the isles, never on a
        // starway or near the gate
        for (k, s) in isles.iter().enumerate() {
            let n = (s.r * s.r) / 6;
            for _ in 0..n {
                let x = s.x + rng.range(-s.r, s.r);
                let y = s.y + rng.range(-s.r, s.r);
                if !inb(x, y) || self.t[idx(x, y)] != T_ASTRAL || self.o[idx(x, y)] != O_NONE {
                    continue;
                }
                if (x - gx).abs().max((y - gy).abs()) <= 4 {
                    continue;
                }
                // keep the ways across clear: nothing beside another thing
                if DIRS
                    .iter()
                    .any(|&(dx, dy)| self.obj(x + dx, y + dy) != O_NONE)
                {
                    continue;
                }
                let o = match rng.range(0, 9) {
                    0..=4 => O_CRYSTAL,
                    5..=7 => O_BOULDER,
                    _ if k > 0 => O_STATUE,
                    _ => O_CRYSTAL,
                };
                self.o[idx(x, y)] = o;
                self.style[idx(x, y)] = rng.range(0, 3) as u8;
            }
        }
        // every walkable tile of the Void can be reached, by the gate
        for i in LAND_N..N {
            if self.t[i] == T_ASTRAL {
                let (x, y) = (i as i32 % W, i as i32 / W);
                self.reach[i] = self.passable(x, y);
            }
        }
        // what lives there, and whoever a world pack settles there
        for &(isle, mon, n) in &DWELLERS {
            if let Some(s) = isles.iter().find(|s| s.id == isle) {
                self.spawns.push((mon, s.x, s.y, s.r - 1, n));
            }
        }
        for s in &isles {
            let sec = format!("void.{}", s.id);
            if let Some(folk) = pack.str(&sec, "folk") {
                if MONS.iter().any(|m| m.id == folk) {
                    let n = pack.int(&sec, "count").unwrap_or(5).clamp(1, 30) as usize;
                    let id = crate::pack::leak(folk.to_string());
                    self.spawns.push((id, s.x, s.y, s.r - 1, n));
                }
            }
        }
        // and the gate on the land, in a ring of standing stones
        let land = self.land_gate_spot();
        self.clear_void(land.0, land.1, 1);
        self.o[idx(land.0, land.1)] = O_MOONGATE;
        self.pois.push(("Moongate", land.0, land.1));
        self.gates = vec![land, gate];
    }

    /// Lay the Void's pale stone on (x, y), at the isles' height.
    fn ground_void(&mut self, x: i32, y: i32) {
        let i = idx(x, y);
        self.t[i] = T_ASTRAL;
        self.lv[i] = ISLE_LV;
    }

    /// Clear the objects within `r` of (x, y) (chebyshev).
    fn clear_void(&mut self, x: i32, y: i32, r: i32) {
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if inb(xx, yy) {
                    self.o[idx(xx, yy)] = O_NONE;
                }
            }
        }
    }

    /// Open ground on the land for the gate: walkable from the town, off
    /// the roads and out of every building, with walkable ground at one
    /// level all round it; the nearest such to `LAND_GATE_NEAR`.
    fn land_gate_spot(&self) -> (i32, i32) {
        let (cx, cy) = LAND_GATE_NEAR;
        let ok = |x: i32, y: i32| {
            if !inb(x, y) || !self.reach[idx(x, y)] || self.guarded(x, y) {
                return false;
            }
            let l = self.level(x, y);
            (-2..=2).all(|dy| {
                (-2..=2).all(|dx| {
                    let (nx, ny) = (x + dx, y + dy);
                    let i = idx(nx, ny);
                    inb(nx, ny)
                        && matches!(self.t[i], T_GRASS | T_DIRT)
                        && self.lv[i] == l
                        && !self.ramp[i]
                        && !self.road[i]
                        && self.bld[i] == 0
                        && matches!(self.o[i], O_NONE | O_TREE | O_BUSH | O_FLOWERS)
                })
            })
        };
        for r in 0..60 {
            for y in cy - r..=cy + r {
                for x in cx - r..=cx + r {
                    if (x - cx).abs().max((y - cy).abs()) == r && ok(x, y) {
                        return (x, y);
                    }
                }
            }
        }
        self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_void_has_its_isles_gates_and_ways() {
        for seed in [1, 42, 1997] {
            let mut m = generate(seed);
            assert_eq!(m.gates.len(), 2);
            let (land, gate) = (m.gates[0], m.gates[1]);
            assert!(land.1 < LAND_H && gate.1 >= VOID_Y0);
            assert_eq!(m.obj(land.0, land.1), O_MOONGATE);
            assert_eq!(m.obj(gate.0, gate.1), O_MOONGATE);
            assert!(m.passable(land.0, land.1) && m.passable(gate.0, gate.1));
            assert!(m.reach[idx(land.0, land.1)]);
            assert!(!m.in_void(land.0, land.1) && m.in_void(gate.0, gate.1));
            // the gate on the land is a walk from the town
            let (sx, sy) = m.start;
            assert!(
                m.find_path(sx, sy, land.0, land.1, 0).is_some(),
                "seed {seed}"
            );
            // every isle is a walk from the gate, and the land is not
            for s in m.isles.clone() {
                let (x, y) = (s.x, s.y);
                let (x, y) = (x..x + 3)
                    .flat_map(|x| (y..y + 3).map(move |y| (x, y)))
                    .find(|&(x, y)| m.passable(x, y))
                    .unwrap();
                assert!(
                    m.find_path(gate.0, gate.1 + 1, x, y, 0).is_some(),
                    "seed {seed}: no way to {}",
                    s.name
                );
            }
            assert!(m.find_path(gate.0, gate.1 + 1, sx, sy, 0).is_none());
            // the gulf between: nothing walkable from the land's edge to the Void
            for y in LAND_H..VOID_Y0 {
                for x in 0..W {
                    assert!(!m.passable(x, y));
                }
            }
            assert!(m.ankhs.iter().any(|&(x, y)| m.in_void(x, y)));
            for (mon, x, y, _, _) in &m.spawns {
                if *y >= VOID_Y0 {
                    assert!(of_the_void(mon), "{mon}");
                    let _ = x;
                }
            }
        }
    }
}
