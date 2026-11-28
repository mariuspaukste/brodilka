//! The people of Saltgrave: not what they seem.
//!
//! By day the drowned town's folk pass for fishermen and their wives:
//! pale, stooped, bald or thin-haired, with wide unblinking eyes. They don't
//! attack a stranger. They stare, turn their backs, mutter that you're not
//! from here, and some of them trail you down the street at a distance. The
//! longer you stay the more they watch (`Salt::suspicion`). Night, the idol,
//! poking about their cottages, a drawn weapon, and R'lyeh standing in the
//! bay all make it climb faster:
//!
//! - first you feel eyes on your back;
//! - then more of them come out into the street, and none of them speaks;
//! - then a bell tolls from the square and they gather round you;
//! - then **Saltgrave turns**. Their faces change, they were never quite
//!   human, and the whole town comes for you, with deep ones rising from the
//!   sea to join them.
//!
//! Striking one of them turns the town at once. Leave and stay away, and
//! after a while Saltgrave falls quiet again, the deep ones go back into the
//! sea, and its people go back to their nets as if nothing had happened.
//! None of this is saved: a new day in a loaded game finds the town calm.
use super::*;

/// suspicion at which each warning is given, and at which the town turns
const WATCHED: [f32; 3] = [25.0, 55.0, 85.0];
const TURN: f32 = 100.0;
/// how long the town stays turned once you are out of it
const TURNED: f32 = 45.0;

/// What the folk mutter at a stranger.
const WHISPERS: [&str; 10] = [
    "Ye're not from Saltgrave.",
    "We keep to ourselves here, stranger.",
    "Best be gone before the tide turns.",
    "Fish don't come to outsiders.",
    "Hush. They're listening.",
    "Ye've the look of the dry land on ye.",
    "Father went down to the reef. He's better now.",
    "Don't go near the square after dark.",
    "*stares at you without blinking*",
    "*turns away*",
];
/// What the priest intones at the idol.
const SERMON: [&str; 4] = [
    "The sleeper dreams, and we dream with him.",
    "Kneel, dry one, or go.",
    "Below the waves there is no dying.",
    "When the stars are right, the sea gives back.",
];

/// The mood of Saltgrave.
#[derive(Clone, Debug, Default)]
pub struct Salt {
    /// how closely they watch the player, 0–100
    pub suspicion: f32,
    /// the last warning given (0 none, 1–3)
    pub level: u8,
    /// seconds the town stays turned (0: calm)
    pub turned: f32,
    /// the deep ones that rose from the sea when it turned
    pub risen: Vec<u32>,
}

/// The folk of Saltgrave and their priest: harmless-seeming until the town
/// turns.
pub fn salt_folk(id: &str) -> bool {
    matches!(id, "saltfolk" | "deeppriest")
}

impl Game {
    /// In or just about Saltgrave.
    pub fn in_salt(&self, x: i32, y: i32) -> bool {
        let (x0, y0, w, h) = SALT;
        x >= x0 - 3 && y >= y0 - 3 && x < x0 + w + 9 && y < y0 + h + 3
    }

    /// Watch the stranger; warn; turn; calm down again.
    pub(super) fn salt_tick(&mut self, dt: f32) {
        let (px, py) = self.ents[0].tile();
        let here = self.in_salt(px, py) && !self.pl.ghost;
        if self.salt.turned > 0.0 {
            // they keep after you while you're in the town
            self.salt.turned = if here {
                self.salt.turned.max(TURNED)
            } else {
                self.salt.turned - dt
            };
            if self.salt.turned <= 0.0 {
                self.salt_calm();
            }
            return;
        }
        if !here {
            self.salt.suspicion = (self.salt.suspicion - dt * 1.5).max(0.0);
            if self.salt.level > 0
                && self.salt.suspicion < WATCHED[self.salt.level as usize - 1] - 10.0
            {
                self.salt.level -= 1;
            }
            return;
        }
        // what makes them watch closer
        let (x0, y0, _, _) = SALT;
        let idol = self.ents[0].dist((x0 + 15) as f32, (y0 + 9) as f32) < 4.5;
        let prying = self.map.t[idx(px, py)] == T_FLOOR;
        // by day, about three minutes in the street before it turns
        let mut rate = 0.55;
        if self.is_night() {
            rate *= 3.0;
        }
        if self.sim.awake > 0.0 {
            rate *= 2.0;
        }
        rate += if idol { 1.5 } else { 0.0 } + if prying { 1.0 } else { 0.0 };
        rate += if self.pl.war { 1.5 } else { 0.0 };
        self.salt.suspicion += dt * rate;
        let lv = WATCHED
            .iter()
            .filter(|&&w| self.salt.suspicion >= w)
            .count() as u8;
        if lv > self.salt.level {
            self.salt.level = lv;
            let line = match lv {
                1 => "You feel eyes on your back. The folk of Saltgrave are watching you.",
                2 => {
                    "Doors creak. More of them are out in the street now, and none of them speaks."
                }
                _ => "A bell tolls from the square. The people of Saltgrave are gathering.",
            };
            self.msg(line, C_WARN);
        }
        if self.salt.suspicion >= TURN {
            self.salt_turn();
        }
    }

    /// Saltgrave turns on the stranger.
    pub fn salt_turn(&mut self) {
        if self.salt.turned > 0.0 {
            return;
        }
        self.salt.turned = TURNED;
        self.salt.suspicion = TURN;
        self.salt.level = 3;
        self.msg(
            "Saltgrave turns on you! The faces of its people slacken and change: they were never quite human.",
            C_WARN,
        );
        let pid = self.ents[0].id;
        for i in 1..self.ents.len() {
            let e = &mut self.ents[i];
            if !e.dead && matches!(e.k, EK::Mon(m) if salt_folk(MONS[m].id)) {
                e.combat.target = pid;
                e.combat.provoked = true;
                e.path.clear();
            }
        }
        // and the deep ones come up out of the sea to join them
        let (x0, y0, w, h) = SALT;
        let c = (x0 + w, y0 + h / 2);
        for _ in 0..4 {
            if let Some(id) = self.deep_one_near(c, 6, h / 2 + 2) {
                self.salt.risen.push(id);
                if let Some(i) = self.find(id) {
                    self.ents[i].combat.target = pid;
                }
            }
        }
        self.hud_ver += 1;
    }

    /// Saltgrave falls quiet again.
    fn salt_calm(&mut self) {
        self.salt.turned = 0.0;
        self.salt.suspicion = WATCHED[0];
        self.salt.level = 1;
        let risen = std::mem::take(&mut self.salt.risen);
        for i in 1..self.ents.len() {
            let e = &mut self.ents[i];
            if e.dead {
                continue;
            }
            if risen.contains(&e.id) {
                // back into the sea
                e.dead = true;
            } else if matches!(e.k, EK::Mon(m) if salt_folk(MONS[m].id)) {
                e.combat.target = 0;
                e.combat.provoked = false;
                e.path.clear();
            }
        }
        if self.find(self.ents[0].combat.target).is_none() {
            self.ents[0].combat.target = 0;
        }
        let (x0, y0, w, h) = SALT;
        let (cx, cy) = ((x0 + w / 2) as f32, (y0 + h / 2) as f32);
        if self.ents[0].dist(cx, cy) < 45.0 {
            self.msg(
                "Saltgrave falls quiet. Its people go back to their nets as if nothing had happened.",
                C_LORE,
            );
        }
        self.hud_ver += 1;
    }

    /// One of the folk, or the priest, while the town is calm: going about
    /// their business, watching the stranger. False once the town has
    /// turned (they fight like any other creature then).
    pub(super) fn salt_folk_tick(&mut self, i: usize, mi: usize, dt: f32) -> bool {
        if self.salt.turned > 0.0 {
            return false;
        }
        if self.ents[i].combat.provoked {
            // struck: the whole town turns
            self.salt_turn();
            return false;
        }
        let priest = MONS[mi].id == "deeppriest";
        let running = self.ents[i].running;
        self.move_ent(i, if running { 3.4 } else { 1.7 }, dt);
        self.ents[i].ai_t -= dt;
        self.ents[i].combat.target = 0;
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let pd = self.ents[i].dist(px, py);
        let (tx, ty) = self.ents[0].tile();
        let watching = !self.pl.ghost && self.in_salt(tx, ty) && pd < 10.0;
        if !watching {
            self.ents[i].running = false;
            if priest {
                return true;
            }
            self.wander(i);
            return true;
        }
        let stare = |g: &mut Game| {
            let (dx, dy) = (px - g.ents[i].x, py - g.ents[i].y);
            if dx.abs() + dy.abs() > 0.05 {
                g.ents[i].dir8 = dir8_of(dx, dy);
            }
        };
        if self.ents[i].ai_t <= 0.0 {
            self.ents[i].ai_t = 5.0 + self.rng.f() * 6.0;
            if pd < 7.0 && self.rng.chance(if priest { 0.6 } else { 0.35 }) {
                let line = *self
                    .rng
                    .pick(if priest { &SERMON[..] } else { &WHISPERS[..] });
                self.ents[i].talk(line, 0xb8c8b0ff);
            }
        }
        if !self.ents[i].path.is_empty() {
            return true;
        }
        self.ents[i].running = false;
        if priest {
            stare(self);
            return true;
        }
        let level = self.salt.level;
        // too close: they shy away
        if pd < 1.8 {
            if let Some(s) = self.greedy_step(i, px, py, true) {
                self.ents[i].path.push(s);
            }
            return true;
        }
        // some of them trail the stranger down the street; once the bell has
        // tolled they all close in round them
        let shadow = self.ents[i].id % 3 == 0 || level >= 2;
        let keep = if level >= 3 { 2.5 } else { 4.0 };
        if shadow && pd > keep + 0.8 {
            self.ents[i].running = level >= 3 && pd > 6.0;
            if let Some(s) = self.greedy_step(i, px, py, false) {
                self.ents[i].path.push(s);
            }
            return true;
        }
        stare(self);
        true
    }

    /// Something the player said or did to one of the folk while the town
    /// is calm (a tap out of war mode): they mutter and turn away.
    pub(super) fn salt_snub(&mut self, i: usize) {
        let line = *self.rng.pick(&WHISPERS);
        self.ents[i].talk(line, 0xb8c8b0ff);
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let (dx, dy) = (self.ents[i].x - px, self.ents[i].y - py);
        if dx.abs() + dy.abs() > 0.05 {
            self.ents[i].dir8 = dir8_of(dx, dy);
        }
        self.salt.suspicion += 4.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folk(g: &Game) -> Vec<usize> {
        (1..g.ents.len())
            .filter(|&i| {
                !g.ents[i].dead && matches!(g.ents[i].k, EK::Mon(m) if MONS[m].id == "saltfolk")
            })
            .collect()
    }
    fn journal_has(g: &Game, s: &str) -> bool {
        g.journal.iter().any(|(_, t, _)| t.contains(s))
    }
    fn run(g: &mut Game, secs: f32) {
        for _ in 0..(secs * 30.0) as i32 {
            g.update(1.0 / 30.0);
        }
    }
    fn into_town(g: &mut Game) {
        let (x0, y0, _, _) = SALT;
        g.ents[0].x = (x0 + 8) as f32;
        g.ents[0].y = (y0 + 11) as f32;
        g.ents[0].path.clear();
        g.ents[0].hp = 5000;
        g.ents[0].mhp = 5000;
        g.snap_camera();
    }

    #[test]
    fn saltgrave_watches_then_turns() {
        let mut g = Game::new(640);
        g.create("Stranger", false, 0, "warrior", [0, 0, 0]);
        g.time = 0.1 * crate::sim::DAY; // morning
        assert!(folk(&g).len() >= 5);
        into_town(&mut g);
        let hp = g.ents[0].hp;
        // a while in the street by day: watched, not harmed
        run(&mut g, 60.0);
        assert_eq!(g.ents[0].hp, hp, "the folk attacked a stranger by day");
        assert!(journal_has(&g, "eyes on your back"));
        assert_eq!(g.salt.turned, 0.0);
        // they come out and trail you
        run(&mut g, 60.0);
        assert!(journal_has(&g, "none of them speaks"));
        let (px, py) = (g.ents[0].x, g.ents[0].y);
        let near = folk(&g)
            .iter()
            .filter(|&&i| g.ents[i].dist(px, py) < 6.0)
            .count();
        assert!(near >= 2, "only {near} close by");
        // linger, and it turns
        run(&mut g, 80.0);
        assert!(g.salt.turned > 0.0, "suspicion {}", g.salt.suspicion);
        assert!(journal_has(&g, "Saltgrave turns on you"));
        assert!(!g.salt.risen.is_empty());
        let i = folk(&g)[0];
        assert!(g.look_for(i).unwrap().ears, "their faces did not change");
        run(&mut g, 15.0);
        assert!(g.ents[0].hp < hp, "the town turned but nobody struck");
        // leave and stay away: it falls quiet again
        let (sx, sy) = g.map.start;
        g.ents[0].x = sx as f32;
        g.ents[0].y = sy as f32;
        g.ents[0].combat.target = 0;
        g.pl.war = false;
        run(&mut g, 60.0);
        assert_eq!(g.salt.turned, 0.0);
        assert!(g.salt.risen.is_empty());
        let i = folk(&g)[0];
        assert!(!g.look_for(i).unwrap().ears);
    }

    #[test]
    fn striking_one_turns_the_town_and_a_tap_only_snubs() {
        let mut g = Game::new(1997);
        g.create("Brute", false, 0, "warrior", [0, 0, 0]);
        g.time = 0.1 * crate::sim::DAY;
        into_town(&mut g);
        run(&mut g, 1.0);
        let i = *folk(&g)
            .iter()
            .min_by(|&&a, &&b| {
                g.ents[a]
                    .dist(g.ents[0].x, g.ents[0].y)
                    .total_cmp(&g.ents[b].dist(g.ents[0].x, g.ents[0].y))
            })
            .unwrap();
        g.salt_snub(i);
        assert_eq!(g.salt.turned, 0.0);
        g.damage(i, 1, true);
        run(&mut g, 0.2);
        assert!(g.salt.turned > 0.0);
    }
}
