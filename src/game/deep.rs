//! The sleeper in the deep: the awakening of Cthulhu.
//!
//! Sunken R'lyeh lies in the bay below Kanakpur (`Map::rlyeh`). Every so
//! many days the stars come right:
//!
//! 1. **Dreams.** Half a day before, the sleepers of Kanakpur cry out in
//!    their dreams of a drowned city; the news spreads as rumours do.
//! 2. **The awakening.** R'lyeh rises from the bay, its cyclopean stones
//!    standing in the foam, and Cthulhu wakes and wades out among them. A
//!    storm breaks over the tropics, the folk of the southern cities cry
//!    out, and deep ones come ashore on the beaches round the bay.
//! 3. **The sinking.** After a day R'lyeh goes down again and he sleeps,
//!    taking the deep ones with him. Or, if someone strikes him down, he is
//!    driven back into the deep at once, and the sea casts up the treasures
//!    of R'lyeh at their feet. That is not dead which can eternal lie: he
//!    wakes again, later.
//!
//! Cthulhu keeps to the sea within reach of R'lyeh. He comes as near the
//! shore as the water lets him to get at anyone on it, strikes with claws
//! and tentacles from three tiles off, casts, and floods the minds of those
//! who look on him, draining their mana. The timers live in the world
//! simulation (`Sim::deep_t`, `awake`, `omen`) so a save keeps them; what
//! stands in the sea is put back after a load.
use super::*;
use crate::sim::DAY;

/// how long R'lyeh stands once risen
pub const AWAKE: f32 = DAY;
/// how far from R'lyeh Cthulhu wades
const LEASH: f32 = 13.0;
/// how far his claws and tentacles reach, in tiles
pub const REACH: f32 = 3.2;
/// at most this many deep ones ashore at once
const DEEP_ONES: usize = 5;
/// the ruins round R'lyeh's heart: offset and which (monolith, door, jumble)
const RUINS: [(i32, i32, u8); 7] = [
    (0, -3, 1),
    (-3, -1, 0),
    (3, -1, 0),
    (-2, 3, 2),
    (2, 3, 2),
    (-4, 2, 0),
    (4, 2, 2),
];
const CALL: &str = "Ph'nglui mglw'nafh Cthulhu R'lyeh wgah'nagl fhtagn!";
/// what the folk of the south cry while R'lyeh stands
pub const CRIES: [&str; 6] = [
    "He comes from the sea!",
    "Iä! Iä! Cthulhu fhtagn!",
    "The temple bells will not stop ringing!",
    "Keep from the shore, stranger!",
    "I dreamt of a city of green stone...",
    "Do not look at it! Do not look!",
];

impl Game {
    fn deep_place(&self) -> String {
        crate::pack::t("Kanakpur")
    }

    /// Advance the sleeper's clock: dreams, the awakening, the sinking.
    pub(super) fn deep_tick(&mut self, dt: f32) {
        let Some(rl) = self.map.rlyeh else { return };
        if self.sim.awake <= 0.0 {
            self.sim.deep_t -= dt;
            if !self.sim.omen && self.sim.deep_t < DAY * 0.5 {
                self.sim.omen = true;
                let place = self.deep_place();
                self.news(
                    format!("The sleepers of {place} cry out in their dreams of a drowned city of green stone."),
                    C_LORE,
                );
            }
            if self.sim.deep_t <= 0.0 {
                self.awaken();
            }
            return;
        }
        self.sim.awake -= dt;
        if self.sim.awake <= 0.0 {
            self.sink(false);
            return;
        }
        // what stands in the sea (put back after a load), and deep ones
        // coming ashore now and then
        self.rise(rl);
        let ashore = self.deep_ones().len();
        if ashore < DEEP_ONES && self.rng.chance(dt / 20.0) {
            self.deep_one_ashore(rl);
        }
    }

    /// The stars are right.
    pub fn awaken(&mut self) {
        let Some(rl) = self.map.rlyeh else { return };
        self.sim.awake = AWAKE;
        self.sim.omen = false;
        let place = self.deep_place();
        self.news(
            format!(
                "The stars are right! R'lyeh rises from the bay of {place}, and Cthulhu wakes!"
            ),
            C_WARN,
        );
        self.rise(rl);
        for _ in 0..3 {
            self.deep_one_ashore(rl);
        }
    }

    /// The living Cthulhu, if he stands in the sea.
    pub fn cthulhu(&self) -> Option<usize> {
        (1..self.ents.len()).find(|&i| {
            !self.ents[i].dead && matches!(self.ents[i].k, EK::Mon(m) if MONS[m].id == "cthulhu")
        })
    }
    fn deep_ones(&self) -> Vec<usize> {
        (1..self.ents.len())
            .filter(|&i| {
                !self.ents[i].dead
                    && matches!(self.ents[i].k, EK::Mon(m) if MONS[m].id == "deepone")
            })
            .collect()
    }

    /// Raise the ruins and Cthulhu among them, if they are not already up.
    fn rise(&mut self, rl: (i32, i32)) {
        for (dx, dy, k) in RUINS {
            let (x, y) = (rl.0 + dx, rl.1 + dy);
            if inb(x, y) && self.map.t[idx(x, y)] == T_WATER && self.map.o[idx(x, y)] == O_NONE {
                self.map.o[idx(x, y)] = O_RUIN;
                self.map.style[idx(x, y)] = k;
            }
        }
        if self.cthulhu().is_none() {
            let mi = mon_idx("cthulhu");
            let d = &MONS[mi];
            let id = self.nid();
            let mut e = Ent::new(id, EK::Mon(mi), d.name.to_string(), rl.0, rl.1, d.hp);
            e.home = rl;
            e.mana = 400;
            e.mmana = 400;
            e.dir8 = 3;
            e.anim.vdir = 3;
            e.talk(CALL, 0x8ad8a0ff);
            self.ents.push(e);
        }
    }

    /// R'lyeh goes down, with Cthulhu and the deep ones; struck down or not,
    /// he will wake again.
    fn sink(&mut self, slain: bool) {
        let Some(rl) = self.map.rlyeh else { return };
        self.sim.awake = 0.0;
        self.sim.omen = false;
        self.sim.deep_t = DAY
            * if slain {
                9.0 + 4.0 * self.rng.f()
            } else {
                5.0 + 3.0 * self.rng.f()
            };
        for y in rl.1 - 6..=rl.1 + 6 {
            for x in rl.0 - 6..=rl.0 + 6 {
                if inb(x, y) && self.map.o[idx(x, y)] == O_RUIN {
                    self.map.o[idx(x, y)] = O_NONE;
                }
            }
        }
        let mut gone = self.deep_ones();
        gone.extend(self.cthulhu());
        for i in gone {
            self.ents[i].dead = true;
            if self.ents[0].combat.target == self.ents[i].id {
                self.ents[0].combat.target = 0;
            }
        }
        let place = self.deep_place();
        let line = if slain {
            format!(
                "{} has driven Cthulhu down into the deep, and R'lyeh sinks again beneath the bay of {place}. Yet that is not dead which can eternal lie.",
                self.pl.name
            )
        } else {
            format!("R'lyeh sinks beneath the bay of {place}. Cthulhu sleeps again, for now.")
        };
        self.news(line, C_LORE);
        self.hud_ver += 1;
    }

    /// Cthulhu has fallen: no body is left in the sea, but its treasures
    /// wash up at the victor's feet, and R'lyeh goes down.
    pub(super) fn cthulhu_slain(&mut self, by_player: bool) {
        // the corpse kill_mon left in the water
        if let Some(k) = self
            .corpses
            .iter()
            .rposition(|c| c.name.contains("Cthulhu"))
        {
            self.corpses.remove(k);
        }
        if by_player {
            let gold = self.rng.range(4000, 7000) as i64;
            self.pl.gold += gold;
            for _ in 0..self.rng.range(3, 5) {
                let g = *self.rng.pick(&GEMS);
                self.pl.add(Item::new(item_idx(g), 1));
            }
            self.pl.add(Item::new(item_idx("scroll_flamestrike"), 1));
            self.msg(
                &format!(
                    "The sea casts up the treasures of R'lyeh at your feet: {gold} gold and gems."
                ),
                C_GOOD,
            );
        }
        self.sink(true);
    }

    /// A deep one comes up out of the sea onto a beach round the bay.
    fn deep_one_ashore(&mut self, rl: (i32, i32)) {
        let mut spots = vec![];
        for y in rl.1 - 16..=rl.1 + 16 {
            for x in rl.0 - 18..=rl.0 + 18 {
                if !self.map.passable(x, y)
                    || self.map.guarded(x, y)
                    || !self.map.reach[idx(x, y)]
                    || self.ents[0].dist(x as f32, y as f32) < 5.0
                {
                    continue;
                }
                let shore = DIRS.iter().any(|&(dx, dy)| {
                    inb(x + dx, y + dy) && self.map.t[idx(x + dx, y + dy)] == T_WATER
                });
                if shore {
                    spots.push((x, y));
                }
            }
        }
        if spots.is_empty() {
            return;
        }
        let (x, y) = *self.rng.pick(&spots);
        let mi = mon_idx("deepone");
        let d = &MONS[mi];
        let id = self.nid();
        let mut e = Ent::new(id, EK::Mon(mi), d.name.to_string(), x, y, d.hp);
        e.ai_t = self.rng.f() * 2.0;
        self.ents.push(e);
    }

    /// Open sea within Cthulhu's reach of R'lyeh.
    fn sea(&self, x: i32, y: i32) -> bool {
        let Some(rl) = self.map.rlyeh else {
            return false;
        };
        inb(x, y)
            && self.map.t[idx(x, y)] == T_WATER
            && self.map.o[idx(x, y)] != O_SHIP
            && (((x - rl.0).pow(2) + (y - rl.1).pow(2)) as f32).sqrt() <= LEASH
    }
    /// A step through the sea toward (gx, gy), if one brings him nearer.
    fn sea_step(&self, i: usize, gx: f32, gy: f32) -> Option<(i32, i32)> {
        let (x, y) = self.ents[i].tile();
        let dist = |x: i32, y: i32| ((x as f32 - gx).powi(2) + (y as f32 - gy).powi(2)).sqrt();
        DIRS.iter()
            .map(|&(dx, dy)| (x + dx, y + dy))
            .filter(|&(nx, ny)| self.sea(nx, ny))
            .min_by(|a, b| dist(a.0, a.1).total_cmp(&dist(b.0, b.1)))
            .filter(|&(nx, ny)| dist(nx, ny) < dist(x, y) - 0.01)
    }

    pub(super) fn cthulhu_tick(&mut self, i: usize, mi: usize, dt: f32) {
        let d = &MONS[mi];
        self.move_ent(i, d.mv, dt);
        {
            let e = &mut self.ents[i];
            e.combat.swing = (e.combat.swing - dt).max(0.0);
            e.combat.cast_t -= dt;
            e.combat.breath_t -= dt;
            e.ai_t -= dt;
            e.regen_t += dt;
            if e.regen_t > 2.0 {
                e.regen_t = 0.0;
                e.hp = (e.hp + d.regen).min(e.mhp);
            }
        }
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let pd = self.ents[i].dist(px, py);
        let pid = self.ents[0].id;
        let seen = !self.pl.ghost && pd < 14.0;
        if !seen {
            self.ents[i].combat.target = 0;
            // wade slowly about R'lyeh
            if self.ents[i].path.is_empty() && self.ents[i].ai_t <= 0.0 {
                self.ents[i].ai_t = 3.0 + self.rng.f() * 4.0;
                let (hx, hy) = self.ents[i].home;
                let (gx, gy) = (hx + self.rng.range(-4, 4), hy + self.rng.range(-4, 4));
                if let Some(s) = self.sea_step(i, gx as f32, gy as f32) {
                    self.ents[i].path.push(s);
                }
            }
            return;
        }
        if self.ents[i].combat.target != pid {
            self.ents[i].combat.target = pid;
            self.ents[i].talk(CALL, 0x8ad8a0ff);
        }
        // face whoever he is after
        let (ddx, ddy) = (px - self.ents[i].x, py - self.ents[i].y);
        if ddx.abs() + ddy.abs() > 0.05 {
            self.ents[i].dir8 = dir8_of(ddx, ddy);
        }
        // the dreams of Cthulhu flood the minds of those who look on him
        if self.ents[i].combat.breath_t <= 0.0 && pd < 12.0 {
            self.ents[i].combat.breath_t = 14.0 + self.rng.f() * 8.0;
            let p = &mut self.ents[0];
            p.mana /= 2;
            self.msg(
                "The dreams of Cthulhu pour into thy mind! Thy thoughts scatter.",
                C_WARN,
            );
            let dmg = self.rng.range(3, 8);
            self.damage(0, dmg, false);
            return;
        }
        if pd <= REACH {
            if self.ents[i].combat.swing <= 0.0 {
                self.ents[i].combat.swing = d.speed;
                self.ents[i].anim.atk = 1.0;
                let def = self.pl.sk(self.pl.weapon_def().skill);
                let hit = ((d.skill as f32 + 50.0) / ((def + 50.0) * 2.0)).clamp(0.05, 0.95);
                if self.rng.chance(hit) {
                    let mut dmg = self.rng.range(d.dmg.0, d.dmg.1);
                    if self.pl.blocking && self.ents[0].down_t <= 0.0 {
                        let parry = self.pl.sk(PARRY);
                        self.gain(PARRY, true);
                        if self.rng.chance(0.3 + parry / 300.0) {
                            dmg /= 3;
                            self.msg("You turn the blow aside.", C_SYS);
                        }
                    }
                    self.damage(0, dmg, false);
                } else {
                    self.float(px, py, "miss", 0xc0c0c0ff);
                }
            }
            return;
        }
        // spells from afar
        if pd < 11.0 && self.ents[i].combat.cast_t <= 0.0 {
            self.ents[i].combat.cast_t = 4.0 + self.rng.f() * 3.0;
            let sp = *self.rng.pick(d.caster);
            let s = &SPELLS[sp];
            self.ents[i].talk(s.words, C_SPELL);
            let dmg = (self.rng.range(s.amt.0, s.amt.1) as f32 * 1.3) as i32;
            self.spell_fx(s.fx, i, 0, dmg, false);
            return;
        }
        // wade toward them, as near as the sea goes
        if self.ents[i].path.is_empty() {
            if let Some(s) = self.sea_step(i, px, py) {
                self.ents[i].path.push(s);
            }
        }
    }

    /// Where to stand to strike a creature in the sea: the nearest dry tile
    /// to it the player can walk to.
    pub(super) fn shore_by(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let mut best: Option<((i32, i32), i32)> = None;
        for dy in -3..=3 {
            for dx in -3..=3 {
                let (nx, ny) = (x + dx, y + dy);
                if self.map.passable(nx, ny) && self.map.reach[idx(nx, ny)] {
                    let d2 = dx * dx + dy * dy;
                    if best.map_or(true, |b| d2 < b.1) {
                        best = Some(((nx, ny), d2));
                    }
                }
            }
        }
        best.map(|b| b.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal_has(g: &Game, s: &str) -> bool {
        g.journal.iter().any(|(_, t, _)| t.contains(s))
    }
    fn run(g: &mut Game, secs: f32) {
        for _ in 0..(secs * 30.0) as i32 {
            g.update(1.0 / 30.0);
        }
    }

    #[test]
    fn rlyeh_lies_in_the_bay_below_kanakpur() {
        for seed in [1u32, 640, 1997, 12345] {
            let g = Game::new(seed);
            let (x, y) = g.map.rlyeh.expect("no R'lyeh");
            let (kx, ky, kw, kh) = crate::world::KANAK;
            assert!(
                x >= kx - 14 && x < kx + kw + 14 && y > ky + kh,
                "seed {seed}: {x},{y}"
            );
            for (dx, dy, _) in RUINS {
                assert_eq!(g.map.t[idx(x + dx, y + dy)], T_WATER);
            }
        }
    }

    #[test]
    fn cthulhu_wakes_strikes_and_is_driven_down() {
        let mut g = Game::new(1997);
        g.create("Diver", false, 0, "warrior", [0, 0, 0]);
        let rl = g.map.rlyeh.unwrap();
        // the dreams come first
        g.sim.deep_t = DAY * 0.5 + 0.5;
        run(&mut g, 1.0);
        assert!(journal_has(&g, "drowned city"));
        assert!(g.cthulhu().is_none());
        // then the stars are right
        g.sim.deep_t = 0.1;
        run(&mut g, 1.0);
        assert!(journal_has(&g, "R'lyeh rises"));
        let c = g.cthulhu().expect("Cthulhu did not rise");
        let (cx, cy) = g.ents[c].tile();
        assert_eq!(g.map.t[idx(cx, cy)], T_WATER);
        assert!(g.map.o.iter().filter(|&&o| o == O_RUIN).count() >= 5);
        assert!(g.look_for(c).is_some());
        // the player on the shore: he wades up and strikes
        let (sx, sy) = g
            .shore_by(cx, cy - 4)
            .or_else(|| (0..12).find_map(|r| g.shore_by(rl.0, rl.1 - 8 - r)))
            .expect("no shore");
        g.ents[0].x = sx as f32;
        g.ents[0].y = sy as f32;
        g.ents[0].hp = 1000;
        g.ents[0].mhp = 1000;
        let hp = g.ents[0].hp;
        run(&mut g, 20.0);
        assert!(g.ents[0].hp < hp, "Cthulhu never struck");
        let c = g.cthulhu().unwrap();
        // he keeps to the sea
        let (cx, cy) = g.ents[c].tile();
        assert_eq!(g.map.t[idx(cx, cy)], T_WATER);
        // struck down: no corpse in the sea, treasure, and R'lyeh sinks
        let gold = g.pl.gold;
        let hp = g.ents[c].hp;
        g.damage(c, hp + 1, true);
        assert!(g.cthulhu().is_none());
        assert!(g.pl.gold > gold);
        assert!(!g.corpses.iter().any(|k| k.name.contains("Cthulhu")));
        assert!(journal_has(&g, "driven Cthulhu down"));
        assert_eq!(g.sim.awake, 0.0);
        assert!(!g.map.o.contains(&O_RUIN));
        assert!(g.sim.deep_t > DAY * 8.0);
        run(&mut g, 2.0);
        assert!(g.cthulhu().is_none());
    }

    #[test]
    fn rlyeh_sinks_again_and_stands_after_a_load() {
        let mut g = Game::new(640);
        g.create("Sleeper", false, 0, "warrior", [0, 0, 0]);
        g.sim.deep_t = 0.05;
        run(&mut g, 0.5);
        assert!(g.cthulhu().is_some());
        // saved and loaded while R'lyeh stands: it is there again
        let s = g.save();
        assert!(s.contains("sim_deep="));
        let mut h = Game::new(640);
        h.load(&s);
        assert!(h.sim.awake > 0.0);
        run(&mut h, 0.5);
        assert!(h.cthulhu().is_some());
        assert!(h.map.o.contains(&O_RUIN));
        // a day on, it sinks and he sleeps
        h.sim.awake = 0.2;
        run(&mut h, 1.0);
        assert!(h.cthulhu().is_none());
        assert!(!h.map.o.contains(&O_RUIN));
        assert!(journal_has(&h, "sleeps again"));
        assert!(h.deep_ones().is_empty());
    }
}
