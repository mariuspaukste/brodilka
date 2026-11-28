//! Travellers on the roads, driven by the world simulation.
//!
//! - **Raids**: when a dark place is strong (`sim::RAID_AT`), it sends a
//!   warband down its road toward the town. It stops short of the guarded
//!   ground and burns the outlying farms for a while; if it lives through
//!   that, it marches home and the place grows bolder and trade suffers. Slay
//!   it first and the raid is broken, and the power behind it is shaken.
//! - **Caravans**: a merchant and a drover walk from the city to the town now
//!   and then. Each one that comes in makes trade brisker and shop prices
//!   lower. When the wilds are restless, wolves may fall on a caravan on the
//!   road; drive them off in time or the caravan is lost.
//!
//! Travellers are not saved: a journey under way when the game is closed is
//! simply over.
use super::*;
use crate::sim::{Sim, DAY, SITE_IDS};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// a warband from this site
    Raid(usize),
    Caravan,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Stage {
    March,
    /// raiders burning farms, or a caravan beset: seconds left
    Hold(f32),
    Return,
}

pub struct Travel {
    pub id: u32,
    pub kind: Kind,
    pub stage: Stage,
    /// tiles to follow, start to goal
    pub route: Vec<(i32, i32)>,
    pub goal: usize,
    pub members: Vec<u32>,
    /// the wolves on a beset caravan
    pub foes: Vec<u32>,
    /// route index where wolves lie in wait, if they do
    pub ambush_at: Option<usize>,
    /// the settlement they head for, as shown
    pub place: String,
    /// raiders the player has slain
    pub player_kills: u32,
    pub age: f32,
}

/// The town as a sentence names it: a pack's name for it, or "the town".
fn town_name() -> String {
    match crate::pack::t("Town") {
        t if t == "Town" => "the town".into(),
        t => t,
    }
}

/// how long a warband burns farms before it turns home
const PILLAGE: f32 = 90.0;
/// how long a beset caravan holds out
const BESET: f32 = 60.0;
/// a journey that has not finished by then is hurried along
const MAX_MARCH: f32 = 240.0;

impl Game {
    /// Prices at the shops follow trade on the roads.
    pub fn buy_price(&self, d: &ItemDef) -> i32 {
        ((d.price as f32 * self.sim.price_factor()).round() as i32).max(1)
    }
    pub fn sell_for(&self, it: &Item) -> i32 {
        ((it.sell_price() as f32 * self.sim.price_factor()).round() as i32).max(1)
    }

    fn near_guarded(&self, (x, y): (i32, i32), r: i32) -> bool {
        (-r..=r).any(|dy| (-r..=r).any(|dx| self.map.guarded(x + dx, y + dy)))
    }
    fn place_near(&self, (x, y): (i32, i32)) -> String {
        let c = |(x0, y0, x1, y1): (i32, i32, i32, i32)| ((x0 + x1) / 2, (y0 + y1) / 2);
        let d = |(cx, cy): (i32, i32)| (cx - x).pow(2) + (cy - y).pow(2);
        if d(c(self.map.city)) < d(c(self.map.town)) {
            crate::pack::t("Duskhold")
        } else {
            town_name()
        }
    }
    fn site_name(&self, k: usize) -> String {
        self.sim.sites[k].name.clone()
    }
    fn news(&mut self, s: String, col: u32) {
        let day = Sim::day(self.time);
        for n in self.sim.news(day, s) {
            self.msg(&n, col);
        }
    }
    /// a free tile within `r` of `at`, off guarded ground
    /// (any of them, however few: close by the walls there may be only one)
    fn free_near(&mut self, at: (i32, i32), r0: i32, r: i32) -> Option<(i32, i32)> {
        let mut free = vec![];
        for y in at.1 - r..=at.1 + r {
            for x in at.0 - r..=at.0 + r {
                if (x - at.0).abs().max((y - at.1).abs()) >= r0
                    && inb(x, y)
                    && self.map.passable(x, y)
                    && self.map.reach[idx(x, y)]
                    && !self.map.guarded(x, y)
                    && !self.ents.iter().any(|e| !e.dead && e.tile() == (x, y))
                {
                    free.push((x, y));
                }
            }
        }
        (!free.is_empty()).then(|| *self.rng.pick(&free))
    }
    fn add_mon(&mut self, mon: &str, at: (i32, i32), march: u32) -> u32 {
        let mi = mon_idx(mon);
        let d = &MONS[mi];
        let id = self.nid();
        let mut e = Ent::new(id, EK::Mon(mi), d.name.to_string(), at.0, at.1, d.hp);
        e.march = march;
        e.ai_t = self.rng.f();
        if !d.caster.is_empty() {
            e.mana = 200;
            e.mmana = 200;
        }
        self.ents.push(e);
        id
    }
    fn despawn(&mut self, ids: &[u32]) {
        for e in self.ents.iter_mut() {
            if ids.contains(&e.id) {
                e.dead = true;
            }
        }
    }
    fn alive(&self, id: u32) -> Option<usize> {
        self.ents.iter().position(|e| e.id == id && !e.dead)
    }

    // ------------------------------------------------------------ setting out
    pub fn launch_raid(&mut self, k: usize) -> bool {
        let Some(road) = self
            .map
            .roads
            .iter()
            .find(|r| r.from == "town" && r.to == SITE_IDS[k])
        else {
            return false;
        };
        let mut route = road.tiles.clone();
        route.reverse();
        // march until just short of guarded ground
        let mut goal = 0;
        for (n, &t) in route.iter().enumerate() {
            if self.near_guarded(t, 4) {
                break;
            }
            goal = n;
        }
        if goal < 6 {
            return false;
        }
        let s = self.sim.sites[k].strength;
        let n = 2 + ((s - crate::sim::RAID_AT) / 15.0).max(0.0) as usize;
        let mut band: Vec<&str> = match k {
            0 => (0..n)
                .map(|j| if j % 2 == 0 { "skeleton" } else { "zombie" })
                .collect(),
            1 => vec!["orc"; n],
            _ => vec!["troll"; 1 + (s >= 80.0) as usize],
        };
        match k {
            1 if s >= 80.0 => band.push("orclord"),
            2 if s >= 90.0 => band.push("ettin"),
            _ => {}
        }
        let id = self.nid();
        let mut members = vec![];
        for m in band {
            if let Some(at) = self.free_near(route[0], 0, 3) {
                members.push(self.add_mon(m, at, id));
            }
        }
        if members.is_empty() {
            return false;
        }
        let place = self.place_near(route[goal]);
        let name = self.site_name(k);
        let line = match self.sim.sites[k].raid.as_str() {
            "" => format!("A warband sets out from {name} toward {place}!"),
            l => l.replace("{name}", &name).replace("{town}", &place),
        };
        self.travels.push(Travel {
            id,
            kind: Kind::Raid(k),
            stage: Stage::March,
            route,
            goal,
            members,
            foes: vec![],
            ambush_at: None,
            place,
            player_kills: 0,
            age: 0.0,
        });
        self.news(line, C_WARN);
        true
    }

    pub fn launch_caravan(&mut self) -> bool {
        let Some(road) = self
            .map
            .roads
            .iter()
            .find(|r| r.from == "city" && r.to == "town")
        else {
            return false;
        };
        let route = road.tiles.clone();
        let goal = route.len().saturating_sub(1);
        if goal < 6 {
            return false;
        }
        let id = self.nid();
        let vd = &VENDORS[V_PROV];
        let merchant = format!("{} the merchant", self.rng.pick(vd.names));
        let mut members = vec![];
        for (v, name) in [(V_PROV, merchant), (V_FOLK, "a drover".to_string())] {
            let at = route[members.len().min(goal)];
            let nid = self.nid();
            let mut e = Ent::new(nid, EK::Npc(v), name, at.0, at.1, 500);
            e.look = self.rng.next_u32();
            e.march = id;
            self.ents.push(e);
            members.push(nid);
        }
        // wolves lie in wait when the wilds are restless
        let wilds = self.sim.sites[4].strength;
        let risk = if wilds < 25.0 {
            0.0
        } else {
            0.2 + wilds / 250.0
        };
        let ambush_at = if self.rng.f() < risk {
            (route.len() / 4..route.len() * 3 / 4).find(|&n| !self.near_guarded(route[n], 7))
        } else {
            None
        };
        let city = crate::pack::t("Duskhold");
        let town = town_name();
        self.travels.push(Travel {
            id,
            kind: Kind::Caravan,
            stage: Stage::March,
            route,
            goal,
            members,
            foes: vec![],
            ambush_at,
            place: town.clone(),
            player_kills: 0,
            age: 0.0,
        });
        self.news(
            format!("A caravan sets out from {city} for {town}."),
            C_LORE,
        );
        true
    }

    // ------------------------------------------------------------ on the way
    /// Move a traveller one step along its road. False when it should act on
    /// its own (raiders burning farms wander about their goal).
    pub(super) fn march_step(&mut self, i: usize) -> bool {
        let id = self.ents[i].march;
        let Some(t) = self.travels.iter().find(|t| t.id == id) else {
            self.ents[i].march = 0;
            return false;
        };
        let back = match t.stage {
            Stage::Hold(_) => return t.kind == Kind::Caravan,
            Stage::March => false,
            Stage::Return => true,
        };
        let end = if back { 0 } else { t.goal };
        let wp = self.ents[i].wp.min(t.route.len() - 1);
        let (gx, gy) = t.route[wp];
        let next = if self.ents[i].dist(gx as f32, gy as f32) < 1.6 && wp != end {
            if back {
                wp.saturating_sub(3).max(end)
            } else {
                (wp + 3).min(end)
            }
        } else {
            wp
        };
        let (gx, gy) = t.route[next];
        self.ents[i].wp = next;
        self.ents[i].home = (gx, gy);
        if self.ents[i].dist(gx as f32, gy as f32) >= 1.0 && self.ents[i].path.is_empty() {
            if let Some(s) = self.greedy_step(i, gx as f32, gy as f32, false) {
                self.ents[i].path.push(s);
            } else if self.ents[i].ai_t <= 0.0 {
                self.ents[i].ai_t = 1.0;
                let (sx, sy) = self.ents[i].tile();
                let w = self.walker(i);
                if let Some(mut p) = self.map.find_path_as(sx, sy, gx, gy, 0, w) {
                    p.truncate(6);
                    self.ents[i].path = p;
                }
            }
        }
        true
    }

    fn at_end(&self, t: &Travel, id: u32, back: bool) -> bool {
        let end = if back { 0 } else { t.goal };
        let (x, y) = t.route[end];
        self.alive(id)
            .map(|i| self.ents[i].wp == end && self.ents[i].dist(x as f32, y as f32) < 2.0)
            .unwrap_or(false)
    }

    /// Launch, advance and end journeys; called with the world's tick.
    pub(super) fn travel_tick(&mut self, dt: f32) {
        if !self.travels.iter().any(|t| matches!(t.kind, Kind::Raid(_))) {
            let roll = self.rng.f();
            if let Some(k) = self.sim.raid_due(dt, roll) {
                self.launch_raid(k);
            }
        }
        if self.sim.caravan_t <= 0.0 {
            self.sim.caravan_t = DAY * (0.7 + 0.6 * self.rng.f());
            if !self.travels.iter().any(|t| t.kind == Kind::Caravan) {
                self.launch_caravan();
            }
        }
        let mut k = 0;
        while k < self.travels.len() {
            if self.travel_step(k, dt) {
                k += 1;
            } else {
                let t = self.travels.remove(k);
                let all: Vec<u32> = t.members.iter().chain(t.foes.iter()).copied().collect();
                self.despawn(&all);
            }
        }
    }

    /// One journey's turn; false when it is over.
    fn travel_step(&mut self, k: usize, dt: f32) -> bool {
        let day = Sim::day(self.time);
        let ids: Vec<u32> = self.travels[k].members.clone();
        let alive: Vec<u32> = ids
            .into_iter()
            .filter(|&id| self.alive(id).is_some())
            .collect();
        self.travels[k].members = alive.clone();
        self.travels[k].age += dt;
        let t = &self.travels[k];
        let (kind, stage, age) = (t.kind, t.stage, t.age);
        match kind {
            Kind::Raid(site) => {
                let name = self.site_name(site);
                let place = t.place.clone();
                if alive.is_empty() {
                    let line = match self.sim.sites[site].broken.as_str() {
                        "" => format!("The warband from {name} was broken on the road."),
                        l => l.replace("{name}", &name).replace("{town}", &place),
                    };
                    if t.player_kills > 0 {
                        self.pl.fame = (self.pl.fame + 150).min(15000);
                        self.pl.karma = (self.pl.karma + 150).min(15000);
                        self.msg(&format!("{place} will speak of thy deeds."), C_GOOD);
                    }
                    for n in self.sim.raid_broken(site, day, line) {
                        self.msg(&n, C_LORE);
                    }
                    return false;
                }
                match stage {
                    Stage::March => {
                        let arrived = alive.iter().any(|&id| self.at_end(t, id, false));
                        if arrived || age > MAX_MARCH {
                            let g = t.route[t.goal];
                            self.travels[k].stage = Stage::Hold(PILLAGE);
                            for &id in &alive {
                                if let Some(i) = self.alive(id) {
                                    self.ents[i].home = g;
                                    self.ents[i].path.clear();
                                }
                            }
                            self.msg(&format!("Smoke rises outside {place}: the raiders are burning the farms!"), C_WARN);
                        }
                    }
                    Stage::Hold(left) => {
                        let left = left - dt;
                        if left > 0.0 {
                            self.travels[k].stage = Stage::Hold(left);
                        } else {
                            let goal = self.travels[k].goal;
                            self.travels[k].stage = Stage::Return;
                            self.travels[k].age = 0.0;
                            for &id in &alive {
                                if let Some(i) = self.alive(id) {
                                    self.ents[i].wp = goal;
                                }
                            }
                            let line = match self.sim.sites[site].hit.as_str() {
                                "" => {
                                    format!("Raiders from {name} burned the farms outside {place}.")
                                }
                                l => l.replace("{name}", &name).replace("{town}", &place),
                            };
                            for n in self.sim.raid_hit(site, day, line) {
                                self.msg(&n, C_WARN);
                            }
                        }
                    }
                    Stage::Return => {
                        if alive.iter().all(|&id| self.at_end(t, id, true)) || age > MAX_MARCH {
                            return false;
                        }
                    }
                }
                true
            }
            Kind::Caravan => {
                let Some(&lead) = alive.first() else {
                    return false;
                };
                let li = self.alive(lead).unwrap();
                let town = t.place.clone();
                match stage {
                    Stage::March => {
                        if let Some(at) = t.ambush_at {
                            if self.ents[li].wp >= at {
                                self.travels[k].ambush_at = None;
                                let pos = self.ents[li].tile();
                                let n = 2 + (self.sim.sites[4].strength / 40.0) as usize;
                                let mut foes = vec![];
                                for _ in 0..n {
                                    if let Some(p) = self.free_near(pos, 3, 6) {
                                        let id = self.add_mon("wolf", p, 0);
                                        let i = self.ents.len() - 1;
                                        self.ents[i].home = pos;
                                        foes.push(id);
                                    }
                                }
                                if !foes.is_empty() {
                                    self.travels[k].foes = foes;
                                    self.travels[k].stage = Stage::Hold(BESET);
                                    for &id in &alive {
                                        if let Some(i) = self.alive(id) {
                                            self.ents[i].path.clear();
                                        }
                                    }
                                    self.ents[li].talk("Wolves! Help us!", 0xffffffff);
                                    self.msg(
                                        &format!("Cries on the road to {town}: wolves have fallen on the caravan!"),
                                        C_WARN,
                                    );
                                }
                                return true;
                            }
                        }
                        if self.at_end(t, lead, false) || age > MAX_MARCH {
                            let city = crate::pack::t("Duskhold");
                            let line = format!(
                                "A caravan from {city} came safe to {town}. Goods are plentiful."
                            );
                            for n in self.sim.caravan(true, day, line) {
                                self.msg(&n, C_LORE);
                            }
                            return false;
                        }
                    }
                    Stage::Hold(left) => {
                        let foes: Vec<u32> = t
                            .foes
                            .iter()
                            .copied()
                            .filter(|&id| self.alive(id).is_some())
                            .collect();
                        self.travels[k].foes = foes.clone();
                        if foes.is_empty() {
                            self.travels[k].stage = Stage::March;
                            let near = self.ents[0].dist(self.ents[li].x, self.ents[li].y) < 16.0;
                            if near && !self.pl.ghost {
                                let gold = self.rng.range(40, 90) as i64;
                                self.pl.gold += gold;
                                self.pl.karma = (self.pl.karma + 100).min(15000);
                                self.ents[li].talk(
                                    "Bless thee, friend! Take this for thy trouble.",
                                    0xffffffff,
                                );
                                self.msg(&format!("The merchant gives thee {gold} gold."), C_GOLD);
                                self.news(
                                    "With a stranger's help, the caravan fought off the wolves."
                                        .into(),
                                    C_LORE,
                                );
                            }
                        } else if left - dt > 0.0 {
                            self.travels[k].stage = Stage::Hold(left - dt);
                        } else {
                            let city = crate::pack::t("Duskhold");
                            let line = format!("The caravan from {city} was lost to wolves on the road. Goods are scarce in {town}.");
                            for n in self.sim.caravan(false, day, line) {
                                self.msg(&n, C_WARN);
                            }
                            return false;
                        }
                    }
                    Stage::Return => return false,
                }
                true
            }
        }
    }

    /// A raider fell to the player: counted for the tale of the raid.
    pub(super) fn raider_slain(&mut self, march: u32) {
        if let Some(t) = self.travels.iter_mut().find(|t| t.id == march) {
            t.player_kills += 1;
        }
    }

    /// Journeys under way, for the Chronicle: (icon, what, where it is).
    pub fn travel_rows(&self) -> Vec<(&'static str, String, (f32, f32))> {
        let mut v = vec![];
        for t in &self.travels {
            let Some(i) = t.members.iter().find_map(|&id| self.alive(id)) else {
                continue;
            };
            let at = (self.ents[i].x, self.ents[i].y);
            match t.kind {
                Kind::Raid(k) => {
                    let what = match t.stage {
                        Stage::March => "marching on",
                        Stage::Hold(_) => "burning farms outside",
                        Stage::Return => "going home from",
                    };
                    v.push((
                        "⚔️",
                        format!(
                            "Warband from {}, {} {}",
                            self.sim.sites[k].name, what, t.place
                        ),
                        at,
                    ));
                }
                Kind::Caravan => {
                    let what = match t.stage {
                        Stage::Hold(_) => "beset by wolves",
                        _ => "on the road",
                    };
                    v.push(("🐴", format!("Caravan to {}, {}", t.place, what), at));
                }
            }
        }
        v
    }
}
