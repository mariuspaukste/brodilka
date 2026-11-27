//! The tropics: the warm, wet lands south of the southern range, and their
//! two cities in the Indian style.
//!
//! - **Suryagarh**, a fort city of red sandstone: curtain walls with round
//!   bastions and three gates, the Raja's palace under white domes, a temple
//!   with its tower, a stepped tank in the square, and flat-roofed havelis
//!   for the trades.
//! - **Kanakpur**, an open temple town by the sea: a great temple, ghats
//!   stepping down into the water, a tank, havelis and a bazaar street.
//!
//! Everything here draws on its own stream of chance, so the north of a
//! seed's world comes out as it always has.

use super::*;

/// the corner and size of Suryagarh, the fort city (walls included)
pub const SURYA: (i32, i32, i32, i32) = (30, 246, 44, 34);
/// the corner and size of Kanakpur, the temple town by the sea
pub const KANAK: (i32, i32, i32, i32) = (116, 268, 44, 30);

impl Map {
    /// Is this tile in the tropics (south of the line that runs along the
    /// southern range's foot, bending south round the elves' wood)?
    pub fn tropic(&self, x: i32, y: i32) -> bool {
        let xi = x.clamp(0, W - 1) as usize;
        !self.tropic_line.is_empty() && y >= self.tropic_line[xi]
    }

    /// In one of the southern cities (with a margin): guarded ground.
    pub fn in_raj(&self, x: i32, y: i32) -> bool {
        self.raj
            .iter()
            .any(|&(x0, y0, x1, y1)| x >= x0 - 3 && y >= y0 - 3 && x < x1 + 3 && y < y1 + 3)
    }

    /// Lay out the tropics: where they begin, their beaches and jungle, the
    /// two cities and the roads to them over the southern passes.
    pub(super) fn tropics(&mut self, elev: &mut [f32], moist: &[f32], seed: u32) {
        let mut rng = Rng::new(seed as u64 ^ 0x1d1a_5a5a);
        // the line: under the southern range in the west, curving south of
        // the elves' wood in the east, ragged along its length
        self.tropic_line = (0..W)
            .map(|x| {
                let base = match x {
                    ..=150 => 214.0,
                    151..=190 => 214.0 + (x - 150) as f32 * 0.55,
                    _ => 236.0,
                };
                (base + (vnoise(x as f32 / 7.0, 3.5, seed ^ 0x3131) - 0.5) * 8.0) as i32
            })
            .collect();

        // the land: wide pale beaches, thick jungle where it is wet, red
        // earth where it is dry, and no toadstools
        for y in 0..H {
            for x in 0..W {
                if !self.tropic(x, y) {
                    continue;
                }
                let i = idx(x, y);
                if self.t[i] == T_GRASS && elev[i] < 0.4 && !self.road[i] {
                    self.t[i] = T_SAND;
                    self.o[i] = if rng.f() < 0.07 { O_TREE } else { O_NONE };
                    continue;
                }
                if self.o[i] == O_MUSHROOM {
                    self.o[i] = O_FLOWERS;
                }
                // the lower rock is jungle-clad hills, terraced like the rest
                if self.t[i] == T_MOUNT && elev[i] < 0.78 && y > self.tropic_line[x as usize] + 6 {
                    self.t[i] = T_GRASS;
                    elev[i] = elev[i].min(0.68);
                    self.o[i] = if rng.f() < 0.45 { O_TREE } else { O_NONE };
                    continue;
                }
                if self.t[i] == T_GRASS && self.o[i] == O_NONE && !self.road[i] {
                    let r = rng.f();
                    if moist[i] > 0.5 && r < 0.3 {
                        self.o[i] = O_TREE;
                    } else if r > 0.97 {
                        self.o[i] = O_BUSH;
                    } else if r > 0.955 {
                        self.o[i] = O_FLOWERS;
                    }
                }
            }
        }

        self.suryagarh(elev, &mut rng);
        self.kanakpur(elev, &mut rng);

        // roads: down from each pass over the southern range, and between
        // the two cities
        let (sx, sy, sw, sh) = SURYA;
        let (kx, ky, kw, kh) = KANAK;
        let north_gate = (sx + 8, sy - 1);
        let east_gate = (sx + sw, sy + 17);
        let west = self.carve_road((50, 206), north_gate, seed ^ 51);
        self.road_post(&west);
        let east = self.carve_road((118, 206), (kx + kw / 2, ky - 1), seed ^ 52);
        self.road_post(&east);
        let between = self.carve_road(east_gate, (kx - 1, ky + 15), seed ^ 53);
        self.road_post(&between);
        // keep the gates and the town's edges clear of anything the roads
        // brushed past
        let _ = (sh, kh);
        self.pois.push(("Tropics", 96, 236));
    }

    /// Clear a rectangle of ground flat and bare (plus a margin of `pad`).
    fn level_out(&mut self, elev: &mut [f32], x0: i32, y0: i32, w: i32, h: i32, pad: i32) {
        for y in y0 - pad..y0 + h + pad {
            for x in x0 - pad..x0 + w + pad {
                self.clear_flat(elev, x, y);
                if inb(x, y) {
                    let i = idx(x, y);
                    self.style[i] = 0;
                    self.ramp[i] = false;
                }
            }
        }
    }

    /// a run of sandstone fort wall from a to b (inclusive), along x or y
    fn fort_wall(&mut self, a: (i32, i32), b: (i32, i32)) {
        for y in a.1.min(b.1)..=a.1.max(b.1) {
            for x in a.0.min(b.0)..=a.0.max(b.0) {
                if inb(x, y) && self.style[idx(x, y)] != WALL_BASTION {
                    let i = idx(x, y);
                    self.t[i] = T_COBBLE;
                    self.o[i] = O_WALL;
                    self.style[i] = WALL_FORT;
                }
            }
        }
    }

    /// a round-shouldered bastion n×n
    fn bastion(&mut self, x: i32, y: i32, n: i32) {
        for ty in y..y + n {
            for tx in x..x + n {
                if inb(tx, ty) {
                    let i = idx(tx, ty);
                    self.t[i] = T_COBBLE;
                    self.o[i] = O_WALL;
                    self.style[i] = WALL_BASTION;
                }
            }
        }
    }

    /// A stepped tank: water in the middle, stone steps (flagstones) round it,
    /// a lamp at each corner.
    fn tank(&mut self, x0: i32, y0: i32, w: i32, h: i32) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let edge = x == x0 || y == y0 || x == x0 + w - 1 || y == y0 + h - 1;
                let i = idx(x, y);
                self.t[i] = if edge { T_COBBLE } else { T_WATER };
                self.o[i] = O_NONE;
                self.road[i] = false;
            }
        }
        for (x, y) in [
            (x0 - 1, y0 - 1),
            (x0 + w, y0 - 1),
            (x0 - 1, y0 + h),
            (x0 + w, y0 + h),
        ] {
            self.set_o(x, y, O_LAMP);
        }
    }

    /// Paved streets of red flagstone from a to b, `width` tiles wide.
    fn street(&mut self, a: (i32, i32), b: (i32, i32), width: i32) {
        for y in a.1.min(b.1)..=a.1.max(b.1) + width - 1 {
            for x in a.0.min(b.0)..=a.0.max(b.0) + width - 1 {
                if inb(x, y) && self.o[idx(x, y)] != O_WALL {
                    let i = idx(x, y);
                    self.t[i] = T_COBBLE;
                    self.road[i] = true;
                }
            }
        }
    }

    /// Palms set about a town: on bare ground only, never in a street, and
    /// clear of walls and doorways.
    fn palms(&mut self, spots: &[(i32, i32)]) {
        for &(x, y) in spots {
            let crowded = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| matches!(self.obj(x + dx, y + dy), O_WALL | O_DOOR | O_SIGN))
            });
            if inb(x, y) && !crowded && self.o[idx(x, y)] == O_NONE && self.t[idx(x, y)] == T_DIRT {
                self.o[idx(x, y)] = O_TREE;
                self.res[idx(x, y)] = 12;
            }
        }
    }

    /// Suryagarh: a fort city of red sandstone.
    fn suryagarh(&mut self, elev: &mut [f32], rng: &mut Rng) {
        let (x0, y0, w, h) = SURYA;
        let (xe, ye) = (x0 + w - 1, y0 + h - 1);
        self.level_out(elev, x0, y0, w, h, 3);
        for y in y0..=ye {
            for x in x0..=xe {
                self.t[idx(x, y)] = T_DIRT;
            }
        }
        // the walls, with gates north (to the pass), east (to Kanakpur) and
        // south (to the sea); bastions at the corners and along the sides
        let (ng, eg, sg) = (x0 + 8, y0 + 17, x0 + 22);
        self.fort_wall((x0, y0), (ng - 1, y0));
        self.fort_wall((ng + 2, y0), (xe, y0));
        self.fort_wall((x0, ye), (sg - 1, ye));
        self.fort_wall((sg + 2, ye), (xe, ye));
        self.fort_wall((x0, y0), (x0, ye));
        self.fort_wall((xe, y0), (xe, eg - 1));
        self.fort_wall((xe, eg + 2), (xe, ye));
        for (bx, by) in [(x0, y0), (xe - 2, y0), (x0, ye - 2), (xe - 2, ye - 2)] {
            self.bastion(bx, by, 3);
        }
        for (bx, by) in [
            (ng - 2, y0),
            (ng + 2, y0),
            (xe - 1, eg - 2),
            (xe - 1, eg + 2),
            (sg - 2, ye - 1),
            (sg + 2, ye - 1),
            (x0, y0 + 16),
        ] {
            self.bastion(bx, by, 2);
        }
        // the streets: from the north gate down to the square, the square
        // east to its gate and south to the sea gate
        self.street((ng, y0 + 1), (ng, y0 + 15), 2);
        self.street((x0 + 1, y0 + 15), (xe - 1, y0 + 15), 4);
        self.street((sg, y0 + 15), (sg, ye - 1), 2);
        self.street((x0 + 16, y0 + 12), (x0 + 30, y0 + 14), 1);
        // the stepped tank in the square, under the palace
        self.tank(x0 + 26, y0 + 17, 7, 5);

        // the Raja's palace at the head of the city, its door on the square
        self.keep_styled(x0 + 16, y0 + 3, V_RAJA, WALL_PALACE);
        // the temple beside it, the priest healing the sick and the dead
        self.building(x0 + 32, y0 + 4, WALL_TEMPLE, V_HEALER);
        // the trades in havelis round the square and down the sea street
        self.building(x0 + 2, y0 + 6, WALL_HAVELI, V_SMITH);
        self.building(x0 + 2, y0 + 20, WALL_HAVELI, V_PROV);
        self.building(x0 + 11, y0 + 20, WALL_HAVELI, V_BARKEEP);
        self.building(x0 + 25, y0 + 25, WALL_HAVELI, V_BANKER);
        self.building(x0 + 34, y0 + 25, WALL_HAVELI, V_MAGE);
        self.add_floors(x0 + 11, y0 + 20, &[INN_ROOMS]);
        self.add_floors(x0 + 34, y0 + 25, &[STUDY]);
        // rooms built on behind: the smith's store under the wall, the
        // jyotishi's chamber of charts
        self.annex(
            x0 + 2,
            y0 + 6,
            (x0 + 3, y0 + 2, 5, 4),
            None,
            &[O_CRATE, O_SACKS, O_JARS],
            &[O_BARREL],
        );
        self.annex(
            x0 + 34,
            y0 + 25,
            (x0 + 35, y0 + 21, 6, 4),
            None,
            &[O_SHELF, O_ORRERY, O_NONE, O_SHELF],
            &[O_CANDLES],
        );
        // stalls and bundles along the square, palms in the yards
        for (dx, dy, o) in [
            (12, 16, O_JARS),
            (13, 16, O_SACKS),
            (19, 19, O_CART),
            (35, 16, O_SACKS),
            (36, 16, O_JARS),
            (38, 19, O_CRATE),
            (12, 13, O_PLANT),
            (31, 13, O_PLANT),
        ] {
            if o == O_CART {
                self.place(x0 + dx, y0 + dy, o, true);
            } else {
                self.set_o(x0 + dx, y0 + dy, o);
            }
        }
        for (dx, dy) in [(12, 15), (31, 15), (ng - x0 + 2, 10), (sg - x0 - 1, 28)] {
            self.set_o(x0 + dx, y0 + dy, O_LAMP);
        }
        let mut spots = vec![];
        for _ in 0..40 {
            spots.push((x0 + rng.range(2, w - 3), y0 + rng.range(2, h - 3)));
        }
        self.palms(&spots);
        // a garden of palms before the palace
        self.palms(&[
            (x0 + 15, y0 + 12),
            (x0 + 15, y0 + 10),
            (x0 + 29, y0 + 12),
            (x0 + 29, y0 + 10),
        ]);

        for (dx, dy) in [
            (ng - x0, 2),
            (ng - x0 + 1, 2),
            (w - 3, 17),
            (sg - x0, h - 3),
            (20, 13),
            (25, 13),
        ] {
            self.npcs.push((V_GUARD, x0 + dx, y0 + dy));
        }
        for (dx, dy) in [
            (14, 16),
            (24, 16),
            (34, 17),
            (9, 18),
            (22, 20),
            (21, 24),
            (37, 22),
            (10, 10),
        ] {
            self.npcs.push((V_FOLK, x0 + dx, y0 + dy));
        }
        self.raj.push((x0, y0, x0 + w, y0 + h));
        self.pois.push(("Suryagarh", x0 + w / 2, y0 + h / 2));
        self.pois.push(("Palace", x0 + 22, y0 + 7));
        self.pois.push(("Sun temple", x0 + 36, y0 + 7));
    }

    /// Kanakpur: an open temple town on the shore, its ghats in the sea.
    fn kanakpur(&mut self, elev: &mut [f32], rng: &mut Rng) {
        let (x0, y0, w, h) = KANAK;
        let (xe, ye) = (x0 + w - 1, y0 + h - 1);
        self.level_out(elev, x0, y0, w, h, 2);
        for y in y0..=ye {
            for x in x0..=xe {
                self.t[idx(x, y)] = T_DIRT;
            }
        }
        // a bay of the sea below the town: the ghats step down into it
        // along the town's south edge, and its shores are sand
        let (cx, cy) = ((x0 + xe) / 2, ye + 14);
        for y in ye + 1..H {
            for x in x0 - 14..=xe + 14 {
                if !inb(x, y) {
                    continue;
                }
                let i = idx(x, y);
                let wob = (vnoise(x as f32 / 5.0, y as f32 / 5.0, 0xba1) - 0.5) * 0.5;
                let d = ((x - cx) as f32 / 34.0).powi(2) + ((y - cy) as f32 / 13.0).powi(2) + wob;
                let ghat = y <= ye + 2 && x >= x0 + 4 && x <= xe - 4;
                if !ghat && d > 1.25 {
                    continue;
                }
                self.o[i] = O_NONE;
                self.road[i] = false;
                self.ramp[i] = false;
                if ghat {
                    self.t[i] = T_COBBLE;
                    elev[i] = 0.5 - (y - ye) as f32 * 0.07;
                } else if d > 1.0 || y <= ye + 2 {
                    self.t[i] = T_SAND;
                    elev[i] = 0.37;
                } else {
                    self.t[i] = T_WATER;
                    elev[i] = 0.2;
                }
            }
        }
        // the main street from the north, the shore street along the ghats
        let mx = x0 + w / 2;
        self.street((mx - 1, y0), (mx - 1, ye), 2);
        self.street((x0 + 1, ye - 1), (xe - 1, ye - 1), 2);
        self.street((x0 + 1, y0 + 13), (xe - 1, y0 + 13), 2);
        // the great temple at the head of the town
        self.building(mx - 4, y0 + 2, WALL_TEMPLE, V_HEALER);
        // a tank with steps before it
        self.tank(mx + 6, y0 + 3, 7, 6);
        // havelis along the streets
        self.building(x0 + 2, y0 + 3, WALL_HAVELI, V_LIBRARIAN);
        self.building(x0 + 2, y0 + 16, WALL_HAVELI, V_PROV);
        self.building(x0 + 11, y0 + 16, WALL_HAVELI, V_SMITH);
        self.building(mx + 3, y0 + 16, WALL_HAVELI, V_BARKEEP);
        self.building(mx + 12, y0 + 16, WALL_HAVELI, V_BANKER);
        self.add_floors(mx + 3, y0 + 16, &[INN_ROOMS]);
        // the bazaar along the cross street: wares set out on either side
        for (dx, o) in [
            (3, O_JARS),
            (4, O_SACKS),
            (7, O_CRATE),
            (9, O_JARS),
            (12, O_SACKS),
            (27, O_JARS),
            (28, O_CRATE),
            (31, O_SACKS),
            (33, O_JARS),
            (36, O_PLANT),
        ] {
            self.set_o(x0 + dx, y0 + 12, o);
        }
        for (dx, dy) in [
            (mx - x0 - 2, 10),
            (mx - x0 + 1, 10),
            (6, h - 3),
            (w - 7, h - 3),
            (mx - x0 - 2, 22),
        ] {
            self.set_o(x0 + dx, y0 + dy, O_LAMP);
        }
        let mut spots = vec![];
        for _ in 0..45 {
            spots.push((x0 + rng.range(1, w - 2), y0 + rng.range(1, h - 2)));
        }
        self.palms(&spots);
        for (dx, dy) in [(mx - x0 - 2, 1), (mx - x0 + 1, 1), (2, 13), (w - 3, 14)] {
            self.npcs.push((V_GUARD, x0 + dx, y0 + dy));
        }
        for (dx, dy) in [
            (8, 13),
            (14, 14),
            (25, 13),
            (32, 14),
            (20, 26),
            (10, 26),
            (30, 26),
            (mx - x0, 9),
        ] {
            self.npcs.push((V_FOLK, x0 + dx, y0 + dy));
        }
        self.raj.push((x0, y0, x0 + w, y0 + h + 3));
        self.pois.push(("Kanakpur", mx, y0 + h / 2));
        self.pois.push(("Ghats", mx, ye + 1));
        self.pois.push(("Great temple", mx, y0 + 5));
    }
}
