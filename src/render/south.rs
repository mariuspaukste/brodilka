//! Drawing the south's buildings: flat roof terraces behind parapets, domed
//! kiosks (chhatris) at their corners, the palace's great white dome, the
//! temple's curved tower (shikhara) with its ribbed crown and saffron flag,
//! and arched, hooded windows (jharokhas).

use super::*;
use std::f32::consts::PI;

const MARBLE: u32 = 0xe0d8c8ff;
const SANDSTONE: u32 = 0x9a5038ff;
const GILT: u32 = 0xd8a840ff;
const SAFFRON: u32 = 0xe8801aff;

/// the south's wall styles
pub fn southern(style: u8) -> bool {
    matches!(
        style & !FACE_S,
        WALL_FORT | WALL_BASTION | WALL_HAVELI | WALL_PALACE | WALL_TEMPLE
    )
}

/// An onion dome standing on (cx, cy): `r` its half-width, `h` its height.
fn onion(c: &mut Canvas, cx: f32, cy: f32, r: f32, h: f32, col: u32, ink: f32) {
    // the profile from the base up: swelling, then drawing in to a point
    let prof = [
        (1.0f32, 0.0f32),
        (1.12, 0.22),
        (1.08, 0.45),
        (0.82, 0.66),
        (0.45, 0.84),
        (0.15, 0.95),
        (0.0, 1.0),
    ];
    let mut pts: Vec<(f32, f32)> = prof
        .iter()
        .map(|&(w, v)| (cx + w * r, cy - v * h))
        .collect();
    pts.extend(prof.iter().rev().map(|&(w, v)| (cx - w * r, cy - v * h)));
    c.outline(INK, ink);
    c.fill(col);
    c.poly(&pts);
    c.outline(0, 0.0);
    // the shaded right side and a highlight on the left
    let right: Vec<(f32, f32)> = prof
        .iter()
        .map(|&(w, v)| (cx + w * r, cy - v * h))
        .chain(
            prof.iter()
                .rev()
                .map(|&(w, v)| (cx + w * r * 0.25, cy - v * h)),
        )
        .collect();
    c.fill(0x00000040);
    c.poly(&right);
    c.fill(0xffffff40);
    c.ell(cx - r * 0.45, cy - h * 0.42, r * 0.18, h * 0.2);
    // ribs
    c.stroke(shade(col, 0.75), ink);
    for k in [-0.5f32, 0.5] {
        c.line(cx + k * r, cy, cx + k * r * 0.2, cy - h * 0.9);
    }
    // the gilt finial
    c.stroke(GILT, 1.5 * ink);
    c.line(cx, cy - h, cx, cy - h * 1.3);
    c.fill(GILT);
    c.ell(cx, cy - h * 1.15, r * 0.12 + ink, r * 0.1 + ink * 0.6);
}

impl Game {
    /// The roof of a southern building, if this is one; true when drawn.
    pub(super) fn south_roof(&self, c: &mut Canvas, b: &Building, z: f32, a: f32) -> bool {
        if !matches!(b.style, WALL_HAVELI | WALL_PALACE | WALL_TEMPLE) {
            return false;
        }
        let ink = self.pix.max(z);
        let (bw, bh) = (b.w - 1, b.h - 1);
        let eave = STOREY_PX * z * wall_h(b.style) * b.storeys() as f32;
        let floor = self.map.level(b.x + 1, b.y + 1) as f32 * LH * z;
        let sp = |x: f32, y: f32, h: f32| {
            let (p, q) = self.to_screen(x, y);
            (p, q - h - floor)
        };
        // the walls' outer faces run half a tile out from the wall tiles' centres
        let (x0, y0, x1, y1) = (
            b.x as f32 - 0.5,
            b.y as f32 - 0.5,
            (b.x + bw) as f32 + 0.5,
            (b.y + bh) as f32 + 0.5,
        );
        if a < 1.0 {
            c.alpha(a);
        }
        let (wall, terrace) = match b.style {
            WALL_HAVELI => (0xc8b48aff, 0xb8a47aff),
            _ => (SANDSTONE, 0x8a4a34ff),
        };
        // the roof terrace: plastered on a haveli, flagged on sandstone
        c.outline(INK, ink);
        self.pat(
            c,
            if b.style == WALL_HAVELI {
                TEX_LIMEWASH
            } else {
                TEX_REDFLAG
            },
            eave + floor,
        );
        c.poly(&[
            sp(x0, y0, eave),
            sp(x1, y0, eave),
            sp(x1, y1, eave),
            sp(x0, y1, eave),
        ]);
        c.outline(0, 0.0);
        c.fill(with_a(shade(terrace, 0.5), 0.35));
        c.poly(&[
            sp(x0, y0, eave),
            sp(x1, y0, eave),
            sp(x1, y1, eave),
            sp(x0, y1, eave),
        ]);
        c.fill(0x00000020);
        let inset = 0.35;
        c.poly(&[
            sp(x0 + inset, y0 + inset, eave),
            sp(x1 - inset, y0 + inset, eave),
            sp(x1 - inset, y1 - inset, eave),
            sp(x0 + inset, y1 - inset, eave),
        ]);
        // the parapet: the back runs first (their inner faces seen), the
        // front ones last, a jali of small openings along each
        let ph = 10.0 * z;
        let th = 0.18;
        let wall_run = |c: &mut Canvas, p: (f32, f32), q: (f32, f32), k: f32| {
            let quad = [
                sp(p.0, p.1, eave),
                sp(q.0, q.1, eave),
                sp(q.0, q.1, eave + ph),
                sp(p.0, p.1, eave + ph),
            ];
            c.outline(INK, ink);
            c.fill(shade(wall, k));
            c.poly(&quad);
            c.outline(0, 0.0);
            // the pierced screen: a row of small dark openings
            let n = (((q.0 - p.0).abs() + (q.1 - p.1).abs()) * 2.0) as i32;
            c.fill(0x1a100c90);
            for i in 0..n {
                let f = (i as f32 + 0.5) / n as f32;
                let (u, v) = (p.0 + (q.0 - p.0) * f, p.1 + (q.1 - p.1) * f);
                let (sx, sy) = sp(u, v, eave + ph * 0.5);
                c.ell(sx, sy, 1.4 * z, 2.6 * z);
            }
            // the coping
            c.stroke(shade(wall, 1.2), ink);
            let (m, n2) = (sp(p.0, p.1, eave + ph), sp(q.0, q.1, eave + ph));
            c.line(m.0, m.1, n2.0, n2.1);
        };
        wall_run(c, (x0 + th, y0 + th), (x1 - th, y0 + th), 0.7);
        wall_run(c, (x0 + th, y0 + th), (x0 + th, y1 - th), 0.85);

        // what stands on the roof
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        match b.style {
            WALL_TEMPLE => self.shikhara(c, cx, y0 + 2.2, 1.45, eave, floor, z),
            WALL_PALACE => {
                // a drum and the great dome, kiosks at the corners
                for (u, v) in [(x0 + 1.0, y0 + 1.0), (x1 - 1.0, y0 + 1.0)] {
                    self.chhatri(c, sp(u, v, eave), z, MARBLE);
                }
                // a raised marble hall in the middle of the terrace
                let (hx0, hy0, hx1, hy1) = (cx - 2.2, cy - 1.8, cx + 2.2, cy + 1.4);
                let hh = 22.0 * z;
                c.outline(INK, ink);
                c.fill(shade(MARBLE, 0.8));
                c.poly(&[
                    sp(hx0, hy1, eave),
                    sp(hx1, hy1, eave),
                    sp(hx1, hy1, eave + hh),
                    sp(hx0, hy1, eave + hh),
                ]);
                c.fill(shade(MARBLE, 0.62));
                c.poly(&[
                    sp(hx1, hy0, eave),
                    sp(hx1, hy1, eave),
                    sp(hx1, hy1, eave + hh),
                    sp(hx1, hy0, eave + hh),
                ]);
                c.fill(MARBLE);
                c.poly(&[
                    sp(hx0, hy0, eave + hh),
                    sp(hx1, hy0, eave + hh),
                    sp(hx1, hy1, eave + hh),
                    sp(hx0, hy1, eave + hh),
                ]);
                // arcades along its faces
                c.outline(0, 0.0);
                c.fill(0x3a2a2080);
                for k in 0..5 {
                    let u = hx0 + 0.45 + k as f32 * 0.85;
                    let (ax, ay) = sp(u, hy1, eave + 3.0 * z);
                    c.rect(ax - 3.0 * z, ay - 14.0 * z, 6.0 * z, 12.0 * z);
                    c.ell(ax, ay - 14.0 * z, 3.0 * z, 3.0 * z);
                }
                for k in 0..4 {
                    let v = hy0 + 0.45 + k as f32 * 0.85;
                    let (ax, ay) = sp(hx1, v, eave + 3.0 * z);
                    c.rect(ax - 3.0 * z, ay - 14.0 * z, 6.0 * z, 12.0 * z);
                    c.ell(ax, ay - 14.0 * z, 3.0 * z, 3.0 * z);
                }
                // the drum and the great dome
                let (dx, dy) = sp(cx, cy - 0.2, eave + hh);
                c.outline(INK, ink);
                c.fill(MARBLE);
                c.rect(dx - 38.0 * z, dy - 20.0 * z, 76.0 * z, 20.0 * z);
                c.outline(0, 0.0);
                c.fill(0x00000030);
                c.rect(dx, dy - 20.0 * z, 38.0 * z, 20.0 * z);
                c.fill(0x5a3a2a90);
                for k in 0..9 {
                    let ax = dx - 34.0 * z + k as f32 * 8.2 * z;
                    c.rect(ax, dy - 15.0 * z, 3.0 * z, 10.0 * z);
                }
                onion(c, dx, dy - 20.0 * z, 40.0 * z, 70.0 * z, MARBLE, ink);
            }
            _ => {
                // a kiosk on the front corner of most houses, two on some
                let hv = h2(b.x, b.y, 91);
                // the head of the stair: a little whitewashed hut with a door
                let (ux0, uy0, ux1, uy1) = (x0 + 0.5, y0 + 0.5, x0 + 1.7, y0 + 1.6);
                let uh = 16.0 * z;
                c.outline(INK, ink);
                c.fill(shade(wall, 0.85));
                c.poly(&[
                    sp(ux0, uy1, eave),
                    sp(ux1, uy1, eave),
                    sp(ux1, uy1, eave + uh),
                    sp(ux0, uy1, eave + uh),
                ]);
                c.fill(shade(wall, 0.65));
                c.poly(&[
                    sp(ux1, uy0, eave),
                    sp(ux1, uy1, eave),
                    sp(ux1, uy1, eave + uh),
                    sp(ux1, uy0, eave + uh),
                ]);
                c.fill(shade(wall, 1.1));
                c.poly(&[
                    sp(ux0, uy0, eave + uh),
                    sp(ux1, uy0, eave + uh),
                    sp(ux1, uy1, eave + uh),
                    sp(ux0, uy1, eave + uh),
                ]);
                c.outline(0, 0.0);
                c.fill(0x2a1a10ff);
                let (dxa, dya) = sp(ux0 + 0.6, uy1, eave);
                c.rect(dxa - 3.0 * z, dya - 11.0 * z, 6.0 * z, 9.0 * z);
                // water pots and a string of washing, now and then
                if hv < 0.6 {
                    for k in 0..3 {
                        let (px2, py2) = sp(x1 - 1.0 - k as f32 * 0.35, y0 + 0.8, eave);
                        c.outline(INK, ink);
                        c.fill([0xa0582eff, 0x8a4a28ff][k % 2]);
                        c.ell(px2, py2 - 3.0 * z, 3.0 * z, 3.0 * z);
                    }
                }
                if hv > 0.3 {
                    let (la, lb) = (
                        sp(x0 + 2.2, cy, eave + 12.0 * z),
                        sp(x1 - 1.6, cy, eave + 12.0 * z),
                    );
                    c.stroke(0x2a1a10a0, ink);
                    c.line(la.0, la.1, lb.0, lb.1);
                    c.outline(0, 0.0);
                    for k in 0..5 {
                        let f = 0.12 + k as f32 * 0.18;
                        let (wx, wy) = (la.0 + (lb.0 - la.0) * f, la.1 + (lb.1 - la.1) * f);
                        c.fill(
                            [0xc83a2aff, 0xe8d8a0ff, 0x2a5aa0ff, 0xe8a020ff, 0x3a8a4aff]
                                [(k + (hv * 9.0) as usize) % 5],
                        );
                        c.rect(wx - 2.0 * z, wy, 4.0 * z, 7.0 * z);
                    }
                }
                self.chhatri(c, sp(x1 - 1.0, y1 - 1.0, eave), z, 0xd8c8a8ff);
                if hv > 0.5 {
                    self.chhatri(c, sp(x0 + 1.0, y1 - 1.0, eave), z, 0xd8c8a8ff);
                }
            }
        }
        if b.style == WALL_PALACE {
            for (u, v) in [(x1 - 1.0, y1 - 1.0), (x0 + 1.0, y1 - 1.0)] {
                self.chhatri(c, sp(u, v, eave), z, MARBLE);
            }
        }
        wall_run(c, (x1 - th, y0 + th), (x1 - th, y1 - th), 0.6);
        wall_run(c, (x0 + th, y1 - th), (x1 - th, y1 - th), 1.0);
        if a < 1.0 {
            c.alpha(1.0);
        }
        c.outline(INK, ink);
        true
    }

    /// A chhatri: a little domed pavilion on four slim pillars.
    fn chhatri(&self, c: &mut Canvas, (sx, sy): (f32, f32), z: f32, col: u32) {
        let ink = self.pix.max(z);
        let (r, ph) = (14.0 * z, 20.0 * z);
        // the platform
        c.outline(INK, ink);
        c.fill(shade(col, 0.8));
        c.poly(&[
            (sx - r - 2.0 * z, sy),
            (sx, sy + 8.0 * z),
            (sx + r + 2.0 * z, sy),
            (sx, sy - 8.0 * z),
        ]);
        // pillars, the back pair darker
        c.outline(0, 0.0);
        for (dx, dy, k) in [
            (-0.7f32, -0.4f32, 0.6f32),
            (0.7, -0.4, 0.6),
            (-0.7, 0.4, 1.0),
            (0.7, 0.4, 0.85),
        ] {
            c.ink_line(
                (sx + dx * r, sy + dy * 8.0 * z),
                (sx + dx * r, sy + dy * 8.0 * z - ph),
                shade(col, k),
                1.6 * z,
            );
        }
        // the eave slab, then the dome
        c.outline(INK, ink);
        c.fill(col);
        c.poly(&[
            (sx - r - 3.0 * z, sy - ph),
            (sx, sy - ph + 8.0 * z),
            (sx + r + 3.0 * z, sy - ph),
            (sx, sy - ph - 8.0 * z),
        ]);
        onion(c, sx, sy - ph - 1.0 * z, r * 0.85, 18.0 * z, col, ink);
    }

    /// A temple's shikhara: a square tower whose sides curve in as it
    /// climbs, ringed course by course, crowned with a ribbed amalaka, a
    /// gilt pot and a saffron flag. (cx, cy) is its centre in tiles,
    /// `s` its half-width at the base, `base` the roof's height in pixels.
    #[allow(clippy::too_many_arguments)]
    fn shikhara(&self, c: &mut Canvas, cx: f32, cy: f32, s: f32, base: f32, floor: f32, z: f32) {
        let ink = self.pix.max(z);
        let tall = 175.0 * z;
        let n = 16;
        let sp = |u: f32, v: f32, h: f32| {
            let (p, q) = self.to_screen(cx + u, cy + v);
            (p, q - h - floor)
        };
        // half-width at height fraction f: curving in, faster near the top
        // a beehive: nearly upright at first, drawing in ever faster
        let half = |f: f32| s * (1.0 - 0.8 * f.powf(2.4));
        for i in 0..n {
            let (f0, f1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
            let (h0, h1) = (base + tall * f0, base + tall * f1);
            let (w0, w1) = (half(f0), half(f1));
            // each course juts a little at its foot
            let lip = if i % 2 == 0 { 0.04 } else { 0.0 };
            let (w0, w1) = (w0 + lip, w1);
            let k = if i % 2 == 0 { 1.0 } else { 0.9 };
            // the face towards +y (seen on the left) and towards +x (right)
            let left = [
                sp(-w0, w0, h0),
                sp(w0, w0, h0),
                sp(w1, w1, h1),
                sp(-w1, w1, h1),
            ];
            let right = [
                sp(w0, w0, h0),
                sp(w0, -w0, h0),
                sp(w1, -w1, h1),
                sp(w1, w1, h1),
            ];
            c.outline(INK, ink);
            c.fill(shade(SANDSTONE, 0.95 * k));
            c.poly(&left);
            c.fill(shade(SANDSTONE, 0.68 * k));
            c.poly(&right);
            c.outline(0, 0.0);
            // the central rib (ratha) standing out up each face, lit and
            // shaded, and the corner ribs
            let rib = |u0: f32, u1: f32, w0: f32, w1: f32, left: bool| {
                if left {
                    [
                        sp(u0 * w0, w0 + 0.02, h0),
                        sp(u1 * w0, w0 + 0.02, h0),
                        sp(u1 * w1, w1 + 0.02, h1),
                        sp(u0 * w1, w1 + 0.02, h1),
                    ]
                } else {
                    [
                        sp(w0 + 0.02, u0 * w0, h0),
                        sp(w0 + 0.02, u1 * w0, h0),
                        sp(w1 + 0.02, u1 * w1, h1),
                        sp(w1 + 0.02, u0 * w1, h1),
                    ]
                }
            };
            c.fill(shade(SANDSTONE, 1.12 * k));
            c.poly(&rib(-0.3, 0.3, w0, w1, true));
            c.fill(shade(SANDSTONE, 0.78 * k));
            c.poly(&rib(-0.3, 0.3, w0, w1, false));
            c.fill(0x1a0c0840);
            c.poly(&rib(0.3, 0.42, w0, w1, true));
            c.poly(&rib(0.3, 0.42, w0, w1, false));
            // small arched niches on the lower courses
            if i % 3 == 1 && i < n - 5 {
                c.fill(0x2a140cc0);
                let (a, _) = (sp(0.0, w0, h0 + 3.0 * z), 0);
                c.rect(a.0 - 2.0 * z, a.1 - 6.0 * z, 4.0 * z, 5.0 * z);
                let (a2, _) = (sp(w0, 0.0, h0 + 3.0 * z), 0);
                c.rect(a2.0 - 2.0 * z, a2.1 - 6.0 * z, 4.0 * z, 5.0 * z);
            }
        }
        // the amalaka: a ribbed stone disc, then the gilt pot
        let top = base + tall;
        let (ax, ay) = sp(0.0, 0.0, top);
        let ar = half(1.0) * 32.0 * z * 1.4;
        c.outline(INK, ink);
        c.fill(shade(SANDSTONE, 0.9));
        c.ell(ax, ay - 3.0 * z, ar, 5.0 * z);
        c.outline(0, 0.0);
        c.stroke(shade(SANDSTONE, 0.55), ink);
        for k in -3..=3 {
            let u = k as f32 / 3.5 * ar;
            c.line(ax + u, ay - 7.0 * z, ax + u * 1.05, ay + 1.0 * z);
        }
        c.outline(INK, ink);
        c.fill(GILT);
        c.ell(ax, ay - 11.0 * z, 4.0 * z, 4.0 * z);
        c.poly(&[
            (ax - 1.5 * z, ay - 14.0 * z),
            (ax + 1.5 * z, ay - 14.0 * z),
            (ax, ay - 20.0 * z),
        ]);
        // the flag on its pole, rippling
        let (fx, fy) = (ax + 2.0 * z, ay - 20.0 * z);
        c.ink_line((fx, fy + 6.0 * z), (fx, fy - 20.0 * z), 0x4a3020ff, 1.2 * z);
        let wv = (self.clock * 4.0).sin() * 2.0 * z;
        c.fill(SAFFRON);
        c.poly(&[
            (fx, fy - 20.0 * z),
            (fx + 16.0 * z, fy - 16.0 * z + wv),
            (fx, fy - 11.0 * z),
        ]);
        let _ = PI;
    }

    /// An arched, hooded window (a jharokha) on a southern wall face: a
    /// carved frame, a pointed arch over a lattice, a sloping hood on two
    /// brackets, and a little dome on top for the grander houses.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn jharokha(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        p: &P,
        k: u8,
        style: u8,
        z: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        let up = k as f32 * STOREY_PX;
        let at = |u: f32, h: f32| p.at(u - 32.0, u * 0.5 - h - up);
        let ink = self.pix.max(z);
        let lit = self.daylight().0 > 0.18 && (x * 5 + y * 3 + k as i32) % 3 != 0;
        let frame = if style == WALL_HAVELI {
            0xb07a3aff
        } else {
            0xe0d0b8ff
        };
        c.outline(INK, ink);
        // the frame and the arch
        c.fill(frame);
        c.poly(&[
            at(9.0, 6.0),
            at(23.0, 6.0),
            at(23.0, 22.0),
            at(20.0, 27.0),
            at(16.0, 30.0),
            at(12.0, 27.0),
            at(9.0, 22.0),
        ]);
        c.outline(0, 0.0);
        c.fill(if lit { 0xe8a040ff } else { 0x1a1410ff });
        c.poly(&[
            at(11.0, 8.0),
            at(21.0, 8.0),
            at(21.0, 21.0),
            at(19.0, 25.0),
            at(16.0, 27.5),
            at(13.0, 25.0),
            at(11.0, 21.0),
        ]);
        // the lattice
        c.stroke(shade(frame, 0.7), ink);
        for u in [13.5f32, 16.0, 18.5] {
            let (a, bq) = (at(u, 8.0), at(u, 23.0));
            c.line(a.0, a.1, bq.0, bq.1);
        }
        for h in [12.0f32, 16.0, 20.0] {
            let (a, bq) = (at(11.0, h), at(21.0, h));
            c.line(a.0, a.1, bq.0, bq.1);
        }
        // the sill on brackets, and the hood (chhajja) above
        c.outline(INK, ink);
        c.fill(shade(frame, 0.8));
        c.poly(&[at(7.0, 4.0), at(25.0, 4.0), at(25.0, 6.0), at(7.0, 6.0)]);
        for u in [9.0f32, 22.0] {
            c.poly(&[at(u, 4.0), at(u + 1.5, 4.0), at(u + 1.5, 0.5), at(u, 2.0)]);
        }
        c.fill(shade(frame, 1.08));
        c.poly(&[at(6.0, 31.0), at(26.0, 31.0), at(28.0, 28.0), at(4.0, 28.0)]);
        if style == WALL_PALACE || h2(x, y, 93) > 0.6 {
            let (dx, dy) = at(16.0, 31.0);
            onion(c, dx, dy, 5.0 * z, 7.0 * z, frame, ink);
        }
        if lit {
            let (gx, gy) = at(16.0, 16.0);
            c.alpha(0.3);
            c.glow(gx, gy, 10.0 * z, 0xffa040ff);
            c.alpha(1.0);
            lights.push((gx, gy, 20.0, 0xffa050ff));
        }
        c.outline(INK, ink);
    }
}
