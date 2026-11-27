//! The sky: the light of the hour, the weather, and the gloom under trees.
//!
//! - **Time of day**: a day is `sim::DAY` seconds in six periods. Morning to
//!   afternoon are lit as before; dusk reddens and darkens into night, when
//!   only lamps, fires and magic give light, and dawn brings it back, rosy.
//! - **Weather** changes twice a day and follows the season: showers and
//!   storms in spring and autumn, a few in summer, fog and snow in winter.
//!   It fades in and out, has its own word in the HUD, and is told in the
//!   journal when it turns. Under a roof none of it falls.
//! - **Forest**: under a thick canopy the light dims and grows green, with
//!   sun (or moon) coming through in patches.
//!
//! All of it follows from the game time and the map, so nothing is saved.
use super::*;
use crate::sim::{Sim, DAY};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Weather {
    Clear,
    Rain,
    Storm,
    Fog,
    Snow,
}

impl Weather {
    pub fn word(self) -> &'static str {
        match self {
            Weather::Clear => "",
            Weather::Rain => "rain",
            Weather::Storm => "storm",
            Weather::Fog => "fog",
            Weather::Snow => "snow",
        }
    }
    fn news(self) -> &'static str {
        match self {
            Weather::Clear => "The weather clears.",
            Weather::Rain => "Rain begins to fall.",
            Weather::Storm => "Thunder rolls over the hills: a storm is coming.",
            Weather::Fog => "A fog comes down.",
            Weather::Snow => "Snow begins to fall.",
        }
    }
}

/// seconds each spell of weather lasts: half a day
const SPELL: f32 = DAY / 2.0;

fn mix(slot: i64) -> f32 {
    let mut h = (slot as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0x5eed_c0de;
    h ^= h >> 29;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 32;
    (h % 10_000) as f32 / 10_000.0
}

impl Game {
    /// How dark it is from the hour, 0 (day) to about 0.65 (deep night), and
    /// the colour the light takes.
    pub fn daylight(&self) -> (f32, u32) {
        let f = (self.time / DAY).fract();
        let ease = |a: f32| 0.5 - 0.5 * (a.clamp(0.0, 1.0) * std::f32::consts::PI).cos();
        const NIGHT: f32 = 0.52;
        match f {
            // morning, midday, afternoon
            f if f < 0.5 => (0.0, 0),
            // dusk: the light reddens, then goes
            f if f < 4.0 / 6.0 => {
                let a = (f - 0.5) * 6.0;
                (
                    NIGHT * ease(a),
                    if a < 0.5 { 0xa04018ff } else { 0x101830ff },
                )
            }
            // night
            f if f < 5.0 / 6.0 => (NIGHT, 0x0a1230ff),
            // dawn: rosy, then day
            f => {
                let a = (f - 5.0 / 6.0) * 6.0;
                (
                    NIGHT * (1.0 - ease(a)),
                    if a < 0.5 { 0x182040ff } else { 0xc06048ff },
                )
            }
        }
    }

    fn weather_of(&self, slot: i64) -> Weather {
        let (x, y) = self.ents[0].tile();
        self.weather_at(slot, self.map.tropic(x, y))
    }

    /// The weather in one spell, north or in the tropics. The tropics never
    /// see snow: their winter is dry and hazy, the summer brings the monsoon.
    fn weather_at(&self, slot: i64, tropic: bool) -> Weather {
        let day = (slot as f32 * SPELL / DAY).floor() as i64;
        let h = mix(slot);
        if tropic {
            return match self.sim.season(day) {
                "summer" => match h {
                    h if h < 0.4 => Weather::Rain,
                    h if h < 0.6 => Weather::Storm,
                    _ => Weather::Clear,
                },
                "winter" => match h {
                    h if h < 0.2 => Weather::Fog,
                    _ => Weather::Clear,
                },
                "autumn" => match h {
                    h if h < 0.25 => Weather::Rain,
                    h if h < 0.32 => Weather::Storm,
                    _ => Weather::Clear,
                },
                _ => match h {
                    h if h < 0.08 => Weather::Storm,
                    h if h < 0.14 => Weather::Rain,
                    _ => Weather::Clear,
                },
            };
        }
        match self.sim.season(day) {
            "winter" => match h {
                h if h < 0.35 => Weather::Snow,
                h if h < 0.5 => Weather::Fog,
                _ => Weather::Clear,
            },
            "summer" => match h {
                h if h < 0.15 => Weather::Rain,
                h if h < 0.23 => Weather::Storm,
                _ => Weather::Clear,
            },
            "autumn" => match h {
                h if h < 0.3 => Weather::Rain,
                h if h < 0.4 => Weather::Storm,
                h if h < 0.55 => Weather::Fog,
                _ => Weather::Clear,
            },
            _ => match h {
                h if h < 0.3 => Weather::Rain,
                h if h < 0.38 => Weather::Storm,
                h if h < 0.48 => Weather::Fog,
                _ => Weather::Clear,
            },
        }
    }

    /// The weather now, and how far it has come in (0..1): it fades in at
    /// the start of its spell and out at the end, unless the next is the same.
    pub fn weather(&self) -> (Weather, f32) {
        let slot = (self.time / SPELL).floor() as i64;
        let f = (self.time / SPELL).fract();
        let w = self.weather_of(slot);
        if w == Weather::Clear {
            return (w, 0.0);
        }
        let fade = 0.12;
        let mut k: f32 = 1.0;
        if self.weather_of(slot - 1) != w {
            k = k.min(f / fade);
        }
        if self.weather_of(slot + 1) != w {
            k = k.min((1.0 - f) / fade);
        }
        (w, k.clamp(0.0, 1.0))
    }

    /// How thick the canopy is over a tile, 0..1, from the trees around it.
    pub fn canopy(&self, x: i32, y: i32) -> f32 {
        let mut n = 0;
        for dy in -3..=3 {
            for dx in -3..=3 {
                if matches!(self.map.obj(x + dx, y + dy), O_TREE | O_BIGTREE) {
                    n += 1;
                }
            }
        }
        ((n as f32 / 49.0 - 0.18) / 0.32).clamp(0.0, 1.0)
    }

    /// Ease the forest's gloom and tell the journal when the weather turns.
    pub(super) fn sky_tick(&mut self, dt: f32) {
        let (x, y) = self.ents[0].tile();
        let target = if self.map.building_at(x, y).is_some() {
            0.0
        } else {
            self.canopy(x, y)
        };
        self.forest_view += (target - self.forest_view) * (dt * 1.2).min(1.0);
        let (w, k) = self.weather();
        let now = if k > 0.5 { w } else { Weather::Clear };
        let code = now as u8;
        if code != self.weather_seen {
            if self.weather_seen != u8::MAX && self.created {
                self.msg(now.news(), C_LORE);
            }
            self.weather_seen = code;
            self.hud_ver += 1;
        }
        let _ = Sim::day(self.time);
    }

    /// The word for the weather in the HUD, if any.
    pub fn weather_word(&self) -> &'static str {
        let (w, k) = self.weather();
        if k > 0.5 {
            w.word()
        } else {
            ""
        }
    }
}
