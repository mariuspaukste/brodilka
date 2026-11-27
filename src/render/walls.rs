//! Thin walls: a house's walls stand on the outer edges of its footprint
//! (see `Map::thin_walls`), each a slab a fraction of a tile thick, so the
//! rooms inside run right up to them.
//!
//! A wall belongs to the tile of the house's outer ring it bounds. The back
//! walls (toward -y and -x) are drawn with that tile, so whoever stands on
//! it is drawn in front of them; the front walls (toward +y and +x) are
//! drawn with the tile just outside, so they hide whoever stands inside and
//! are hidden by whoever stands out in the street. Their outward faces sit
//! where a thick wall's did, and carry its windows, timbers and torches.

use super::*;

/// How thick a wall is, in tiles.
const THICK: f32 = 0.16;
/// The height of a doorway, as a share of a storey.
const DOOR_H: f32 = 0.82;

#[derive(Clone, Copy, PartialEq)]
enum Side {
    N,
    W,
    S,
    E,
}

impl Game {
    /// Draw the thin walls drawn with tile (x, y): the back walls of the
    /// house it is in, and the front walls of the houses it is just
    /// outside of. `floor` is the storey the player stands on in the house
    /// of the back walls, when upstairs in it: those are drawn from there up.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn thin_walls_at(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        near_front: bool,
        back: bool,
        floor: u8,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        if back {
            if let Some(b) = self.map.thin_at(x, y) {
                if y == b.y {
                    self.thin_wall(c, x, y, Side::N, b.style, floor, false, z, t, lights);
                }
                if x == b.x {
                    self.thin_wall(c, x, y, Side::W, b.style, floor, false, z, t, lights);
                }
            }
            return;
        }
        let here = self.map.building_at(x, y);
        // the front walls of a house just behind this tile, faded when the
        // player is close behind them
        for (ox, oy, side) in [(x, y - 1, Side::S), (x - 1, y, Side::E)] {
            let Some(bi) = self.map.building_at(ox, oy) else {
                continue;
            };
            let b = &self.map.buildings[bi];
            if here == Some(bi) || !b.thin() {
                continue;
            }
            let fade = near_front;
            if fade {
                c.alpha(0.35);
                c.mute_ink = true;
            }
            self.thin_wall(c, ox, oy, side, b.style, 0, fade, z, t, lights);
            if fade {
                c.alpha(1.0);
                c.mute_ink = false;
            }
        }
    }

    /// One tile's run of wall on one side of ring tile (x, y), from storey
    /// `floor` up to the top of what is drawn of the house.
    #[allow(clippy::too_many_arguments)]
    fn thin_wall(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        side: Side,
        style: u8,
        floor: u8,
        faded: bool,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        let Some(bi) = self.map.building_at(x, y) else {
            return;
        };
        let b = &self.map.buildings[bi];
        let storeys = self.wall_storeys(x, y);
        let hgt = STOREY_PX * z * wall_h(style) * storeys as f32;
        let (sx, sy) = self.scr_h(x as f32, y as f32, self.map.surface(x, y));
        let (hw, hh) = (32.0 * z, 16.0 * z);
        // a point on the tile: u along x, v along y from its centre, h up
        let at = |u: f32, v: f32, h: f32| (sx + (u - v) * hw, sy + (u + v) * hh - h);
        let th = THICK;
        let door = self.map.obj(x, y) == O_DOOR;
        // the slab: u0..u1 along x, v0..v1 along y
        let (u0, u1, v0, v1) = match side {
            Side::N => (-0.5, 0.5, -0.5, -0.5 + th),
            Side::W => (
                -0.5,
                -0.5 + th,
                if y == b.y { -0.5 + th } else { -0.5 },
                0.5,
            ),
            Side::S => (-0.5, 0.5, 0.5 - th, 0.5),
            Side::E => (0.5 - th, 0.5, -0.5, 0.5),
        };
        let lo = floor as f32 * STOREY_PX * z;
        let ink = self.pix.max(z);
        let tex = wall_tex(style);
        let inner = matches!(side, Side::N | Side::W);
        // a doorway leaves the wall standing only over the door
        let bottom = if door && !inner {
            DOOR_H * STOREY_PX * z
        } else {
            lo
        };
        // the visible upright face: south-facing (along x) or east-facing
        let along_x = matches!(side, Side::N | Side::S);
        let face = |h0: f32, h1: f32| -> [(f32, f32); 4] {
            if along_x {
                [
                    at(u0, v1, h0),
                    at(u1, v1, h0),
                    at(u1, v1, h1),
                    at(u0, v1, h1),
                ]
            } else {
                [
                    at(u1, v1, h0),
                    at(u1, v0, h0),
                    at(u1, v0, h1),
                    at(u1, v1, h1),
                ]
            }
        };
        // the end of a run, where a doorway opens beside it
        let cap = |h0: f32, h1: f32| -> [(f32, f32); 4] {
            if along_x {
                [
                    at(u1, v1, h0),
                    at(u1, v0, h0),
                    at(u1, v0, h1),
                    at(u1, v1, h1),
                ]
            } else {
                [
                    at(u0, v1, h0),
                    at(u1, v1, h0),
                    at(u1, v1, h1),
                    at(u0, v1, h1),
                ]
            }
        };
        let open_end = match side {
            Side::S => self.map.obj(x + 1, y) == O_DOOR && !door,
            Side::E => self.map.obj(x, y + 1) == O_DOOR && !door,
            _ => false,
        };
        let top = [
            at(u0, v0, hgt),
            at(u1, v0, hgt),
            at(u1, v1, hgt),
            at(u0, v1, hgt),
        ];
        if style == WALL_HAVELI {
            // lime-wash runs on unbroken from tile to tile
            c.outline(0x12100c30, ink);
        } else {
            c.outline(INK, ink);
        }
        self.face_pat(c, tex, !along_x);
        let f = face(bottom, hgt);
        c.poly(&f);
        c.outline(0, 0.0);
        // shading: the south faces darker than the east, the inside of a
        // house darker still, and soot toward the ground
        let shade_a = match (style, along_x, inner) {
            (WALL_ELVEN, _, _) => 0,
            (WALL_DWARF, true, _) => 0x10141aa8,
            (WALL_DWARF, false, _) => 0x10141a70,
            (4, true, _) => 0xa0,
            (4, false, _) => 0x70,
            (_, true, false) => 0x68,
            (_, false, false) => 0x28,
            (_, true, true) => 0x78,
            (_, false, true) => 0x50,
        };
        if style == WALL_ELVEN {
            c.fill(if along_x { 0xe8ecf070 } else { 0xf0f4f890 });
            c.poly(&f);
            if along_x {
                c.fill(0x2a3a5020);
                c.poly(&f);
            }
        } else {
            c.fill(shade_a);
            c.poly(&f);
        }
        if style == WALL_DWARF {
            // a band of bronze knotwork high on each face, where the old
            // runes are cut
            let band = hgt * 0.8;
            if band > bottom {
                let q = face(band, band);
                c.stroke(0x8a6a2aff, 2.0 * ink);
                c.line(q[0].0, q[0].1, q[1].0, q[1].1);
                c.stroke(0xd8b050c0, ink);
                c.line(q[0].0, q[0].1 - z, q[1].0, q[1].1 - z);
            }
        }
        if bottom < hgt * 0.3 {
            c.fill(0x0a060430);
            c.poly(&face(bottom, hgt * 0.3));
        }
        if open_end {
            // the reveal of the doorway: the wall's cut end, lit from the side
            c.outline(INK, ink);
            self.face_pat(c, tex, along_x);
            let k = cap(0.0, hgt);
            c.poly(&k);
            c.outline(0, 0.0);
            c.fill(if along_x { 0x00000038 } else { 0x00000060 });
            c.poly(&k);
        }
        // the top of the wall, a thin strip
        c.outline(INK, ink);
        self.pat(c, tex, hgt);
        c.poly(&top);
        c.outline(0, 0.0);
        c.fill(0x0c080660);
        c.poly(&top);
        c.outline(INK, ink);

        // the face's own tile, as a thick wall's left (along x) or right
        // face would have it
        let (fx, fy) = match side {
            Side::N => (sx + (1.0 - th) * hw, sy - (1.0 - th) * hh),
            Side::W => (sx - (1.0 - th) * hw, sy - (1.0 - th) * hh),
            _ => (sx, sy),
        };
        let mask = if along_x { 1 } else { 2 };
        if door && !inner {
            self.thin_door(c, x, y, side, &at, (u0, u1, v0, v1), z);
        } else {
            if style == 3 {
                // half-timbering, outside and in
                self.timbers_on(c, x, y, fx, fy, storeys, z, mask, floor as f32);
            }
            let p = P {
                x: sx,
                y: sy,
                s: z,
                f: 1.0,
            };
            let out = side == Side::S
                && self.map.tile(x, y + 1) != T_FLOOR
                && !matches!(self.map.obj(x, y + 1), O_WALL | O_DOOR);
            if out && (x + y) % 3 == 0 && style != WALL_TEMPLE && style != 4 {
                for k in 0..storeys {
                    if south::southern(style) {
                        self.jharokha(c, x, y, &p, k, style, z, lights);
                    } else {
                        self.house_window(c, x, y, &p, k, style, z, lights);
                    }
                }
            }
            if !inner && !faded {
                let m = if side == Side::S { 2 } else { 1 };
                self.wall_deco_on(c, x, y, sx, sy, style, z, t, lights, m);
            }
        }
        // the shade under the eaves, outside; a beam where each storey
        // meets the next
        c.outline(0, 0.0);
        if !inner && style != 4 && style != WALL_ELVEN {
            let e = 5.0 * z;
            c.fill(0x00000050);
            c.poly(&face(hgt - e, hgt));
        }
        if storeys > 1 {
            c.stroke(0x2a1a10ff, 1.5 * ink);
            for k in 1..storeys {
                let up = k as f32 * STOREY_PX * z;
                if up <= bottom {
                    continue;
                }
                let q = face(up, up);
                c.line(q[0].0, q[0].1, q[1].0, q[1].1);
            }
        }
        c.outline(INK, ink);
    }

    /// A shut door in a thin wall: a plank leaf filling the doorway, with
    /// its frame. (An open one is drawn swung in, with the door's own tile.)
    #[allow(clippy::too_many_arguments)]
    fn thin_door(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        side: Side,
        at: &dyn Fn(f32, f32, f32) -> (f32, f32),
        (u0, u1, v0, v1): (f32, f32, f32, f32),
        z: f32,
    ) {
        let ink = self.pix.max(z);
        let dh = DOOR_H * STOREY_PX * z;
        let along_x = side == Side::S;
        // the doorway's back: the gloom of the house beyond
        let (fu, fv) = if along_x {
            (0.0, (v0 + v1) * 0.5)
        } else {
            ((u0 + u1) * 0.5, 0.0)
        };
        let pt = |s: f32, h: f32| {
            if along_x {
                at(u0 + (u1 - u0) * s, fv, h)
            } else {
                at(fu, v1 + (v0 - v1) * s, h)
            }
        };
        if self.map.door_shut(x, y) {
            c.outline(INK, ink);
            c.fill(if along_x { 0x4a2e18ff } else { 0x5a3a20ff });
            c.poly(&[pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, dh), pt(0.0, dh)]);
            c.outline(0, 0.0);
            // planks, an iron strap or two, and a ring on the meeting side
            c.stroke(0x2a180cff, ink);
            for k in 1..4 {
                let s = k as f32 / 4.0;
                let (a, b) = (pt(s, 0.0), pt(s, dh - 1.0 * z));
                c.line(a.0, a.1, b.0, b.1);
            }
            c.stroke(0x2e2e30ff, 1.2 * ink);
            for hh in [0.22, 0.72] {
                let (a, b) = (pt(0.05, dh * hh), pt(0.95, dh * hh));
                c.line(a.0, a.1, b.0, b.1);
            }
            let meet = if self.map.obj(x - 1, y) == O_DOOR || self.map.obj(x, y - 1) == O_DOOR {
                0.15
            } else {
                0.85
            };
            let (rx, ry) = pt(meet, dh * 0.45);
            c.fill(0x8a7a50ff);
            c.ell(rx, ry, 1.2 * z, 1.2 * z);
        }
        // the lintel beam over the doorway
        c.outline(0, 0.0);
        c.stroke(0x2a1a10ff, 2.0 * ink);
        let fa = if along_x {
            at(u0, v1, dh)
        } else {
            at(u1, v1, dh)
        };
        let fb = if along_x {
            at(u1, v1, dh)
        } else {
            at(u1, v0, dh)
        };
        c.line(fa.0, fa.1, fb.0, fb.1);
        c.outline(INK, ink);
    }
}
