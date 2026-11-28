//! The living world: a calendar, the powers beyond the map, and the strength
//! of each hostile place on it.
//!
//! - **Regions** are forces off the map (a witch-realm, a clan, the wild). Each
//!   has a strength 0–100 that drifts by itself each day and falls a little
//!   when its creatures are slain on the map.
//! - **Sites** are the hostile places on the map (graveyard, orc camp, troll
//!   hills, the lair, the wilds). A site's strength sets how many of its
//!   creatures roam and how fast they return; it falls as they are slain and
//!   regrows each day toward its region's strength (or its own base).
//! - When a site or region crosses a threshold, a notice goes to the journal
//!   and into the rumours townsfolk repeat.
//!
//! All of it is plain numbers, advanced continuously with game time and kept
//! in the save. A world pack sets the calendar, regions and sites in
//! `[sim]`, `[region.<id>]` and `[site.<id>]`.

use crate::pack::{Pack, Value};

/// Seconds of game time in a day (matches the six four-hour periods).
pub const DAY: f32 = 480.0;
/// Sites the engine's map has, in the order spawners are grouped.
pub const SITE_IDS: [&str; 5] = ["graveyard", "orcs", "hills", "lair", "wilds"];
/// the labels the engine's map gives the sites' places
pub const SITE_NAMES: [&str; 5] = [
    "Graveyard",
    "Orc Camp",
    "Troll Hills",
    "Dragon Lair",
    "the wilds",
];
/// how the engine's own world names them in a sentence
const SITE_PROSE: [&str; 5] = [
    "the graveyard",
    "the orc camp",
    "the troll hills",
    "the dragon's lair",
    "the wilds",
];
/// Keys a pack may give each section (checked by `data::pack_warnings`).
pub const SIM_KEYS: [&str; 3] = ["start", "era", "months"];
/// `drift` is in tenths of a point a day: 5 is half a point a day.
pub const REGION_KEYS: [&str; 7] = ["name", "strength", "drift", "rise", "fall", "min", "shadow"];
pub const SITE_KEYS: [&str; 10] = [
    "name", "region", "strength", "base", "regrow", "rise", "fall", "raid", "hit", "broken",
];
/// Sites that send warbands down the road when strong.
pub const RAIDERS: [usize; 3] = [0, 1, 2];
/// A site sends raids at or above this strength.
pub const RAID_AT: f32 = 60.0;
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
/// how many notices the chronicle keeps
const KEEP: usize = 12;

/// The site a monster belongs to.
pub fn site_of(mon_id: &str) -> usize {
    match mon_id {
        "skeleton" | "zombie" | "lich" => 0,
        "orc" | "orclord" => 1,
        "troll" | "ettin" => 2,
        "dragon" => 3,
        _ => 4,
    }
}

#[derive(Clone, Debug)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub strength: f32,
    /// strength gained per day by itself (negative: waning)
    pub drift: f32,
    /// it never falls below this, however many of its creatures are slain
    pub min: f32,
    pub rise: String,
    pub fall: String,
    /// its places cast the Shadow over the land around them
    pub shadow: bool,
    band: i32,
}

#[derive(Clone, Debug)]
pub struct Site {
    pub id: &'static str,
    pub name: String,
    /// index into `regions`, or none: then it regrows toward `base`
    pub region: Option<usize>,
    pub strength: f32,
    pub base: f32,
    /// strength regained (or shed) per day toward its target
    pub regrow: f32,
    pub rise: String,
    pub fall: String,
    /// news lines for its raids: setting out, burning the farms, broken
    pub raid: String,
    pub hit: String,
    pub broken: String,
    /// seconds before it may raid again
    pub raid_cool: f32,
    level: i32,
}

#[derive(Clone, Debug)]
pub struct Sim {
    pub era: String,
    pub months: Vec<String>,
    /// the first day, as (year, month 1-12, day 1-30)
    pub start: (i32, i32, i32),
    pub regions: Vec<Region>,
    pub sites: Vec<Site>,
    /// (day, text), newest last
    pub events: Vec<(i64, String)>,
    /// the last day a dawn notice was given
    pub day_seen: i64,
    /// how well goods move on the roads, 0–100: sets shop prices
    pub trade: f32,
    /// seconds until the next caravan sets out
    pub caravan_t: f32,
    /// the sleeper in the deep (`game/deep.rs`): seconds until the stars are
    /// right and R'lyeh rises, seconds left while it stands (0: sunk), and
    /// whether the dreams that come before have been told of
    pub deep_t: f32,
    pub awake: f32,
    pub omen: bool,
}

/// Strength bands a site moves through; a notice marks each crossing.
pub const LEVELS: [&str; 5] = ["cleared", "quiet", "stirring", "strong", "swarming"];
fn level_of(s: f32) -> i32 {
    match s {
        s if s < 5.0 => 0,
        s if s < 25.0 => 1,
        s if s < 55.0 => 2,
        s if s < 80.0 => 3,
        _ => 4,
    }
}
fn band_of(s: f32) -> i32 {
    (s / 25.0).floor().clamp(0.0, 3.0) as i32
}

pub fn ordinal(n: i32) -> String {
    let suf = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}

fn num(v: Option<&Value>) -> Option<f32> {
    match v? {
        Value::Int(n) => Some(*n as f32),
        _ => None,
    }
}

impl Sim {
    pub fn new(p: &Pack) -> Sim {
        let era = p.str("sim", "era").unwrap_or("").to_string();
        let months = match p.strs("sim", "months") {
            Some(m) if m.len() == 12 => m,
            _ => MONTHS.iter().map(|s| s.to_string()).collect(),
        };
        let start = p
            .str("sim", "start")
            .and_then(parse_date)
            .unwrap_or((1, 1, 1));
        let mut regions: Vec<Region> = p
            .ids("region")
            .into_iter()
            .map(|id| {
                let sec = format!("region.{id}");
                let s = num(p.get(&sec, "strength"))
                    .unwrap_or(50.0)
                    .clamp(0.0, 100.0);
                Region {
                    id: id.to_string(),
                    name: p.str(&sec, "name").unwrap_or(id).to_string(),
                    strength: s,
                    drift: num(p.get(&sec, "drift")).unwrap_or(0.0) / 10.0,
                    min: num(p.get(&sec, "min")).unwrap_or(5.0).clamp(0.0, 100.0),
                    rise: p.str(&sec, "rise").unwrap_or("").to_string(),
                    fall: p.str(&sec, "fall").unwrap_or("").to_string(),
                    shadow: p.get(&sec, "shadow") == Some(&Value::Bool(true)),
                    band: band_of(s),
                }
            })
            .collect();
        regions.sort_by(|a, b| a.id.cmp(&b.id));
        let sites = SITE_IDS
            .iter()
            .zip(SITE_NAMES)
            .zip(SITE_PROSE)
            .map(|((&id, dname), prose)| {
                let sec = format!("site.{id}");
                let region = p
                    .str(&sec, "region")
                    .and_then(|r| regions.iter().position(|g| g.id == r));
                let base = num(p.get(&sec, "base")).unwrap_or(50.0).clamp(0.0, 100.0);
                let target = region.map(|r| regions[r].strength).unwrap_or(base);
                let s = num(p.get(&sec, "strength"))
                    .unwrap_or(target)
                    .clamp(0.0, 100.0);
                Site {
                    id,
                    name: p
                        .str(&sec, "name")
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            // the place's own label, as the pack words it
                            let t = p.tr(dname);
                            if t == dname {
                                prose.to_string()
                            } else {
                                t
                            }
                        }),
                    region,
                    strength: s,
                    base,
                    regrow: num(p.get(&sec, "regrow")).unwrap_or(6.0).max(0.0),
                    rise: p.str(&sec, "rise").unwrap_or("").to_string(),
                    fall: p.str(&sec, "fall").unwrap_or("").to_string(),
                    raid: p.str(&sec, "raid").unwrap_or("").to_string(),
                    hit: p.str(&sec, "hit").unwrap_or("").to_string(),
                    broken: p.str(&sec, "broken").unwrap_or("").to_string(),
                    raid_cool: DAY * 0.5,
                    level: level_of(s),
                }
            })
            .collect();
        Sim {
            era,
            months,
            start,
            regions,
            sites,
            events: vec![],
            day_seen: 0,
            trade: 50.0,
            caravan_t: DAY * 0.25,
            deep_t: DAY * 2.0,
            awake: 0.0,
            omen: false,
        }
    }

    pub fn day(time: f32) -> i64 {
        (time / DAY).floor() as i64
    }
    /// (year, month 1-12, day 1-30) of a day since the start; months are 30 days
    pub fn date(&self, day: i64) -> (i32, i32, i32) {
        let (y, m, d) = self.start;
        let n = (y as i64) * 360 + (m as i64 - 1) * 30 + (d as i64 - 1) + day;
        (
            n.div_euclid(360) as i32,
            (n.rem_euclid(360) / 30 + 1) as i32,
            (n.rem_euclid(30) + 1) as i32,
        )
    }
    pub fn date_str(&self, day: i64) -> String {
        let (y, m, d) = self.date(day);
        let month = &self.months[(m - 1) as usize];
        if self.era.is_empty() {
            format!("{} of {}, Year {}", ordinal(d), month, y)
        } else {
            format!("{} of {}, {} {}", ordinal(d), month, self.era, y)
        }
    }
    pub fn season(&self, day: i64) -> &'static str {
        match self.date(day).1 {
            3..=5 => "spring",
            6..=8 => "summer",
            9..=11 => "autumn",
            _ => "winter",
        }
    }
    fn note(&mut self, day: i64, s: String, out: &mut Vec<String>) {
        self.events.push((day, s.clone()));
        if self.events.len() > KEEP {
            self.events.remove(0);
        }
        out.push(s);
    }

    /// Advance the world by `dt` seconds ending at game time `time`.
    /// Returns notices for the journal.
    pub fn tick(&mut self, dt: f32, time: f32) -> Vec<String> {
        let mut out = vec![];
        let f = dt / DAY;
        let day = Sim::day(time);
        for r in self.regions.iter_mut() {
            r.strength = (r.strength + r.drift * f).clamp(r.min, 100.0);
        }
        // trade settles back toward middling, two points a day
        self.trade += (50.0 - self.trade).clamp(-2.0 * f, 2.0 * f);
        self.caravan_t -= dt;
        for s in self.sites.iter_mut() {
            s.raid_cool -= dt;
        }
        for k in 0..self.sites.len() {
            let target = match self.sites[k].region {
                Some(r) => self.regions[r].strength,
                None => self.sites[k].base,
            };
            let s = &mut self.sites[k];
            let step = s.regrow * f;
            if s.strength < target {
                s.strength = (s.strength + step).min(target);
            } else {
                s.strength = (s.strength - step).max(target);
            }
        }
        self.check(day, &mut out);
        out
    }

    /// One of a site's creatures was slain: the site and its region weaken.
    pub fn slain(&mut self, mon_id: &str, fame: i32, time: f32) -> Vec<String> {
        let k = site_of(mon_id);
        let loss = (fame as f32 / 30.0).clamp(0.5, 25.0);
        let s = &mut self.sites[k];
        s.strength = (s.strength - loss).max(0.0);
        if let Some(r) = s.region {
            let r = &mut self.regions[r];
            r.strength = (r.strength - loss * 0.15).max(r.min);
        }
        let mut out = vec![];
        self.check(Sim::day(time), &mut out);
        out
    }

    /// Notices for thresholds crossed, with a margin so a value hovering on a
    /// line does not chatter.
    fn check(&mut self, day: i64, out: &mut Vec<String>) {
        for k in 0..self.sites.len() {
            let s = &self.sites[k];
            let lv = level_of(s.strength);
            let held = level_of(s.strength + if lv > s.level { -2.0 } else { 2.0 });
            if lv == s.level || held == s.level {
                continue;
            }
            let up = lv > s.level;
            let text = if lv == 0 {
                format!("{} lies empty, for now.", cap(&s.name))
            } else if up && !s.rise.is_empty() {
                s.rise.replace("{name}", &s.name)
            } else if !up && !s.fall.is_empty() {
                s.fall.replace("{name}", &s.name)
            } else if up {
                let how = [
                    "",
                    "stirs again",
                    "grows restless",
                    "grows strong",
                    "teems with foes",
                ];
                format!("Word comes that {} {}.", s.name, how[lv as usize])
            } else {
                let how = ["", "falls quiet", "grows quieter", "weakens", ""];
                format!("{} {}.", cap(&s.name), how[lv as usize])
            };
            self.sites[k].level = lv;
            self.note(day, text, out);
        }
        for k in 0..self.regions.len() {
            let r = &self.regions[k];
            let b = band_of(r.strength);
            let held = band_of(r.strength + if b > r.band { -2.0 } else { 2.0 });
            if b == r.band || held == r.band {
                continue;
            }
            let up = b > r.band;
            let text = match (up, r.rise.is_empty(), r.fall.is_empty()) {
                (true, false, _) => r.rise.replace("{name}", &r.name),
                (false, _, false) => r.fall.replace("{name}", &r.name),
                (true, ..) => format!("The power of {} grows.", r.name),
                (false, ..) => format!("The power of {} wanes.", r.name),
            };
            self.regions[k].band = b;
            self.note(day, text, out);
        }
    }

    /// Whether a strong site sends a warband now (`roll` in 0..1). A site at
    /// full strength raids about twice a day; once a raid ends it rests half a
    /// day.
    pub fn raid_due(&mut self, dt: f32, roll: f32) -> Option<usize> {
        for &k in RAIDERS.iter() {
            let s = &self.sites[k];
            if s.strength < RAID_AT || s.raid_cool > 0.0 {
                continue;
            }
            let per_day = 0.5 + 1.5 * (s.strength - RAID_AT) / (100.0 - RAID_AT);
            if roll < per_day * dt / DAY {
                self.sites[k].raid_cool = DAY * 0.5;
                return Some(k);
            }
        }
        None
    }
    fn after_raid(&mut self, k: usize) {
        self.sites[k].raid_cool = DAY * 0.5;
    }
    /// A raid burned the farms: trade falls, the site and its region gain.
    pub fn raid_hit(&mut self, k: usize, day: i64, news: String) -> Vec<String> {
        self.after_raid(k);
        self.trade = (self.trade - 15.0).max(0.0);
        self.sites[k].strength = (self.sites[k].strength + 8.0).min(100.0);
        if let Some(r) = self.sites[k].region {
            self.regions[r].strength = (self.regions[r].strength + 3.0).min(100.0);
        }
        let mut out = vec![];
        self.note(day, news, &mut out);
        self.check(day, &mut out);
        out
    }
    /// A raid was destroyed before it could do harm.
    pub fn raid_broken(&mut self, k: usize, day: i64, news: String) -> Vec<String> {
        self.after_raid(k);
        self.trade = (self.trade + 3.0).min(100.0);
        if let Some(r) = self.sites[k].region {
            let r = &mut self.regions[r];
            r.strength = (r.strength - 4.0).max(r.min);
        }
        let mut out = vec![];
        self.note(day, news, &mut out);
        self.check(day, &mut out);
        out
    }
    /// A caravan came in (trade rises) or was lost (trade falls).
    pub fn caravan(&mut self, arrived: bool, day: i64, news: String) -> Vec<String> {
        self.trade = if arrived {
            (self.trade + 12.0).min(100.0)
        } else {
            (self.trade - 10.0).max(0.0)
        };
        let mut out = vec![];
        self.note(day, news, &mut out);
        out
    }
    /// Shop prices for this much trade: scarce goods cost more, and fetch more.
    pub fn price_factor(&self) -> f32 {
        1.3 - 0.6 * self.trade / 100.0
    }
    pub fn trade_word(&self) -> &'static str {
        match self.trade {
            t if t < 20.0 => "poor",
            t if t < 40.0 => "lean",
            t if t < 65.0 => "steady",
            _ => "brisk",
        }
    }
    /// Post a line of news (also used for raids setting out).
    pub fn news(&mut self, day: i64, s: String) -> Vec<String> {
        let mut out = vec![];
        self.note(day, s, &mut out);
        out
    }

    /// How many of a site's creatures may roam, for a spawner made for `base`
    /// at middling strength. A lone boss stays alone; it only leaves when the
    /// site is nearly cleared.
    pub fn cap(&self, site: usize, base: usize) -> usize {
        let s = self.sites[site].strength;
        if base <= 1 {
            return (s >= 15.0) as usize;
        }
        ((base as f32 * s / 50.0).round() as usize).min(base * 2)
    }
    /// Respawn delay factor: a strong site refills faster.
    pub fn pace(&self, site: usize) -> f32 {
        1.5 - self.sites[site].strength / 100.0
    }
    /// A rumour from the last few days, if any.
    pub fn rumour(&self, day: i64, pick: u32) -> Option<&str> {
        let recent: Vec<&(i64, String)> = self
            .events
            .iter()
            .rev()
            .take(4)
            .filter(|(d, _)| day - d <= 3)
            .collect();
        if recent.is_empty() {
            return None;
        }
        Some(&recent[pick as usize % recent.len()].1)
    }

    // ------------------------------------------------------------ saving
    pub fn save_lines(&self) -> Vec<(&'static str, String)> {
        let f = |v: f32| format!("{:.2}", v);
        vec![
            (
                "sim_regions",
                self.regions
                    .iter()
                    .map(|r| format!("{}:{}:{}", r.id, f(r.strength), r.band))
                    .collect::<Vec<_>>()
                    .join(";"),
            ),
            (
                "sim_sites",
                self.sites
                    .iter()
                    .map(|s| format!("{}:{}:{}", s.id, f(s.strength), s.level))
                    .collect::<Vec<_>>()
                    .join(";"),
            ),
            (
                "sim_events",
                self.events
                    .iter()
                    .map(|(d, t)| {
                        format!("{}|{}", d, t.replace(['|', '\n'], " ").replace('~', "-"))
                    })
                    .collect::<Vec<_>>()
                    .join("~"),
            ),
            ("sim_day", self.day_seen.to_string()),
            (
                "sim_deep",
                format!("{}:{}:{}", f(self.deep_t), f(self.awake), self.omen as u8),
            ),
            (
                "sim_trade",
                format!(
                    "{}:{}:{}",
                    f(self.trade),
                    f(self.caravan_t),
                    self.sites
                        .iter()
                        .map(|s| f(s.raid_cool))
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            ),
        ]
    }
    pub fn load_line(&mut self, k: &str, v: &str) -> bool {
        let triples = |v: &str| -> Vec<(String, f32, i32)> {
            v.split(';')
                .filter_map(|e| {
                    let mut it = e.split(':');
                    Some((
                        it.next()?.to_string(),
                        it.next()?.parse().ok()?,
                        it.next()?.parse().ok()?,
                    ))
                })
                .collect()
        };
        match k {
            "sim_regions" => {
                for (id, s, b) in triples(v) {
                    if let Some(r) = self.regions.iter_mut().find(|r| r.id == id) {
                        r.strength = s.clamp(r.min, 100.0);
                        r.band = b.clamp(0, 3);
                    }
                }
            }
            "sim_sites" => {
                for (id, s, l) in triples(v) {
                    if let Some(x) = self.sites.iter_mut().find(|x| x.id == id) {
                        x.strength = s.clamp(0.0, 100.0);
                        x.level = l.clamp(0, 4);
                    }
                }
            }
            "sim_events" => {
                self.events = v
                    .split('~')
                    .filter_map(|e| {
                        let (d, t) = e.split_once('|')?;
                        Some((d.parse().ok()?, t.to_string()))
                    })
                    .collect();
                let n = self.events.len();
                self.events.drain(..n.saturating_sub(KEEP));
            }
            "sim_day" => self.day_seen = v.parse().unwrap_or(0),
            "sim_deep" => {
                let mut it = v.split(':');
                self.deep_t = it.next().and_then(|x| x.parse().ok()).unwrap_or(DAY * 2.0);
                self.awake = it
                    .next()
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0.0f32)
                    .max(0.0);
                self.omen = it.next() == Some("1");
            }
            "sim_trade" => {
                let mut it = v.split(':');
                self.trade = it
                    .next()
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(50.0f32)
                    .clamp(0.0, 100.0);
                self.caravan_t = it.next().and_then(|x| x.parse().ok()).unwrap_or(DAY * 0.25);
                for (s, c) in self
                    .sites
                    .iter_mut()
                    .zip(it.next().unwrap_or("").split(','))
                {
                    s.raid_cool = c.parse().unwrap_or(0.0);
                }
            }
            _ => return false,
        }
        true
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// "1640-03-01" as (1640, 3, 1); months 1-12, days 1-30
pub fn parse_date(s: &str) -> Option<(i32, i32, i32)> {
    let mut it = s.trim().splitn(3, '-');
    let y: i32 = it.next()?.parse().ok()?;
    let m: i32 = it.next()?.parse().ok()?;
    let d: i32 = it.next()?.parse().ok()?;
    ((1..=12).contains(&m) && (1..=30).contains(&d)).then_some((y, m, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn northish() -> Pack {
        Pack::parse(
            r##"[pack]
id = "t"
[sim]
start = "640-03-01"
era = "W.Y."
[region.vathmor]
name = "Vathmor"
strength = 70
drift = 5
[site.orcs]
name = "the war-camp"
region = "vathmor"
rise = "Orcs gather at {name}."
"##,
        )
        .unwrap()
    }

    #[test]
    fn the_calendar_counts_thirty_day_months() {
        let s = Sim::new(&northish());
        assert_eq!(s.date(0), (640, 3, 1));
        assert_eq!(s.date(29), (640, 3, 30));
        assert_eq!(s.date(30), (640, 4, 1));
        assert_eq!(s.date(300), (641, 1, 1));
        assert_eq!(s.date_str(0), "1st of March, W.Y. 640");
        assert_eq!(s.season(0), "spring");
        assert_eq!(
            Sim::new(&crate::pack::builtin()).date_str(11),
            "12th of January, Year 1"
        );
        assert_eq!(ordinal(13), "13th");
        assert_eq!(ordinal(22), "22nd");
    }

    #[test]
    fn slaying_weakens_a_site_and_its_region_then_it_regrows() {
        let mut s = Sim::new(&northish());
        assert_eq!(s.sites[1].strength, 70.0);
        assert_eq!(s.cap(1, 6), 8);
        // a camp cleared to the last orc
        let mut notes = vec![];
        for _ in 0..40 {
            notes.extend(s.slain("orc", 60, 100.0));
        }
        assert_eq!(s.sites[1].strength, 0.0);
        assert_eq!(s.cap(1, 6), 0);
        assert_eq!(s.cap(1, 1), 0);
        assert!(s.regions[0].strength < 70.0);
        assert!(notes.iter().any(|n| n.contains("lies empty")), "{notes:?}");
        // other sites are untouched
        assert_eq!(s.sites[0].strength, 50.0);
        // days later the camp has refilled toward Vathmor
        let before = s.sites[1].strength;
        let mut notes = vec![];
        for k in 0..(DAY as i32 * 8) {
            notes.extend(s.tick(1.0, 100.0 + k as f32));
        }
        assert!(s.sites[1].strength > before + 40.0);
        assert!(
            notes.iter().any(|n| n == "Orcs gather at the war-camp."),
            "{notes:?}"
        );
    }

    #[test]
    fn a_region_drifts_and_notices_do_not_chatter() {
        let mut s = Sim::new(&northish());
        let mut notes = vec![];
        for k in 0..(DAY as i32 * 2) {
            notes.extend(s.tick(1.0, k as f32));
        }
        // drift 5 is half a point a day
        assert!((s.regions[0].strength - 71.0).abs() < 0.01);
        assert!(notes.is_empty(), "{notes:?}");
        // hovering on the 75 line gives one notice, not one per crossing
        s.regions[0].strength = 75.5;
        let mut n = s.tick(0.0, 0.0);
        for _ in 0..10 {
            s.regions[0].strength = 74.5;
            n.extend(s.tick(0.0, 0.0));
            s.regions[0].strength = 75.5;
            n.extend(s.tick(0.0, 0.0));
        }
        assert!(n.len() <= 1, "{n:?}");
    }

    #[test]
    fn state_survives_a_save() {
        let mut s = Sim::new(&northish());
        s.slain("lich", 400, 0.0);
        s.events.push((3, "A test | of ~ saving".into()));
        s.day_seen = 3;
        let mut t = Sim::new(&northish());
        for (k, v) in s.save_lines() {
            assert!(t.load_line(k, &v));
        }
        assert!((t.sites[0].strength - s.sites[0].strength).abs() < 0.01);
        assert_eq!(t.events.last().unwrap().1, "A test   of - saving");
        assert_eq!(t.day_seen, 3);
    }
}
