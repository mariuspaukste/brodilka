//! Karak Dhum, the dwarf hold in the eastern range: a great bowl hewn out of
//! the mountains north of the elves' valley, walled in by rock on every side.
//!
//! - **The gate** on the west face: a passage cut through the rock between
//!   walls of dark granite, two statues of the old kings either side of it,
//!   and a forecourt where the gatewardens stand.
//! - **The hall street** runs from the gate through the hold to the mines in
//!   the east, with the King's hall at its head and the trades in squat
//!   granite halls on either side under heavy slate roofs, their chimneys
//!   all smoking.
//! - **The great forge**, an open yard of forges and anvils where anyone may
//!   smelt and smith, and **the mines**, a gallery driven into the rock with
//!   ore carts standing in it.
//!
//! Roads run down from the forecourt to the elves' valley road and up to the
//! old pass over the range. Everything here draws on its own stream of
//! chance, so the rest of a seed's world comes out as it always has.

use super::*;

/// the corner and size of the hold's floor (the rock round it not included)
pub const HOLD: (i32, i32, i32, i32) = (160, 36, 40, 20);

impl Map {
    /// In the dwarf hold, its gate forecourt or its mines: guarded ground.
    pub fn in_hold(&self, x: i32, y: i32) -> bool {
        let (x0, y0, x1, y1) = self.hold;
        x >= x0 && y >= y0 && x < x1 && y < y1
    }

    /// A run of dark granite wall from a to b (inclusive), along x or y.
    fn granite_wall(&mut self, a: (i32, i32), b: (i32, i32)) {
        for y in a.1.min(b.1)..=a.1.max(b.1) {
            for x in a.0.min(b.0)..=a.0.max(b.0) {
                if inb(x, y) {
                    let i = idx(x, y);
                    self.t[i] = T_COBBLE;
                    self.o[i] = O_WALL;
                    self.style[i] = WALL_DWARF;
                    self.road[i] = false;
                }
            }
        }
    }

    /// Lay a rectangle of ground (x0..x1, y0..y1 exclusive) as `t`, bare, at
    /// height `e`; `road` marks it as a way to walk.
    #[allow(clippy::too_many_arguments)]
    fn hew(
        &mut self,
        elev: &mut [f32],
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        t: u8,
        e: f32,
        road: bool,
    ) {
        for y in y0..y1 {
            for x in x0..x1 {
                if !inb(x, y) {
                    continue;
                }
                let i = idx(x, y);
                self.t[i] = t;
                self.o[i] = O_NONE;
                self.style[i] = 0;
                self.ramp[i] = false;
                self.road[i] = road;
                elev[i] = e;
            }
        }
    }

    /// Karak Dhum: the hold, its gate, its forge and mines, and the roads to it.
    pub(super) fn dwarf_hold(&mut self, elev: &mut [f32], seed: u32) {
        let mut rng = Rng::new(seed as u64 ^ 0xd3a7_f00d);
        let (x0, y0, w, h) = HOLD;
        let (x1, y1) = (x0 + w, y0 + h);
        // the street rows, and the gate's forecourt west of the rock
        let (s0, s1) = (y0 + 8, y0 + 10);
        let fx = x0 - 12;

        // ---- the mountain: a ragged mass of rock six to nine tiles thick
        // round the hold (the old pass and the elves' valley left alone)
        for y in y0 - 10..y1 + 10 {
            for x in x0 - 11..x1 + 10 {
                if !inb(x, y) {
                    continue;
                }
                let dx = if x < x0 { x0 - x } else { (x - x1 + 1).max(0) };
                let dy = if y < y0 { y0 - y } else { (y - y1 + 1).max(0) };
                if dx == 0 && dy == 0 {
                    continue;
                }
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                let thick = 6.0 + vnoise(x as f32 / 4.0, y as f32 / 4.0, seed ^ 0xd1) * 3.0;
                let i = idx(x, y);
                if d < thick && !self.road[i] && !self.in_elf(x, y) {
                    self.t[i] = T_MOUNT;
                    self.o[i] = O_NONE;
                    self.style[i] = 0;
                    elev[i] = elev[i].max(0.82);
                }
            }
        }

        // ---- the floor of the hold: hewn rock, high and level
        self.hew(elev, x0, y0, x1, y1, T_CAVE, 0.67, false);
        // the hall street from the gate to the mines, the back lane and the
        // cross street between them, and the forge yard
        self.hew(elev, fx, s0, x1, s1 + 1, T_COBBLE, 0.67, true);
        self.hew(elev, x0, y0 + 18, x0 + 30, y1, T_COBBLE, 0.67, true);
        self.hew(elev, x0 + 28, y0 + 11, x0 + 30, y1, T_COBBLE, 0.67, true);
        self.hew(elev, x0 + 31, y0 + 12, x1, y1, T_COBBLE, 0.67, false);
        // the mines: a gallery driven east into the rock
        self.hew(elev, x1, s0, x1 + 4, s1 + 1, T_CAVE, 0.67, false);
        // the forecourt before the gate, a step down from the hold
        self.hew(elev, fx - 2, y0 + 4, fx + 3, y0 + 15, T_COBBLE, 0.6, true);

        // ---- the gate: the passage through the rock walled in granite, a
        // gatehouse tower either side of its mouth, the old kings' statues
        // before it and braziers on the forecourt
        self.granite_wall((fx + 3, s0 - 1), (x0 - 1, s0 - 1));
        self.granite_wall((fx + 3, s1 + 1), (x0 - 1, s1 + 1));
        for (tx, ty) in [(fx + 3, s0 - 3), (fx + 3, s1 + 1)] {
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)] {
                let (x, y) = (tx + dx, ty + dy);
                self.granite_wall((x, y), (x, y));
            }
        }
        // the cliff face either side of the gatehouse, sheer and solid
        for y in y0 - 2..y1 + 2 {
            for x in fx + 3..x0 {
                let i = idx(x, y);
                if self.o[i] != O_WALL && !(s0..=s1).contains(&y) {
                    self.t[i] = T_MOUNT;
                    self.o[i] = O_NONE;
                    self.road[i] = false;
                    elev[i] = elev[i].max(0.82);
                }
            }
        }
        self.set_o(fx + 1, s0 - 2, O_STATUE);
        self.set_o(fx + 1, s1 + 2, O_STATUE);
        for (x, y) in [
            (fx - 1, s0 - 3),
            (fx - 1, s1 + 3),
            (fx + 2, s0 - 4),
            (fx + 2, s1 + 4),
        ] {
            self.set_o(x, y, O_LAMP);
        }

        // ---- the halls: the trades along the street, the King's hall at
        // its head, the alehouse and the vault on the back lane
        self.building(x0 + 1, y0 + 1, WALL_DWARF, V_SMITH);
        self.building(x0 + 10, y0 + 1, WALL_DWARF, V_HEALER);
        self.keep_styled(x0 + 19, y0, V_THANE, WALL_DWARF);
        self.building(x0 + 32, y0 + 1, WALL_DWARF, V_MAGE);
        self.building(x0 + 1, y0 + 11, WALL_DWARF, V_BANKER);
        self.building(x0 + 10, y0 + 11, WALL_DWARF, V_BARKEEP);
        self.building(x0 + 19, y0 + 11, WALL_DWARF, V_PROV);
        self.add_floors(x0 + 10, y0 + 11, &[INN_ROOMS]);
        self.guests.push((V_FOLK, x0 + 14, y0 + 14, 1));
        self.add_floors(x0 + 32, y0 + 1, &[STUDY]);
        // the halls on the north side stand against the rock, which closes
        // over and between them; the old kings' statues stand before it
        for y in y0..y0 + 7 {
            for x in x0..x1 {
                let i = idx(x, y);
                if self.t[i] == T_CAVE {
                    self.t[i] = T_MOUNT;
                    elev[i] = 0.82;
                }
            }
        }
        for lx in [0, 9, 18, 31] {
            self.set_o(x0 + lx, y0 + 7, O_STATUE);
        }
        // lamps along the street, against the backs of the halls on the south
        for lx in [5, 14, 23, 34] {
            self.set_o(x0 + lx, s1, O_LAMP);
        }
        self.set_o(x0, y0 + 16, O_LAMP);

        // ---- the great forge: forges and anvils in the open yard, fuel and
        // ore piled round them
        for (lx, ly) in [(33, 13), (37, 13)] {
            self.set_o(x0 + lx, y0 + ly, O_FORGE);
        }
        for (lx, ly) in [(33, 15), (37, 15), (35, 17)] {
            self.set_o(x0 + lx, y0 + ly, O_ANVIL);
        }
        for (lx, ly, o) in [
            (31, 12, O_BARREL),
            (32, 12, O_BARREL),
            (39, 12, O_CRATE),
            (39, 13, O_SACKS),
            (39, 17, O_CRATE),
            (39, 19, O_BARREL),
            (31, 19, O_SACKS),
            (35, 12, O_LAMP),
        ] {
            self.set_o(x0 + lx, y0 + ly, o);
        }
        self.place(x0 + 31, y0 + 15, O_CART, false);

        // ---- the mines: carts of ore and crates of tools in the gallery
        self.place(x1 + 1, s0, O_CART, true);
        self.set_o(x1 + 3, s0, O_CRATE);
        self.set_o(x1 + 3, s1, O_BARREL);
        self.set_o(x1 + 1, s1, O_SACKS);
        self.set_o(x1 - 1, s0 - 1, O_LAMP);

        // ---- the dwarves: gatewardens at the gate, the King's door and the
        // mine, smiths at the forges, miners in the gallery, folk in the halls
        for (x, y) in [
            (fx, s0),
            (fx, s1),
            (x0, s0),
            (x0 + 23, s0),
            (x0 + 26, s0),
            (x1 - 2, s1),
        ] {
            self.npcs.push((V_GUARD, x, y));
        }
        let mut folk = vec![
            (x0 + 35, y0 + 14),
            (x0 + 34, y0 + 18),
            (x1 + 3, s0 + 1),
            (x1 + 2, s0 + 1),
        ];
        for _ in 0..5 {
            folk.push((
                x0 + rng.range(2, 27),
                [s0, s0 + 1, s1, y0 + 18, y0 + 19][rng.range(0, 4) as usize],
            ));
        }
        for (x, y) in folk {
            if self.obj(x, y) == O_NONE {
                self.npcs.push((V_FOLK, x, y));
            }
        }

        // ---- the roads: down to the road into the elves' valley, with a
        // post on it, and up to the foot of the old pass over the range
        let gate = (fx - 2, s0 + 1);
        let valley = self
            .roads
            .iter()
            .find(|r| r.to == "valley")
            .and_then(|r| {
                r.tiles
                    .iter()
                    .copied()
                    .filter(|&(x, _)| x < x0 - 6)
                    .min_by_key(|&(x, y)| (x - gate.0).pow(2) + (y - gate.1).pow(2))
            })
            .unwrap_or((150, 78));
        let down = self.carve_road(gate, valley, seed ^ 61);
        self.road_post(&down);
        self.carve_road((fx - 1, y0 + 4), (156, 27), seed ^ 62);

        self.hold = (fx - 3, y0, x1 + 4, y1);
        self.pois.push(("Karak Dhum", x0 + w / 2, s0 + 1));
        self.pois.push(("Dwarf gate", fx, s0 + 1));
        self.pois.push(("King's hall", x0 + 25, y0 + 3));
        self.pois.push(("Great forge", x0 + 35, y0 + 15));
        self.pois.push(("Mines", x1 + 2, s0 + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hold stands in the eastern range walled in by rock, can be walked
    /// to from the town through its gate, keeps its forge and its King, and
    /// every hall's door opens onto ground the town can reach.
    #[test]
    fn karak_dhum_is_hewn_into_the_mountains_and_reachable() {
        for seed in [1409u32, 7, 99, 1997] {
            let m = generate(seed);
            let (x0, y0, w, h) = HOLD;
            assert!(m.pois.iter().any(|p| p.0 == "Karak Dhum"));
            // rock above and below the hold, all along it
            for x in x0..x0 + w {
                assert_eq!(
                    m.tile(x, y0 - 2),
                    T_MOUNT,
                    "seed {seed}: rock at {x},{}",
                    y0 - 2
                );
                assert_eq!(m.tile(x, y0 + h + 1), T_MOUNT, "seed {seed}: rock at {x}");
            }
            let (_, kx, ky) = *m.npcs.iter().find(|n| n.0 == V_THANE).expect("king");
            assert_eq!(m.obj(kx, ky - 1), O_THRONE);
            assert!(m.in_hold(kx, ky) && m.guarded(kx, ky));
            assert!(
                m.reach[idx(x0 + w / 2, y0 + 9)],
                "seed {seed}: hold cut off"
            );
            assert!(
                m.reach[idx(x0 + w + 2, y0 + 9)],
                "seed {seed}: mines cut off"
            );
            let (sx, sy) = m.start;
            let mut m = m;
            assert!(m.find_path_as(sx, sy, x0 + 20, y0 + 9, 1, PERSON).is_some());
            let inside: Vec<&Building> = m
                .buildings
                .iter()
                .filter(|b| b.style == WALL_DWARF)
                .collect();
            assert_eq!(inside.len(), 7);
            for b in inside {
                let ok = (1..b.w - 1).any(|x| {
                    let (dx, dy) = (b.x + x, b.y + b.h);
                    m.obj(dx, dy - 1) == O_DOOR && m.reach[idx(dx, dy)]
                });
                assert!(ok, "seed {seed}: hall at {},{} cut off", b.x, b.y);
            }
            let yard = |o: u8| {
                (y0..y0 + h)
                    .any(|y| (x0..x0 + w).any(|x| m.obj(x, y) == o && m.reach[idx(x, y + 1)]))
            };
            assert!(yard(O_FORGE) && yard(O_ANVIL));
            // no spawns inside, and the Shadow kept out by the rock
            assert!(m.spawns.iter().all(|s| !m.in_hold(s.1, s.2)));
        }
    }
}
