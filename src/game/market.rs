//! The market in Duskhold's square: six stalls round the fountain, each
//! with its keeper beside the counter crying the wares, and a crowd of
//! townsfolk going from stall to stall, looking things over, haggling and
//! buying. The market keeps daylight hours: after dusk the stalls are covered
//! and the crowd drifts about the square.
//!
//! The player can buy from any keeper; each sells only what is on their own
//! stall (`data::STALL_WARES`).
use super::*;

/// facing the stall, from the place before it
const FACE_STALL: u8 = 0;
/// facing the square, from beside the counter
const FACE_FRONT: u8 = 3;

impl Game {
    /// the stalls are open by day
    pub fn market_open(&self) -> bool {
        self.daylight().0 < 0.3
    }

    /// What a trader sells: a stallholder only their own stall's wares.
    pub fn wares(&self, i: usize) -> &'static [&'static str] {
        match self.ents[i].k {
            EK::Npc(V_STALL) => {
                let (hx, hy) = self.ents[i].home;
                STALL_WARES[self.map.stall_kind(hx + 1, hy).unwrap_or(0)]
            }
            EK::Npc(v) => VENDORS[v].sells,
            _ => &[],
        }
    }

    /// someone other than `me` stands on `t` or is on the way there
    fn taken(&self, t: (i32, i32), me: usize) -> bool {
        self.ents
            .iter()
            .enumerate()
            .any(|(j, e)| j != me && !e.dead && (e.tile() == t || e.path.last() == Some(&t)))
    }

    /// A stallholder keeps to the end of the counter, answers the customer
    /// before it and otherwise cries the wares across the square.
    pub(super) fn keeper_tick(&mut self, i: usize, pd: f32) {
        let (hx, hy) = self.ents[i].home;
        if self.ents[i].tile() != (hx, hy) {
            if self.ents[i].path.is_empty() {
                let (sx, sy) = self.ents[i].tile();
                let w = self.walker(i);
                if let Some(p) = self.map.find_path_as(sx, sy, hx, hy, 0, w) {
                    self.ents[i].path = p;
                }
            }
            return;
        }
        self.ents[i].dir8 = FACE_FRONT;
        self.ents[i].ai_t = 2.0 + self.rng.f() * 4.0;
        if !self.market_open() || pd > 12.0 {
            return;
        }
        let kind = self.map.stall_kind(hx + 1, hy).unwrap_or(0);
        let customer = [(hx + 1, hy + 1), (hx + 2, hy + 1)].iter().any(|&t| {
            self.ents
                .iter()
                .any(|e| e.k == EK::Npc(V_FOLK) && e.path.is_empty() && e.tile() == t)
        });
        if customer && self.rng.chance(0.5) {
            let line = *self.rng.pick(&KEEP_REPLY);
            self.ents[i].talk(line, 0xffffffff);
        } else if self.rng.chance(0.3) {
            let line = *self.rng.pick(STALL_CRIES[kind]);
            self.ents[i].talk(line, 0xffe8b0ff);
        }
    }

    /// A shopper at the market: to a free stall, a while looking it over
    /// (and a word with the keeper), then on to the next; now and then a
    /// stroll across the square instead.
    pub(super) fn shopper_tick(&mut self, i: usize, pd: f32) {
        if !self.ents[i].path.is_empty() {
            self.ents[i].ai_t = 0.5;
            return;
        }
        let here = self.ents[i].tile();
        if self.map.stalls.contains(&here) {
            self.ents[i].dir8 = FACE_STALL;
            if pd < 10.0 && self.rng.chance(0.35) {
                let line = *self.rng.pick(&SHOP_TALK);
                self.ents[i].talk(line, 0xffffffff);
            }
            self.ents[i].ai_t = 2.5 + self.rng.f() * 5.0;
            if self.rng.chance(0.5) {
                return;
            }
        }
        let target = if self.rng.chance(0.8) {
            let free: Vec<(i32, i32)> = self
                .map
                .stalls
                .iter()
                .copied()
                .filter(|&s| !self.taken(s, i))
                .collect();
            (!free.is_empty()).then(|| *self.rng.pick(&free))
        } else {
            let (x0, y0, x1, y1) = self.map.market;
            Some((self.rng.range(x0, x1 - 1), self.rng.range(y0, y1 - 1)))
        };
        let Some((tx, ty)) = target else {
            return;
        };
        let w = self.walker(i);
        if !self.map.passable_as(tx, ty, w) || self.taken((tx, ty), i) {
            return;
        }
        if let Some(p) = self.map.find_path_as(here.0, here.1, tx, ty, 0, w) {
            if p.len() < 30 {
                self.ents[i].path = p;
                self.ents[i].ai_t = 0.6;
            }
        }
    }
}
