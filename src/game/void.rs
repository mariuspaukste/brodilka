//! The moongates and the Ethereal Void (`world/void.rs` makes it).
//!
//! - **The gates** come in pairs (`Map::gates`): step into one and you come
//!   out beside the other, on horseback or on foot, alive or a ghost. The
//!   land's gate, in its ring of standing stones east of the town, opens
//!   only at night; by day the stones stand silent. The Void's never closes.
//! - **In the Void** there is no weather and no day: a cold violet starlight
//!   all the while. Coming to an isle is told in the journal.
//!
//! Nothing here is saved but where the player stands.
use super::*;

/// the light of the Void: how dark, and its colour (as `daylight`)
pub const VOID_LIGHT: (f32, u32) = (0.38, 0x3a1868ff);

impl Game {
    /// Is the player out in the Ethereal Void?
    pub fn in_void(&self) -> bool {
        let (x, y) = self.ents[0].tile();
        self.map.in_void(x, y)
    }

    /// Is the moongate at `gates[k]` open? The Void's always is; the land's
    /// only at night.
    pub fn gate_open(&self, k: usize) -> bool {
        k % 2 == 1 || self.daylight().0 > 0.3
    }

    /// The light the player sees by: the hour's on the land, starlight in
    /// the Void.
    pub fn view_light(&self) -> (f32, u32) {
        if self.in_void() {
            VOID_LIGHT
        } else {
            self.daylight()
        }
    }

    pub(super) fn void_tick(&mut self) {
        let (x, y) = self.ents[0].tile();
        // the isles, by name, as you come to them
        let isle = if self.map.in_void(x, y) {
            self.map.isle_at(x, y)
        } else {
            None
        };
        if isle != self.isle_seen {
            if let Some(k) = isle {
                let name = self.map.isles[k].name;
                self.msg(&format!("You come to {name}."), C_LORE);
            }
            self.isle_seen = isle;
        }
        // into a gate: wait till they stand in it
        let at = (self.ents[0].fl == 0)
            .then(|| self.map.gates.iter().position(|&g| g == (x, y)))
            .flatten();
        let Some(k) = at else {
            self.gate_seen = false;
            return;
        };
        let p = &self.ents[0];
        if p.dist(x as f32, y as f32) > 0.3 {
            return;
        }
        if !self.gate_open(k) {
            if !self.gate_seen {
                self.msg(
                    "The stones stand silent. Their gate opens only under the night sky.",
                    C_SYS,
                );
                self.gate_seen = true;
            }
            return;
        }
        self.through_gate(k);
    }

    /// Carry the player through the gate at `gates[k]` to its pair, and set
    /// them down beside it.
    pub fn through_gate(&mut self, k: usize) {
        let (gx, gy) = self.map.gates[k ^ 1];
        let out = [
            (0, 1),
            (1, 0),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ]
        .into_iter()
        .map(|(dx, dy)| (gx + dx, gy + dy))
        .find(|&(x, y)| self.map.passable(x, y) && !self.occupied(x, y, 0))
        .unwrap_or((gx, gy + 1));
        self.cancel_activity();
        let p = &mut self.ents[0];
        let from = (p.x, p.y);
        p.x = out.0 as f32;
        p.y = out.1 as f32;
        p.path.clear();
        p.combat.target = 0;
        self.pl.chase = false;
        self.pl.pending = Pending::None;
        self.snap_camera();
        for (x, y) in [from, (out.0 as f32, out.1 as f32)] {
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
        }
        self.isle_seen = None;
        self.gate_seen = true;
        if k % 2 == 0 {
            self.msg(
                "The gate takes you. You stand among the stars, in the Ethereal Void.",
                C_LORE,
            );
        } else {
            self.msg(
                "The gate takes you, and sets you down on the land again.",
                C_LORE,
            );
        }
        self.touch_ui();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::DAY;

    fn run(g: &mut Game, secs: f32) {
        for _ in 0..(secs * 30.0) as i32 {
            g.update(1.0 / 30.0);
        }
    }

    #[test]
    fn the_moongate_opens_at_night_and_carries_you_there_and_back() {
        let mut g = Game::new(1997);
        g.create("Wanderer", false, 0, "warrior", [0, 0, 0]);
        g.ents[0].hp = 5000;
        g.ents[0].mhp = 5000;
        let (land, gate) = (g.map.gates[0], g.map.gates[1]);
        // by day the stones stand silent
        g.time = 0.2 * DAY;
        assert!(!g.gate_open(0) && g.gate_open(1));
        g.ents[0].x = land.0 as f32;
        g.ents[0].y = land.1 as f32 + 2.0;
        g.ents[0].path.clear();
        g.snap_camera();
        assert!(g.walk_to(land.0, land.1, 0));
        run(&mut g, 3.0);
        assert_eq!(g.ents[0].tile(), land);
        assert!(!g.in_void());
        assert!(g.journal.iter().any(|j| j.1.contains("stand silent")));
        // at night it opens and takes them into the Void
        g.time = 0.75 * DAY;
        assert!(g.gate_open(0));
        run(&mut g, 0.5);
        assert!(g.in_void(), "not carried across: at {:?}", g.ents[0].tile());
        let (x, y) = g.ents[0].tile();
        assert!((x - gate.0).abs() <= 1 && (y - gate.1).abs() <= 1);
        assert_eq!(g.view_light(), VOID_LIGHT);
        // no weather out here
        assert_eq!(g.weather_word(), "");
        // and back again
        assert!(g.walk_to(gate.0, gate.1, 0));
        run(&mut g, 3.0);
        assert!(!g.in_void());
        let (x, y) = g.ents[0].tile();
        assert!((x - land.0).abs() <= 1 && (y - land.1).abs() <= 1);
    }

    #[test]
    fn the_void_has_its_dwellers_and_they_count_for_no_site() {
        let mut g = Game::new(42);
        g.create("Wanderer", false, 0, "warrior", [0, 0, 0]);
        let void: Vec<usize> = (1..g.ents.len())
            .filter(|&i| {
                let (x, y) = g.ents[i].tile();
                g.map.in_void(x, y)
            })
            .collect();
        assert!(void.len() >= 10, "{}", void.len());
        for &i in &void {
            let EK::Mon(m) = g.ents[i].k else {
                panic!("not a monster")
            };
            assert!(of_the_void(MONS[m].id));
        }
        let before = g.sim.sites.iter().map(|s| s.strength).collect::<Vec<_>>();
        let i = void[0];
        let hp = g.ents[i].hp;
        g.damage(i, hp + 1, true);
        let after = g.sim.sites.iter().map(|s| s.strength).collect::<Vec<_>>();
        assert_eq!(before, after);
    }
}
