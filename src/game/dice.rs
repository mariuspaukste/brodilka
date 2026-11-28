//! The dice: Dungeons & Dragons rules for every blow and every spell.
//!
//! - **Attack rolls.** A swing hits when a d20 plus the attacker's attack
//!   bonus meets the defender's Armour Class. The player's bonus is a tenth
//!   of their weapon skill plus their Strength modifier (Wrestling for a
//!   kick); a monster's, a tenth of its skill.
//! - **Armour Class.** The player's is 10, plus a tenth of their weapon skill
//!   (how well they fend blows off), their Dexterity modifier and 2 for a
//!   shield; a monster's, 10 plus a tenth of its skill. Armour itself still
//!   soaks damage after a hit, as before.
//! - **Natural 20s and 1s.** A natural 20 always hits and is a critical: the
//!   damage dice are rolled twice. A natural 1 always misses: a fumble.
//! - **Saving throws.** Whoever a spell or a dragon's breath strikes rolls
//!   a d20 plus their save bonus against the caster's Difficulty Class
//!   (10 + a tenth of the player's Magery + their Intelligence modifier; 13
//!   for a monster's spell, 16 for dragon fire). A save halves the damage.
//!   Magic Arrow, like the magic missile, never misses and allows no save.
//!   The player saves with their Intelligence modifier and a twentieth of
//!   their Magery, a monster with a thirtieth of its skill.
//!
//!
//! The dice can also keep you out of a fight:
//!
//! - **Stealth.** At peace, on foot or on horseback, a monster that might
//!   see you rolls Perception (d20 + a tenth of its skill) against your
//!   Stealth (10 + your Dexterity modifier, +2 by night, +2 under trees,
//!   -4 on a horse), every couple of seconds. Fail and it hasn't seen you;
//!   within two tiles it always does. In war mode you are seen.
//! - **Talking it down.** At peace, tap a monster that is after you to talk
//!   your way out: Intimidation or Persuasion (d20 + the better of your
//!   Strength and Intelligence modifiers + a point per 250 fame, up to 3)
//!   against 10 + a tenth of its skill. Make it and it lets you go for 45
//!   seconds (unless you strike it); fail and you may try again in a few
//!   seconds; roll a natural 1 and it is enraged and will hear no more for
//!   half a minute. The mindless (slimes and the walking dead) cannot be
//!   reasoned with.
//!
//! An ability modifier is (score - 40) / 20, from -1 to +3, scaled to this
//! game's 10-100 abilities. The rolls go to the journal, a line each, unless
//! the player turns them off (the Journal's "Dice" button; kept in the save).
//! The odds come out close to the game's old ones, so its balance stands.
use super::*;

/// the colour of the dice in the journal
pub const C_DICE: u32 = 0xc8b8e8ff;

/// One roll of the d20 against a target number.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Roll {
    /// what the die showed
    pub nat: i32,
    pub bonus: i32,
    /// the Armour Class or Difficulty Class it was rolled against
    pub target: i32,
}

impl Roll {
    pub fn total(&self) -> i32 {
        self.nat + self.bonus
    }
    /// a hit (or a saved throw): a natural 20 always, a natural 1 never
    pub fn success(&self) -> bool {
        self.nat == 20 || (self.nat != 1 && self.total() >= self.target)
    }
    pub fn crit(&self) -> bool {
        self.nat == 20
    }
    pub fn fumble(&self) -> bool {
        self.nat == 1
    }
    /// "14+5=19 vs AC 15"
    fn sum(&self, what: &str) -> String {
        let sign = if self.bonus < 0 { '-' } else { '+' };
        format!(
            "d20 {}{}{}={} vs {} {}",
            self.nat,
            sign,
            self.bonus.abs(),
            self.total(),
            what,
            self.target
        )
    }
}

/// those that cannot be talked out of a fight
pub fn mindless(id: &str) -> bool {
    matches!(id, "slime" | "skeleton" | "zombie")
}

/// an ability modifier, from a score of 10-100: -1 to +3
pub fn ability_mod(score: i32) -> i32 {
    ((score - 40) / 20).clamp(-1, 3)
}

impl Game {
    pub fn d20(&mut self) -> i32 {
        self.rng.range(1, 20)
    }

    /// The player's Armour Class.
    pub fn player_ac(&self) -> i32 {
        let fend = (self.pl.sk(self.pl.weapon_def().skill) / 10.0) as i32;
        let shield = if self.pl.shield.is_some() { 2 } else { 0 };
        10 + fend + ability_mod(self.pl.dex) + shield
    }

    /// A monster's Armour Class.
    pub fn mon_ac(d: &MonDef) -> i32 {
        10 + d.skill / 10
    }

    /// The player's attack bonus with this skill.
    pub fn player_attack_bonus(&self, skill: usize) -> i32 {
        (self.pl.sk(skill) / 10.0) as i32 + ability_mod(self.pl.str_)
    }

    /// The player swings at monster `ti` with `skill`: roll, and say so.
    pub(super) fn player_attack_roll(&mut self, ti: usize, skill: usize) -> Roll {
        let d = self.ents[ti].mon().unwrap_or(&MONS[0]);
        let r = Roll {
            nat: self.d20(),
            bonus: self.player_attack_bonus(skill),
            target: Self::mon_ac(d),
        };
        let nm = self.ents[ti].name.clone();
        let (x, y) = (self.ents[ti].x, self.ents[ti].y);
        let verdict = if r.crit() {
            self.float(x, y - 0.6, "CRIT!", 0xffd040ff);
            "a natural 20, a critical hit!".to_string()
        } else if r.fumble() {
            "a natural 1: thou fumblest.".to_string()
        } else if r.success() {
            "hit.".to_string()
        } else {
            "miss.".to_string()
        };
        self.dice_log(&format!("You at {nm}: {}, {verdict}", r.sum("AC")));
        r
    }

    /// Monster `i` swings at the player: roll, and say so.
    pub(super) fn mon_attack_roll(&mut self, i: usize) -> Roll {
        let d = self.ents[i].mon().unwrap_or(&MONS[0]);
        let r = Roll {
            nat: self.d20(),
            bonus: d.skill / 10,
            target: self.player_ac(),
        };
        let nm = cap(&self.ents[i].name);
        let verdict = if r.crit() {
            let (x, y) = (self.ents[0].x, self.ents[0].y);
            self.float(x, y - 0.6, "CRIT!", 0xff6040ff);
            "a natural 20, a critical hit!"
        } else if r.fumble() {
            "a natural 1, a fumble."
        } else if r.success() {
            "hit."
        } else {
            "miss."
        };
        self.dice_log(&format!("{nm} at you: {}, {verdict}", r.sum("AC")));
        r
    }

    /// Whoever is at `i` throws to save against a spell (or breath) of
    /// this kind; returns the damage they take.
    pub(super) fn saving_throw(&mut self, i: usize, dmg: i32, kind: u8, by_player: bool) -> i32 {
        let dc = if by_player {
            10 + (self.pl.sk(MAGERY) / 10.0) as i32 + ability_mod(self.pl.int_)
        } else if kind == FX_BREATH {
            16
        } else {
            13
        };
        let bonus = if i == 0 {
            ability_mod(self.pl.int_) + (self.pl.sk(MAGERY) / 20.0) as i32
        } else {
            self.ents[i].mon().map(|d| d.skill / 30).unwrap_or(0)
        };
        let r = Roll {
            nat: self.d20(),
            bonus,
            target: dc,
        };
        let who = if i == 0 {
            "You save".to_string()
        } else {
            format!("{} saves", cap(&self.ents[i].name))
        };
        let what = if kind == FX_BREATH {
            "the fire"
        } else {
            "the spell"
        };
        if r.success() {
            self.dice_log(&format!(
                "{who} against {what}: {}, made: half damage.",
                r.sum("DC")
            ));
            (dmg / 2).max(1)
        } else {
            self.dice_log(&format!("{who} against {what}: {}, failed.", r.sum("DC")));
            dmg
        }
    }

    /// The player's Stealth: the number a monster's Perception must meet.
    pub fn stealth_dc(&self) -> i32 {
        let (x, y) = self.ents[0].tile();
        let mut dc = 10 + ability_mod(self.pl.dex);
        if self.view_light().0 > 0.3 {
            dc += 2;
        }
        if self.canopy(x, y) > 0.3 {
            dc += 2;
        }
        if self.pl.riding {
            dc -= 4;
        }
        dc
    }

    /// Does monster `i`, `pd` tiles off, see the player? In war mode, or
    /// close by, always; otherwise every couple of seconds it rolls its
    /// Perception against the player's Stealth.
    pub(super) fn notices(&mut self, i: usize, pd: f32) -> bool {
        if self.pl.war || pd < 2.0 {
            return true;
        }
        if self.ents[i].notice_t > 0.0 {
            return false;
        }
        self.ents[i].notice_t = 2.0 + self.rng.f();
        let d = self.ents[i].mon().unwrap_or(&MONS[0]);
        let r = Roll {
            nat: self.d20(),
            bonus: d.skill / 10,
            target: self.stealth_dc(),
        };
        if r.success() {
            let nm = cap(&self.ents[i].name);
            self.dice_log(&format!("{nm} spots you: {}.", r.sum("Stealth")));
        }
        r.success()
    }

    /// Try to talk monster `i`, which is after the player, out of it.
    pub(super) fn parley(&mut self, i: usize) {
        let d = self.ents[i].mon().unwrap_or(&MONS[0]);
        let nm = cap(&self.ents[i].name);
        if mindless(d.id) {
            self.msg(&format!("{nm} cannot be reasoned with."), C_WARN);
            return;
        }
        if self.ents[i].calm_t < 0.0 {
            self.msg(&format!("{nm} is past hearing anything you say."), C_WARN);
            return;
        }
        if self.pl.parley_t > 0.0 {
            return;
        }
        self.pl.parley_t = 3.0;
        let fame = (self.pl.fame / 250).clamp(0, 3);
        let (s, n) = (ability_mod(self.pl.str_), ability_mod(self.pl.int_));
        let how = if s >= n { "intimidate" } else { "persuade" };
        let r = Roll {
            nat: self.d20(),
            bonus: s.max(n) + fame,
            target: 10 + d.skill / 10,
        };
        let line = format!("You try to {how} {}: {}", self.ents[i].name, r.sum("DC"));
        if r.success() {
            self.dice_log(&format!("{line}, made."));
            let e = &mut self.ents[i];
            e.calm_t = 45.0;
            e.combat.target = 0;
            e.combat.provoked = false;
            e.path.clear();
            e.talk("...", 0xd8d0c0ff);
            self.msg(
                &format!("{nm} thinks better of it and lets you go."),
                C_GOOD,
            );
        } else if r.fumble() {
            self.dice_log(&format!("{line}, a natural 1."));
            self.ents[i].calm_t = -30.0;
            self.msg(&format!("{nm} is enraged by your words!"), C_WARN);
        } else {
            self.dice_log(&format!("{line}, failed."));
            self.msg(&format!("{nm} is not swayed."), C_WARN);
        }
    }

    /// A line of dice in the journal, unless the player has turned them off.
    fn dice_log(&mut self, s: &str) {
        if self.pl.dice_log {
            self.msg(s, C_DICE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_twenties_hit_and_ones_miss() {
        let r = |nat, bonus, target| Roll { nat, bonus, target };
        assert!(r(20, -5, 40).success() && r(20, 0, 10).crit());
        assert!(!r(1, 30, 5).success() && r(1, 0, 0).fumble());
        assert!(r(10, 5, 15).success() && !r(9, 5, 15).success());
        assert_eq!(r(14, 5, 15).sum("AC"), "d20 14+5=19 vs AC 15");
        assert_eq!(r(3, -1, 13).sum("DC"), "d20 3-1=2 vs DC 13");
        assert_eq!(ability_mod(10), -1);
        assert_eq!(ability_mod(50), 0);
        assert_eq!(ability_mod(100), 3);
    }

    #[test]
    fn the_dice_are_fair_and_go_to_the_journal() {
        let mut g = Game::new(7);
        g.create("Roller", false, 0, "warrior", [0, 0, 0]);
        let mut seen = [0; 21];
        for _ in 0..4000 {
            let n = g.d20();
            assert!((1..=20).contains(&n));
            seen[n as usize] += 1;
        }
        assert!(seen[1..].iter().all(|&k| k > 120), "{seen:?}");
        // a swing at an orc is rolled and logged
        let oi = mon_idx("orc");
        let id = g.nid();
        let (sx, sy) = g.map.start;
        g.ents
            .push(Ent::new(id, EK::Mon(oi), "an orc".into(), sx + 1, sy, 500));
        let i = g.ents.len() - 1;
        let r = g.player_attack_roll(i, SWORDS);
        assert_eq!(r.target, 10 + MONS[oi].skill / 10);
        assert!(g
            .journal
            .iter()
            .any(|j| j.1.starts_with("You at an orc: d20 ")));
        // and kept quiet when the player turns them off
        let n = g.journal.len();
        g.ui_cmd("dice");
        assert!(!g.pl.dice_log);
        let n2 = g.journal.len();
        g.player_attack_roll(i, SWORDS);
        assert_eq!(g.journal.len(), n2);
        assert!(n2 >= n);
        // a save halves the damage, or doesn't
        let d = g.saving_throw(i, 20, FX_FIREBALL, true);
        assert!(d == 10 || d == 20);
    }

    #[test]
    fn stealth_and_talk_can_keep_you_out_of_a_fight() {
        let mut g = Game::new(11);
        g.create("Sneak", false, 0, "warrior", [0, 0, 0]);
        g.ents[0].hp = 5000;
        g.ents[0].mhp = 5000;
        let oi = mon_idx("orc");
        let (sx, sy) = g.map.start;
        // in war mode, or close, a monster always sees you
        let id = g.nid();
        g.ents
            .push(Ent::new(id, EK::Mon(oi), "an orc".into(), sx + 4, sy, 500));
        let i = g.ents.len() - 1;
        g.pl.war = true;
        assert!(g.notices(i, 4.0));
        g.pl.war = false;
        assert!(g.notices(i, 1.5));
        // at peace, further off, it rolls, and not again at once
        let mut seen = 0;
        for _ in 0..400 {
            g.ents[i].notice_t = 0.0;
            if g.notices(i, 5.0) {
                seen += 1;
            }
        }
        assert!(seen > 40 && seen < 360, "{seen}");
        assert!(!g.notices(i, 5.0) || g.ents[i].notice_t > 0.0);
        // talked down: it lets you go a while
        let pid = g.ents[0].id;
        let mut calmed = false;
        for _ in 0..200 {
            g.ents[i].combat.target = pid;
            g.ents[i].calm_t = 0.0;
            g.pl.parley_t = 0.0;
            g.parley(i);
            if g.ents[i].calm_t > 0.0 {
                calmed = true;
                break;
            }
        }
        assert!(calmed, "never talked down");
        assert_eq!(g.ents[i].combat.target, 0);
        assert!(g.journal.iter().any(|j| j.1.contains("lets you go")));
        // the mindless will not listen
        let zi = mon_idx("zombie");
        let zid = g.nid();
        g.ents.push(Ent::new(
            zid,
            EK::Mon(zi),
            "a zombie".into(),
            sx + 3,
            sy,
            500,
        ));
        let z = g.ents.len() - 1;
        g.ents[z].combat.target = pid;
        g.pl.parley_t = 0.0;
        g.parley(z);
        assert_eq!(g.ents[z].combat.target, pid);
        assert!(g
            .journal
            .iter()
            .any(|j| j.1.contains("cannot be reasoned with")));
    }
}
