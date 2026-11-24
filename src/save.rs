//! Plain-text save format (key=value lines). No external crates.
use crate::data::*;
use crate::game::*;

fn items_str(v: &[Item]) -> String {
    v.iter().map(item_str).collect::<Vec<_>>().join(";")
}
fn item_str(i: &Item) -> String {
    format!(
        "{}*{}*{}*{}",
        ITEMS[i.id].id, i.qty, i.blessed as u8, i.exc as u8
    )
}
fn parse_item(s: &str) -> Option<Item> {
    let p: Vec<&str> = s.split('*').collect();
    let id = try_item_idx(p.first()?)?;
    Some(Item {
        id,
        qty: p.get(1)?.parse().ok()?,
        blessed: p.get(2) == Some(&"1"),
        exc: p.get(3) == Some(&"1"),
    })
}
fn parse_items(s: &str) -> Vec<Item> {
    s.split(';')
        .filter(|x| !x.is_empty())
        .filter_map(parse_item)
        .collect()
}

pub fn save_seed(s: &str) -> Option<u32> {
    s.lines()
        .find_map(|l| l.strip_prefix("seed="))
        .and_then(|v| v.parse().ok())
}

impl Game {
    pub fn save(&self) -> String {
        let pl = &self.pl;
        let p = &self.ents[0];
        let sk: Vec<String> = pl.skills.iter().map(|v| v.to_string()).collect();
        let mut o = String::new();
        let mut kv = |k: &str, v: String| {
            o.push_str(k);
            o.push('=');
            o.push_str(&v.replace('\n', " "));
            o.push('\n');
        };
        kv("v", "1".into());
        kv("seed", self.seed.to_string());
        kv("name", pl.name.clone());
        kv("female", (pl.female as u8).to_string());
        kv(
            "hair",
            HAIR.iter()
                .position(|&h| h == pl.hair)
                .unwrap_or(0)
                .to_string(),
        );
        kv("stats", format!("{},{},{}", pl.str_, pl.dex, pl.int_));
        kv("skills", sk.join(","));
        kv("gold", pl.gold.to_string());
        kv("bank_gold", pl.bank_gold.to_string());
        kv("karma", pl.karma.to_string());
        kv("fame", pl.fame.to_string());
        kv("kills", pl.kills.to_string());
        kv("spells", pl.spells.to_string());
        kv("ghost", (pl.ghost as u8).to_string());
        kv(
            "pos",
            format!("{},{}", p.x.round() as i32, p.y.round() as i32),
        );
        kv("hp", p.hp.to_string());
        kv("mana", p.mana.to_string());
        kv("time", (self.time as i64).to_string());
        kv("zoom", format!("{:.3}", self.zoom_mul));
        for (k, v) in self.sim.save_lines() {
            kv(k, v);
        }
        kv("pack", items_str(&pl.pack));
        kv("bank", items_str(&pl.bank));
        kv(
            "weapon",
            pl.weapon.as_ref().map(item_str).unwrap_or_default(),
        );
        kv(
            "shield",
            pl.shield.as_ref().map(item_str).unwrap_or_default(),
        );
        kv("armor", pl.armor.as_ref().map(item_str).unwrap_or_default());
        if let Some(h) = &pl.horse {
            kv(
                "horse",
                format!(
                    "{},{},{}",
                    h.coat,
                    pl.riding as u8,
                    h.name.replace(',', " ")
                ),
            );
            if let Some(i) = self.my_horse() {
                let (x, y) = self.ents[i].tile();
                kv("horsepos", format!("{},{}", x, y));
            }
        }
        if let Some(c) = self.corpses.iter().find(|c| c.player) {
            kv(
                "corpse",
                format!("{},{},{}|{}", c.x, c.y, c.gold, items_str(&c.items)),
            );
        }
        o
    }

    pub fn load(&mut self, s: &str) -> bool {
        let mut pl = Pl::new();
        let mut pos = self.map.start;
        let (mut hp, mut mana) = (1, 0);
        let mut ok = false;
        let mut corpse: Option<(i32, i32, i64, Vec<Item>)> = None;
        let mut horse_pos: Option<(i32, i32)> = None;
        for line in s.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            if self.sim.load_line(k, v) {
                continue;
            }
            let nums = |v: &str| {
                v.split(',')
                    .map(|x| x.trim().parse::<i32>().unwrap_or(0))
                    .collect::<Vec<_>>()
            };
            match k {
                "name" => {
                    pl.name = v.to_string();
                    ok = true;
                }
                "female" => pl.female = v == "1",
                "hair" => pl.hair = HAIR[v.parse::<usize>().unwrap_or(0) % HAIR.len()],
                "stats" => {
                    let n = nums(v);
                    if n.len() == 3 {
                        pl.str_ = n[0].clamp(1, 100);
                        pl.dex = n[1].clamp(1, 100);
                        pl.int_ = n[2].clamp(1, 100);
                    }
                }
                "skills" => {
                    for (i, x) in nums(v).into_iter().enumerate().take(N_SKILLS) {
                        pl.skills[i] = x.clamp(0, 1000);
                    }
                }
                "gold" => pl.gold = v.parse().unwrap_or(0),
                "bank_gold" => pl.bank_gold = v.parse().unwrap_or(0),
                "karma" => pl.karma = v.parse().unwrap_or(0),
                "fame" => pl.fame = v.parse().unwrap_or(0),
                "kills" => pl.kills = v.parse().unwrap_or(0),
                "spells" => pl.spells = v.parse().unwrap_or(0),
                "ghost" => pl.ghost = v == "1",
                "pos" => {
                    let n = nums(v);
                    if n.len() == 2 {
                        pos = (n[0], n[1]);
                    }
                }
                "hp" => hp = v.parse().unwrap_or(1),
                "mana" => mana = v.parse().unwrap_or(0),
                "time" => self.time = v.parse::<f32>().unwrap_or(60.0),
                "zoom" => self.zoom_mul = v.parse::<f32>().unwrap_or(1.0).clamp(0.5, 2.5),
                "pack" => pl.pack = parse_items(v),
                "bank" => pl.bank = parse_items(v),
                "weapon" => pl.weapon = parse_item(v),
                "shield" => pl.shield = parse_item(v),
                "armor" => pl.armor = parse_item(v),
                "horse" => {
                    let mut it = v.splitn(3, ',');
                    let coat = it.next().and_then(|c| c.parse::<u8>().ok()).unwrap_or(0);
                    let riding = it.next() == Some("1");
                    let name = it.next().unwrap_or("").trim();
                    pl.horse = Some(Steed {
                        name: if name.is_empty() {
                            HORSE_NAMES[0].to_string()
                        } else {
                            name.chars().take(20).collect()
                        },
                        coat: coat % HORSE_COATS.len() as u8,
                    });
                    pl.riding = riding;
                }
                "horsepos" => {
                    let n = nums(v);
                    if n.len() == 2 {
                        horse_pos = Some((n[0], n[1]));
                    }
                }
                "corpse" => {
                    if let Some((head, its)) = v.split_once('|') {
                        let n: Vec<i64> = head.split(',').map(|x| x.parse().unwrap_or(0)).collect();
                        if n.len() == 3 {
                            corpse = Some((n[0] as i32, n[1] as i32, n[2], parse_items(its)));
                        }
                    }
                }
                _ => {}
            }
        }
        if !ok {
            return false;
        }
        // a save from an older map may stand where it can no longer get out
        if !self.map.passable(pos.0, pos.1) || !self.map.reach[crate::world::idx(pos.0, pos.1)] {
            pos = self.map.start;
        }
        let mhp = pl.mhp();
        let mm = pl.mmana();
        let nm = pl.name.clone();
        let ghost = pl.ghost;
        if ghost {
            pl.riding = false;
        }
        self.pl = pl;
        let p = &mut self.ents[0];
        p.name = nm;
        p.x = pos.0 as f32;
        p.y = pos.1 as f32;
        p.path.clear();
        p.mhp = mhp;
        p.mmana = mm;
        p.hp = if ghost { 0 } else { hp.clamp(1, mhp) };
        p.mana = mana.clamp(0, mm);
        if let Some((x, y, gold, items)) = corpse {
            let id = self.nid();
            let name = format!("the corpse of {}", self.pl.name);
            self.corpses.push(Corpse {
                id,
                x,
                y,
                name,
                items,
                gold,
                t: 3600.0,
                player: true,
                size: 1.0,
                color: 0xc8a080ff,
                look: None,
                dir8: 3,
                age: 10.0,
            });
        }
        // the horse waits where it was left (or by the player, if that spot is gone)
        self.ents.retain(|e| e.tame() != 1);
        if let Some(h) = self.pl.horse.clone() {
            if !self.pl.riding {
                let (x, y) = horse_pos
                    .filter(|&(x, y)| {
                        self.map.passable(x, y) && self.map.reach[crate::world::idx(x, y)]
                    })
                    .unwrap_or(pos);
                self.spawn_horse(x, y, &h.name, h.coat);
            }
        }
        // move monsters away from the loaded position so we don't wake up in a fight
        let (px, py) = (pos.0 as f32, pos.1 as f32);
        for e in self.ents.iter_mut().skip(1) {
            if matches!(e.k, EK::Mon(_)) && e.tame() == 0 && e.dist(px, py) < 6.0 {
                e.dead = true;
            }
        }
        self.ents.retain(|e| !e.dead || e.k == EK::Player);
        let (x, y) = (self.ents[0].x, self.ents[0].y);
        self.camx = (x - y) * 32.0;
        self.camy = (x + y) * 16.0;
        self.created = true;
        let name = self.pl.name.clone();
        self.msg(&format!("Welcome back, {}.", name), C_GOLD);
        if self.pl.ghost {
            self.msg("Thou art still a ghost. Seek a healer or an ankh.", C_WARN);
        }
        self.touch_ui();
        true
    }
}
