//! The Shadow: a darkness on the land around the places a dark power holds.
//!
//! A region with `shadow = true` casts it from each of its sites on the map.
//! How far it reaches follows the region's strength and the site's own, so it
//! creeps outward while the power grows and draws back as its places are
//! cleared. Guarded ground holds it off: the town, the city and their walls
//! keep it out entirely, road posts and waystones thin it.
//!
//! Inside it the ground darkens, trees wither (and green again when it
//! lifts), the creatures of the dark heal their wounds and the player's heal
//! slowly. The field is worked out from the simulation's numbers every few
//! seconds, so nothing about it needs saving.
use super::*;
use crate::sim::{Sim, SITE_NAMES};

/// seconds between recomputing the field
const EVERY: f32 = 3.0;
/// tiles over which the Shadow fades out at its edge
const EDGE: f32 = 6.0;
/// how deep the Shadow must be to wither trees, heal the dark and weary the
/// player
pub const DEEP: u8 = 3;

impl Game {
    /// Where on the map a site lies.
    fn site_pos(&self, k: usize) -> Option<(i32, i32)> {
        self.map
            .pois
            .iter()
            .find(|(n, _, _)| *n == SITE_NAMES[k])
            .map(|&(_, x, y)| (x, y))
    }

    /// The Shadow over every tile as the world stands now.
    pub fn shadow_field(&self) -> Vec<u8> {
        let mut src = vec![];
        for (k, s) in self.sim.sites.iter().enumerate() {
            let Some(r) = s.region else { continue };
            let reg = &self.sim.regions[r];
            if !reg.shadow {
                continue;
            }
            if let Some(p) = self.site_pos(k) {
                let reach = (reg.strength - 25.0).max(0.0) * 0.35 + s.strength * 0.15;
                src.push((p, reach));
            }
        }
        let mut out = vec![0u8; N];
        if src.is_empty() {
            return out;
        }
        // distance outside a rectangle (0 inside)
        let outside = |x: i32, y: i32, (x0, y0, x1, y1): (i32, i32, i32, i32)| {
            let dx = (x0 - x).max(x - (x1 - 1)).max(0);
            let dy = (y0 - y).max(y - (y1 - 1)).max(0);
            dx.max(dy) as f32
        };
        let wards: Vec<((i32, i32), f32)> = self
            .map
            .posts
            .iter()
            .map(|&p| (p, 5.0))
            .chain(self.map.ankhs.iter().map(|&p| (p, 4.0)))
            .collect();
        for y in 0..LAND_H {
            for x in 0..W {
                let mut v: f32 = 0.0;
                for &((sx, sy), reach) in &src {
                    let d = (((x - sx).pow(2) + (y - sy).pow(2)) as f32).sqrt();
                    v = v.max(((reach - d) / EDGE).clamp(0.0, 1.0));
                }
                if v <= 0.0 {
                    continue;
                }
                // the walls and the hedge hold it off
                let walls = [
                    self.map.town,
                    self.map.city,
                    self.map.stable,
                    self.map.village,
                    self.map.hold,
                ];
                for &rect in walls.iter().chain(self.map.elves.iter()) {
                    v *= ((outside(x, y, rect) - 3.0) / EDGE).clamp(0.0, 1.0);
                }
                for &((wx, wy), r) in &wards {
                    let d = (((x - wx).pow(2) + (y - wy).pow(2)) as f32).sqrt();
                    v *= (0.25 + (d - r) / EDGE).clamp(0.25, 1.0);
                }
                out[idx(x, y)] = (v * GLOOM_MAX as f32).round() as u8;
            }
        }
        out
    }

    /// Bring the Shadow up to date: the land under it, the trees, the radar.
    pub(super) fn shadow_tick(&mut self, dt: f32) {
        self.shadow_t -= dt;
        if self.shadow_t > 0.0 {
            self.ease_gloom(dt);
            return;
        }
        self.shadow_t = EVERY;
        let field = self.shadow_field();
        let mut changed = vec![];
        for (i, &g) in field.iter().enumerate() {
            if self.map.gloom[i] == g {
                continue;
            }
            self.map.gloom[i] = g;
            let (x, y) = (i as i32 % W, i as i32 / W);
            changed.push((x, y));
            if self.map.o[i] == O_TREE {
                let style = self.map.style[i];
                if g > DEEP && style & 0x7f == 0 {
                    self.map.style[i] = (style & 0x80) | 1;
                    self.map.withered[i] = true;
                } else if g < DEEP && self.map.withered[i] {
                    self.map.style[i] = style & 0x80;
                    self.map.withered[i] = false;
                }
            }
            let c = mini_color(&self.map, i);
            self.mini[i * 4..i * 4 + 4].copy_from_slice(&c);
        }
        if !changed.is_empty() {
            self.mini_ver += 1;
            self.ground.stale_tiles(&changed);
        }
        // news when the darkened share of the land crosses a tenth, with a
        // margin so an edge that wavers does not repeat it
        let pct = self.shadow_share() * 100.0;
        let (up, down) = (
            ((pct - 2.0) / 10.0).floor().max(0.0) as i32,
            ((pct + 2.0) / 10.0).floor() as i32,
        );
        if self.shadow_band < 0 {
            self.shadow_band = (pct / 10.0).floor() as i32;
        } else if up > self.shadow_band || down < self.shadow_band {
            let rising = up > self.shadow_band;
            self.shadow_band = if rising { up } else { down };
            let line = if rising {
                "The Shadow lengthens across the land. The trees wither where it falls."
            } else {
                "The Shadow draws back, and green comes again to the woods."
            };
            let day = Sim::day(self.time);
            for n in self.sim.news(day, line.to_string()) {
                self.msg(&n, C_LORE);
            }
        }
        self.ease_gloom(dt);
    }

    /// How dark it is about the player, eased so the view darkens gradually;
    /// and a word when they walk into the deep Shadow.
    fn ease_gloom(&mut self, dt: f32) {
        let (x, y) = self.ents[0].tile();
        let here = if inb(x, y) {
            self.map.gloom[idx(x, y)]
        } else {
            0
        };
        let target = here as f32 / GLOOM_MAX as f32;
        self.gloom_view += (target - self.gloom_view) * (dt * 0.8).min(1.0);
        let deep = here >= DEEP;
        if deep && !self.in_shadow && !self.pl.ghost {
            self.msg(
                "The Shadow lies heavy here. Thy wounds close slowly, and the creatures of the dark grow strong.",
                C_WARN,
            );
        }
        if deep != self.in_shadow {
            self.in_shadow = deep;
            self.hud_ver += 1;
        }
    }

    /// The Shadow on a tile.
    pub fn gloom_at(&self, x: i32, y: i32) -> u8 {
        if inb(x, y) {
            self.map.gloom[idx(x, y)]
        } else {
            0
        }
    }

    /// The share of the land under the Shadow, 0..1.
    pub fn shadow_share(&self) -> f32 {
        self.map.gloom.iter().filter(|&&g| g >= 2).count() as f32 / LAND_N as f32
    }
}
