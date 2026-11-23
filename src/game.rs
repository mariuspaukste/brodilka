//! Core simulation: entities, player, skills, combat, magic, gathering, AI.
use crate::data::*;
use crate::render::Canvas;
use crate::rng::Rng;
use crate::world::*;
use std::collections::VecDeque;

pub const C_SYS: u32 = 0xe8e0c8ff;
pub const C_GAIN: u32 = 0x8fd8ffff;
pub const C_WARN: u32 = 0xff7a66ff;
pub const C_GOOD: u32 = 0x9cf09cff;
pub const C_GOLD: u32 = 0xffd35aff;
pub const C_SPELL: u32 = 0xa9c8ffff;

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: usize,
    pub qty: u32,
    pub blessed: bool,
    pub exc: bool,
}
impl Item {
    pub fn new(id: usize, qty: u32) -> Item {
        Item {
            id,
            qty,
            blessed: false,
            exc: false,
        }
    }
    pub fn def(&self) -> &'static ItemDef {
        &ITEMS[self.id]
    }
    pub fn label(&self) -> String {
        let d = self.def();
        let mut s = String::new();
        if self.exc {
            s.push_str("exceptional ");
        }
        s.push_str(d.name);
        if self.qty > 1 {
            s = format!("{} {}", self.qty, plural(&s));
        }
        s
    }
    pub fn ar(&self) -> i32 {
        let a = self.def().ar;
        if self.exc {
            a * 6 / 5
        } else {
            a
        }
    }
    pub fn sell_price(&self) -> i32 {
        let p = self.def().price / 2;
        (if self.exc { p * 3 / 2 } else { p }).max(1)
    }
}
pub fn plural(s: &str) -> String {
    if s.ends_with("silk")
        || s.ends_with("ash")
        || s.ends_with("moss")
        || s.ends_with("garlic")
        || s.ends_with("ginseng")
        || s.ends_with("fish")
        || s.ends_with("nightshade")
    {
        s.to_string()
    } else if s.ends_with('y') && !s.ends_with("ey") {
        format!("{}ies", &s[..s.len() - 1])
    } else if s.ends_with("sh") || s.ends_with('s') {
        format!("{s}es")
    } else {
        format!("{s}s")
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EK {
    Player,
    Npc(usize),
    Mon(usize),
}

pub struct Ent {
    pub id: u32,
    pub k: EK,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub path: Vec<(i32, i32)>,
    pub hp: i32,
    pub mhp: i32,
    pub mana: i32,
    pub mmana: i32,
    pub target: u32,
    pub swing: f32,
    pub face: f32,
    pub walk: f32,
    pub atk: f32,
    pub hurt: f32,
    pub say: String,
    pub say_t: f32,
    pub say_col: u32,
    pub home: (i32, i32),
    pub spawner: i32,
    pub ai_t: f32,
    pub cast_t: f32,
    pub breath_t: f32,
    pub regen_t: f32,
    pub dead: bool,
    pub provoked: bool,
    pub look: u32,
    /// facing away from the viewer
    pub back: bool,
    /// 8-way facing in tile space, index into DIRS (3 = towards the viewer)
    pub dir8: u8,
    /// kick animation timer (1 -> 0)
    pub kick: f32,
    /// knocked down: time left lying / getting up, and the total
    pub down_t: f32,
    pub down_total: f32,
    /// facing as drawn: turns one step at a time towards dir8
    pub vdir: u8,
    pub turn_t: f32,
    /// weapon-drawing animation timer (1 -> 0)
    pub ready: f32,
    pub running: bool,
    /// drawn height above level 0, following the ground (in levels)
    pub vz: f32,
}

impl Ent {
    pub fn new(id: u32, k: EK, name: String, x: i32, y: i32, hp: i32) -> Ent {
        Ent {
            id,
            k,
            name,
            x: x as f32,
            y: y as f32,
            path: vec![],
            hp,
            mhp: hp,
            mana: 0,
            mmana: 0,
            target: 0,
            swing: 1.0,
            face: 1.0,
            walk: 0.0,
            atk: 0.0,
            hurt: 0.0,
            say: String::new(),
            say_t: 0.0,
            say_col: 0xffffffff,
            home: (x, y),
            spawner: -1,
            ai_t: 0.0,
            cast_t: 3.0,
            breath_t: 6.0,
            regen_t: 0.0,
            dead: false,
            provoked: false,
            look: 0,
            back: false,
            dir8: 3,
            kick: 0.0,
            down_t: 0.0,
            down_total: 1.0,
            vdir: 3,
            turn_t: 0.0,
            ready: 0.0,
            running: false,
            vz: 0.0,
        }
    }
    pub fn tile(&self) -> (i32, i32) {
        (self.x.round() as i32, self.y.round() as i32)
    }
    pub fn dist(&self, x: f32, y: f32) -> f32 {
        ((self.x - x).powi(2) + (self.y - y).powi(2)).sqrt()
    }
    pub fn talk(&mut self, s: &str, col: u32) {
        self.say = s.to_string();
        self.say_t = 3.0 + s.len() as f32 * 0.04;
        self.say_col = col;
    }
    pub fn mon(&self) -> Option<&'static MonDef> {
        match self.k {
            EK::Mon(m) => Some(&MONS[m]),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    None,
    Gather {
        skill: usize,
        x: i32,
        y: i32,
        t: f32,
    },
    Bandage {
        t: f32,
    },
    Cast {
        sp: usize,
        tgt: u32,
        t: f32,
    },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pending {
    None,
    Talk(u32),
    Loot(u32),
    Gather(usize, i32, i32),
    Station(u8, i32, i32),
    Ankh(i32, i32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Win {
    None,
    Paperdoll,
    Pack,
    Skills,
    Spells,
    Journal,
    Help,
    Map,
    Vendor(u32, bool),
    Bank,
    Loot(u32),
    Craft,
    Res,
    Ctx(u32),
}

pub struct Corpse {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub items: Vec<Item>,
    pub gold: i64,
    pub t: f32,
    pub player: bool,
    pub size: f32,
    pub color: u32,
    /// baked figure to show lying dead, with its facing and age
    pub look: Option<crate::bake::Look>,
    pub dir8: u8,
    pub age: f32,
}

pub struct Fx {
    pub kind: u8,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub tgt: u32,
    pub t: f32,
    pub dur: f32,
    pub dmg: i32,
    pub by_player: bool,
}

pub struct Float {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: u32,
    pub t: f32,
}

pub struct Decal {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub t: f32,
    pub color: u32,
}

pub struct Spawner {
    pub mon: usize,
    pub x: i32,
    pub y: i32,
    pub r: i32,
    pub max: usize,
    pub t: f32,
}

pub struct Pl {
    pub name: String,
    pub female: bool,
    pub hair: u32,
    pub str_: i32,
    pub dex: i32,
    pub int_: i32,
    pub skills: [i32; N_SKILLS],
    pub gold: i64,
    pub bank_gold: i64,
    pub pack: Vec<Item>,
    pub bank: Vec<Item>,
    pub weapon: Option<Item>,
    pub shield: Option<Item>,
    pub armor: Option<Item>,
    pub spells: u32,
    pub karma: i32,
    pub fame: i32,
    pub kills: i32,
    pub war: bool,
    pub ghost: bool,
    pub chase: bool,
    pub action: Action,
    pub pending: Pending,
    pub targeting: i32,
    pub last_spell: i32,
    pub hp_t: f32,
    pub mana_t: f32,
    pub med_t: f32,
    pub potion_cd: f32,
    pub cap_warned: bool,
    pub repath_t: f32,
    pub still_t: f32,
    pub blocking: bool,
    pub kick_cd: f32,
}

impl Pl {
    pub fn new() -> Pl {
        Pl {
            name: String::new(),
            female: false,
            hair: HAIR[0],
            str_: 30,
            dex: 25,
            int_: 25,
            skills: [0; N_SKILLS],
            gold: 0,
            bank_gold: 0,
            pack: vec![],
            bank: vec![],
            weapon: None,
            shield: None,
            armor: None,
            spells: 0,
            karma: 0,
            fame: 0,
            kills: 0,
            war: false,
            ghost: false,
            chase: false,
            action: Action::None,
            pending: Pending::None,
            targeting: -1,
            last_spell: -1,
            hp_t: 0.0,
            mana_t: 0.0,
            med_t: 0.0,
            potion_cd: 0.0,
            cap_warned: false,
            repath_t: 0.0,
            still_t: 0.0,
            blocking: false,
            kick_cd: 0.0,
        }
    }
    pub fn sk(&self, s: usize) -> f32 {
        self.skills[s] as f32 / 10.0
    }
    pub fn total_skill(&self) -> i32 {
        self.skills.iter().sum()
    }
    pub fn mhp(&self) -> i32 {
        30 + self.str_
    }
    pub fn mmana(&self) -> i32 {
        self.int_.max(1)
    }
    pub fn count(&self, id: usize) -> u32 {
        self.pack.iter().filter(|i| i.id == id).map(|i| i.qty).sum()
    }
    pub fn take(&mut self, id: usize, mut n: u32) -> bool {
        if self.count(id) < n {
            return false;
        }
        let mut k = 0;
        while n > 0 && k < self.pack.len() {
            if self.pack[k].id == id {
                let t = self.pack[k].qty.min(n);
                self.pack[k].qty -= t;
                n -= t;
                if self.pack[k].qty == 0 {
                    self.pack.remove(k);
                    continue;
                }
            }
            k += 1;
        }
        true
    }
    pub fn add(&mut self, it: Item) {
        if it.def().stack {
            if let Some(e) = self
                .pack
                .iter_mut()
                .find(|e| e.id == it.id && e.blessed == it.blessed)
            {
                e.qty += it.qty;
                return;
            }
            self.pack.push(it);
        } else {
            for _ in 0..it.qty.max(1) {
                self.pack.push(Item {
                    qty: 1,
                    ..it.clone()
                });
            }
        }
    }
    pub fn weapon_def(&self) -> &'static ItemDef {
        match &self.weapon {
            Some(w) => w.def(),
            None => &ITEMS[0],
        }
    }
    pub fn has_tool(&self, skill: usize) -> bool {
        let s = skill as i32;
        self.weapon
            .as_ref()
            .map(|w| w.def().tool == s)
            .unwrap_or(false)
            || self.pack.iter().any(|i| i.def().tool == s)
    }
    pub fn has_book(&self) -> bool {
        let b = item_idx("spellbook");
        self.pack.iter().any(|i| i.id == b)
    }
    pub fn ar(&self) -> i32 {
        self.armor.as_ref().map(|a| a.ar()).unwrap_or(0)
            + self.shield.as_ref().map(|a| a.ar()).unwrap_or(0)
    }
    pub fn title(&self) -> String {
        let mut best = 0;
        for s in 0..N_SKILLS {
            if self.skills[s] > self.skills[best] {
                best = s;
            }
        }
        let rank = skill_rank(self.skills[best]);
        let karma = match self.karma {
            10000.. => "Glorious ",
            5000.. => "Noble ",
            2500.. => "Honorable ",
            1000.. => "Good ",
            _ => "",
        };
        let lord = if self.fame >= 10000 {
            if self.female {
                "Lady "
            } else {
                "Lord "
            }
        } else {
            ""
        };
        let mut t = String::new();
        if !karma.is_empty() || !lord.is_empty() {
            t = format!("The {}{}", karma, lord);
        }
        t.push_str(&self.name);
        if !rank.is_empty() {
            t.push_str(&format!(", {} {}", rank, SKILL_TITLE[best]));
        }
        t
    }
    pub fn stat_mut(&mut self, s: usize) -> &mut i32 {
        match s {
            0 => &mut self.str_,
            1 => &mut self.dex,
            _ => &mut self.int_,
        }
    }
}

pub struct Game {
    pub rng: Rng,
    pub map: Map,
    pub ents: Vec<Ent>,
    pub pl: Pl,
    pub corpses: Vec<Corpse>,
    pub fx: Vec<Fx>,
    pub floats: Vec<Float>,
    pub decals: Vec<Decal>,
    pub spawners: Vec<Spawner>,
    pub journal: VecDeque<(u32, String, u32)>,
    pub jid: u32,
    pub time: f32,
    pub clock: f32,
    pub created: bool,
    pub win: Win,
    pub hud_ver: u32,
    pub ui_ver: u32,
    pub w: f32,
    pub h: f32,
    pub zoom: f32,
    pub camx: f32,
    pub camy: f32,
    pub next_id: u32,
    pub ptr_down: bool,
    pub ptr_t: f32,
    pub ptr_x0: f32,
    pub ptr_y0: f32,
    pub ptr_x: f32,
    pub ptr_y: f32,
    pub steering: bool,
    pub keys: u8,
    pub cmds: Canvas,
    pub mini: Vec<u8>,
    pub dest: Option<(i32, i32)>,
    pub seed: u32,
    pub tab: u8,
    pub zoom_mul: f32,
    /// CSS pixels per art pixel (the host renders at low resolution)
    pub pix: f32,
    pub sprites: crate::render::SpriteCache,
}

const PLAYER_SPEED: f32 = 4.6;

impl Game {
    pub fn new(seed: u32) -> Game {
        let map = generate(seed);
        let mut g = Game {
            rng: Rng::new(seed as u64 * 7919 + 1),
            ents: vec![],
            pl: Pl::new(),
            corpses: vec![],
            fx: vec![],
            floats: vec![],
            decals: vec![],
            spawners: vec![],
            journal: VecDeque::new(),
            jid: 0,
            time: 60.0,
            clock: 0.0,
            created: false,
            win: Win::None,
            hud_ver: 1,
            ui_ver: 1,
            w: 800.0,
            h: 600.0,
            zoom: 1.0,
            camx: 0.0,
            camy: 0.0,
            next_id: 1,
            ptr_down: false,
            ptr_t: 0.0,
            ptr_x0: 0.0,
            ptr_y0: 0.0,
            ptr_x: 0.0,
            ptr_y: 0.0,
            steering: false,
            keys: 0,
            cmds: Canvas::default(),
            mini: vec![0; N * 4],
            dest: None,
            seed,
            tab: 0,
            zoom_mul: 1.0,
            pix: 1.0,
            sprites: Default::default(),
            map,
        };
        let (sx, sy) = g.map.start;
        let id = g.nid();
        g.ents
            .push(Ent::new(id, EK::Player, "Stranger".into(), sx, sy, 50));
        let npcs = g.map.npcs.clone();
        for (v, x, y) in npcs {
            let id = g.nid();
            let vd = &VENDORS[v];
            let nm = format!("{} {}", g.rng.pick(vd.names), vd.title);
            let mut e = Ent::new(id, EK::Npc(v), nm, x, y, 500);
            e.look = g.rng.next_u32();
            e.ai_t = g.rng.f() * 5.0;
            g.ents.push(e);
        }
        let sp = g.map.spawns.clone();
        for (m, x, y, r, max) in sp {
            g.spawners.push(Spawner {
                mon: mon_idx(m),
                x,
                y,
                r,
                max,
                t: 0.0,
            });
        }
        for si in 0..g.spawners.len() {
            for _ in 0..g.spawners[si].max {
                g.spawn(si);
            }
        }
        g.build_minimap();
        let (px, py) = (g.ents[0].x, g.ents[0].y);
        g.camx = (px - py) * 32.0;
        g.camy = (px + py) * 16.0;
        g
    }

    pub fn nid(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    pub fn build_minimap(&mut self) {
        for i in 0..N {
            let c = mini_color(&self.map, i);
            self.mini[i * 4..i * 4 + 4].copy_from_slice(&c);
        }
    }

    pub fn msg(&mut self, s: &str, col: u32) {
        self.jid += 1;
        self.journal.push_back((self.jid, s.to_string(), col));
        while self.journal.len() > 100 {
            self.journal.pop_front();
        }
        self.hud_ver += 1;
    }
    pub fn touch_ui(&mut self) {
        self.ui_ver += 1;
        self.hud_ver += 1;
    }
    pub fn find(&self, id: u32) -> Option<usize> {
        if id == 0 {
            return None;
        }
        self.ents.iter().position(|e| e.id == id && !e.dead)
    }
    pub fn p(&self) -> &Ent {
        &self.ents[0]
    }
    pub fn pm(&mut self) -> &mut Ent {
        &mut self.ents[0]
    }

    fn spawn(&mut self, si: usize) {
        let s = &self.spawners[si];
        let (mi, sx, sy, r) = (s.mon, s.x, s.y, s.r);
        for _ in 0..30 {
            let x = sx + self.rng.range(-r, r);
            let y = sy + self.rng.range(-r, r);
            if !self.map.passable(x, y) || self.map.in_town(x, y) {
                continue;
            }
            if self.created && self.ents[0].dist(x as f32, y as f32) < 9.0 {
                continue;
            }
            let d = &MONS[mi];
            let id = self.nid();
            let mut e = Ent::new(id, EK::Mon(mi), d.name.to_string(), x, y, d.hp);
            e.home = (sx, sy);
            e.spawner = si as i32;
            e.ai_t = self.rng.f() * 3.0;
            if d.caster.len() > 0 {
                e.mana = 200;
                e.mmana = 200;
            }
            self.ents.push(e);
            return;
        }
    }

    // ------------------------------------------------------------ character
    pub fn create(
        &mut self,
        name: &str,
        female: bool,
        hair: usize,
        tmpl: &str,
        custom: [usize; 3],
    ) {
        let mut pl = Pl::new();
        pl.name = if name.trim().is_empty() {
            "Stranger".to_string()
        } else {
            name.trim().chars().take(16).collect()
        };
        pl.female = female;
        pl.hair = HAIR[hair % HAIR.len()];
        pl.gold = 100;
        let mut items: Vec<(&str, u32)> = vec![];
        if let Some(t) = TEMPLATES.iter().find(|t| t.id == tmpl) {
            for &(s, v) in t.skills.iter() {
                pl.skills[s] = v;
            }
            pl.str_ = t.stats.0;
            pl.dex = t.stats.1;
            pl.int_ = t.stats.2;
            items.extend_from_slice(t.items);
            pl.spells = t.spells;
        } else {
            let vals = [500, 300, 200];
            for (k, &s) in custom.iter().enumerate() {
                if s < N_SKILLS && pl.skills[s] == 0 {
                    pl.skills[s] = vals[k];
                }
            }
            pl.str_ = 30;
            pl.dex = 25;
            pl.int_ = 25;
            items.push(("dagger", 1));
            items.push(("bandage", 10));
            if pl.skills[MAGERY] > 0 {
                items.extend_from_slice(&[
                    ("spellbook", 1),
                    ("sa", 20),
                    ("ga", 10),
                    ("gi", 10),
                    ("ss", 10),
                    ("bp", 5),
                ]);
                pl.spells = 0b11;
                pl.int_ += 10;
                pl.str_ -= 10;
            }
            if pl.skills[MINING] > 0 {
                items.push(("pickaxe", 1));
            }
            if pl.skills[SMITHING] > 0 {
                items.extend_from_slice(&[("smithhammer", 1), ("ingot", 10)]);
            }
            if pl.skills[LUMBER] > 0 {
                items.push(("hatchet", 1));
            }
            if pl.skills[FISHING] > 0 {
                items.push(("fishingpole", 1));
            }
            if pl.skills[MACE] > 0 {
                items.push(("mace", 1));
            }
            if pl.skills[PARRY] > 0 {
                items.push(("buckler", 1));
            }
            items.push(("leather", 1));
        }
        for (id, q) in items {
            let mut it = Item::new(item_idx(id), q);
            it.blessed = true;
            pl.add(it);
        }
        self.pl = pl;
        // auto-equip best starting gear
        let mut k = 0;
        while k < self.pl.pack.len() {
            let d = self.pl.pack[k].def();
            let slot_free = match d.k {
                K::Weapon => {
                    self.pl.weapon.is_none() && (d.tool < 0 || !self.pack_has_plain_weapon())
                }
                K::Armor => self.pl.armor.is_none(),
                K::Shield => self.pl.shield.is_none(),
                _ => false,
            };
            if slot_free {
                self.equip(k);
                continue;
            }
            k += 1;
        }
        let mhp = self.pl.mhp();
        let mm = self.pl.mmana();
        let (sx, sy) = self.map.start;
        let nm = self.pl.name.clone();
        let p = self.pm();
        p.name = nm;
        p.mhp = mhp;
        p.hp = mhp;
        p.mmana = mm;
        p.mana = mm;
        p.x = sx as f32;
        p.y = sy as f32;
        p.path.clear();
        self.created = true;
        self.win = Win::None;
        self.msg("Welcome to the realm, traveller.", C_GOLD);
        self.msg(
            "Tap the ground to walk, hold to steer. Tap creatures to attack.",
            C_SYS,
        );
        self.msg("Skills improve as thou usest them.", C_SYS);
        self.touch_ui();
    }
    fn pack_has_plain_weapon(&self) -> bool {
        self.pl
            .pack
            .iter()
            .any(|i| i.def().k == K::Weapon && i.def().tool < 0)
    }

    pub fn sync_stats(&mut self) {
        let mhp = self.pl.mhp();
        let mm = self.pl.mmana();
        let p = &mut self.ents[0];
        p.mhp = mhp;
        p.mmana = mm;
        p.hp = p.hp.min(mhp);
        p.mana = p.mana.min(mm);
        self.hud_ver += 1;
    }

    // ------------------------------------------------------------ skills
    /// Try to raise a skill. `ok` false means the task was trivial.
    pub fn gain(&mut self, s: usize, ok: bool) {
        if !ok || self.pl.skills[s] >= 1000 {
            return;
        }
        let v = self.pl.skills[s] as f32 / 1000.0;
        let chance = 0.45 * (1.0 - v) + 0.04;
        if !self.rng.chance(chance) {
            return;
        }
        if self.pl.total_skill() >= SKILL_CAP {
            if !self.pl.cap_warned {
                self.pl.cap_warned = true;
                self.msg("Thou hast reached the total skill cap of 700.", C_WARN);
            }
            return;
        }
        self.pl.skills[s] += 1;
        let val = self.pl.skills[s];
        self.msg(
            &format!(
                "Your skill in {} has increased by 0.1. It is now {}.{}.",
                SKILL_NAME[s],
                val / 10,
                val % 10
            ),
            C_GAIN,
        );
        let st = SKILL_STAT[s];
        let total = self.pl.str_ + self.pl.dex + self.pl.int_;
        if self.rng.chance(0.22) && total < STAT_CAP && *self.pl.stat_mut(st) < 100 {
            *self.pl.stat_mut(st) += 1;
            self.msg(
                &format!("Your {} has increased by 1.", STAT_NAME[st]),
                C_GAIN,
            );
            self.sync_stats();
        }
        if self.win == Win::Skills || self.win == Win::Paperdoll {
            self.ui_ver += 1;
        }
    }

    // ------------------------------------------------------------ equipment
    pub fn equip(&mut self, k: usize) {
        if k >= self.pl.pack.len() {
            return;
        }
        let it = self.pl.pack.remove(k);
        match it.def().k {
            K::Weapon => {
                if it.def().two {
                    if let Some(s) = self.pl.shield.take() {
                        self.pl.pack.push(s);
                    }
                }
                if let Some(w) = self.pl.weapon.replace(it) {
                    self.pl.pack.push(w);
                }
            }
            K::Shield => {
                if self
                    .pl
                    .weapon
                    .as_ref()
                    .map(|w| w.def().two)
                    .unwrap_or(false)
                {
                    let w = self.pl.weapon.take().unwrap();
                    self.pl.pack.push(w);
                }
                if let Some(s) = self.pl.shield.replace(it) {
                    self.pl.pack.push(s);
                }
            }
            K::Armor => {
                if let Some(a) = self.pl.armor.replace(it) {
                    self.pl.pack.push(a);
                }
            }
            _ => self.pl.pack.insert(k, it),
        }
        self.touch_ui();
    }
    pub fn unequip(&mut self, slot: &str) {
        let it = match slot {
            "weapon" => self.pl.weapon.take(),
            "shield" => self.pl.shield.take(),
            _ => self.pl.armor.take(),
        };
        if let Some(i) = it {
            self.pl.pack.push(i);
        }
        self.touch_ui();
    }

    // ------------------------------------------------------------ update
    pub fn update(&mut self, dt: f32) {
        self.clock += dt;
        if !self.created {
            return;
        }
        self.time += dt;
        self.map.tick(dt);
        self.player_input(dt);
        self.player_tick(dt);
        let n = self.ents.len();
        for i in 1..n {
            if self.ents[i].dead {
                continue;
            }
            match self.ents[i].k {
                EK::Mon(m) => self.mon_tick(i, m, dt),
                EK::Npc(v) => self.npc_tick(i, v, dt),
                _ => {}
            }
        }
        for i in 0..self.ents.len() {
            let (tx, ty) = self.ents[i].tile();
            let ground = self.map.surface(tx, ty);
            let e = &mut self.ents[i];
            if (ground - e.vz).abs() > 1.2 {
                e.vz = ground; // spawned or moved by a jump in position
            }
            e.vz += (ground - e.vz) * (dt * 9.0).min(1.0);
            e.atk = (e.atk - dt * 3.0).max(0.0);
            e.kick = (e.kick - dt * 2.6).max(0.0);
            e.down_t = (e.down_t - dt).max(0.0);
            e.ready = (e.ready - dt * 2.5).max(0.0);
            // turn through the in-between directions, as Pagan's figures do
            e.turn_t += dt;
            if e.vdir != e.dir8 && e.turn_t > 0.045 {
                e.turn_t = 0.0;
                let diff = (e.dir8 as i32 - e.vdir as i32).rem_euclid(8);
                e.vdir = ((e.vdir as i32 + if diff <= 4 { 1 } else { -1 }).rem_euclid(8)) as u8;
            }
            e.hurt = (e.hurt - dt * 3.0).max(0.0);
            if e.say_t > 0.0 {
                e.say_t -= dt;
            }
        }
        self.fx_tick(dt);
        for f in self.floats.iter_mut() {
            f.t += dt;
        }
        self.floats.retain(|f| f.t < 1.2);
        for d in self.decals.iter_mut() {
            d.t += dt;
        }
        self.decals.retain(|d| d.t < 90.0);
        let before = self.corpses.len();
        for c in self.corpses.iter_mut() {
            c.t -= dt;
            c.age += dt;
            if !c.player && c.items.is_empty() && c.gold == 0 && c.t > 8.0 {
                c.t = 8.0;
            }
        }
        self.corpses.retain(|c| c.t > 0.0);
        if before != self.corpses.len() {
            if let Win::Loot(id) = self.win {
                if !self.corpses.iter().any(|c| c.id == id) {
                    self.win = Win::None;
                    self.touch_ui();
                }
            }
        }
        self.ents.retain(|e| !e.dead || e.k == EK::Player);
        for si in 0..self.spawners.len() {
            self.spawners[si].t -= dt;
            if self.spawners[si].t <= 0.0 {
                self.spawners[si].t = 25.0 + self.rng.f() * 20.0;
                let alive = self.ents.iter().filter(|e| e.spawner == si as i32).count();
                if alive < self.spawners[si].max {
                    self.spawn(si);
                }
            }
        }
        // camera follows player
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let tx = (px - py) * 32.0;
        let ty = (px + py) * 16.0;
        let k = (dt * 10.0).min(1.0);
        self.camx += (tx - self.camx) * k;
        self.camy += (ty - self.camy) * k;
    }

    fn move_ent(&mut self, i: usize, speed: f32, dt: f32) -> bool {
        let e = &mut self.ents[i];
        let Some(&(tx, ty)) = e.path.first() else {
            return false;
        };
        let dx = tx as f32 - e.x;
        let dy = ty as f32 - e.y;
        let d = (dx * dx + dy * dy).sqrt();
        let step = speed * dt;
        let sdx = dx - dy;
        if sdx.abs() > 0.01 {
            e.face = sdx.signum();
        }
        let sdy = dx + dy;
        if sdy.abs() > 0.01 {
            e.back = sdy < 0.0;
        }
        if d > 0.01 {
            e.dir8 = dir8_of(dx, dy);
        }
        e.walk += dt * speed * 2.4;
        if d <= step {
            e.x = tx as f32;
            e.y = ty as f32;
            e.path.remove(0);
        } else {
            e.x += dx / d * step;
            e.y += dy / d * step;
        }
        true
    }

    /// convert a screen-space direction into a tile step
    pub fn screen_dir_to_tile(sx: f32, sy: f32) -> (i32, i32) {
        // inverse iso: tile dx = sx/64 + sy/32 ; dy = sy/32 - sx/64
        let tx = sx / 64.0 + sy / 32.0;
        let ty = sy / 32.0 - sx / 64.0;
        let ang = ty.atan2(tx);
        let oct = ((ang / (std::f32::consts::PI / 4.0)).round() as i32).rem_euclid(8);
        [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ][oct as usize]
    }

    fn player_input(&mut self, dt: f32) {
        // held pointer becomes steering after a moment
        if self.ptr_down {
            self.ptr_t += dt;
            let moved =
                ((self.ptr_x - self.ptr_x0).powi(2) + (self.ptr_y - self.ptr_y0).powi(2)).sqrt();
            if !self.steering && (self.ptr_t > 0.28 || moved > 24.0) && self.pl.targeting < 0 {
                self.steering = true;
            }
        }
        if self.ents[0].down_t > 0.0 || self.pl.blocking {
            return;
        }
        let mut dir: Option<(f32, f32)> = None;
        if self.steering {
            let (px, py) = self.scr_h(self.ents[0].x, self.ents[0].y, self.ents[0].vz);
            let (dx, dy) = (self.ptr_x - px, self.ptr_y - (py - 20.0 * self.zoom));
            if dx * dx + dy * dy > 200.0 {
                dir = Some((dx, dy));
            }
        } else if self.keys != 0 {
            let k = self.keys;
            let mut sx = 0.0;
            let mut sy = 0.0;
            if k & 1 != 0 {
                sy -= 1.0;
            }
            if k & 2 != 0 {
                sx += 1.0;
            }
            if k & 4 != 0 {
                sy += 1.0;
            }
            if k & 8 != 0 {
                sx -= 1.0;
            }
            if sx != 0.0 || sy != 0.0 {
                dir = Some((sx * 2.0, sy));
            }
        }
        if let Some((sx, sy)) = dir {
            self.cancel_activity();
            if self.ents[0].path.len() <= 1 {
                let (dx, dy) = Self::screen_dir_to_tile(sx, sy);
                let (tx, ty) = if let Some(&p) = self.ents[0].path.first() {
                    p
                } else {
                    self.ents[0].tile()
                };
                let cands = [(dx, dy), rot(dx, dy, 1), rot(dx, dy, -1)];
                for (cx, cy) in cands {
                    if self.map.can_step(tx, ty, cx, cy) {
                        let p = self.pm();
                        if p.path.is_empty() {
                            p.path.push((tx + cx, ty + cy));
                        } else if p.path.len() == 1 {
                            p.path.push((tx + cx, ty + cy));
                        }
                        break;
                    }
                }
            }
        }
    }

    pub fn cancel_activity(&mut self) {
        if matches!(self.pl.action, Action::Gather { .. }) {
            self.pl.action = Action::None;
            self.hud_ver += 1;
        }
        self.pl.pending = Pending::None;
        self.pl.chase = false;
        self.dest = None;
    }

    fn player_tick(&mut self, dt: f32) {
        self.pl.kick_cd = (self.pl.kick_cd - dt).max(0.0);
        if self.ents[0].down_t > 0.0 || self.pl.blocking {
            // finish the step in progress, then stand still
            self.ents[0].path.truncate(1);
        }
        // run when heading somewhere far, or steering with the pointer far away
        let far_steer = self.steering && {
            let (px, py) = self.scr_h(self.ents[0].x, self.ents[0].y, self.ents[0].vz);
            ((self.ptr_x - px).powi(2) + (self.ptr_y - py).powi(2)).sqrt() > 140.0
        };
        let run = !self.pl.ghost && !self.pl.blocking && (far_steer || self.ents[0].path.len() > 5);
        self.ents[0].running = run;
        let moving = self.move_ent(
            0,
            if run {
                PLAYER_SPEED * 1.55
            } else {
                PLAYER_SPEED
            },
            dt,
        );
        if moving {
            self.pl.still_t = 0.0;
        } else {
            self.pl.still_t += dt;
        }
        if self.ents[0].path.is_empty() {
            self.dest = None;
        }
        let ghost = self.pl.ghost;
        // regeneration
        if !ghost {
            self.pl.hp_t += dt;
            if self.pl.hp_t > 5.0 {
                self.pl.hp_t = 0.0;
                let p = self.pm();
                if p.hp < p.mhp {
                    p.hp += 1;
                }
                self.hud_ver += 1;
            }
            let med = self.pl.sk(MEDITATION);
            let still = self.pl.still_t > 1.0 && !self.pl.war;
            let rate = (3.2 - med / 40.0) / if still { 2.0 } else { 1.0 };
            self.pl.mana_t += dt;
            if self.pl.mana_t > rate {
                self.pl.mana_t = 0.0;
                let p = self.pm();
                if p.mana < p.mmana {
                    p.mana += 1;
                    self.hud_ver += 1;
                }
            }
            if still && self.ents[0].mana < self.ents[0].mmana {
                self.pl.med_t += dt;
                if self.pl.med_t > 4.0 {
                    self.pl.med_t = 0.0;
                    self.gain(MEDITATION, true);
                }
            }
        }
        self.pl.potion_cd = (self.pl.potion_cd - dt).max(0.0);

        // arrival at pending interaction
        if self.ents[0].path.is_empty() && self.pl.pending != Pending::None {
            let pend = self.pl.pending;
            self.pl.pending = Pending::None;
            self.arrive(pend);
        }

        // actions
        match self.pl.action {
            Action::Gather { skill, x, y, t } => {
                let t = t - dt;
                if t <= 0.0 {
                    self.pl.action = Action::None;
                    self.do_gather(skill, x, y);
                } else {
                    self.pl.action = Action::Gather { skill, x, y, t };
                }
            }
            Action::Bandage { t } => {
                let t = t - dt;
                if t <= 0.0 {
                    self.pl.action = Action::None;
                    self.finish_bandage();
                } else {
                    self.pl.action = Action::Bandage { t };
                }
            }
            Action::Cast { sp, tgt, t } => {
                let t = t - dt;
                if t <= 0.0 {
                    self.pl.action = Action::None;
                    self.finish_cast(sp, tgt);
                } else {
                    self.pl.action = Action::Cast { sp, tgt, t };
                }
            }
            Action::None => {}
        }

        // melee
        let p = &mut self.ents[0];
        p.swing = (p.swing - dt).max(0.0);
        if ghost || !self.pl.war {
            return;
        }
        let tid = self.ents[0].target;
        let Some(ti) = self.find(tid) else {
            if tid != 0 {
                self.ents[0].target = 0;
                self.hud_ver += 1;
            }
            return;
        };
        let (tx, ty) = (self.ents[ti].x, self.ents[ti].y);
        let d = self.ents[0].dist(tx, ty);
        if d > 22.0 {
            self.ents[0].target = 0;
            self.hud_ver += 1;
            return;
        }
        if d <= 1.5 {
            let sdx = (tx - ty) - (self.ents[0].x - self.ents[0].y);
            if sdx.abs() > 0.05 {
                self.ents[0].face = sdx.signum();
            }
            let sdy = (tx + ty) - (self.ents[0].x + self.ents[0].y);
            if sdy.abs() > 0.05 {
                self.ents[0].back = sdy < 0.0;
            }
            let (ddx, ddy) = (tx - self.ents[0].x, ty - self.ents[0].y);
            self.ents[0].dir8 = dir8_of(ddx, ddy);
            if self.ents[0].swing <= 0.0
                && !matches!(self.pl.action, Action::Cast { .. })
                && !self.pl.blocking
                && self.ents[0].down_t <= 0.0
            {
                self.player_swing(ti);
            }
        } else if self.pl.chase && !self.pl.blocking && self.ents[0].down_t <= 0.0 {
            self.pl.repath_t -= dt;
            if self.pl.repath_t <= 0.0 || self.ents[0].path.is_empty() {
                self.pl.repath_t = 0.4;
                let (sx, sy) = self.ents[0]
                    .path
                    .first()
                    .copied()
                    .unwrap_or(self.ents[0].tile());
                let (gx, gy) = (tx.round() as i32, ty.round() as i32);
                if let Some(mut path) = self.map.find_path(sx, sy, gx, gy, 1) {
                    if let Some(&f) = self.ents[0].path.first() {
                        path.insert(0, f);
                    }
                    self.ents[0].path = path;
                } else {
                    self.pl.chase = false;
                    self.msg("Thou canst not reach that.", C_WARN);
                }
            }
        }
    }

    fn player_swing(&mut self, ti: usize) {
        let wd = self.pl.weapon_def();
        let wskill = wd.skill;
        let delay = wd.speed * 140.0 / (self.pl.dex as f32 + 100.0);
        self.ents[0].swing = delay;
        self.ents[0].atk = 1.0;
        let mon = self.ents[ti].mon().unwrap_or(&MONS[0]);
        let atk = self.pl.sk(wskill);
        let def = mon.skill as f32;
        let hit = ((atk + 50.0) / ((def + 50.0) * 2.0)).clamp(0.05, 0.95);
        self.gain(wskill, atk < def + 40.0);
        self.ents[ti].provoked = true;
        if self.ents[ti].target == 0 {
            self.ents[ti].target = self.ents[0].id;
        }
        if !self.rng.chance(hit) {
            let (x, y) = (self.ents[ti].x, self.ents[ti].y);
            self.float(x, y, "miss", 0xc0c0c0ff);
            return;
        }
        self.gain(TACTICS, true);
        let w = self.pl.weapon.as_ref();
        let exc = w.map(|w| w.exc).unwrap_or(false);
        let mut dmg = self.rng.range(wd.dmg.0, wd.dmg.1) as f32;
        dmg *= 0.5 + self.pl.sk(TACTICS) / 100.0;
        dmg *= 1.0 + self.pl.str_ as f32 / 200.0;
        if exc {
            dmg *= 1.2;
        }
        dmg *= 100.0 / (100.0 + mon.ar as f32 * 1.5);
        self.damage(ti, dmg.round().max(1.0) as i32, true);
    }

    /// blood spray and a splat on the ground
    pub fn bleed(&mut self, i: usize, dmg: i32) {
        let (x, y) = (self.ents[i].x, self.ents[i].y);
        let col = blood_color(&self.ents[i]);
        let dir = self.ents[i].face;
        self.fx.push(Fx {
            kind: FX_BLOOD,
            x0: x,
            y0: y,
            x1: x + dir * 0.6,
            y1: y - dir * 0.6,
            tgt: col,
            t: 0.0,
            dur: 0.55,
            dmg: 0,
            by_player: false,
        });
        let r = (0.08 + dmg as f32 * 0.012).min(0.3);
        let (ox, oy) = (self.rng.f() - 0.5, self.rng.f() - 0.5);
        self.decals.push(Decal {
            x: x + ox * 0.7,
            y: y + oy * 0.7,
            r,
            t: 0.0,
            color: col,
        });
        if self.decals.len() > 160 {
            self.decals.remove(0);
        }
        self.ents[i].hurt = 1.0;
    }

    fn knockable(&self, i: usize) -> bool {
        !matches!(
            self.ents[i].mon().map(|m| m.id),
            Some("dragon") | Some("slime") | Some("mongbat")
        )
    }

    /// Fall over, lie a moment, get back up (Pagan's fallBackwards + standUp).
    pub fn knock_down(&mut self, i: usize, t: f32) {
        let e = &mut self.ents[i];
        e.down_t = t;
        e.down_total = t;
        e.path.truncate(1);
        e.atk = 0.0;
        e.kick = 0.0;
        if i == 0 {
            self.pl.action = Action::None;
            self.pl.blocking = false;
            self.touch_ui();
        }
    }

    pub fn set_block(&mut self, on: bool) {
        if self.pl.ghost || self.ents[0].down_t > 0.0 {
            self.pl.blocking = false;
            return;
        }
        if on {
            self.enter_war();
        }
        if on {
            if let Action::Gather { .. } = self.pl.action {
                self.pl.action = Action::None;
            }
        }
        self.pl.blocking = on;
        self.hud_ver += 1;
    }

    /// Pagan's kick: short range, staggers and pushes the foe back, may floor it.
    pub fn kick(&mut self) {
        if self.pl.ghost || self.ents[0].down_t > 0.0 || self.pl.blocking {
            return;
        }
        if self.pl.kick_cd > 0.0 {
            return;
        }
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        // the current target, else the nearest hostile in reach
        let mut ti = self
            .find(self.ents[0].target)
            .filter(|&i| self.ents[i].dist(px, py) <= 1.6);
        if ti.is_none() {
            ti = (1..self.ents.len())
                .filter(|&i| {
                    !self.ents[i].dead
                        && matches!(self.ents[i].k, EK::Mon(_))
                        && self.ents[i].dist(px, py) <= 1.6
                })
                .min_by(|&a, &b| {
                    self.ents[a]
                        .dist(px, py)
                        .partial_cmp(&self.ents[b].dist(px, py))
                        .unwrap()
                });
        }
        self.pl.kick_cd = 1.4;
        self.ents[0].kick = 1.0;
        self.ents[0].swing = self.ents[0].swing.max(0.5);
        let Some(ti) = ti else {
            self.msg("Thou kickest at the air.", C_SYS);
            return;
        };
        self.enter_war();
        self.ents[0].target = self.ents[ti].id;
        let (tx, ty) = (self.ents[ti].x, self.ents[ti].y);
        self.ents[0].dir8 = dir8_of(tx - px, ty - py);
        let mon = self.ents[ti].mon().unwrap_or(&MONS[0]);
        let atk = self.pl.sk(WRESTLING);
        let hit = ((atk + 60.0) / ((mon.skill as f32 + 50.0) * 2.0)).clamp(0.1, 0.95);
        self.gain(WRESTLING, atk < mon.skill as f32 + 40.0);
        if !self.rng.chance(hit) {
            self.float(tx, ty, "miss", 0xc0c0c0ff);
            return;
        }
        let dmg = self.rng.range(2, 6) + self.pl.str_ / 15;
        // shove one tile away if there is room
        let (mx, my) = self.ents[ti].tile();
        let (dx, dy) = (
            (tx - px).round().clamp(-1.0, 1.0) as i32,
            (ty - py).round().clamp(-1.0, 1.0) as i32,
        );
        if (dx != 0 || dy != 0)
            && self.map.can_step(mx, my, dx, dy)
            && !self.occupied(mx + dx, my + dy, ti)
        {
            let e = &mut self.ents[ti];
            e.path.clear();
            e.path.push((mx + dx, my + dy));
        }
        self.ents[ti].swing += 1.2;
        let floored = self.knockable(ti) && self.rng.chance(0.3);
        self.damage(ti, dmg, true);
        if floored && !self.ents[ti].dead && self.ents[ti].down_t <= 0.0 {
            self.knock_down(ti, 1.5);
            let nm = self.ents[ti].name.clone();
            self.msg(&format!("Thy kick sends {} sprawling!", nm), C_GOOD);
        }
    }

    pub fn float(&mut self, x: f32, y: f32, s: &str, col: u32) {
        self.floats.push(Float {
            x,
            y,
            text: s.to_string(),
            color: col,
            t: 0.0,
        });
    }

    /// Damage any entity by index.
    pub fn damage(&mut self, i: usize, dmg: i32, by_player: bool) {
        if self.ents[i].dead {
            return;
        }
        if i == 0 {
            self.damage_player(dmg);
            return;
        }
        let (x, y) = (self.ents[i].x, self.ents[i].y);
        self.float(x, y, &dmg.to_string(), 0xffe060ff);
        self.bleed(i, dmg);
        let pid = self.ents[0].id;
        let e = &mut self.ents[i];
        e.hp -= dmg;
        e.hurt = 1.0;
        if by_player {
            e.provoked = true;
            if e.target == 0 {
                e.target = pid;
            }
        }
        if self.ents[i].hp <= 0 {
            self.kill_mon(i, by_player);
        } else if by_player
            && self.ents[i].down_t <= 0.0
            && dmg * 10 >= self.ents[i].mhp * 3
            && self.knockable(i)
            && self.rng.chance(0.4)
        {
            self.knock_down(i, 1.5);
            let nm = self.ents[i].name.clone();
            self.msg(&format!("{} is knocked down!", cap(&nm)), C_GOOD);
        }
        self.hud_ver += 1;
    }

    fn damage_player(&mut self, mut dmg: i32) {
        if self.pl.ghost {
            return;
        }
        if self.pl.shield.is_some() {
            let parry = self.pl.sk(PARRY);
            self.gain(PARRY, true);
            if self.rng.chance(parry / 100.0 * 0.35) {
                self.msg("You block the attack.", C_SYS);
                dmg /= 3;
            }
        }
        let ar = self.pl.ar() as f32;
        dmg = (dmg as f32 * 100.0 / (100.0 + ar * 1.5))
            .round()
            .max(if dmg > 0 { 1.0 } else { 0.0 }) as i32;
        // being hit interrupts bandaging sometimes, and spells always
        if let Action::Cast { .. } = self.pl.action {
            if self.rng.chance(0.5) {
                self.pl.action = Action::None;
                self.msg(
                    "Your concentration is disturbed, thus ruining thy spell.",
                    C_WARN,
                );
            }
        }
        let (x, y) = (self.ents[0].x, self.ents[0].y);
        self.float(x, y, &dmg.to_string(), 0xff5040ff);
        if dmg > 0 {
            self.bleed(0, dmg);
        }
        let p = self.pm();
        p.hp -= dmg;
        p.hurt = 1.0;
        self.hud_ver += 1;
        if self.ents[0].hp <= 0 {
            self.kill_player();
        } else if !self.pl.blocking
            && self.ents[0].down_t <= 0.0
            && dmg * 100 >= self.ents[0].mhp * 22
            && self.rng.chance(0.5)
        {
            self.knock_down(0, 1.9);
            self.msg("You are knocked off your feet!", C_WARN);
        }
    }

    fn kill_mon(&mut self, i: usize, by_player: bool) {
        let mi = match self.ents[i].k {
            EK::Mon(m) => m,
            _ => return,
        };
        let d = &MONS[mi];
        self.ents[i].dead = true;
        let (x, y) = self.ents[i].tile();
        let mut items = vec![];
        let gold = if d.gold.1 > 0 {
            self.rng.range(d.gold.0, d.gold.1) as i64
        } else {
            0
        };
        if d.regs {
            for _ in 0..self.rng.range(1, 3) {
                let r = *self.rng.pick(&REAGENTS);
                let q = self.rng.range(2, 5) as u32;
                items.push(Item::new(item_idx(r), q));
            }
        }
        for &dr in d.drops.iter() {
            if !self.rng.chance(0.4) {
                continue;
            }
            let id = match dr {
                "gems" => self.rng.pick(&GEMS).to_string(),
                "scroll" => format!("scroll_{}", self.rng.pick(SPELLS).id),
                o => o.to_string(),
            };
            let q = if ITEMS[item_idx(&id)].stack {
                self.rng.range(1, 3) as u32
            } else {
                1
            };
            items.push(Item::new(item_idx(&id), q));
        }
        if d.id == "rabbit" && self.rng.chance(0.5) {
            items.push(Item::new(item_idx("fish"), 1)); // "meat" placeholder: edible
        }
        let bc = blood_color(&self.ents[i]);
        let look = self.look_for(i);
        let dir8 = self.ents[i].dir8;
        self.decals.push(Decal {
            x: x as f32,
            y: y as f32,
            r: 0.35 + d.size * 0.25,
            t: -1.5,
            color: bc,
        });
        let cid = self.nid();
        self.corpses.push(Corpse {
            id: cid,
            x,
            y,
            name: format!("a corpse of {}", d.name),
            items,
            gold,
            t: 240.0,
            player: false,
            size: d.size,
            color: d.color,
            look,
            dir8,
            age: 0.0,
        });
        if by_player {
            self.pl.kills += 1;
            if d.fame > 0 {
                let g = (d.fame as f32 * (1.0 - self.pl.fame as f32 / 15000.0)).max(1.0) as i32;
                self.pl.fame = (self.pl.fame + g).min(15000);
                self.msg("You have gained some fame.", C_GOOD);
            }
            if d.karma < 0 {
                let g = (-d.karma as f32 * (1.0 - self.pl.karma.max(0) as f32 / 15000.0)).max(1.0)
                    as i32;
                self.pl.karma = (self.pl.karma + g).min(15000);
                self.msg("You have gained some karma.", C_GOOD);
            }
        }
        if self.ents[0].target == self.ents[i].id {
            self.ents[0].target = 0;
        }
        self.hud_ver += 1;
    }

    fn kill_player(&mut self) {
        let look = self.look_for(0);
        let dir8 = self.ents[0].dir8;
        let (x, y) = self.ents[0].tile();
        let mut items = vec![];
        let mut keep = vec![];
        for it in self.pl.pack.drain(..) {
            if it.blessed {
                keep.push(it);
            } else {
                items.push(it);
            }
        }
        self.pl.pack = keep;
        for slot in [&mut self.pl.weapon, &mut self.pl.shield, &mut self.pl.armor] {
            if let Some(it) = slot {
                if !it.blessed {
                    items.push(slot.take().unwrap());
                }
            }
        }
        let gold = self.pl.gold;
        self.pl.gold = 0;
        let cid = self.nid();
        let name = format!("the corpse of {}", self.pl.name);
        self.corpses.push(Corpse {
            id: cid,
            x,
            y,
            name,
            items,
            gold,
            t: 3600.0,
            player: true,
            size: 1.0,
            color: 0xc8a080ff,
            look,
            dir8,
            age: 0.0,
        });
        self.pl.ghost = true;
        self.pl.war = false;
        self.pl.action = Action::None;
        self.pl.targeting = -1;
        let p = self.pm();
        p.hp = 0;
        p.mana = 0;
        p.target = 0;
        for e in self.ents.iter_mut() {
            e.target = 0;
        }
        self.msg("You have died.", C_WARN);
        self.msg(
            "Seek a healer in town or an ankh shrine to be resurrected.",
            C_SYS,
        );
        self.win = Win::None;
        self.touch_ui();
    }

    pub fn resurrect(&mut self) {
        if !self.pl.ghost {
            return;
        }
        self.pl.ghost = false;
        let mhp = self.pl.mhp();
        let p = self.pm();
        p.hp = (mhp / 3).max(10);
        let (x, y) = (p.x, p.y);
        self.fx.push(Fx {
            kind: FX_RES,
            x0: x,
            y0: y,
            x1: x,
            y1: y,
            tgt: 0,
            t: 0.0,
            dur: 1.2,
            dmg: 0,
            by_player: true,
        });
        self.msg(
            "You have been resurrected. Return to thy corpse to recover thy belongings.",
            C_GOOD,
        );
        self.win = Win::None;
        self.touch_ui();
    }

    // ------------------------------------------------------------ NPCs
    fn npc_tick(&mut self, i: usize, v: usize, dt: f32) {
        self.move_ent(i, 2.0, dt);
        let e = &mut self.ents[i];
        e.ai_t -= dt;
        if e.ai_t > 0.0 {
            return;
        }
        e.ai_t = 3.0 + self.rng.f() * 6.0;
        let pd = self.ents[0].dist(self.ents[i].x, self.ents[i].y);
        if pd < 6.0 && self.rng.chance(0.3) {
            let line = *self.rng.pick(VENDORS[v].chatter);
            self.ents[i].talk(line, 0xffffffff);
        }
        // wander a little around home
        let (hx, hy) = self.ents[i].home;
        let r = if v == V_GUARD { 4 } else { 1 };
        let (tx, ty) = (hx + self.rng.range(-r, r), hy + self.rng.range(-r, r));
        let (sx, sy) = self.ents[i].tile();
        if self.map.passable(tx, ty) && self.ents[i].path.is_empty() {
            if let Some(p) = self.map.find_path(sx, sy, tx, ty, 0) {
                if p.len() < 8 {
                    self.ents[i].path = p;
                }
            }
        }
    }

    // ------------------------------------------------------------ monsters
    fn occupied(&self, x: i32, y: i32, me: usize) -> bool {
        self.ents.iter().enumerate().any(|(j, e)| {
            j != me
                && !e.dead
                && matches!(e.k, EK::Mon(_))
                && (e.tile() == (x, y) || e.path.first() == Some(&(x, y)))
        })
    }
    fn greedy_step(&self, i: usize, gx: f32, gy: f32, away: bool) -> Option<(i32, i32)> {
        let (x, y) = self.ents[i].tile();
        let cur = ((x as f32 - gx).powi(2) + (y as f32 - gy).powi(2)).sqrt();
        let mut best: Option<((i32, i32), f32)> = None;
        for &(dx, dy) in DIRS.iter() {
            if !self.map.can_step(x, y, dx, dy) || self.occupied(x + dx, y + dy, i) {
                continue;
            }
            let (nx, ny) = (x + dx, y + dy);
            let d = ((nx as f32 - gx).powi(2) + (ny as f32 - gy).powi(2)).sqrt();
            let score = if away { -d } else { d };
            if best.map(|b| score < b.1).unwrap_or(true) {
                best = Some(((nx, ny), score));
            }
        }
        match best {
            Some((p, s)) if (away && -s > cur) || (!away && s < cur) => Some(p),
            _ => None,
        }
    }

    fn mon_tick(&mut self, i: usize, mi: usize, dt: f32) {
        let d = &MONS[mi];
        self.move_ent(i, d.mv, dt);
        {
            let e = &mut self.ents[i];
            e.swing = (e.swing - dt).max(0.0);
            e.cast_t -= dt;
            e.breath_t -= dt;
            e.ai_t -= dt;
            if d.regen > 0 {
                e.regen_t += dt;
                if e.regen_t > 2.0 {
                    e.regen_t = 0.0;
                    e.hp = (e.hp + d.regen).min(e.mhp);
                }
            }
        }
        if self.ents[i].down_t > 0.0 {
            self.ents[i].path.truncate(1);
            return;
        }
        // guards protect the town
        let (mx, my) = self.ents[i].tile();
        if self.map.in_town(mx, my) && !d.passive {
            self.guard_strike(i);
            return;
        }
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let pd = self.ents[i].dist(px, py);
        let ghost = self.pl.ghost;
        let (hx, hy) = self.ents[i].home;
        let home_d = self.ents[i].dist(hx as f32, hy as f32);
        let pid = self.ents[0].id;

        if d.passive {
            if self.ents[i].provoked && pd < 10.0 && !ghost {
                if self.ents[i].path.is_empty() {
                    if let Some(s) = self.greedy_step(i, px, py, true) {
                        self.ents[i].path.push(s);
                    }
                }
                return;
            }
            self.ents[i].provoked = false;
            self.wander(i);
            return;
        }

        // acquire / drop target
        if !ghost
            && self.ents[i].target == 0
            && pd < 7.0
            && !self.map.in_town(px.round() as i32, py.round() as i32)
        {
            self.ents[i].target = pid;
        }
        if self.ents[i].target != 0 && (ghost || home_d > 22.0 || pd > 16.0) {
            self.ents[i].target = 0;
            self.ents[i].provoked = false;
        }
        if self.ents[i].target != pid {
            if home_d > 12.0 && self.ents[i].path.is_empty() {
                if let Some(s) = self.greedy_step(i, hx as f32, hy as f32, false) {
                    self.ents[i].path.push(s);
                }
                let e = &mut self.ents[i];
                e.hp = (e.hp + 1).min(e.mhp);
            } else {
                self.wander(i);
            }
            return;
        }
        // face target
        let sdx = (px - py) - (self.ents[i].x - self.ents[i].y);
        if sdx.abs() > 0.05 {
            self.ents[i].face = sdx.signum();
        }
        let sdy = (px + py) - (self.ents[i].x + self.ents[i].y);
        if sdy.abs() > 0.05 {
            self.ents[i].back = sdy < 0.0;
        }
        let (ddx, ddy) = (px - self.ents[i].x, py - self.ents[i].y);
        if ddx.abs() + ddy.abs() > 0.05 {
            self.ents[i].dir8 = dir8_of(ddx, ddy);
        }
        // dragon breath
        if d.breath && pd < 6.0 && self.ents[i].breath_t <= 0.0 {
            self.ents[i].breath_t = 9.0 + self.rng.f() * 5.0;
            self.ents[i].talk("*breathes fire*", 0xff9040ff);
            let (x0, y0) = (self.ents[i].x, self.ents[i].y);
            let dmg = self.rng.range(20, 34);
            self.fx.push(Fx {
                kind: FX_BREATH,
                x0,
                y0,
                x1: px,
                y1: py,
                tgt: pid,
                t: 0.0,
                dur: 0.7,
                dmg,
                by_player: false,
            });
            return;
        }
        // spellcasting
        if !d.caster.is_empty() && pd < 10.0 && pd > 1.2 && self.ents[i].cast_t <= 0.0 {
            self.ents[i].cast_t = 3.5 + self.rng.f() * 3.0;
            let sp = *self.rng.pick(d.caster);
            let s = &SPELLS[sp];
            self.ents[i].talk(s.words, C_SPELL);
            let dmg = (self.rng.range(s.amt.0, s.amt.1) as f32
                * (0.7 + d.magery as f32 / 1000.0 * 0.5)) as i32;
            self.spell_fx(s.fx, i, 0, dmg, false);
            return;
        }
        if pd > 1.5 {
            if self.ents[i].path.is_empty() {
                if let Some(s) = self.greedy_step(i, px, py, false) {
                    self.ents[i].path.push(s);
                } else if self.ents[i].ai_t <= 0.0 {
                    // try a proper path around obstacles
                    self.ents[i].ai_t = 1.0;
                    let (sx, sy) = self.ents[i].tile();
                    if let Some(mut p) =
                        self.map
                            .find_path(sx, sy, px.round() as i32, py.round() as i32, 1)
                    {
                        p.truncate(4);
                        self.ents[i].path = p;
                    }
                }
            }
        } else if self.ents[i].swing <= 0.0 {
            self.ents[i].swing = d.speed;
            self.ents[i].atk = 1.0;
            let def = self.pl.sk(self.pl.weapon_def().skill);
            let hit = ((d.skill as f32 + 50.0) / ((def + 50.0) * 2.0)).clamp(0.05, 0.95);
            let wsk = self.pl.weapon_def().skill;
            self.gain(wsk, def < d.skill as f32 + 30.0);
            if self.rng.chance(hit) {
                let mut dmg = self.rng.range(d.dmg.0, d.dmg.1);
                if self.pl.blocking && self.ents[0].down_t <= 0.0 {
                    let parry = self.pl.sk(PARRY);
                    let shield = if self.pl.shield.is_some() { 0.15 } else { 0.0 };
                    self.gain(PARRY, true);
                    if self.rng.chance(0.5 + parry / 250.0 + shield) {
                        dmg /= 5;
                        self.msg("You block the blow.", C_SYS);
                        let (x, y) = (self.ents[0].x, self.ents[0].y);
                        self.fx.push(Fx {
                            kind: FX_SPARK,
                            x0: x,
                            y0: y,
                            x1: x,
                            y1: y,
                            tgt: 0,
                            t: 0.0,
                            dur: 0.35,
                            dmg: 0,
                            by_player: false,
                        });
                    }
                }
                self.damage(0, dmg, false);
            } else {
                self.float(px, py, "miss", 0xc0c0c0ff);
            }
        }
    }

    fn wander(&mut self, i: usize) {
        if self.ents[i].ai_t > 0.0 || !self.ents[i].path.is_empty() {
            return;
        }
        self.ents[i].ai_t = 1.5 + self.rng.f() * 4.0;
        let (hx, hy) = self.ents[i].home;
        let r = if let EK::Mon(m) = self.ents[i].k {
            self.spawners
                .get(self.ents[i].spawner as usize)
                .map(|s| s.r)
                .unwrap_or(5)
                .max(2)
                + if MONS[m].id == "dragon" { 0 } else { 2 }
        } else {
            3
        };
        let (x, y) = self.ents[i].tile();
        let (dx, dy) = *self.rng.pick(&DIRS);
        let (nx, ny) = (x + dx, y + dy);
        if (nx - hx).abs() <= r
            && (ny - hy).abs() <= r
            && self.map.can_step(x, y, dx, dy)
            && !self.map.in_town(nx, ny)
            && !self.occupied(nx, ny, i)
        {
            self.ents[i].path.push((nx, ny));
        }
    }

    fn guard_strike(&mut self, i: usize) {
        let (mx, my) = (self.ents[i].x, self.ents[i].y);
        if let Some(gi) = self
            .ents
            .iter()
            .position(|e| e.k == EK::Npc(V_GUARD) && !e.dead)
        {
            let g = &mut self.ents[gi];
            g.x = mx.round() + 1.0;
            g.y = my.round();
            g.path.clear();
            g.atk = 1.0;
            g.face = -1.0;
            g.talk("Halt, foul creature!", 0xffffffff);
        }
        self.fx.push(Fx {
            kind: FX_LIGHTNING,
            x0: mx,
            y0: my,
            x1: mx,
            y1: my,
            tgt: 0,
            t: 0.0,
            dur: 0.4,
            dmg: 0,
            by_player: false,
        });
        let hp = self.ents[i].hp;
        self.damage(i, hp + 1, false);
        if self.ents[i].dead {
            if let Some(c) = self.corpses.last_mut() {
                c.items.clear();
                c.gold = 0;
            }
        }
    }

    // ------------------------------------------------------------ effects
    pub fn spell_fx(&mut self, kind: u8, from: usize, to: usize, dmg: i32, by_player: bool) {
        let (x0, y0) = (self.ents[from].x, self.ents[from].y);
        let (x1, y1) = (self.ents[to].x, self.ents[to].y);
        let tgt = self.ents[to].id;
        let dist = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let dur = match kind {
            FX_ARROW | FX_FIREBALL | FX_EBOLT => 0.12 + dist * 0.05,
            FX_LIGHTNING => 0.35,
            FX_EXPLOSION => 0.6,
            FX_FLAMESTRIKE => 0.7,
            FX_HARM => 0.4,
            _ => 0.5,
        };
        self.fx.push(Fx {
            kind,
            x0,
            y0,
            x1,
            y1,
            tgt,
            t: 0.0,
            dur,
            dmg,
            by_player,
        });
    }

    fn fx_tick(&mut self, dt: f32) {
        let mut hits: Vec<(u32, i32, bool, u8)> = vec![];
        for f in self.fx.iter_mut() {
            let before = f.t;
            f.t += dt;
            // projectiles follow their target
            let hit_at = match f.kind {
                FX_ARROW | FX_FIREBALL | FX_EBOLT | FX_BREATH => f.dur,
                FX_EXPLOSION => f.dur * 0.3,
                _ => f.dur * 0.2,
            };
            if before < hit_at && f.t >= hit_at && f.dmg > 0 {
                hits.push((f.tgt, f.dmg, f.by_player, f.kind));
            }
        }
        for f in self.fx.iter_mut() {
            if let Some(e) = self.ents.iter().find(|e| e.id == f.tgt && !e.dead) {
                f.x1 = e.x;
                f.y1 = e.y;
            }
        }
        self.fx.retain(|f| f.t < f.dur);
        for (tgt, dmg, byp, kind) in hits {
            if let Some(i) = self.ents.iter().position(|e| e.id == tgt && !e.dead) {
                if i == 0 && self.pl.ghost {
                    continue;
                }
                if kind == FX_BREATH && i == 0 {
                    self.msg("You are engulfed in dragon fire!", C_WARN);
                }
                self.damage(i, dmg, byp);
            }
        }
    }

    // ------------------------------------------------------------ magic
    pub fn begin_cast(&mut self, sp: usize) {
        if self.pl.ghost {
            self.msg("Thou art a ghost and cannot cast spells.", C_WARN);
            return;
        }
        if !self.pl.has_book() {
            self.msg("Thou hast no spellbook.", C_WARN);
            return;
        }
        if self.pl.spells & (1 << sp) == 0 {
            self.msg("Thou dost not know that spell.", C_WARN);
            return;
        }
        if matches!(self.pl.action, Action::Cast { .. }) {
            self.msg("Thou art already casting a spell.", C_WARN);
            return;
        }
        let s = &SPELLS[sp];
        if self.ents[0].mana < CIRCLE_MANA[s.circle] {
            self.msg("Insufficient mana for this spell.", C_WARN);
            return;
        }
        for r in s.regs {
            if self.pl.count(item_idx(r)) == 0 {
                self.msg("More reagents are needed for this spell.", C_WARN);
                return;
            }
        }
        self.pl.last_spell = sp as i32;
        self.win = Win::None;
        if s.hostile {
            // auto-target current opponent if valid, else ask for a target
            let t = self.ents[0].target;
            if let Some(ti) = self.find(t) {
                if self.ents[ti].dist(self.ents[0].x, self.ents[0].y) <= s.range {
                    self.start_cast(sp, t);
                    self.touch_ui();
                    return;
                }
            }
            self.pl.targeting = sp as i32;
            self.msg("Select a target.", C_SYS);
        } else {
            let id = self.ents[0].id;
            self.start_cast(sp, id);
        }
        self.touch_ui();
    }

    pub fn start_cast(&mut self, sp: usize, tgt: u32) {
        let s = &SPELLS[sp];
        self.pl.targeting = -1;
        self.pl.action = Action::Cast {
            sp,
            tgt,
            t: 0.4 + s.circle as f32 * 0.25,
        };
        self.ents[0].talk(s.words, C_SPELL);
        self.ents[0].atk = 0.6;
        if s.hostile && tgt != 0 {
            self.ents[0].target = tgt;
        }
        self.hud_ver += 1;
    }

    fn finish_cast(&mut self, sp: usize, tgt: u32) {
        let s = &SPELLS[sp];
        let Some(ti) = self.find(tgt) else {
            self.msg("The target is gone.", C_WARN);
            return;
        };
        if self.ents[ti].dist(self.ents[0].x, self.ents[0].y) > s.range + 1.0 {
            self.msg("That is too far away.", C_WARN);
            return;
        }
        let mana = CIRCLE_MANA[s.circle];
        if self.ents[0].mana < mana {
            self.msg("Insufficient mana for this spell.", C_WARN);
            return;
        }
        for r in s.regs {
            if !self.pl.take(item_idx(r), 1) {
                self.msg("More reagents are needed for this spell.", C_WARN);
                return;
            }
        }
        let min = CIRCLE_MIN[s.circle];
        let mag = self.pl.skills[MAGERY];
        let chance = ((mag - min + 200) as f32 / 400.0).clamp(0.0, 1.0);
        self.gain(MAGERY, mag < min + 450);
        if !self.rng.chance(chance) {
            self.pm().mana -= mana / 2;
            let (x, y) = (self.ents[0].x, self.ents[0].y);
            self.fx.push(Fx {
                kind: FX_FIZZLE,
                x0: x,
                y0: y,
                x1: x,
                y1: y,
                tgt: 0,
                t: 0.0,
                dur: 0.6,
                dmg: 0,
                by_player: true,
            });
            self.msg("The spell fizzles.", C_WARN);
            self.hud_ver += 1;
            return;
        }
        self.pm().mana -= mana;
        let scale = 0.7 + self.pl.sk(MAGERY) / 100.0 * 0.6;
        let amt = (self.rng.range(s.amt.0, s.amt.1) as f32 * scale).round() as i32;
        if s.hostile {
            if ti == 0 {
                self.msg("Thou canst not target thyself with that.", C_WARN);
                return;
            }
            if let EK::Npc(_) = self.ents[ti].k {
                self.msg("Thou canst not harm the townsfolk.", C_WARN);
                return;
            }
            self.enter_war();
            self.spell_fx(s.fx, 0, ti, amt, true);
        } else {
            let e = &mut self.ents[ti];
            let healed = amt.min(e.mhp - e.hp);
            e.hp += healed;
            let (x, y) = (e.x, e.y);
            self.fx.push(Fx {
                kind: FX_HEAL,
                x0: x,
                y0: y,
                x1: x,
                y1: y,
                tgt: 0,
                t: 0.0,
                dur: 0.8,
                dmg: 0,
                by_player: true,
            });
            self.float(x, y, &format!("+{}", healed), 0x70ff90ff);
        }
        self.hud_ver += 1;
    }

    // ------------------------------------------------------------ healing
    pub fn bandage(&mut self) {
        if self.pl.ghost {
            return;
        }
        if matches!(self.pl.action, Action::Bandage { .. }) {
            self.msg("Thou art already applying a bandage.", C_WARN);
            return;
        }
        if self.ents[0].hp >= self.ents[0].mhp {
            self.msg("Thou art not hurt.", C_SYS);
            return;
        }
        if !self.pl.take(item_idx("bandage"), 1) {
            self.msg("Thou hast no bandages.", C_WARN);
            return;
        }
        let t = (4.5 - self.pl.dex as f32 / 40.0).max(2.0);
        self.pl.action = Action::Bandage { t };
        self.msg("You begin applying the bandages.", C_SYS);
        self.touch_ui();
    }
    fn finish_bandage(&mut self) {
        let s = self.pl.sk(HEALING);
        self.gain(HEALING, true);
        if !self.rng.chance(0.55 + s / 220.0) {
            self.msg("Your fingers slip!", C_WARN);
            return;
        }
        let amt = self
            .rng
            .range((s / 5.0 + 3.0) as i32, (s / 2.5 + 6.0) as i32);
        let p = self.pm();
        let healed = amt.min(p.mhp - p.hp);
        p.hp += healed;
        let (x, y) = (p.x, p.y);
        self.float(x, y, &format!("+{}", healed), 0x70ff90ff);
        self.msg("You finish applying the bandages.", C_GOOD);
        self.hud_ver += 1;
    }
    pub fn drink_potion(&mut self) {
        if self.pl.ghost {
            return;
        }
        if self.pl.potion_cd > 0.0 {
            self.msg("Thou must wait before drinking another potion.", C_WARN);
            return;
        }
        if !self.pl.take(item_idx("healpotion"), 1) {
            self.msg("Thou hast no heal potions.", C_WARN);
            return;
        }
        self.pl.potion_cd = 8.0;
        let amt = self.rng.range(18, 30);
        let p = self.pm();
        let healed = amt.min(p.mhp - p.hp);
        p.hp += healed;
        let (x, y) = (p.x, p.y);
        self.float(x, y, &format!("+{}", healed), 0x70ff90ff);
        self.touch_ui();
    }

    // ------------------------------------------------------------ interaction
    pub fn arrive(&mut self, pend: Pending) {
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        match pend {
            Pending::Talk(id) => {
                if let Some(i) = self.find(id) {
                    if self.ents[i].dist(px, py) <= 3.0 {
                        self.win = Win::Ctx(id);
                        self.touch_ui();
                    }
                }
            }
            Pending::Loot(id) => {
                if let Some(c) = self.corpses.iter().find(|c| c.id == id) {
                    if ((c.x as f32 - px).abs()).max((c.y as f32 - py).abs()) <= 1.6 {
                        self.win = Win::Loot(id);
                        self.touch_ui();
                    }
                }
            }
            Pending::Gather(s, x, y) => self.start_gather(s, x, y),
            Pending::Station(o, _, _) => {
                if o == O_ANVIL {
                    self.win = Win::Craft;
                } else {
                    self.smelt();
                }
                self.touch_ui();
            }
            Pending::Ankh(_, _) => {
                if self.pl.ghost {
                    self.win = Win::Res;
                    self.touch_ui();
                } else {
                    self.msg("Thou feelest at peace near the shrine.", C_SYS);
                }
            }
            Pending::None => {}
        }
    }

    pub fn walk_to(&mut self, x: i32, y: i32, range: i32) -> bool {
        let (sx, sy) = self.ents[0]
            .path
            .first()
            .copied()
            .unwrap_or(self.ents[0].tile());
        if let Some(mut path) = self.map.find_path(sx, sy, x, y, range) {
            if let Some(&f) = self.ents[0].path.first() {
                path.insert(0, f);
            }
            self.dest = path.last().copied();
            self.ents[0].path = path;
            true
        } else {
            false
        }
    }

    pub fn start_gather(&mut self, skill: usize, x: i32, y: i32) {
        if self.pl.ghost {
            return;
        }
        if !self.pl.has_tool(skill) {
            let tool = match skill {
                MINING => "a pickaxe",
                LUMBER => "a hatchet",
                _ => "a fishing pole",
            };
            self.msg(&format!("Thou needest {} for that.", tool), C_WARN);
            return;
        }
        let i = idx(x, y);
        if skill != FISHING && self.map.res[i] == 0 {
            let s = if skill == MINING {
                "There is no metal here to mine."
            } else {
                "There's not enough wood here to harvest."
            };
            self.msg(s, C_WARN);
            return;
        }
        let t = if skill == FISHING { 3.0 } else { 1.8 };
        self.pl.action = Action::Gather { skill, x, y, t };
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let sdx = (x - y) as f32 - (px - py);
        if sdx.abs() > 0.05 {
            self.ents[0].face = sdx.signum();
        }
        self.ents[0].dir8 = dir8_of(x as f32 - px, y as f32 - py);
        self.ents[0].atk = 1.0;
        self.hud_ver += 1;
    }

    fn do_gather(&mut self, skill: usize, x: i32, y: i32) {
        let s = self.pl.sk(skill);
        let chance = ((s + 35.0) / 100.0).clamp(0.15, 0.98);
        self.gain(skill, true);
        let i = idx(x, y);
        let ok = self.rng.chance(chance);
        match skill {
            MINING => {
                if ok {
                    let n = self.rng.range(1, 2) as u32;
                    self.pl.add(Item::new(item_idx("ore"), n));
                    self.map.res[i] = self.map.res[i].saturating_sub(n as u8);
                    self.msg(
                        &format!(
                            "You dig some iron ore and put it in your backpack. ({})",
                            self.pl.count(item_idx("ore"))
                        ),
                        C_SYS,
                    );
                    if s >= 50.0 && self.rng.chance(0.02) {
                        let g = *self.rng.pick(&GEMS);
                        self.pl.add(Item::new(item_idx(g), 1));
                        self.msg(&format!("You found a {}!", ITEMS[item_idx(g)].name), C_GOLD);
                    }
                } else {
                    self.msg(
                        "You loosen some rocks but fail to find any usable ore.",
                        C_SYS,
                    );
                }
            }
            LUMBER => {
                if ok {
                    let n = self.rng.range(1, 3) as u32;
                    self.pl.add(Item::new(item_idx("logs"), n));
                    self.map.res[i] = self.map.res[i].saturating_sub(n as u8);
                    self.msg(
                        &format!(
                            "You put some logs into your backpack. ({})",
                            self.pl.count(item_idx("logs"))
                        ),
                        C_SYS,
                    );
                } else {
                    self.msg(
                        "You hack at the tree for a while, but fail to produce any usable wood.",
                        C_SYS,
                    );
                }
            }
            _ => {
                if ok {
                    if s >= 40.0 && self.rng.chance(0.015) {
                        let g = *self.rng.pick(&GEMS);
                        self.pl.add(Item::new(item_idx(g), 1));
                        self.msg(
                            &format!("You fish up a shimmering {}!", ITEMS[item_idx(g)].name),
                            C_GOLD,
                        );
                    } else {
                        self.pl.add(Item::new(item_idx("fish"), 1));
                        self.msg("You pull out a nice fish!", C_SYS);
                    }
                } else {
                    self.msg("You fish a while, but fail to catch anything.", C_SYS);
                }
            }
        }
        if skill != FISHING && self.map.res[i] == 0 {
            self.map.regrow.push((i, 300.0));
            let s = if skill == MINING {
                "There is no metal here to mine."
            } else {
                "There's not enough wood here to harvest."
            };
            self.msg(s, C_SYS);
        } else {
            // keep working automatically
            self.start_gather(skill, x, y);
        }
        if self.win == Win::Pack {
            self.ui_ver += 1;
        }
    }

    pub fn near_obj(&self, o: u8, r: i32) -> bool {
        let (x, y) = self.ents[0].tile();
        for yy in y - r..=y + r {
            for xx in x - r..=x + r {
                if self.map.obj(xx, yy) == o {
                    return true;
                }
            }
        }
        false
    }

    pub fn smelt(&mut self) {
        if !self.near_obj(O_FORGE, 2) {
            self.msg("Thou must be near a forge to smelt ore.", C_WARN);
            return;
        }
        let ore = self.pl.count(item_idx("ore"));
        if ore == 0 {
            self.msg("Thou hast no ore to smelt.", C_WARN);
            return;
        }
        self.pl.take(item_idx("ore"), ore);
        let mut ingots = 0;
        for k in 0..ore {
            let s = self.pl.sk(MINING);
            if self.rng.chance(((s + 50.0) / 100.0).min(1.0)) {
                ingots += 1;
            }
            if k < 6 {
                self.gain(MINING, true);
            }
        }
        if ingots > 0 {
            self.pl.add(Item::new(item_idx("ingot"), ingots));
        }
        self.msg(
            &format!("You smelt {} ore into {} iron ingots.", ore, ingots),
            C_SYS,
        );
        self.touch_ui();
    }

    pub fn craft(&mut self, id: &str) {
        let Some(&(_, ing, min)) = RECIPES.iter().find(|r| r.0 == id) else {
            return;
        };
        if !self.near_obj(O_ANVIL, 2) {
            self.msg("Thou must be near an anvil to do that.", C_WARN);
            return;
        }
        if !self.pl.has_tool(SMITHING) {
            self.msg("Thou needest a smith's hammer.", C_WARN);
            return;
        }
        let ingot = item_idx("ingot");
        if self.pl.count(ingot) < ing {
            self.msg("Thou dost not have enough iron ingots.", C_WARN);
            return;
        }
        let sk = self.pl.skills[SMITHING];
        let chance = ((sk - min + 250) as f32 / 500.0).clamp(0.0, 1.0);
        self.gain(SMITHING, sk < min + 500);
        if !self.rng.chance(chance) {
            self.pl.take(ingot, ing / 2);
            self.msg(
                "You failed to create the item, and some of your materials are lost.",
                C_WARN,
            );
            self.touch_ui();
            return;
        }
        self.pl.take(ingot, ing);
        let exc = self
            .rng
            .chance(((sk - min - 300) as f32 / 1000.0).clamp(0.0, 0.4));
        let mut it = Item::new(item_idx(id), 1);
        it.exc = exc;
        let label = it.label();
        self.pl.add(it);
        if exc {
            self.msg(
                &format!(
                    "You create an exceptional quality {}.",
                    ITEMS[item_idx(id)].name
                ),
                C_GOLD,
            );
        } else {
            self.msg(
                &format!(
                    "You create the item and put it in your backpack: {}.",
                    label
                ),
                C_SYS,
            );
        }
        self.touch_ui();
    }

    pub fn use_item(&mut self, k: usize) {
        if k >= self.pl.pack.len() {
            return;
        }
        let d = self.pl.pack[k].def();
        match d.k {
            K::Weapon | K::Armor | K::Shield => self.equip(k),
            K::Book => {
                self.win = Win::Spells;
                self.touch_ui();
            }
            K::Scroll => {
                if !self.pl.has_book() {
                    self.msg("Thou needest a spellbook to inscribe this scroll.", C_WARN);
                    return;
                }
                let sp = d.spell as usize;
                if self.pl.spells & (1 << sp) != 0 {
                    self.msg("That spell is already in thy spellbook.", C_WARN);
                    return;
                }
                self.pl.take(self.pl.pack[k].id, 1);
                self.pl.spells |= 1 << sp;
                self.msg(
                    &format!("{} has been added to thy spellbook.", SPELLS[sp].name),
                    C_GOOD,
                );
                self.touch_ui();
            }
            K::Consumable => match d.id {
                "bandage" => self.bandage(),
                "healpotion" => self.drink_potion(),
                "fish" => {
                    let id = self.pl.pack[k].id;
                    self.pl.take(id, 1);
                    let p = self.pm();
                    p.hp = (p.hp + 3).min(p.mhp);
                    self.msg("You eat the fish. Not bad, if a little raw.", C_SYS);
                    self.touch_ui();
                }
                _ => {}
            },
            K::Tool => {
                let s = match d.tool as usize {
                    FISHING => "Tap the water nearby to fish.",
                    SMITHING => "Tap an anvil to craft, or a forge to smelt ore.",
                    _ => "Tap something to use this on.",
                };
                self.msg(s, C_SYS);
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ screen helpers
    pub fn to_screen(&self, x: f32, y: f32) -> (f32, f32) {
        let ix = (x - y) * 32.0;
        let iy = (x + y) * 16.0;
        (
            (ix - self.camx) * self.zoom + self.w / 2.0,
            (iy - self.camy) * self.zoom + self.h / 2.0,
        )
    }
    /// screen position of a point standing `h` levels above level 0
    pub fn scr_h(&self, x: f32, y: f32, h: f32) -> (f32, f32) {
        let (a, b) = self.to_screen(x, y);
        (a, b - h * crate::render::LH * self.zoom)
    }
    /// the ground point under a screen position, allowing for raised ground
    pub fn to_world_ground(&self, sx: f32, sy: f32) -> (f32, f32) {
        for h in (0..=4).rev() {
            let (wx, wy) = self.to_world(sx, sy + h as f32 * crate::render::LH * self.zoom);
            let (tx, ty) = (wx.round() as i32, wy.round() as i32);
            if (self.map.surface(tx, ty) - h as f32).abs() < 0.6 {
                return (wx, wy);
            }
        }
        self.to_world(sx, sy)
    }
    pub fn to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        let ix = (sx - self.w / 2.0) / self.zoom + self.camx;
        let iy = (sy - self.h / 2.0) / self.zoom + self.camy;
        ((ix / 32.0 + iy / 16.0) / 2.0, (iy / 16.0 - ix / 32.0) / 2.0)
    }

    /// handle a tap at screen coords
    pub fn tap(&mut self, sx: f32, sy: f32) {
        if !self.created {
            return;
        }
        // entity hit test
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in self.ents.iter().enumerate() {
            if e.dead {
                continue;
            }
            let size = e.mon().map(|m| m.size).unwrap_or(1.0);
            let (ex, ey) = self.scr_h(e.x, e.y, e.vz);
            let cy = ey - 20.0 * size * self.zoom;
            let rw = (16.0 * size + 8.0) * self.zoom;
            let rh = (24.0 * size + 8.0) * self.zoom;
            let (dx, dy) = ((sx - ex).abs(), (sy - cy).abs());
            if dx < rw && dy < rh {
                let d = dx / rw + dy / rh + if i == 0 { 0.6 } else { 0.0 };
                if best.map(|b| d < b.1).unwrap_or(true) {
                    best = Some((i, d));
                }
            }
        }
        // spell targeting
        if self.pl.targeting >= 0 {
            let sp = self.pl.targeting as usize;
            if let Some((i, _)) = best {
                let id = self.ents[i].id;
                if SPELLS[sp].hostile && i == 0 {
                    self.msg("Thou canst not target thyself with that.", C_WARN);
                    return;
                }
                self.start_cast(sp, id);
            } else {
                self.msg("Target cancelled.", C_SYS);
                self.pl.targeting = -1;
            }
            self.touch_ui();
            return;
        }
        if let Some((i, _)) = best {
            let id = self.ents[i].id;
            match self.ents[i].k {
                EK::Player => {
                    self.win = Win::Paperdoll;
                    self.touch_ui();
                }
                EK::Npc(v) => {
                    self.cancel_activity();
                    let (x, y) = self.ents[i].tile();
                    if self.ents[i].dist(self.ents[0].x, self.ents[0].y) <= 3.0 {
                        self.ents[0].path.truncate(1);
                        self.pl.pending = Pending::Talk(id);
                    } else if self.walk_to(x, y, 2) {
                        self.pl.pending = Pending::Talk(id);
                    }
                    if v == V_GUARD {
                        let line = *self.rng.pick(VENDORS[v].chatter);
                        self.ents[i].talk(line, 0xffffffff);
                    }
                }
                EK::Mon(_) => {
                    if self.pl.ghost {
                        self.msg("Thou art dead and cannot fight.", C_WARN);
                        return;
                    }
                    self.cancel_activity();
                    self.enter_war();
                    self.ents[0].target = id;
                    self.pl.chase = true;
                    self.pl.repath_t = 0.0;
                    let nm = self.ents[i].name.clone();
                    self.ents[i].talk(&nm, 0xff8080ff);
                    self.touch_ui();
                }
            }
            return;
        }
        let (wx, wy) = self.to_world_ground(sx, sy);
        let (tx, ty) = (wx.round() as i32, wy.round() as i32);
        // corpses
        if let Some(c) = self
            .corpses
            .iter()
            .find(|c| (c.x as f32 - wx).abs() < 0.8 && (c.y as f32 - wy).abs() < 0.8)
        {
            let id = c.id;
            let (cx, cy) = (c.x, c.y);
            self.cancel_activity();
            if self.walk_to(cx, cy, 1) {
                self.pl.pending = Pending::Loot(id);
            }
            return;
        }
        // tall things: check the tapped tile and the tile "behind" it for trees and mountains
        let mut probe = vec![(tx, ty)];
        probe.extend_from_slice(&[(tx + 1, ty + 1), (tx + 1, ty), (tx, ty + 1)]);
        for (x, y) in probe {
            let o = self.map.obj(x, y);
            let t = self.map.tile(x, y);
            let gather = if o == O_TREE {
                Some(LUMBER)
            } else if t == T_MOUNT {
                Some(MINING)
            } else {
                None
            };
            if let Some(sk) = gather {
                self.cancel_activity();
                if self.pl.ghost {
                    self.msg("Thou art a ghost.", C_WARN);
                    return;
                }
                let (px, py) = self.ents[0].tile();
                if (px - x).abs().max((py - y).abs()) <= 1 && self.ents[0].path.is_empty() {
                    self.start_gather(sk, x, y);
                } else if self.walk_to(x, y, 1) {
                    self.pl.pending = Pending::Gather(sk, x, y);
                }
                return;
            }
            if matches!(o, O_ANVIL | O_FORGE | O_ANKH) {
                self.cancel_activity();
                let pend = if o == O_ANKH {
                    Pending::Ankh(x, y)
                } else {
                    Pending::Station(o, x, y)
                };
                let (px, py) = self.ents[0].tile();
                if (px - x).abs().max((py - y).abs()) <= 2 {
                    self.ents[0].path.truncate(1);
                    self.pl.pending = pend;
                } else if self.walk_to(x, y, 1) {
                    self.pl.pending = pend;
                }
                return;
            }
        }
        if self.map.tile(tx, ty) == T_WATER {
            self.cancel_activity();
            if self.pl.ghost {
                return;
            }
            let (px, py) = self.ents[0].tile();
            if (px - tx).abs().max((py - ty).abs()) <= 4 {
                self.start_gather(FISHING, tx, ty);
            } else if self.walk_to(tx, ty, 3) {
                self.pl.pending = Pending::Gather(FISHING, tx, ty);
            }
            return;
        }
        self.cancel_activity();
        if !self.walk_to(tx, ty, 0) {
            self.walk_to(tx, ty, 1);
        }
    }

    /// Go into combat stance, drawing the weapon (Pagan's readyWeapon).
    pub fn enter_war(&mut self) {
        if !self.pl.war {
            self.pl.war = true;
            self.ents[0].ready = 1.0;
            self.touch_ui();
        }
    }

    pub fn toggle_war(&mut self) {
        if self.pl.ghost {
            return;
        }
        if !self.pl.war {
            self.enter_war();
            return;
        }
        self.pl.war = false;
        if !self.pl.war {
            self.ents[0].target = 0;
            self.pl.chase = false;
        }
        self.touch_ui();
    }
}

/// 8-way direction index (into DIRS) for a tile-space delta
pub fn dir8_of(dx: f32, dy: f32) -> u8 {
    let oct = ((dy.atan2(dx) / (std::f32::consts::PI / 4.0)).round() as i32).rem_euclid(8);
    ((oct + 2) % 8) as u8
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn blood_color(e: &Ent) -> u32 {
    match e.mon().map(|m| m.id) {
        Some("slime") => 0x4a8a2aff,
        Some("skeleton") | Some("lich") => 0x9a9480ff,
        Some("zombie") => 0x3a3a1aff,
        _ => 0x6a0a08ff,
    }
}

fn rot(dx: i32, dy: i32, r: i32) -> (i32, i32) {
    let k = DIRS.iter().position(|&d| d == (dx, dy)).unwrap_or(0) as i32;
    DIRS[((k + r).rem_euclid(8)) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_generates_and_town_reachable() {
        let mut g = Game::new(12345);
        let (sx, sy) = g.map.start;
        assert!(g.map.passable(sx, sy));
        for &(_, x, y) in g.map.pois.clone().iter() {
            let p = g.map.find_path(sx, sy, x, y, 4);
            assert!(p.is_some(), "poi at {x},{y} unreachable");
        }
        for &(x, y) in g.map.ankhs.clone().iter() {
            assert!(g.map.find_path(sx, sy, x, y, 1).is_some());
        }
        assert!(g.ents.len() > 30);
    }

    #[test]
    fn create_and_fight() {
        for seed in [1u32, 7, 99, 2024] {
            let mut g = Game::new(seed);
            g.create("Tester", false, 0, "warrior", [0, 0, 0]);
            assert!(g.pl.weapon.is_some());
            assert!(g.pl.armor.is_some());
            // simulate some minutes of play: attack nearest monster repeatedly
            for step in 0..20000 {
                if step % 50 == 0 && g.ents[0].target == 0 && !g.pl.ghost {
                    let p = (g.ents[0].x, g.ents[0].y);
                    if let Some(m) = g
                        .ents
                        .iter()
                        .filter(|e| matches!(e.k, EK::Mon(_)) && !e.dead)
                        .min_by(|a, b| a.dist(p.0, p.1).partial_cmp(&b.dist(p.0, p.1)).unwrap())
                    {
                        let id = m.id;
                        g.pl.war = true;
                        g.ents[0].target = id;
                        g.pl.chase = true;
                    }
                }
                if g.pl.ghost {
                    g.resurrect();
                }
                g.update(1.0 / 30.0);
            }
            assert!(g.pl.kills > 0, "seed {seed}: no kills");
            assert!(g.pl.skills[SWORDS] > 500);
        }
    }

    #[test]
    fn mage_casts() {
        let mut g = Game::new(5);
        g.create("Merlin", true, 2, "mage", [0, 0, 0]);
        let id = g
            .ents
            .iter()
            .find(|e| matches!(e.k, EK::Mon(_)))
            .unwrap()
            .id;
        g.pm().hp = 10;
        g.begin_cast(1);
        for _ in 0..60 {
            g.update(1.0 / 30.0);
        }
        assert!(g.ents[0].hp > 10 || g.journal.iter().any(|j| j.1.contains("fizzles")));
        let _ = id;
    }

    #[test]
    fn block_kick_knockdown() {
        let mut g = Game::new(11);
        g.create("Kicker", false, 0, "warrior", [0, 0, 0]);
        let (sx, sy) = g.map.start;
        // an orc right next to us, outside any spawner
        let oi = mon_idx("orc");
        let id = g.nid();
        let mut e = Ent::new(id, EK::Mon(oi), "an orc".into(), sx + 1, sy, 500);
        e.home = (sx + 1, sy);
        g.ents.push(e);
        let i = g.ents.len() - 1;
        g.pl.war = true;
        g.kick();
        assert!(g.ents[0].kick > 0.0);
        assert!(g.pl.kick_cd > 0.0);
        // knock-down puts the target on the ground and stops it acting
        g.knock_down(i, 1.5);
        assert!(g.ents[i].down_t > 0.0);
        // blocking stops the player walking
        g.set_block(true);
        g.ents[0].path = vec![(sx, sy + 1), (sx, sy + 2)];
        g.update(0.05);
        assert!(g.ents[0].path.len() <= 1);
        g.set_block(false);
        // a huge blow while not blocking may floor the player
        let mut floored = false;
        for _ in 0..20 {
            g.ents[0].hp = g.ents[0].mhp;
            g.ents[0].down_t = 0.0;
            g.damage(0, g.ents[0].mhp / 2, false);
            if g.ents[0].down_t > 0.0 {
                floored = true;
                break;
            }
        }
        assert!(floored);
    }

    #[test]
    fn ledges_block_and_slopes_climb() {
        let m = crate::world::generate(1997);
        let (mut ledge, mut slope) = (0, 0);
        for y in 1..crate::world::H - 1 {
            for x in 1..crate::world::W - 1 {
                if !m.passable(x, y) || !m.passable(x + 1, y) {
                    continue;
                }
                let d = m.level(x + 1, y) as i32 - m.level(x, y) as i32;
                if d.abs() >= 2 {
                    assert!(!m.can_step(x, y, 1, 0), "cliff climbed at {x},{y}");
                }
                if d.abs() == 1 {
                    if m.is_ramp(x, y) || m.is_ramp(x + 1, y) {
                        assert!(m.can_step(x, y, 1, 0));
                        slope += 1;
                    } else {
                        assert!(!m.can_step(x, y, 1, 0), "ledge climbed at {x},{y}");
                        ledge += 1;
                    }
                }
            }
        }
        assert!(ledge > 100 && slope > 50, "ledges {ledge} slopes {slope}");
        // entities stand on the ground they occupy
        let mut g = Game::new(1997);
        g.create("Tester", false, 0, "warrior", [0, 0, 0]);
        for _ in 0..30 {
            g.update(0.05);
        }
        let (tx, ty) = g.ents[0].tile();
        assert!((g.ents[0].vz - g.map.surface(tx, ty)).abs() < 0.05);
    }

    #[test]
    fn save_roundtrip() {
        let mut g = Game::new(3);
        g.create("Saver", false, 1, "blacksmith", [0, 0, 0]);
        g.pl.gold = 1234;
        g.pl.skills[MINING] = 777;
        let s = g.save();
        let mut h = Game::new(3);
        assert!(h.load(&s));
        assert_eq!(h.pl.gold, 1234);
        assert_eq!(h.pl.skills[MINING], 777);
        assert_eq!(h.pl.name, "Saver");
        assert_eq!(h.pl.pack.len(), g.pl.pack.len());
        assert!(h.pl.weapon.is_some());
    }

    #[test]
    fn render_and_ui_do_not_panic() {
        let mut g = Game::new(42);
        g.w = 390.0;
        g.h = 800.0;
        g.render();
        let _ = g.ui_json();
        g.create("R", false, 0, "custom", [MAGERY, MINING, FISHING]);
        for w in [
            Win::Paperdoll,
            Win::Pack,
            Win::Skills,
            Win::Spells,
            Win::Journal,
            Win::Help,
            Win::Bank,
            Win::Craft,
            Win::Res,
            Win::Map,
        ] {
            g.win = w;
            let j = g.ui_json();
            assert!(j.starts_with('{'));
        }
        for _ in 0..100 {
            g.update(0.05);
            g.render();
        }
        let _ = g.hud_json();
        g.tap(200.0, 300.0);
        g.ui_cmd("open:pack");
        g.ui_cmd("buy:dagger:1");
    }
}
