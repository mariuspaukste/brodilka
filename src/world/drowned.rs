//! Saltgrave: a fishing town on the eastern shore of the tropics, drowned
//! and taken by the things of the sea.
//!
//! Its people are long gone. Roofless cottages of grey stone stand broken
//! along a cobbled street that runs down to a jetty, with the sea in their
//! rooms. In the square the deep ones have raised an idol of their sleeping
//! god among skulls and black candles. Wrecked boats lie on the beach. Deep
//! ones walk the street, a priest of theirs keeps the idol, and great crabs
//! scuttle on the shore (`world::street_beasts`'s neighbour in `generate`
//! sets their spawners). It is no one's ground: no guard comes here.

use super::*;

/// the corner and size of Saltgrave (x0, y0, w, h); the sea runs east of it
pub const SALT: (i32, i32, i32, i32) = (190, 262, 30, 22);

impl Map {
    pub(super) fn saltgrave(&mut self, elev: &mut [f32], rng: &mut Rng) {
        let (x0, y0, w, h) = SALT;
        let (xe, ye) = (x0 + w - 1, y0 + h - 1);
        self.level_out(elev, x0, y0, w, h, 2);
        // the open sea east of the town, and its beach
        for y in y0 - 10..=ye + 10 {
            for x in xe - 1..W {
                if !inb(x, y) {
                    continue;
                }
                let i = idx(x, y);
                self.o[i] = O_NONE;
                self.road[i] = false;
                self.ramp[i] = false;
                if x <= xe {
                    self.t[i] = T_SAND;
                    elev[i] = 0.5;
                } else {
                    self.t[i] = T_WATER;
                    elev[i] = 0.2;
                }
            }
        }
        // bare, salt-killed ground: dirt and drifts of sand
        for y in y0..=ye {
            for x in x0..xe - 1 {
                let i = idx(x, y);
                self.t[i] = if x > xe - 6 || hash(x, y, 0x5a17) < 0.25 {
                    T_SAND
                } else {
                    T_DIRT
                };
            }
        }
        // the street down to the sea and the jetty out into it
        let sy = y0 + 10;
        self.street((x0, sy), (xe, sy), 2);
        for y in sy..sy + 2 {
            for x in xe + 1..=xe + 8 {
                let i = idx(x, y);
                self.t[i] = T_BRIDGE;
                self.road[i] = true;
            }
        }
        // the square, where the idol stands
        for y in y0 + 8..=y0 + 13 {
            for x in x0 + 12..=x0 + 18 {
                self.t[idx(x, y)] = T_COBBLE;
            }
        }
        // broken cottages, roofless, the walls fallen in places
        let cottages = [
            (x0 + 2, y0 + 2, 7, 6, false),
            (x0 + 11, y0 + 1, 6, 6, false),
            (x0 + 20, y0 + 2, 6, 6, false),
            (x0 + 2, y0 + 13, 6, 6, true),
            (x0 + 10, y0 + 15, 7, 6, true),
            (x0 + 20, y0 + 14, 5, 6, true),
        ];
        for (bx, by, bw, bh, south) in cottages {
            let door = bx + bw / 2;
            for y in by..by + bh {
                for x in bx..bx + bw {
                    let i = idx(x, y);
                    let edge = x == bx || y == by || x == bx + bw - 1 || y == by + bh - 1;
                    self.t[i] = T_FLOOR;
                    self.o[i] = O_NONE;
                    if !edge {
                        continue;
                    }
                    // the doorway faces the street
                    let doorway = x == door && if south { y == by } else { y == by + bh - 1 };
                    let r = rng.f();
                    if doorway || r < 0.15 {
                        // fallen: rubble and weed
                        if r < 0.06 {
                            self.o[i] = O_BOULDER;
                        }
                        continue;
                    }
                    self.o[i] = O_WALL;
                    self.style[i] = if r < 0.45 {
                        WALL_DROWNED_LOW
                    } else {
                        WALL_DROWNED
                    };
                }
            }
            // the sea in the room, and what the tide left
            let (ix, iy) = (bx + 1 + rng.range(0, bw - 3), by + 1 + rng.range(0, bh - 3));
            self.t[idx(ix, iy)] = T_WATER;
            for _ in 0..2 {
                let (fx, fy) = (bx + 1 + rng.range(0, bw - 3), by + 1 + rng.range(0, bh - 3));
                let i = idx(fx, fy);
                if self.o[i] == O_NONE && self.t[i] != T_WATER {
                    self.o[i] = *rng.pick(&[O_BARREL, O_CRATE, O_SACKS, O_BONES, O_SKULLS]);
                }
            }
        }
        // the idol of the sleeper, with skulls and black candles at its feet
        let (ix, iy) = (x0 + 15, y0 + 9);
        self.o[idx(ix, iy)] = O_RUIN;
        self.style[idx(ix, iy)] = RUIN_IDOL;
        for (dx, dy, o) in [
            (-2, 0, O_CANDLES),
            (2, 0, O_CANDLES),
            (-1, 1, O_SKULLS),
            (1, 1, O_BONES),
            (-3, 3, O_SKULLS),
            (3, 3, O_SKULLS),
        ] {
            self.set_o(ix + dx, iy + dy, o);
        }
        // tide pools in the open ground, wrecks on the beach and out by the jetty
        for _ in 0..12 {
            let (x, y) = (x0 + rng.range(0, w - 3), y0 + rng.range(0, h - 1));
            let i = idx(x, y);
            if matches!(self.t[i], T_DIRT | T_SAND) && self.o[i] == O_NONE && !self.road[i] {
                self.t[i] = T_WATER;
            }
        }
        for (x, y) in [(xe - 1, y0 + 4), (xe - 2, y0 + 17), (xe + 5, sy + 3)] {
            let i = idx(x, y);
            self.o[i] = O_RUIN;
            self.style[i] = RUIN_WRECK;
        }
        for _ in 0..10 {
            let (x, y) = (x0 + rng.range(0, w - 1), y0 + rng.range(0, h - 1));
            let i = idx(x, y);
            if self.o[i] == O_NONE && self.t[i] != T_WATER && !self.road[i] {
                self.o[i] = if rng.f() < 0.5 { O_BONES } else { O_BOULDER };
            }
        }
        self.pois.push(("Saltgrave", x0 + 15, sy));
    }
}
