//! The beasts of the streets: pigeons, dogs and cats in the town, Duskhold
//! and the village, and in Suryagarh and Kanakpur cows, pigs and monkeys as
//! well.
//!
//! They are `MonDef::domestic`: passive, at home on guarded ground (the
//! guards leave them be) and out of doors, each kind with its own habits:
//!
//! - **pigeons** potter about the squares in flocks, pecking at the ground,
//!   and take wing when anyone comes close, the whole flock with them,
//!   landing again a few yards off;
//! - **dogs** roam, now and then trot along at a passer-by's heels, chase
//!   the cats, and keep their distance from monsters but bark at them;
//! - **cats** sit a great deal, wash, and run from dogs;
//! - **cows** amble and graze, and are not to be hurried;
//! - **pigs** root about and grunt, and squeal and bolt when struck;
//! - **monkeys** go about in troops, sit a great deal, chatter and groom, and
//!   flee from dogs. They are thieves: one will dart up to a player carrying
//!   bread or fish, snatch it and run, and sit down out of reach to eat it.
//!   Struck, it drops what it took; killed, its corpse still holds it. One
//!   of a troop robs at a time, and a monkey that has eaten is sated a while.
//!
//! Struck, any of them flees. Slaying a dog, a cat, a cow or a monkey costs
//! karma, and none of them counts against a site of the wild in the world
//! simulation.
use super::*;

/// How high a pigeon flies, in levels of the terraced ground.
pub const FLIGHT: f32 = 1.3;
/// The colour of the beasts' noises over their heads.
const C_BEAST: u32 = 0xd8d0c0ff;

impl Game {
    pub(super) fn critter_tick(&mut self, i: usize, mi: usize, dt: f32) {
        let d = &MONS[mi];
        let flying = d.flies && self.ents[i].running;
        let speed = if flying {
            5.5
        } else if self.ents[i].running {
            d.mv * 2.0
        } else {
            d.mv
        };
        self.move_ent(i, speed, dt);
        {
            let e = &mut self.ents[i];
            e.ai_t -= dt;
            e.busy_t -= dt;
            if d.flies {
                // up while there is somewhere to fly to, then down again
                let up = flying && !e.path.is_empty();
                e.anim.air = if up {
                    (e.anim.air + dt * 3.0).min(1.0)
                } else {
                    (e.anim.air - dt * 2.2).max(0.0)
                };
                if !up && e.anim.air <= 0.0 {
                    e.running = false;
                }
            }
        }
        if self.ents[i].down_t > 0.0 {
            self.ents[i].path.truncate(1);
            return;
        }
        if d.flies && self.ents[i].anim.air > 0.0 {
            return;
        }
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let pd = self.ents[i].dist(px, py);
        let ghost = self.pl.ghost;
        // struck: off, away from whoever did it
        if self.ents[i].combat.provoked {
            // a thief struck lets go of what it took
            if let Some(it) = self.ents[i].stolen.take() {
                self.msg(
                    &format!(
                        "{} drops your {}, and you snatch it back.",
                        cap(&self.ents[i].name),
                        it.def().name
                    ),
                    C_SYS,
                );
                self.pl.add(it);
                self.touch_ui();
            }
            self.ents[i].chase = 0;
            if pd < 10.0 && !ghost {
                if d.flies {
                    self.take_wing(i, px, py);
                    return;
                }
                if self.ents[i].path.is_empty() {
                    if matches!(d.id, "pig" | "monkey") && self.ents[i].ai_t <= 0.0 {
                        self.ents[i].ai_t = 2.0;
                        let say = if d.id == "pig" {
                            "*squeals*"
                        } else {
                            "*screeches*"
                        };
                        self.ents[i].talk(say, C_BEAST);
                    }
                    // a cow won't be hurried
                    self.ents[i].running = d.id != "cow";
                    if let Some(s) = self.street_step(i, px, py, true) {
                        self.ents[i].path.push(s);
                    }
                }
                return;
            }
            self.ents[i].combat.provoked = false;
        }
        if !self.ents[i].path.is_empty() {
            return;
        }
        match d.id {
            "pigeon" => self.pigeon_tick(i, pd),
            "dog" => self.dog_tick(i, pd),
            "cat" => self.cat_tick(i, pd),
            "monkey" => self.monkey_tick(i, pd),
            _ => {
                self.ents[i].running = false;
                // cows amble, pigs bustle
                let (pause, go) = if d.id == "cow" {
                    (4.0, 0.45)
                } else {
                    (1.8, 0.6)
                };
                if self.potter(i, pause, go, 2) && pd < 9.0 && self.rng.chance(0.12) {
                    let say = if d.id == "cow" {
                        "*moo*"
                    } else {
                        *self.rng.pick(&["*oink*", "*grunts*", "*snuffles*"])
                    };
                    self.ents[i].talk(say, C_BEAST);
                }
            }
        }
    }

    /// Take a step about home now and then: after a pause of up to `pause`
    /// seconds, with chance `go`, within the spawner's reach plus `extra`,
    /// and only out of doors on guarded ground. Out of reach of home, head back. True when it
    /// has just made up its mind (a moment for a noise).
    fn potter(&mut self, i: usize, pause: f32, go: f32, extra: i32) -> bool {
        if self.ents[i].ai_t > 0.0 {
            return false;
        }
        self.ents[i].ai_t = pause * (0.4 + self.rng.f());
        let (hx, hy) = self.ents[i].home;
        let r = self
            .spawners
            .get(self.ents[i].spawner as usize)
            .map(|s| s.r)
            .unwrap_or(4)
            + extra;
        let (x, y) = self.ents[i].tile();
        if (x - hx).abs() > r || (y - hy).abs() > r || !self.map.guarded(x, y) {
            // never in through a door that someone has left open
            let step = self
                .street_step(i, hx as f32, hy as f32, false)
                .or_else(|| self.greedy_step(i, hx as f32, hy as f32, false))
                .filter(|&(sx, sy)| self.map.outdoors(sx, sy) || !self.map.outdoors(x, y));
            if let Some(s) = step {
                self.ents[i].path.push(s);
            }
            return true;
        }
        if !self.rng.chance(go) {
            return true;
        }
        let (dx, dy) = *self.rng.pick(&DIRS);
        let (nx, ny) = (x + dx, y + dy);
        if (nx - hx).abs() <= r
            && (ny - hy).abs() <= r
            && self.map.outdoors(nx, ny)
            && self.map.guarded(nx, ny)
            && self.map.can_step(x, y, dx, dy)
            && !self.occupied(nx, ny, i)
            && (nx, ny) != self.ents[0].tile()
        {
            self.ents[i].path.push((nx, ny));
        }
        true
    }

    /// A step towards (or `away` from) a point that keeps to open, guarded
    /// ground: the best of the eight that brings it nearer (or further).
    fn street_step(&self, i: usize, gx: f32, gy: f32, away: bool) -> Option<(i32, i32)> {
        let (x, y) = self.ents[i].tile();
        let cur = ((x as f32 - gx).powi(2) + (y as f32 - gy).powi(2)).sqrt();
        let mut best: Option<((i32, i32), f32)> = None;
        for &(dx, dy) in DIRS.iter() {
            let (nx, ny) = (x + dx, y + dy);
            if !self.map.outdoors(nx, ny)
                || !self.map.guarded(nx, ny)
                || !self.map.can_step(x, y, dx, dy)
                || self.occupied(nx, ny, i)
            {
                continue;
            }
            let d = ((nx as f32 - gx).powi(2) + (ny as f32 - gy).powi(2)).sqrt();
            let score = if away { -d } else { d };
            if best.map_or(true, |b| score < b.1) {
                best = Some(((nx, ny), score));
            }
        }
        match best {
            Some((p, s)) if (away && -s > cur) || (!away && s < cur) => Some(p),
            _ => None,
        }
    }

    /// The nearest living beast of kind `id` (or, with "", any monster that
    /// is no beast of the streets) within `r`, with its distance.
    fn nearest_mon(&self, i: usize, id: &str, r: f32) -> Option<(usize, f32)> {
        let (x, y) = (self.ents[i].x, self.ents[i].y);
        let mut best: Option<(usize, f32)> = None;
        for (j, e) in self.ents.iter().enumerate() {
            if j == i || e.dead || e.fl != 0 {
                continue;
            }
            let EK::Mon(m) = e.k else { continue };
            let md = &MONS[m];
            let hit = if id.is_empty() {
                !md.domestic && !md.passive
            } else {
                md.id == id
            };
            if !hit || (e.x - x).abs() > r || (e.y - y).abs() > r {
                continue;
            }
            let dd = e.dist(x, y);
            if dd < r && best.map_or(true, |b| dd < b.1) {
                best = Some((j, dd));
            }
        }
        best
    }

    // ------------------------------------------------------------ pigeons
    fn pigeon_tick(&mut self, i: usize, pd: f32) {
        // a pigeon set down on a roof or a wall flies back to the street
        let (x, y) = self.ents[i].tile();
        if !self.map.outdoors(x, y) {
            let (hx, hy) = self.ents[i].home;
            let (fx, fy) = (2.0 * x as f32 - hx as f32, 2.0 * y as f32 - hy as f32);
            self.take_wing(i, fx, fy);
            return;
        }
        // startled by anyone close: the player coming on, or standing right
        // by; anyone walking straight through the flock
        let p = &self.ents[0];
        let near_p = !self.pl.ghost
            && (pd < 1.3
                || (pd < 2.4 && (!p.path.is_empty() || self.pl.war || self.pl.riding))
                || (pd < 3.5 && self.pl.riding && !p.path.is_empty()));
        let threat = if near_p {
            Some((p.x, p.y))
        } else {
            let (ex, ey) = (self.ents[i].x, self.ents[i].y);
            self.ents
                .iter()
                .enumerate()
                .find(|&(j, e)| {
                    j != i
                        && j != 0
                        && !e.dead
                        && !e.path.is_empty()
                        && e.dist(ex, ey) < 1.4
                        && !matches!(e.k, EK::Mon(m) if MONS[m].id == "pigeon")
                })
                .map(|(_, e)| (e.x, e.y))
        };
        if let Some((tx, ty)) = threat {
            // the whole flock goes up together
            let (ex, ey) = (self.ents[i].x, self.ents[i].y);
            let flock: Vec<usize> = (1..self.ents.len())
                .filter(|&j| {
                    let e = &self.ents[j];
                    !e.dead
                        && e.dist(ex, ey) < 3.5
                        && e.anim.air <= 0.0
                        && !e.running
                        && matches!(e.k, EK::Mon(m) if MONS[m].id == "pigeon")
                })
                .collect();
            for j in flock {
                self.take_wing(j, tx, ty);
            }
            return;
        }
        // potter and peck about, cooing now and then
        if self.potter(i, 1.4, 0.65, 2) && pd < 7.0 && self.rng.chance(0.06) {
            self.ents[i].talk("*coo*", C_BEAST);
        }
    }

    /// Fly up and away from (fx, fy) to open ground a few tiles off, near
    /// home. With nowhere to land, it hops away on foot.
    fn take_wing(&mut self, i: usize, fx: f32, fy: f32) {
        let (x, y) = self.ents[i].tile();
        let (hx, hy) = self.ents[i].home;
        let r = self
            .spawners
            .get(self.ents[i].spawner as usize)
            .map(|s| s.r)
            .unwrap_or(4)
            + 4;
        let (ax, ay) = (x as f32 - fx, y as f32 - fy);
        let away = if ax.abs() + ay.abs() < 0.01 {
            self.rng.f() * std::f32::consts::TAU
        } else {
            ay.atan2(ax)
        };
        for _ in 0..14 {
            let ang = away + (self.rng.f() - 0.5) * 2.2;
            let dist = 3.5 + self.rng.f() * 4.0;
            let (lx, ly) = (
                (x as f32 + ang.cos() * dist).round() as i32,
                (y as f32 + ang.sin() * dist).round() as i32,
            );
            if (lx - hx).abs() > r
                || (ly - hy).abs() > r
                || !self.map.outdoors(lx, ly)
                || !self.map.guarded(lx, ly)
                || !self.map.reach[idx(lx, ly)]
                || self.occupied(lx, ly, i)
            {
                continue;
            }
            // straight there, over whatever lies between, but no shut door
            let mut path = vec![];
            let (mut cx, mut cy) = (x, y);
            while (cx, cy) != (lx, ly) {
                cx += (lx - cx).signum();
                cy += (ly - cy).signum();
                path.push((cx, cy));
            }
            if path.iter().any(|&(px, py)| self.map.door_shut(px, py)) {
                continue;
            }
            let e = &mut self.ents[i];
            e.path = path;
            e.running = true;
            e.combat.provoked = false;
            e.ai_t = 0.5 + self.rng.f();
            return;
        }
        if let Some(s) = self.street_step(i, fx, fy, true) {
            self.ents[i].path.push(s);
        }
    }

    // ------------------------------------------------------------ dogs
    fn dog_tick(&mut self, i: usize, pd: f32) {
        // a monster about: keep off and bark at it
        if let Some((m, md)) = self.nearest_mon(i, "", 7.0) {
            let (mx, my) = (self.ents[m].x, self.ents[m].y);
            self.ents[i].chase = 0;
            self.ents[i].running = md < 3.5;
            if md < 3.5 {
                if let Some(s) = self.street_step(i, mx, my, true) {
                    self.ents[i].path.push(s);
                }
            } else {
                let (dx, dy) = (mx - self.ents[i].x, my - self.ents[i].y);
                self.ents[i].dir8 = dir8_of(dx, dy);
            }
            if self.ents[i].ai_t <= 0.0 {
                self.ents[i].ai_t = 1.5 + self.rng.f() * 2.0;
                let say = *self.rng.pick(&["*barks*", "*growls*", "*barks furiously*"]);
                self.ents[i].talk(say, C_BEAST);
            }
            return;
        }
        // after a cat or a monkey, or tagging along behind someone
        if self.ents[i].chase != 0 && self.ents[i].busy_t > 0.0 {
            if let Some(t) = self.find(self.ents[i].chase) {
                let (tx, ty) = (self.ents[t].x, self.ents[t].y);
                let td = self.ents[i].dist(tx, ty);
                let cat = t != 0;
                if (cat && td > 1.2) || (!cat && td > 2.2 && td < 14.0) {
                    self.ents[i].running = cat || td > 5.0;
                    if let Some(s) = self.street_step(i, tx, ty, false) {
                        self.ents[i].path.push(s);
                    }
                    if cat && self.rng.chance(0.03) {
                        self.ents[i].talk("*barks*", C_BEAST);
                    }
                    return;
                }
                if !cat && td <= 2.2 {
                    self.ents[i].running = false;
                    let (dx, dy) = (tx - self.ents[i].x, ty - self.ents[i].y);
                    if dx.abs() + dy.abs() > 0.05 {
                        self.ents[i].dir8 = dir8_of(dx, dy);
                    }
                    return;
                }
            }
        }
        self.ents[i].chase = 0;
        self.ents[i].running = false;
        if self.ents[i].ai_t <= 0.0 {
            // spot a cat or a monkey and go for it, or take up with the
            // player a while
            let quarry = self
                .nearest_mon(i, "cat", 5.0)
                .or_else(|| self.nearest_mon(i, "monkey", 5.0));
            if let Some((c, _)) = quarry {
                if self.rng.chance(0.3) {
                    let id = self.ents[c].id;
                    let e = &mut self.ents[i];
                    e.chase = id;
                    e.busy_t = 4.0 + self.rng.f() * 3.0;
                    e.talk("*barks*", C_BEAST);
                    return;
                }
            }
            if pd < 5.0 && !self.pl.ghost && self.rng.chance(0.15) {
                let id = self.ents[0].id;
                let e = &mut self.ents[i];
                e.chase = id;
                e.busy_t = 8.0 + self.rng.f() * 10.0;
                e.talk("*wags its tail*", C_BEAST);
                return;
            }
            if pd < 8.0 && self.rng.chance(0.05) {
                let say = *self
                    .rng
                    .pick(&["*woof*", "*barks*", "*sniffs*", "*scratches*"]);
                self.ents[i].talk(say, C_BEAST);
            }
        }
        self.potter(i, 2.5, 0.6, 3);
    }

    // ------------------------------------------------------------ cats
    fn cat_tick(&mut self, i: usize, pd: f32) {
        // a dog close: off and away
        if let Some((g, gd)) = self.nearest_mon(i, "dog", 2.8) {
            let (gx, gy) = (self.ents[g].x, self.ents[g].y);
            self.ents[i].running = true;
            if let Some(s) = self.street_step(i, gx, gy, true) {
                self.ents[i].path.push(s);
            }
            if gd < 1.6 && self.ents[i].ai_t <= 0.0 {
                self.ents[i].ai_t = 5.0;
                self.ents[i].talk("*hisses*", C_BEAST);
            }
            return;
        }
        self.ents[i].running = false;
        if self.ents[i].ai_t <= 0.0 && pd < 3.0 && !self.pl.ghost && self.rng.chance(0.15) {
            let say = *self.rng.pick(&["*meow*", "*purrs*", "*mrrp*"]);
            self.ents[i].talk(say, C_BEAST);
        }
        self.potter(i, 5.0, 0.4, 2);
    }

    // ------------------------------------------------------------ monkeys
    /// Can a monkey rob the player now, and of what?
    fn monkey_bait(&self) -> Option<usize> {
        if self.pl.ghost || self.pl.war || self.pl.riding || self.ents[0].fl != 0 {
            return None;
        }
        MONKEY_FOOD
            .iter()
            .map(|id| item_idx(id))
            .find(|&id| self.pl.count(id) > 0)
    }

    fn monkey_tick(&mut self, i: usize, pd: f32) {
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let pid = self.ents[0].id;
        // a dog close: off and away, screeching
        if let Some((g, gd)) = self.nearest_mon(i, "dog", 2.8) {
            let (gx, gy) = (self.ents[g].x, self.ents[g].y);
            self.ents[i].running = true;
            self.ents[i].chase = 0;
            if let Some(s) = self.street_step(i, gx, gy, true) {
                self.ents[i].path.push(s);
            }
            if gd < 1.8 && self.ents[i].ai_t <= 0.0 {
                self.ents[i].ai_t = 4.0;
                self.ents[i].talk("*screeches*", C_BEAST);
            }
            return;
        }
        // with something snatched: off out of reach, then sit and eat it
        if self.ents[i].stolen.is_some() {
            if pd < 6.0 && !self.pl.ghost {
                self.ents[i].running = true;
                if let Some(s) = self.street_step(i, px, py, true) {
                    self.ents[i].path.push(s);
                } else if self.ents[i].ai_t <= 0.0 {
                    // cornered
                    self.ents[i].ai_t = 2.5;
                    self.ents[i].talk("*bares its teeth*", C_BEAST);
                }
                return;
            }
            self.ents[i].running = false;
            if self.ents[i].busy_t <= 0.0 {
                self.ents[i].stolen = None;
                // sated a good while
                self.ents[i].busy_t = 60.0 + self.rng.f() * 60.0;
                if pd < 12.0 {
                    self.ents[i].talk("*smacks its lips*", C_BEAST);
                }
            } else if pd < 10.0 && self.rng.chance(0.004) {
                self.ents[i].talk("*munches*", C_BEAST);
            }
            return;
        }
        // after the player's food: close in, snatch it, and away
        if self.ents[i].chase == pid {
            let bait = self.monkey_bait();
            if bait.is_none() || self.ents[i].busy_t <= 0.0 || pd > 8.0 {
                self.ents[i].chase = 0;
                self.ents[i].running = false;
                self.ents[i].busy_t = 15.0;
                return;
            }
            if pd < 1.5 {
                let id = bait.unwrap();
                self.pl.take(id, 1);
                let it = Item::new(id, 1);
                self.msg(
                    &format!(
                        "{} snatches your {} and is off with it!",
                        cap(&self.ents[i].name),
                        it.def().name
                    ),
                    C_WARN,
                );
                self.touch_ui();
                let e = &mut self.ents[i];
                e.stolen = Some(it);
                e.chase = 0;
                // how long before it has eaten it
                e.busy_t = 20.0 + self.rng.f() * 10.0;
                e.running = true;
                e.talk("*shrieks*", C_BEAST);
                return;
            }
            self.ents[i].running = pd > 2.5;
            if let Some(s) = self.street_step(i, px, py, false) {
                self.ents[i].path.push(s);
            }
            return;
        }
        self.ents[i].chase = 0;
        self.ents[i].running = false;
        // one thief at a time: none of the troop is after the player or
        // still eating what it took
        if self.ents[i].ai_t <= 0.0
            && self.ents[i].busy_t <= 0.0
            && pd < 5.0
            && self.monkey_bait().is_some()
            && self.rng.chance(0.2)
        {
            let busy = self.ents.iter().enumerate().any(|(j, e)| {
                j != i
                    && !e.dead
                    && matches!(e.k, EK::Mon(m) if MONS[m].id == "monkey")
                    && (e.chase == pid || e.stolen.is_some())
            });
            if !busy {
                let e = &mut self.ents[i];
                e.chase = pid;
                e.busy_t = 7.0;
                e.ai_t = 0.5;
                return;
            }
        }
        if self.potter(i, 3.0, 0.5, 2) && pd < 7.0 && self.rng.chance(0.1) {
            let say = *self
                .rng
                .pick(&["*chatters*", "*hoots*", "*scratches*", "*grooms itself*"]);
            self.ents[i].talk(say, C_BEAST);
        }
    }
}

/// What a monkey will snatch from the player, in order of preference.
const MONKEY_FOOD: [&str; 2] = ["bread", "fish"];

#[cfg(test)]
mod tests {
    use super::*;

    fn of(g: &Game, id: &str) -> Vec<usize> {
        (1..g.ents.len())
            .filter(|&i| !g.ents[i].dead && matches!(g.ents[i].k, EK::Mon(m) if MONS[m].id == id))
            .collect()
    }

    #[test]
    fn the_streets_have_their_beasts() {
        for seed in [1u32, 1997] {
            let mut g = Game::new(seed);
            g.create("Walker", false, 0, "warrior", [0, 0, 0]);
            for id in ["pigeon", "dog", "cat", "cow", "pig", "monkey"] {
                let all = of(&g, id);
                assert!(!all.is_empty(), "seed {seed}: no {id}");
                for &i in &all {
                    let (x, y) = g.ents[i].tile();
                    assert!(
                        g.map.guarded(x, y) && g.map.outdoors(x, y),
                        "{id} at {x},{y}"
                    );
                    assert!(street_look_ok(&g, i));
                }
            }
            // cows, pigs and monkeys only in the cities of the south;
            // pigeons, dogs and cats in the north's town and city too
            for id in ["cow", "pig", "monkey"] {
                assert!(of(&g, id).iter().all(|&i| {
                    let (x, y) = g.ents[i].tile();
                    g.map.in_raj(x, y)
                }));
            }
            for id in ["pigeon", "dog", "cat"] {
                assert!(of(&g, id).iter().any(|&i| {
                    let (x, y) = g.ents[i].tile();
                    g.map.in_town(x, y)
                }));
                assert!(of(&g, id).iter().any(|&i| {
                    let (x, y) = g.ents[i].tile();
                    g.map.in_raj(x, y)
                }));
            }
            // a few minutes of street life: the guards leave them be, they
            // keep out of doors and near home
            let before: usize = ["pigeon", "dog", "cat", "cow", "pig", "monkey"]
                .iter()
                .map(|id| of(&g, id).len())
                .sum();
            for _ in 0..30 * 120 {
                g.update(1.0 / 30.0);
            }
            let after: usize = ["pigeon", "dog", "cat", "cow", "pig", "monkey"]
                .iter()
                .map(|id| of(&g, id).len())
                .sum();
            assert_eq!(before, after, "seed {seed}: street beasts died");
            for id in ["cow", "pig", "cat", "monkey"] {
                for i in of(&g, id) {
                    let (x, y) = g.ents[i].tile();
                    assert!(g.map.outdoors(x, y), "seed {seed}: {id} indoors at {x},{y}");
                    assert!(g.map.guarded(x, y), "seed {seed}: {id} strayed to {x},{y}");
                }
            }
        }
    }

    fn street_look_ok(g: &Game, i: usize) -> bool {
        g.look_for(i).is_some()
    }

    #[test]
    fn pigeons_take_wing_together_and_land_again() {
        let mut g = Game::new(1997);
        g.create("Walker", false, 0, "warrior", [0, 0, 0]);
        // let the flocks settle, then walk into one
        for _ in 0..60 {
            g.update(1.0 / 30.0);
        }
        let flock = of(&g, "pigeon");
        let p = flock
            .iter()
            .copied()
            .find(|&i| {
                let (x, y) = g.ents[i].tile();
                g.map.in_town(x, y) && g.ents[i].anim.air == 0.0
            })
            .expect("no pigeon in town");
        let (x, y) = (g.ents[p].x, g.ents[p].y);
        let near: Vec<u32> = flock
            .iter()
            .filter(|&&j| g.ents[j].dist(x, y) < 2.0)
            .map(|&j| g.ents[j].id)
            .collect();
        {
            let pl = &mut g.ents[0];
            pl.x = x + 1.0;
            pl.y = y;
            pl.path = vec![(x as i32, y as i32)];
        }
        let mut flew = 0;
        for _ in 0..20 {
            g.update(1.0 / 30.0);
        }
        for id in &near {
            if let Some(j) = g.find(*id) {
                if g.ents[j].anim.air > 0.0 {
                    flew += 1;
                }
            }
        }
        assert_eq!(flew, near.len(), "the flock did not go up together");
        // the player stands back; they come down again on open ground
        // (though a passer-by may put one up again)
        g.ents[0].path.clear();
        g.ents[0].x = x + 30.0;
        let mut landed = vec![false; near.len()];
        for _ in 0..30 * 6 {
            g.update(1.0 / 30.0);
            for (k, id) in near.iter().enumerate() {
                let j = g.find(*id).unwrap();
                let (lx, ly) = g.ents[j].tile();
                if g.ents[j].anim.air == 0.0 && g.map.outdoors(lx, ly) {
                    landed[k] = true;
                }
            }
        }
        assert!(landed.iter().all(|&l| l), "a pigeon never came down");
    }

    #[test]
    fn a_monkey_snatches_bread_and_drops_it_when_struck() {
        let mut g = Game::new(1997);
        g.create("Walker", false, 0, "warrior", [0, 0, 0]);
        g.pl.pack.retain(|it| !MONKEY_FOOD.contains(&it.def().id));
        let bread = item_idx("bread");
        g.pl.add(Item::new(bread, 3));
        // stand still in the street beside a monkey of the south
        let m = of(&g, "monkey")
            .into_iter()
            .find(|&i| {
                let (x, y) = g.ents[i].tile();
                g.map.in_raj(x, y)
            })
            .unwrap();
        let (mx, my) = g.ents[m].tile();
        let spot = DIRS
            .iter()
            .map(|&(dx, dy)| (mx + 2 * dx, my + 2 * dy))
            .find(|&(x, y)| g.map.passable(x, y) && g.map.outdoors(x, y) && g.map.guarded(x, y))
            .unwrap();
        {
            let p = &mut g.ents[0];
            p.x = spot.0 as f32;
            p.y = spot.1 as f32;
            p.path.clear();
        }
        let mut stolen = false;
        for _ in 0..30 * 120 {
            g.ents[0].path.clear();
            g.update(1.0 / 30.0);
            if g.pl.count(bread) < 3 {
                stolen = true;
                break;
            }
        }
        assert!(stolen, "no monkey took the bread");
        assert_eq!(g.pl.count(bread), 2);
        let thief = (1..g.ents.len())
            .find(|&i| g.ents[i].stolen.is_some())
            .expect("no monkey holds the bread");
        assert_eq!(g.ents[thief].stolen.as_ref().unwrap().id, bread);
        // one thief at a time
        assert_eq!(
            (1..g.ents.len())
                .filter(|&i| g.ents[i].stolen.is_some())
                .count(),
            1
        );
        // it makes off out of reach
        for _ in 0..30 * 3 {
            g.ents[0].path.clear();
            g.update(1.0 / 30.0);
        }
        let tid = g.ents[thief].id;
        let t = g.find(tid).unwrap();
        assert!(g.ents[t].dist(g.ents[0].x, g.ents[0].y) > 2.5);
        // struck, it lets go
        g.damage(t, 1, true);
        g.update(1.0 / 30.0);
        assert_eq!(g.pl.count(bread), 3);
        assert!(g.ents[g.find(tid).unwrap()].stolen.is_none());
    }

    #[test]
    fn a_monkey_left_alone_eats_what_it_took() {
        let mut g = Game::new(1);
        g.create("Walker", false, 0, "warrior", [0, 0, 0]);
        let m = of(&g, "monkey")[0];
        let id = g.ents[m].id;
        g.ents[m].stolen = Some(Item::new(item_idx("fish"), 1));
        g.ents[m].busy_t = 5.0;
        for _ in 0..30 * 10 {
            g.update(1.0 / 30.0);
        }
        let m = g.find(id).unwrap();
        assert!(g.ents[m].stolen.is_none());
        // and is sated a while
        assert!(g.ents[m].busy_t > 30.0);
    }

    #[test]
    fn slaying_a_cow_costs_karma() {
        let mut g = Game::new(1997);
        g.create("Butcher", false, 0, "warrior", [0, 0, 0]);
        let k0 = g.pl.karma;
        let c = of(&g, "cow")[0];
        let id = g.ents[c].id;
        let hp = g.ents[c].hp;
        g.damage(c, hp + 1, true);
        assert!(g.find(id).is_none());
        assert!(g.pl.karma < k0);
        // a pig is only pork
        let k1 = g.pl.karma;
        let p = of(&g, "pig")[0];
        let hp = g.ents[p].hp;
        g.damage(p, hp + 1, true);
        assert_eq!(g.pl.karma, k1);
        // a monkey is Hanuman's own, and its corpse keeps what it stole
        let k2 = g.pl.karma;
        let m = of(&g, "monkey")[0];
        g.ents[m].stolen = Some(Item::new(item_idx("bread"), 1));
        let hp = g.ents[m].hp;
        g.damage(m, hp + 1, true);
        assert!(g.pl.karma < k2);
        let c = g.corpses.last().unwrap();
        assert!(c.items.iter().any(|it| it.id == item_idx("bread")));
    }
}
