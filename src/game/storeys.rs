//! Doors and storeys.
//!
//! **Doors** are shut until someone opens them. People (the player and the
//! townsfolk) open a door by walking into it; it swings shut on its own a few
//! seconds after, once no one stands in the doorway. Beasts and monsters
//! cannot open doors, so a shut door keeps them out. The player can also tap
//! a door nearby to open or shut it.
//!
//! **Storeys**: some buildings have floors above the ground floor (see
//! `world::Floor`). Every entity stands on a storey (`Ent::fl`). Walking onto
//! stairs takes the player up or down a flight; townsfolk keep to their own
//! floor. Only what is on the player's floor of the building they are in is
//! drawn there: the view is cut away above it.
use super::*;

impl Game {
    /// Where entity `i` may walk: its storey, and whether it opens doors and
    /// takes stairs.
    pub fn walker(&self, i: usize) -> Walker {
        let e = &self.ents[i];
        match e.k {
            EK::Player => Walker {
                fl: e.fl,
                doors: true,
                // no riding up the stairs
                stairs: !self.pl.riding,
            },
            EK::Npc(_) => Walker {
                fl: e.fl,
                doors: true,
                stairs: false,
            },
            EK::Mon(_) if e.tame() != 0 => Walker {
                fl: 0,
                doors: false,
                stairs: false,
            },
            _ => BEAST,
        }
    }

    /// An entity has just stepped onto (x, y): take the stairs if it is on
    /// them.
    pub(super) fn on_arrive(&mut self, i: usize, x: i32, y: i32) {
        let w = self.walker(i);
        if !w.stairs {
            return;
        }
        let fl = self.ents[i].fl;
        let (o, style) = self.map.obj_on(x, y, fl);
        if o != O_STAIRS {
            return;
        }
        let up = style & STAIR_UP != 0;
        let next = if up { fl + 1 } else { fl.saturating_sub(1) };
        if next == fl {
            return;
        }
        // only onto stairs that lead back the other way, so a flight is one
        let (o2, s2) = self.map.obj_on(x, y, next);
        let back = if up { STAIR_DOWN } else { STAIR_UP };
        if o2 != O_STAIRS || s2 & back == 0 {
            return;
        }
        let e = &mut self.ents[i];
        e.fl = next;
        e.path.clear();
        if i == 0 {
            self.dest = None;
            self.pl.pending = Pending::None;
            self.touch_ui();
            self.hud_ver += 1;
            let line = if up {
                "Thou climbest the stairs."
            } else {
                "Thou goest down the stairs."
            };
            self.msg(line, C_SYS);
        }
    }

    /// Doors swing shut once their time is up and the doorway is clear.
    pub(super) fn door_tick(&mut self, dt: f32) {
        for k in 0..self.map.doors.len() {
            let i = self.map.doors[k];
            if self.map.door[i] <= 0.0 || self.map.door[i].is_infinite() {
                continue;
            }
            self.map.door[i] -= dt;
            if self.map.door[i] <= 0.0 {
                let (x, y) = (i as i32 % W, i as i32 / W);
                let blocked = self.ents.iter().any(|e| {
                    !e.dead && e.fl == 0 && (e.tile() == (x, y) || e.path.first() == Some(&(x, y)))
                });
                self.map.door[i] = if blocked { 1.0 } else { 0.0 };
            }
        }
    }

    /// Open or shut a door the player tapped, if they are near enough.
    pub(super) fn tap_door(&mut self, x: i32, y: i32) -> bool {
        if self.ents[0].fl != 0 || self.map.obj(x, y) != O_DOOR {
            return false;
        }
        let (px, py) = self.ents[0].tile();
        if (px - x).abs().max((py - y).abs()) > 2 {
            return false;
        }
        if self.map.door_shut(x, y) {
            self.map.open_door(x, y);
        } else {
            let leaves = self.map.door_leaves(x, y);
            let blocked = self
                .ents
                .iter()
                .any(|e| !e.dead && e.fl == 0 && leaves.contains(&e.tile()));
            if blocked {
                self.msg("Something is in the way.", C_SYS);
            } else {
                self.map.shut_door(x, y);
            }
        }
        true
    }

    /// The building the player is in and the storey they are on, if inside.
    pub fn floor_view(&self) -> Option<(usize, u8)> {
        let (x, y) = self.ents[0].tile();
        self.map.building_at(x, y).map(|b| (b, self.ents[0].fl))
    }

    /// Whether an entity can be seen from where the player stands: on an
    /// upper floor only from that floor, and nothing on lower floors of the
    /// building the player is upstairs in.
    pub fn ent_visible(&self, e: &Ent) -> bool {
        if e.k == EK::Player {
            return true;
        }
        let (x, y) = e.tile();
        let here = self.map.building_at(x, y);
        match self.floor_view() {
            Some((b, f)) if here == Some(b) => e.fl == f,
            _ => e.fl == 0,
        }
    }

    /// The tile under a tap: on the player's upper floor first, if they are
    /// upstairs.
    pub(super) fn tap_tile(&self, sx: f32, sy: f32) -> (f32, f32) {
        if let Some((b, f)) = self.floor_view() {
            if f > 0 {
                let lift = f as f32 * crate::render::STOREY
                    + self
                        .map
                        .surface(self.map.buildings[b].x + 1, self.map.buildings[b].y + 1);
                let (wx, wy) = self.to_world(sx, sy + lift * crate::render::LH * self.zoom);
                let (tx, ty) = (wx.round() as i32, wy.round() as i32);
                if self.map.buildings[b].room(tx, ty) {
                    return (wx, wy);
                }
            }
        }
        self.to_world_ground(sx, sy)
    }
}
