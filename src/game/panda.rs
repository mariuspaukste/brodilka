//! The giant pandas of the bamboo groves (`world/bamboo.rs`).
//!
//! A panda shambles about its grove and every little while makes for a
//! stand of bamboo, sits down in it and eats: a stalk held to its mouth,
//! chewing, ten seconds or so, now and then munching aloud. Each meal eats
//! a stand further down (its `res`); eaten to the ground, a stand is a few
//! stumps until it grows again some minutes later. A panda goes on to the
//! next stand that still has leaves.
//!
//! Pandas are gentle: struck, one lopes away. Slaying one costs karma and
//! weakens no site of the wild (`data::gentle_wild`).
use super::*;

/// how much of a stand of bamboo one meal eats (a stand starts at 12)
pub const BAMBOO_MEAL: u8 = 4;
/// seconds an eaten stand takes to grow again
pub const BAMBOO_REGROW: f32 = 240.0;
/// how far a panda looks for its next meal, from where it is
const LOOK: i32 = 6;

impl Game {
    pub(super) fn panda_tick(&mut self, i: usize, pd: f32, dt: f32) {
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        // struck: off at a lope, away from whoever did it
        if self.ents[i].combat.provoked {
            let e = &mut self.ents[i];
            e.busy_t = 0.0;
            e.goal = None;
            e.running = true;
            if pd < 10.0 && !self.pl.ghost {
                if self.ents[i].path.is_empty() {
                    if let Some(s) = self.greedy_step(i, px, py, true) {
                        self.ents[i].path.push(s);
                    }
                }
                return;
            }
            self.ents[i].combat.provoked = false;
        }
        self.ents[i].running = false;
        let (x, y) = self.ents[i].tile();
        // at its meal
        if self.ents[i].busy_t > 0.0 {
            self.ents[i].busy_t -= dt;
            if self.ents[i].busy_t <= 0.0 {
                if let Some((gx, gy)) = self.ents[i].goal.take() {
                    self.eat_bamboo(gx, gy);
                }
                self.ents[i].busy_t = 0.0;
                self.ents[i].ai_t = 3.0 + self.rng.f() * 5.0;
            } else if pd < 9.0 && self.rng.chance(0.003) {
                self.ents[i].talk("*munches bamboo*", 0xd8d0c0ff);
            }
            return;
        }
        // making for a stand of bamboo
        if let Some((gx, gy)) = self.ents[i].goal {
            if !self.bamboo_left(gx, gy) {
                self.ents[i].goal = None;
                return;
            }
            if (x - gx).abs().max((y - gy).abs()) <= 1 && self.ents[i].path.is_empty() {
                // sit down to it, facing it
                let (dx, dy) = (gx as f32 - self.ents[i].x, gy as f32 - self.ents[i].y);
                if dx.abs() + dy.abs() > 0.05 {
                    self.ents[i].dir8 = dir8_of(dx, dy);
                }
                self.ents[i].busy_t = 8.0 + self.rng.f() * 6.0;
                return;
            }
            if self.ents[i].path.is_empty() {
                if let Some(s) = self.greedy_step(i, gx as f32, gy as f32, false) {
                    self.ents[i].path.push(s);
                } else if let Some(mut p) = self.map.find_path(x, y, gx, gy, 1) {
                    p.truncate(6);
                    self.ents[i].path = p;
                } else {
                    self.ents[i].goal = None;
                }
            }
            return;
        }
        // hungry again: the nearest stand with leaves on it, not too far
        // from home
        if self.ents[i].ai_t <= 0.0 && self.rng.chance(0.6) {
            self.ents[i].ai_t = 2.0 + self.rng.f() * 3.0;
            let (hx, hy) = self.ents[i].home;
            let mut best: Option<((i32, i32), i32)> = None;
            for gy in y - LOOK..=y + LOOK {
                for gx in x - LOOK..=x + LOOK {
                    if !self.bamboo_left(gx, gy) || (gx - hx).abs().max((gy - hy).abs()) > 10 {
                        continue;
                    }
                    let d = (gx - x).abs().max((gy - y).abs());
                    if best.is_none_or(|b| d < b.1) {
                        best = Some(((gx, gy), d));
                    }
                }
            }
            if let Some((g, _)) = best {
                self.ents[i].goal = Some(g);
                return;
            }
        }
        self.wander(i);
    }

    /// Is there a stand of bamboo with leaves on it at (x, y)?
    pub fn bamboo_left(&self, x: i32, y: i32) -> bool {
        self.map.obj(x, y) == O_BAMBOO && self.map.res[idx(x, y)] > 0
    }

    /// A panda's meal off the stand at (x, y): eaten to the ground, it
    /// grows again in a while.
    fn eat_bamboo(&mut self, x: i32, y: i32) {
        if !self.bamboo_left(x, y) {
            return;
        }
        let i = idx(x, y);
        self.map.res[i] = self.map.res[i].saturating_sub(BAMBOO_MEAL);
        if self.map.res[i] == 0 {
            self.map.regrow.push((i, BAMBOO_REGROW));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pandas_eat_the_bamboo_down_and_it_grows_again() {
        let mut g = Game::new(1997);
        g.create("Watcher", false, 0, "warrior", [0, 0, 0]);
        let pandas: Vec<u32> = g
            .ents
            .iter()
            .filter(|e| matches!(e.k, EK::Mon(m) if MONS[m].id == "panda"))
            .map(|e| e.id)
            .collect();
        assert!(pandas.len() >= 4, "{}", pandas.len());
        let full: usize = (0..N)
            .filter(|&i| g.map.o[i] == O_BAMBOO && g.map.res[i] > 0)
            .count();
        let mut ate = false;
        let mut sat = false;
        for _ in 0..30 * 120 {
            g.update(1.0 / 30.0);
            for &id in &pandas {
                if let Some(i) = g.find(id) {
                    if g.ents[i].busy_t > 0.0 {
                        sat = true;
                        // at its meal, beside a stand with leaves
                        let (x, y) = g.ents[i].tile();
                        let (gx, gy) = g.ents[i].goal.unwrap();
                        assert!((x - gx).abs().max((y - gy).abs()) <= 1);
                    }
                }
            }
            let eaten = (0..N).any(|i| g.map.o[i] == O_BAMBOO && g.map.res[i] < 12);
            if eaten {
                ate = true;
            }
            if ate && sat {
                break;
            }
        }
        assert!(sat, "no panda sat down to eat");
        assert!(ate, "no bamboo was eaten");
        // eaten to the ground, a stand grows again
        let i = (0..N).find(|&i| g.map.o[i] == O_BAMBOO).unwrap();
        let (x, y) = (i as i32 % W, i as i32 / W);
        for _ in 0..3 {
            g.map.res[i] = g.map.res[i].max(1);
            g.eat_bamboo(x, y);
        }
        g.map.res[i] = 1;
        g.eat_bamboo(x, y);
        assert_eq!(g.map.res[i], 0);
        assert!(!g.bamboo_left(x, y));
        g.map.tick(BAMBOO_REGROW + 1.0);
        assert!(g.bamboo_left(x, y));
        let _ = full;
    }

    #[test]
    fn slaying_a_panda_costs_karma_and_weakens_no_site() {
        let mut g = Game::new(42);
        g.create("Brute", false, 0, "warrior", [0, 0, 0]);
        let i = g
            .ents
            .iter()
            .position(|e| matches!(e.k, EK::Mon(m) if MONS[m].id == "panda"))
            .unwrap();
        let k = g.pl.karma;
        let before: Vec<f32> = g.sim.sites.iter().map(|s| s.strength).collect();
        let hp = g.ents[i].hp;
        g.damage(i, hp + 1, true);
        assert!(g.pl.karma < k);
        let after: Vec<f32> = g.sim.sites.iter().map(|s| s.strength).collect();
        assert_eq!(before, after);
    }
}
