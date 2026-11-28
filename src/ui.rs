//! UI state as JSON (rendered by the JS host as HTML "gumps") and UI commands.
use crate::data::*;
use crate::game::*;
use crate::world::*;
use std::fmt::Write;

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => {}
            c => o.push(c),
        }
    }
    o.push('"');
    o
}
/// shown text, as the world pack words it, escaped for JSON
fn tx(s: &str) -> String {
    esc(&crate::pack::t(s))
}
fn hex(c: u32) -> String {
    format!("\"#{:06x}\"", c >> 8)
}

/// a baked icon ("#k" into the icon sheet), or the item's emoji
fn icon(d: &ItemDef) -> String {
    match crate::bake::icons::icon_index(d.id) {
        Some(k) => format!("#{k}"),
        None => d.icon.to_string(),
    }
}

#[derive(Default)]
struct Row {
    icon: String,
    text: String,
    sub: String,
    bar: f32,
    btns: Vec<(String, String)>,
    dim: bool,
}
impl Row {
    fn new(icon: &str, text: &str, sub: &str) -> Row {
        Row {
            icon: icon.into(),
            text: text.into(),
            sub: sub.into(),
            bar: -1.0,
            ..Default::default()
        }
    }
    fn btn(mut self, l: &str, cmd: &str) -> Row {
        self.btns.push((l.into(), cmd.into()));
        self
    }
    fn json(&self) -> String {
        let mut s = format!(
            "{{\"i\":{},\"t\":{},\"s\":{}",
            esc(&self.icon),
            tx(&self.text),
            tx(&self.sub)
        );
        if self.bar >= 0.0 {
            let _ = write!(s, ",\"bar\":{:.3}", self.bar);
        }
        if self.dim {
            s.push_str(",\"dim\":true");
        }
        s.push_str(",\"b\":[");
        for (k, (l, c)) in self.btns.iter().enumerate() {
            if k > 0 {
                s.push(',');
            }
            let _ = write!(s, "[{},{}]", tx(l), esc(c));
        }
        s.push_str("]}");
        s
    }
}

struct WinJ {
    id: &'static str,
    title: String,
    head: Vec<String>,
    rows: Vec<Row>,
    foot: Vec<(String, String)>,
    tabs: Vec<(String, String, bool)>,
    /// a baked figure to show: atlas id, frame, frame size, anchor
    doll: Option<(u32, usize, usize, usize, f32, f32)>,
}
impl WinJ {
    fn new(id: &'static str, title: &str) -> WinJ {
        WinJ {
            id,
            title: title.into(),
            head: vec![],
            rows: vec![],
            foot: vec![],
            tabs: vec![],
            doll: None,
        }
    }
    fn foot(&mut self, l: &str, c: &str) {
        self.foot.push((l.into(), c.into()));
    }
    fn json(&self) -> String {
        let mut s = format!(
            "{{\"id\":{},\"title\":{},\"head\":[",
            esc(self.id),
            tx(&self.title)
        );
        s.push_str(
            &self
                .head
                .iter()
                .map(|h| tx(h))
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push_str("],\"rows\":[");
        s.push_str(
            &self
                .rows
                .iter()
                .map(|r| r.json())
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push_str("],\"foot\":[");
        s.push_str(
            &self
                .foot
                .iter()
                .map(|(l, c)| format!("[{},{}]", tx(l), esc(c)))
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push_str("],\"tabs\":[");
        s.push_str(
            &self
                .tabs
                .iter()
                .map(|(l, c, a)| format!("[{},{},{}]", tx(l), esc(c), a))
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push(']');
        if let Some((id, f, fw, fh, ax, ay)) = self.doll {
            let _ = write!(s, ",\"doll\":[{id},{f},{fw},{fh},{ax},{ay}]");
        }
        s.push('}');
        s
    }
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn dir_name(dx: f32, dy: f32) -> &'static str {
    // tile axes are rotated 45° on screen: screen-up = (-1,-1)
    let sx = dx - dy;
    let sy = dx + dy;
    let a = (-sy).atan2(sx).to_degrees();
    let a = if a < 0.0 { a + 360.0 } else { a };
    [
        "east",
        "north-east",
        "north",
        "north-west",
        "west",
        "south-west",
        "south",
        "south-east",
    ][(((a + 22.5) / 45.0) as usize) % 8]
}

impl Game {
    pub fn hud_json(&self) -> String {
        let p = &self.ents[0];
        let tgt = self.find(p.combat.target).map(|i| {
            let e = &self.ents[i];
            format!(
                "{{\"name\":{},\"hp\":{},\"mhp\":{}}}",
                tx(&e.name),
                e.hp.max(0),
                e.mhp
            )
        });
        let act = match self.pl.action {
            Action::Gather { skill, .. } => match skill {
                MINING => "Mining…",
                LUMBER => "Chopping…",
                _ => "Fishing…",
            },
            Action::Bandage { .. } => "Bandaging…",
            Action::Cast { .. } => "Casting…",
            Action::None => "",
        };
        let mut j = String::from("[");
        let n = self.journal.len();
        for (k, (id, t, c)) in self.journal.iter().skip(n.saturating_sub(7)).enumerate() {
            if k > 0 {
                j.push(',');
            }
            let _ = write!(j, "[{},{},{}]", id, tx(t), hex(*c));
        }
        j.push(']');
        let last = if self.pl.last_spell >= 0 {
            SPELLS[self.pl.last_spell as usize].name
        } else {
            ""
        };
        let targeting = if self.pl.targeting >= 0 {
            SPELLS[self.pl.targeting as usize].name
        } else {
            ""
        };
        format!(
            "{{\"created\":{},\"name\":{},\"hp\":{},\"mhp\":{},\"mana\":{},\"mmana\":{},\"gold\":{},\"war\":{},\"ghost\":{},\"tgt\":{},\"act\":{},\"last\":{},\"targeting\":{},\"band\":{},\"pot\":{},\"night\":{},\"period\":{},\"blocking\":{},\"down\":{},\"j\":{}}}",
            self.created,
            esc(&self.pl.name),
            p.hp.max(0),
            p.mhp,
            p.mana.max(0),
            p.mmana,
            self.pl.gold,
            self.pl.war,
            self.pl.ghost,
            tgt.unwrap_or("null".into()),
            tx(act),
            tx(last),
            tx(targeting),
            self.pl.count(item_idx("bandage")),
            self.pl.count(item_idx("healpotion")),
            self.is_night(),
            {
                let mut s = crate::pack::t(self.period());
                let w = self.weather_word();
                if !w.is_empty() {
                    s = format!("{s} · {}", crate::pack::t(w));
                }
                if self.in_shadow {
                    s = format!("{s} · {}", crate::pack::t("in the Shadow"));
                }
                esc(&s)
            },
            self.pl.blocking,
            p.down_t > 0.0,
            j
        )
    }

    pub fn ui_json(&self) -> String {
        if !self.created {
            let mut s = String::from("{\"screen\":\"create\",\"templates\":[");
            s.push_str(
                &TEMPLATES
                    .iter()
                    .map(|t| format!("[{},{},{}]", esc(t.id), tx(t.name), tx(t.desc)))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            s.push_str("],\"skills\":[");
            s.push_str(
                &(0..N_SKILLS)
                    .map(|k| format!("[{},{}]", k, tx(SKILL_NAME[k])))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            s.push_str("],\"hair\":[");
            s.push_str(&HAIR.iter().map(|&h| hex(h)).collect::<Vec<_>>().join(","));
            s.push_str("]}");
            return s;
        }
        let w = self.window();
        format!(
            "{{\"screen\":\"game\",\"win\":{}}}",
            w.map(|w| w.json()).unwrap_or("null".into())
        )
    }

    fn item_row(&self, it: &Item) -> Row {
        let d = it.def();
        let mut sub = match d.k {
            K::Weapon => format!("{} · dmg {}–{}", SKILL_NAME[d.skill], d.dmg.0, d.dmg.1),
            K::Armor | K::Shield => format!("armor {}", it.ar()),
            _ => String::new(),
        };
        if it.blessed {
            if !sub.is_empty() {
                sub.push_str(" · ");
            }
            sub.push_str("blessed");
        }
        Row::new(&icon(d), &it.label(), &sub)
    }

    fn window(&self) -> Option<WinJ> {
        let pl = &self.pl;
        let p = &self.ents[0];
        Some(match self.win {
            Win::None => return None,
            Win::Paperdoll => {
                let mut w = WinJ::new("paperdoll", &pl.name);
                // the hero as baked, in their gear, facing out of the window
                if let Some(look) = self.look_for_ent(p) {
                    if let Some(info) = self.sprites.ids.get(&look.key()) {
                        let (f, _) =
                            crate::bake::frame_index(&look, crate::bake::Anim::Idle, 0.0, 3);
                        w.doll = Some((info.id, f, info.fw, info.fh, info.ax, info.ay));
                    }
                }
                w.head.push(pl.title());
                w.head.push(format!(
                    "Str {}  ·  Dex {}  ·  Int {}",
                    pl.str_, pl.dex, pl.int_
                ));
                w.head.push(format!(
                    "Hits {}/{}  ·  Mana {}/{}",
                    p.hp.max(0),
                    p.mhp,
                    p.mana,
                    p.mmana
                ));
                let wd = pl.weapon_def();
                w.head.push(format!(
                    "Armor rating {}  ·  {} {}–{}",
                    pl.ar(),
                    wd.name,
                    wd.dmg.0,
                    wd.dmg.1
                ));
                w.head.push(format!(
                    "Karma {}  ·  Fame {}  ·  Kills {}",
                    pl.karma, pl.fame, pl.kills
                ));
                if pl.ghost {
                    w.head.push("Thou art a ghost.".into());
                }
                if let Some(h) = &pl.horse {
                    w.head.push(if pl.riding {
                        format!("Riding {}", h.name)
                    } else {
                        format!("Thy horse: {}", h.name)
                    });
                }
                for (slot, it) in [
                    ("weapon", &pl.weapon),
                    ("shield", &pl.shield),
                    ("armor", &pl.armor),
                ] {
                    match it {
                        Some(i) => w
                            .rows
                            .push(self.item_row(i).btn("Remove", &format!("unequip:{}", slot))),
                        None => {
                            let mut r = Row::new("▫️", &format!("(no {})", slot), "");
                            r.dim = true;
                            w.rows.push(r)
                        }
                    }
                }
                if pl.riding {
                    w.foot("Dismount", "ride");
                } else if self.my_horse().is_some() && !pl.ghost {
                    w.foot("Mount", "ride");
                }
                w.foot("Skills", "open:skills");
                w.foot("Backpack", "open:pack");
                w.foot("Spellbook", "open:spells");
                w.foot("Help", "open:help");
                w
            }
            Win::Pack => {
                let mut w = WinJ::new("pack", "Backpack");
                w.head.push(format!("🪙 {} gold", pl.gold));
                if pl.pack.is_empty() {
                    w.head.push("Thy backpack is empty.".into());
                }
                for (k, it) in pl.pack.iter().enumerate() {
                    let d = it.def();
                    let mut r = self.item_row(it);
                    r = match d.k {
                        K::Weapon | K::Armor | K::Shield => r.btn("Equip", &format!("use:{}", k)),
                        K::Consumable => r.btn(
                            match d.id {
                                "fish" | "bread" => "Eat",
                                "ale" => "Drink",
                                _ => "Use",
                            },
                            &format!("use:{}", k),
                        ),
                        K::Scroll => r.btn("Inscribe", &format!("use:{}", k)),
                        K::Book => r.btn("Open", &format!("use:{}", k)),
                        K::Tool => r.btn("Use", &format!("use:{}", k)),
                        _ => r,
                    };
                    if !it.blessed {
                        r = r.btn("Drop", &format!("drop:{}", k));
                    }
                    w.rows.push(r);
                }
                w.foot("Paperdoll", "open:paperdoll");
                w
            }
            Win::Skills => {
                let mut w = WinJ::new("skills", "Skills");
                let tot = pl.total_skill();
                w.head.push(format!(
                    "Total {}.{} / 700.0 — skills rise as thou usest them",
                    tot / 10,
                    tot % 10
                ));
                for s in 0..N_SKILLS {
                    let v = pl.skills[s];
                    let mut r = Row::new(
                        "",
                        SKILL_NAME[s],
                        &format!("{}.{}  {}", v / 10, v % 10, skill_rank(v)),
                    );
                    r.bar = v as f32 / 1000.0;
                    r.dim = v == 0;
                    w.rows.push(r);
                }
                w
            }
            Win::Spells => {
                let mut w = WinJ::new("spells", "Spellbook");
                if !pl.has_book() {
                    w.head
                        .push("Thou hast no spellbook. A mage in town sells them.".into());
                    return Some(w);
                }
                w.head.push(format!(
                    "Magery {:.1} · Mana {}/{}",
                    pl.sk(MAGERY),
                    p.mana,
                    p.mmana
                ));
                for (k, s) in SPELLS.iter().enumerate() {
                    let known = pl.spells & (1 << k) != 0;
                    let regs: Vec<String> = s
                        .regs
                        .iter()
                        .map(|r| format!("{}{}", ITEMS[item_idx(r)].icon, pl.count(item_idx(r))))
                        .collect();
                    let sub = format!(
                        "Circle {} · {} mana · {}",
                        s.circle,
                        CIRCLE_MANA[s.circle],
                        regs.join(" ")
                    );
                    let mut r = Row::new(
                        if s.hostile { "🔥" } else { "#5" },
                        &format!("{} — {}", s.name, s.words),
                        &sub,
                    );
                    if known {
                        r = r.btn("Cast", &format!("cast:{}", k));
                        let can = p.mana >= CIRCLE_MANA[s.circle]
                            && s.regs.iter().all(|x| pl.count(item_idx(x)) > 0);
                        r.dim = !can;
                    } else {
                        r.sub = format!("Circle {} · not in thy book (buy a scroll)", s.circle);
                        r.dim = true;
                    }
                    w.rows.push(r);
                }
                w
            }
            Win::Journal => {
                let mut w = WinJ::new("journal", "Journal");
                for (_, t, _) in self.journal.iter().rev().take(60) {
                    w.rows.push(Row::new("", t, ""));
                }
                w
            }
            Win::Help => {
                let mut w = WinJ::new("help", "How to play");
                for (i, t) in [
                    ("👆", "Tap the ground to walk there. Hold and drag to steer, like the mouse in the old days."),
                    ("⚔️", "Go to war, then tap a creature to attack it; at peace a tap only names it. Skills rise with use."),
                    ("🧑", "Tap townsfolk to trade, bank or be healed. Tap thyself for thy paperdoll."),
                    ("🍺", "The pub sells ale and bread; the library sells spellbooks and scrolls, and buys the scrolls thou findest."),
                    ("🌲", "With a hatchet tap trees; with a pickaxe tap mountains; with a pole tap water."),
                    ("🔨", "Smelt ore at the forge, craft arms at the anvil (needs a smith's hammer)."),
                    ("📖", "Spells need reagents and mana. Buy scrolls from the mage to learn more."),
                    ("☠️", "Death leaves thy belongings on thy corpse. Find a healer or an ankh, then go back for them."),
                    ("🛡️", "Town guards slay any monster that enters the town."),
                    ("📜", "The world moves without thee: the Chronicle (C, or from the map) tells the date, how strong each dark place is, and the news. Slay its creatures and it weakens; leave it and it grows back. Strong places raid the farms; caravans bring cheaper goods; a dark power's Shadow spreads from its places."),
                    ("🐎", "The stablemaster east of town sells horses. Tap thy horse to mount, tap thyself to get down; riding is fast, but a heavy blow throws thee."),
                    ("🛡️", "In war mode hold BLOCK to catch blows; KICK shoves a foe back and may floor it. Heavy blows can knock thee down."),
                    ("🗺️", "Tap the radar for the map of the whole land: pinch or scroll to zoom, drag to look about, tap a spot to walk there."),
                    ("⌨️", "Keys: WASD/arrows move · Tab war · Space block · F kick · B bandage · P pack · K skills · E mount · M map · L places · C chronicle · 1-9 cast · Esc close"),
                ] {
                    w.rows.push(Row::new(i, t, ""));
                }
                w
            }
            Win::Chronicle => {
                use crate::sim::{Sim, LEVELS};
                let sim = &self.sim;
                let day = Sim::day(self.time);
                let mut w = WinJ::new("map", "Chronicle");
                w.tabs.push(("Places".into(), "open:map".into(), false));
                w.tabs
                    .push(("Chronicle".into(), "open:chronicle".into(), true));
                w.head
                    .push(format!("{} · {}", sim.date_str(day), sim.season(day)));
                const BANDS: [&str; 4] = ["weak", "holding", "strong", "mighty"];
                for r in sim.regions.iter() {
                    let b = (r.strength / 25.0).floor().clamp(0.0, 3.0) as usize;
                    let mut row = Row::new("🏴", &cap_first(&r.name), BANDS[b]);
                    row.bar = r.strength / 100.0;
                    w.rows.push(row);
                }
                let mut row = Row::new(
                    "⚖️",
                    "Trade on the roads",
                    &format!("{} · prices ×{:.2}", sim.trade_word(), sim.price_factor()),
                );
                row.bar = sim.trade / 100.0;
                w.rows.push(row);
                if sim.regions.iter().any(|r| r.shadow) {
                    let share = self.shadow_share();
                    let mut row = Row::new(
                        "🌑",
                        "The Shadow",
                        &format!("lies over {}% of the land", (share * 100.0).round() as i32),
                    );
                    row.bar = share;
                    w.rows.push(row);
                }
                for (icon, what, (x, y)) in self.travel_rows() {
                    let (dx, dy) = (x - p.x, y - p.y);
                    let d = (dx * dx + dy * dy).sqrt();
                    w.rows.push(Row::new(
                        icon,
                        &what,
                        &format!("{} paces {}", d as i32, dir_name(dx, dy)),
                    ));
                }
                const ICONS: [&str; 5] = ["🪦", "⛺", "🪨", "🐉", "🌲"];
                const LABELS: [&str; 5] =
                    ["Graveyard", "Orc Camp", "Troll Hills", "Dragon Lair", ""];
                for (k, s) in sim.sites.iter().enumerate() {
                    let lv = LEVELS[match s.strength {
                        v if v < 5.0 => 0,
                        v if v < 25.0 => 1,
                        v if v < 55.0 => 2,
                        v if v < 80.0 => 3,
                        _ => 4,
                    }];
                    let at = self.map.pois.iter().find(|(n, _, _)| *n == LABELS[k]);
                    let sub = match at {
                        Some((_, x, y)) => {
                            let (dx, dy) = (*x as f32 - p.x, *y as f32 - p.y);
                            let d = (dx * dx + dy * dy).sqrt();
                            format!("{lv} · {} paces {}", d as i32, dir_name(dx, dy))
                        }
                        None => lv.to_string(),
                    };
                    let mut row = Row::new(ICONS[k], &cap_first(&s.name), &sub);
                    row.bar = s.strength / 100.0;
                    row.dim = s.strength < 5.0;
                    w.rows.push(row);
                }
                if sim.events.is_empty() {
                    w.rows.push(Row::new("📜", "No news yet.", ""));
                }
                for (d, t) in sim.events.iter().rev() {
                    w.rows.push(Row::new("📜", t, &sim.date_str(*d)));
                }
                w
            }
            Win::Map => {
                let mut w = WinJ::new("map", "Places of note");
                w.tabs.push(("Places".into(), "open:map".into(), true));
                w.tabs
                    .push(("Chronicle".into(), "open:chronicle".into(), false));
                for (name, x, y) in self.map.pois.iter() {
                    let (dx, dy) = (*x as f32 - p.x, *y as f32 - p.y);
                    let d = (dx * dx + dy * dy).sqrt();
                    w.rows.push(Row::new(
                        "📍",
                        name,
                        &format!("{} paces {}", d as i32, dir_name(dx, dy)),
                    ));
                }
                for (x, y) in self.map.ankhs.iter() {
                    let (dx, dy) = (*x as f32 - p.x, *y as f32 - p.y);
                    let d = (dx * dx + dy * dy).sqrt();
                    w.rows.push(Row::new(
                        "☥",
                        "Ankh shrine",
                        &format!("{} paces {}", d as i32, dir_name(dx, dy)),
                    ));
                }
                if let Some(c) = self.corpses.iter().find(|c| c.player) {
                    let (dx, dy) = (c.x as f32 - p.x, c.y as f32 - p.y);
                    let d = (dx * dx + dy * dy).sqrt();
                    w.rows.push(Row::new(
                        "☠️",
                        "Thy corpse",
                        &format!("{} paces {}", d as i32, dir_name(dx, dy)),
                    ));
                }
                w
            }
            Win::Ctx(id) => {
                let i = self.find(id)?;
                let e = &self.ents[i];
                let mut w = WinJ::new("ctx", &e.name);
                let v = match e.k {
                    EK::Npc(v) => v,
                    _ => return None,
                };
                let vd = &VENDORS[v];
                if pl.ghost {
                    if v == V_HEALER {
                        w.head
                            .push("\"Thou art but a spirit. Shall I restore thee?\"".into());
                        w.foot("Resurrect", "res_yes");
                    } else {
                        w.head.push("They cannot see thee, ghost.".into());
                    }
                    return Some(w);
                }
                w.head.push(format!(
                    "\"{}\"",
                    vd.chatter[(self.clock as usize / 4) % vd.chatter.len()]
                ));
                if !self.wares(i).is_empty() {
                    w.foot("Buy", &format!("vendor:{}:buy", id));
                }
                if !vd.buys.is_empty() || !vd.buys_ids.is_empty() {
                    w.foot("Sell", &format!("vendor:{}:sell", id));
                }
                if v == V_BANKER {
                    w.foot("Bank box", "open:bank");
                }
                if v == V_STABLE && pl.horse.is_some() && !pl.riding {
                    w.foot("Fetch my horse", "fetchhorse");
                }
                w.foot("Bye", "close");
                w
            }
            Win::Vendor(id, sell) => {
                let i = self.find(id)?;
                let e = &self.ents[i];
                let v = match e.k {
                    EK::Npc(v) => v,
                    _ => return None,
                };
                let vd = &VENDORS[v];
                let mut w = WinJ::new("vendor", &e.name);
                w.tabs
                    .push(("Buy".into(), format!("vendor:{}:buy", id), !sell));
                if !vd.buys.is_empty() || !vd.buys_ids.is_empty() {
                    w.tabs
                        .push(("Sell".into(), format!("vendor:{}:sell", id), sell));
                }
                w.head.push(format!("🪙 Thou hast {} gold", pl.gold));
                w.head
                    .push(format!("Trade on the roads is {}.", self.sim.trade_word()));
                if !sell {
                    for sid in self.wares(i) {
                        let ix = item_idx(sid);
                        let d = &ITEMS[ix];
                        let price = self.buy_price(d);
                        let mut sub = format!("{} gp", price);
                        if d.k == K::Weapon {
                            sub = format!(
                                "{} gp · {} {}–{}",
                                price, SKILL_NAME[d.skill], d.dmg.0, d.dmg.1
                            );
                        } else if d.k == K::Armor || d.k == K::Shield {
                            sub = format!("{} gp · armor {}", price, d.ar);
                        } else if d.k == K::Scroll && pl.spells & (1 << d.spell) != 0 {
                            sub.push_str(" · known");
                        } else if d.k == K::Mount {
                            sub.push_str(" · saddled and bridled");
                        }
                        let mut r =
                            Row::new(&icon(d), d.name, &sub).btn("Buy", &format!("buy:{}:1", sid));
                        if d.stack {
                            r = r.btn("×10", &format!("buy:{}:10", sid));
                        }
                        r.dim = pl.gold < price as i64;
                        w.rows.push(r);
                    }
                } else {
                    let mut any = false;
                    for (k, it) in pl.pack.iter().enumerate() {
                        let d = it.def();
                        if vd.buys.contains(&d.k) || vd.buys_ids.contains(&d.id) {
                            any = true;
                            let mut r = self.item_row(it);
                            r.sub = format!("{} gp each", self.sell_for(it));
                            r = r.btn("Sell 1", &format!("sell:{}:1", k));
                            if it.qty > 1 {
                                r = r.btn("All", &format!("sell:{}:{}", k, it.qty));
                            }
                            w.rows.push(r);
                        }
                    }
                    if !any {
                        w.head.push("Thou hast nothing I wish to buy.".into());
                    }
                }
                w
            }
            Win::Bank => {
                let mut w = WinJ::new("bank", "Bank box");
                w.tabs
                    .push(("Bank box".into(), "banktab:0".into(), self.tab == 0));
                w.tabs
                    .push(("Backpack".into(), "banktab:1".into(), self.tab == 1));
                w.head.push(format!(
                    "🏦 {} gold in the bank · 🪙 {} carried",
                    pl.bank_gold, pl.gold
                ));
                w.foot("Deposit all gold", "bank:depgold");
                w.foot("Withdraw 100", "bank:wd:100");
                w.foot("Withdraw 1000", "bank:wd:1000");
                if self.tab == 0 {
                    if pl.bank.is_empty() {
                        w.head.push("The box is empty.".into());
                    }
                    for (k, it) in pl.bank.iter().enumerate() {
                        w.rows
                            .push(self.item_row(it).btn("Take", &format!("bank:take:{}", k)));
                    }
                } else {
                    for (k, it) in pl.pack.iter().enumerate() {
                        w.rows
                            .push(self.item_row(it).btn("Store", &format!("bank:put:{}", k)));
                    }
                }
                w
            }
            Win::Loot(cid) => {
                let c = self.corpses.iter().find(|c| c.id == cid)?;
                let mut w = WinJ::new("loot", &c.name);
                if c.gold > 0 {
                    w.rows.push(
                        Row::new("#10", &format!("{} gold coins", c.gold), "")
                            .btn("Take", "loot:gold"),
                    );
                }
                for (k, it) in c.items.iter().enumerate() {
                    w.rows
                        .push(self.item_row(it).btn("Take", &format!("loot:{}", k)));
                }
                if c.gold == 0 && c.items.is_empty() {
                    w.head.push("Nothing of value remains.".into());
                } else {
                    w.foot("Take all", "lootall");
                }
                w
            }
            Win::Craft => {
                let mut w = WinJ::new("craft", "Blacksmithy");
                let ing = pl.count(item_idx("ingot"));
                w.head.push(format!(
                    "Blacksmithy {:.1} · 🧱 {} ingots · 🪨 {} ore",
                    pl.sk(SMITHING),
                    ing,
                    pl.count(item_idx("ore"))
                ));
                if !pl.has_tool(SMITHING) {
                    w.head
                        .push("Thou needest a smith's hammer (the blacksmith sells them).".into());
                }
                for &(id, n, min) in RECIPES {
                    let d = &ITEMS[item_idx(id)];
                    let ch = ((pl.skills[SMITHING] - min + 250) as f32 / 500.0).clamp(0.0, 1.0);
                    let mut r = Row::new(
                        &icon(d),
                        d.name,
                        &format!("{} ingots · {:.0}% success", n, ch * 100.0),
                    )
                    .btn("Make", &format!("craft:{}", id));
                    r.dim = ing < n || ch <= 0.0;
                    w.rows.push(r);
                }
                if self.near_obj(O_FORGE, 3) {
                    w.foot("Smelt all ore", "smelt");
                }
                w
            }
            Win::Res => {
                let mut w = WinJ::new("res", "Resurrection");
                w.head.push(
                    "A warm light surrounds thee. Wilt thou return to the world of the living?"
                        .into(),
                );
                w.foot("Yes", "res_yes");
                w.foot("No", "close");
                w
            }
        })
    }

    pub fn ui_cmd(&mut self, cmd: &str) {
        let a: Vec<&str> = cmd.split(':').collect();
        let arg = |k: usize| a.get(k).copied().unwrap_or("");
        let num = |k: usize| a.get(k).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
        match a[0] {
            "create" => {
                // create:name:f|m:hair:template[:s1:s2:s3]
                let custom = [num(5) as usize, num(6) as usize, num(7) as usize];
                self.create(arg(1), arg(2) == "f", num(3) as usize, arg(4), custom);
            }
            "open" => {
                self.win = match arg(1) {
                    "pack" => Win::Pack,
                    "skills" => Win::Skills,
                    "spells" => Win::Spells,
                    "paperdoll" => Win::Paperdoll,
                    "journal" => Win::Journal,
                    "help" => Win::Help,
                    "map" => Win::Map,
                    "chronicle" => Win::Chronicle,
                    "bank" => {
                        if self.near_npc(V_BANKER) {
                            self.tab = 0;
                            Win::Bank
                        } else {
                            Win::None
                        }
                    }
                    _ => Win::None,
                };
            }
            // travel: walk to a tile picked on the big map
            "goto" => {
                let (x, y) = (num(1) as i32, num(2) as i32);
                self.cancel_activity();
                self.pl.chase = false;
                self.ents[0].combat.target = 0;
                if self.walk_to(x, y, 1) {
                    self.msg("Thou settest out.", C_SYS);
                } else {
                    self.msg("Thou canst find no way there from here.", C_WARN);
                }
            }
            "toggle" => {
                let w = match arg(1) {
                    "pack" => Win::Pack,
                    "skills" => Win::Skills,
                    "spells" => Win::Spells,
                    "paperdoll" => Win::Paperdoll,
                    "journal" => Win::Journal,
                    "map" => Win::Map,
                    "chronicle" => Win::Chronicle,
                    _ => Win::Help,
                };
                self.win = if self.win == w { Win::None } else { w };
            }
            "close" => self.win = Win::None,
            "escape" => {
                if self.pl.targeting >= 0 {
                    self.pl.targeting = -1;
                } else {
                    self.win = Win::None;
                }
            }
            "war" => self.toggle_war(),
            "block" => self.set_block(arg(1) == "1"),
            "kick" => self.kick(),
            "ride" => self.ride(),
            "fetchhorse" => {
                if self.near_npc(V_STABLE) {
                    self.fetch_horse();
                    self.win = Win::None;
                }
            }
            "bandage" => self.bandage(),
            "potion" => self.drink_potion(),
            "cast" => {
                let k = num(1) as usize;
                if k < SPELLS.len() {
                    self.begin_cast(k);
                }
            }
            "lastspell" => {
                if self.pl.last_spell >= 0 {
                    self.begin_cast(self.pl.last_spell as usize);
                } else {
                    self.win = Win::Spells;
                }
            }
            "canceltarget" => self.pl.targeting = -1,
            "use" => self.use_item(num(1) as usize),
            "drop" => {
                let k = num(1) as usize;
                if k < self.pl.pack.len() && !self.pl.pack[k].blessed {
                    let it = self.pl.pack.remove(k);
                    self.msg(&format!("You discard {}.", it.label()), C_SYS);
                }
            }
            "unequip" => self.unequip(arg(1)),
            "vendor" => {
                let id = num(1) as u32;
                self.win = Win::Vendor(id, arg(2) == "sell");
            }
            "buy" => self.buy(arg(1), num(2).max(1) as u32),
            "sell" => self.sell(num(1) as usize, num(2).max(1) as u32),
            "banktab" => self.tab = num(1) as u8,
            "bank" => self.bank(arg(1), num(2)),
            "loot" => {
                self.loot(arg(1));
            }
            "lootall" => {
                if let Win::Loot(cid) = self.win {
                    loop {
                        let Some(c) = self.corpses.iter().find(|c| c.id == cid) else {
                            break;
                        };
                        if c.gold > 0 {
                            self.loot("gold");
                        } else if !c.items.is_empty() {
                            if !self.loot("0") {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    self.win = Win::None;
                }
            }
            "craft" => self.craft(arg(1)),
            "smelt" => self.smelt(),
            "res_yes" => {
                let ok = self.near_npc(V_HEALER) || self.near_obj(O_ANKH, 3);
                if ok {
                    self.resurrect();
                } else {
                    self.msg("Thou must be near a healer or an ankh shrine.", C_WARN);
                }
            }
            "zoom" => {
                let f = if arg(1) == "in" { 1.15 } else { 1.0 / 1.15 };
                self.zoom_by(f);
            }
            _ => {}
        }
        self.touch_ui();
    }

    pub fn zoom_by(&mut self, f: f32) {
        self.zoom_mul = (self.zoom_mul * f).clamp(0.4, 2.0);
        self.apply_zoom();
    }
    pub fn apply_zoom(&mut self) {
        // one world unit per art pixel: a 64-pixel floor tile, Pagan's framing
        self.zoom = self.pix.max(0.25) * self.zoom_mul;
    }

    fn near_npc(&self, v: usize) -> bool {
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        self.ents
            .iter()
            .any(|e| e.k == EK::Npc(v) && e.dist(px, py) <= 5.0)
    }

    fn vendor_ctx(&self) -> Option<(usize, bool, usize)> {
        if let Win::Vendor(id, sell) = self.win {
            let i = self.find(id)?;
            if self.ents[i].dist(self.ents[0].x, self.ents[0].y) > 5.0 {
                return None;
            }
            if let EK::Npc(v) = self.ents[i].k {
                return Some((v, sell, i));
            }
        }
        None
    }

    fn buy(&mut self, id: &str, n: u32) {
        let Some((_, _, vi)) = self.vendor_ctx() else {
            self.msg("Thou art too far away to trade.", C_WARN);
            self.win = Win::None;
            return;
        };
        if self.pl.ghost || !self.wares(vi).contains(&id) {
            return;
        }
        let ix = item_idx(id);
        let d = &ITEMS[ix];
        if d.k == K::Mount && self.pl.horse.is_some() {
            self.msg("Thou hast a horse already, and canst ride but one.", C_WARN);
            return;
        }
        let n = if d.stack { n } else { 1 };
        let cost = self.buy_price(d) as i64 * n as i64;
        if self.pl.gold < cost {
            self.msg("Thou canst not afford that.", C_WARN);
            return;
        }
        self.pl.gold -= cost;
        if d.k == K::Mount {
            self.msg(&format!("You buy a {} for {} gold.", d.name, cost), C_GOLD);
            self.lead_out_horse(horse_coat_of(id));
            self.win = Win::None;
            return;
        }
        self.pl.add(Item::new(ix, n));
        let label = Item::new(ix, n).label();
        self.msg(&format!("You buy {} for {} gold.", label, cost), C_GOLD);
    }

    fn sell(&mut self, k: usize, n: u32) {
        let Some((v, _, _)) = self.vendor_ctx() else {
            self.msg("Thou art too far away to trade.", C_WARN);
            self.win = Win::None;
            return;
        };
        if k >= self.pl.pack.len() {
            return;
        }
        let it = self.pl.pack[k].clone();
        let d = it.def();
        let vd = &VENDORS[v];
        if !(vd.buys.contains(&d.k) || vd.buys_ids.contains(&d.id)) {
            return;
        }
        let n = n.min(it.qty);
        let gold = self.sell_for(&it) as i64 * n as i64;
        self.pl.pack[k].qty -= n;
        if self.pl.pack[k].qty == 0 {
            self.pl.pack.remove(k);
        }
        self.pl.gold += gold;
        self.msg(
            &format!(
                "You sell {} for {} gold.",
                Item { qty: n, ..it }.label(),
                gold
            ),
            C_GOLD,
        );
    }

    fn bank(&mut self, op: &str, n: i64) {
        if !self.near_npc(V_BANKER) {
            self.msg("Thou must be near a banker.", C_WARN);
            self.win = Win::None;
            return;
        }
        match op {
            "depgold" => {
                let g = self.pl.gold;
                self.pl.bank_gold += g;
                self.pl.gold = 0;
                self.msg(&format!("Thou hast deposited {} gold.", g), C_GOLD);
            }
            "wd" => {
                let g = n.min(self.pl.bank_gold);
                self.pl.bank_gold -= g;
                self.pl.gold += g;
                self.msg(&format!("Thou hast withdrawn {} gold.", g), C_GOLD);
            }
            "put" => {
                let k = n as usize;
                if k < self.pl.pack.len() {
                    let it = self.pl.pack.remove(k);
                    if it.def().stack {
                        if let Some(e) = self
                            .pl
                            .bank
                            .iter_mut()
                            .find(|e| e.id == it.id && e.blessed == it.blessed)
                        {
                            e.qty += it.qty;
                            return;
                        }
                    }
                    self.pl.bank.push(it);
                }
            }
            "take" => {
                let k = n as usize;
                if k < self.pl.bank.len() {
                    let it = self.pl.bank.remove(k);
                    self.pl.add(it);
                }
            }
            _ => {}
        }
    }

    fn loot(&mut self, what: &str) -> bool {
        let Win::Loot(cid) = self.win else {
            return false;
        };
        let (px, py) = (self.ents[0].x, self.ents[0].y);
        let Some(ci) = self.corpses.iter().position(|c| c.id == cid) else {
            return false;
        };
        let c = &self.corpses[ci];
        if ((c.x as f32 - px).abs()).max((c.y as f32 - py).abs()) > 2.5 {
            self.msg("That is too far away.", C_WARN);
            self.win = Win::None;
            return false;
        }
        if self.pl.ghost {
            self.msg("Thou canst not carry things as a ghost.", C_WARN);
            return false;
        }
        if what == "gold" {
            let g = self.corpses[ci].gold;
            self.corpses[ci].gold = 0;
            self.pl.gold += g;
            if g > 0 {
                self.msg(&format!("You take {} gold.", g), C_GOLD);
            }
        } else {
            let k: usize = what.parse().unwrap_or(usize::MAX);
            if k >= self.corpses[ci].items.len() {
                return false;
            }
            let it = self.corpses[ci].items.remove(k);
            self.msg(&format!("You take {}.", it.label()), C_SYS);
            self.pl.add(it);
        }
        let c = &self.corpses[ci];
        if c.player && c.items.is_empty() && c.gold == 0 {
            self.corpses[ci].t = 5.0;
            self.corpses[ci].player = false;
        }
        true
    }
}
