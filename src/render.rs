//! Isometric renderer. Produces a compact command buffer that the JS host
//! replays onto a 2D canvas (no wasm-bindgen needed).
use crate::data::*;
use crate::game::*;
use crate::world::*;

#[derive(Default)]
pub struct Canvas {
    pub cmd: Vec<u32>,
    pub strs: Vec<u8>,
    /// width of ink borders in CSS px (one "art pixel")
    pub ink: f32,
}

impl Canvas {
    pub fn clear(&mut self) {
        self.cmd.clear();
        self.strs.clear();
    }
    #[inline]
    fn f(&mut self, v: f32) {
        self.cmd.push(v.to_bits());
    }
    pub fn fill(&mut self, c: u32) {
        self.cmd.push(1);
        self.cmd.push(c);
    }
    pub fn stroke(&mut self, c: u32, w: f32) {
        self.cmd.push(2);
        self.cmd.push(c);
        self.f(w);
    }
    pub fn poly(&mut self, p: &[(f32, f32)]) {
        self.cmd.push(3);
        self.cmd.push(p.len() as u32);
        for &(x, y) in p {
            self.f(x);
            self.f(y);
        }
    }
    pub fn poly_s(&mut self, p: &[(f32, f32)]) {
        self.cmd.push(4);
        self.cmd.push(p.len() as u32);
        for &(x, y) in p {
            self.f(x);
            self.f(y);
        }
    }
    pub fn ell(&mut self, x: f32, y: f32, rx: f32, ry: f32) {
        self.cmd.push(5);
        self.f(x);
        self.f(y);
        self.f(rx.max(0.1));
        self.f(ry.max(0.1));
    }
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.cmd.push(6);
        self.f(x);
        self.f(y);
        self.f(w);
        self.f(h);
    }
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.cmd.push(7);
        self.f(x1);
        self.f(y1);
        self.f(x2);
        self.f(y2);
    }
    /// align: 0 centre, 1 left
    pub fn text(&mut self, x: f32, y: f32, size: f32, align: u32, s: &str, outline: u32) {
        let off = self.strs.len() as u32;
        self.strs.extend_from_slice(s.as_bytes());
        self.cmd.push(8);
        self.f(x);
        self.f(y);
        self.f(size);
        self.cmd.push(align);
        self.cmd.push(off);
        self.cmd.push(s.len() as u32);
        self.cmd.push(outline);
    }
    /// a line with a dark ink border, for limbs and weapons
    pub fn ink_line(&mut self, a: (f32, f32), b: (f32, f32), col: u32, w: f32) {
        self.stroke(INK, w + 2.0 * self.ink.max(1.0));
        self.line(a.0, a.1, b.0, b.1);
        self.stroke(col, w);
        self.line(a.0, a.1, b.0, b.1);
    }
    pub fn alpha(&mut self, a: f32) {
        self.cmd.push(9);
        self.f(a);
    }
    pub fn dark(&mut self, x: f32, y: f32, r0: f32, r1: f32, a: f32, c: u32) {
        self.cmd.push(10);
        self.f(x);
        self.f(y);
        self.f(r0);
        self.f(r1);
        self.f(a);
        self.cmd.push(c);
    }
    pub fn ell_s(&mut self, x: f32, y: f32, rx: f32, ry: f32) {
        self.cmd.push(11);
        self.f(x);
        self.f(y);
        self.f(rx.max(0.1));
        self.f(ry.max(0.1));
    }
    /// While width > 0, every fill (poly/ellipse/rect) also gets an ink outline.
    pub fn outline(&mut self, c: u32, w: f32) {
        self.cmd.push(13);
        self.cmd.push(c);
        self.f(w);
    }
    pub fn glow(&mut self, x: f32, y: f32, r: f32, c: u32) {
        self.cmd.push(12);
        self.f(x);
        self.f(y);
        self.f(r);
        self.cmd.push(c);
    }
}

pub fn shade(c: u32, f: f32) -> u32 {
    let r = (((c >> 24) & 255) as f32 * f).clamp(0.0, 255.0) as u32;
    let g = (((c >> 16) & 255) as f32 * f).clamp(0.0, 255.0) as u32;
    let b = (((c >> 8) & 255) as f32 * f).clamp(0.0, 255.0) as u32;
    (r << 24) | (g << 16) | (b << 8) | (c & 255)
}
fn fi32(i: i32) -> f32 {
    i as f32
}
pub fn mix(a: u32, b: u32, k: f32) -> u32 {
    let ch = |c: u32, sh: u32| ((c >> sh) & 255) as f32;
    let m = |sh: u32| ((ch(a, sh) * (1.0 - k) + ch(b, sh) * k).clamp(0.0, 255.0) as u32) << sh;
    m(24) | m(16) | m(8) | (a & 255)
}
pub fn h2(x: i32, y: i32, k: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
        ^ (y as u32).wrapping_mul(0x1656_67b1)
        ^ (k as u32).wrapping_mul(0x9e37_79b9);
    h = (h ^ (h >> 15)).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    (h & 0xffff) as f32 / 65536.0
}
pub fn with_a(c: u32, a: f32) -> u32 {
    (c & 0xffffff00) | ((a.clamp(0.0, 1.0) * 255.0) as u32)
}

fn tile_color(t: u8) -> u32 {
    match t {
        T_WATER => 0x101a3cff,
        T_SAND => 0x7a6a4cff,
        T_GRASS => 0x434826ff,
        T_DIRT => 0x58402eff,
        T_COBBLE => 0x5a5856ff,
        T_FLOOR => 0x583a22ff,
        T_MOUNT => 0x5e5650ff,
        T_BRIDGE => 0x4a321eff,
        T_CAVE => 0x221a16ff,
        _ => 0x000000ff,
    }
}

const WALL_COLORS: [u32; 5] = [0x6a6660ff, 0x6a6660ff, 0x664830ff, 0x7a7060ff, 0x4a4642ff];
const INK: u32 = 0x120c08ff;

/// Small helper for drawing in "model units" around a screen anchor.
struct P {
    x: f32,
    y: f32,
    s: f32,
    f: f32,
}
impl P {
    #[inline]
    fn at(&self, dx: f32, dy: f32) -> (f32, f32) {
        (self.x + dx * self.f * self.s, self.y + dy * self.s)
    }
}

struct Hum {
    skin: u32,
    body: u32,
    legs: u32,
    robe: bool,
    hair: Option<u32>,
    long_hair: bool,
    helm: Option<u32>,
    hat: Option<u32>,
    weapon: u8,
    shield: Option<u32>,
    heads: u8,
    skull: bool,
    eyes: Option<u32>,
}
const HUM: Hum = Hum {
    skin: 0xe8b890ff,
    body: 0xb09070ff,
    legs: 0x4a3a2aff,
    robe: false,
    hair: None,
    long_hair: false,
    helm: None,
    hat: None,
    weapon: 0,
    shield: None,
    heads: 1,
    skull: false,
    eyes: None,
};

impl Game {
    pub fn render(&mut self) {
        let mut c = std::mem::take(&mut self.cmds);
        c.clear();
        c.ink = self.pix.max(1.0);
        self.draw(&mut c);
        self.cmds = c;
    }

    fn darkness(&self) -> f32 {
        let ph = (self.time / 480.0).fract();
        // 0..0.55 day, 0.55..0.65 dusk, 0.65..0.92 night, 0.92..1 dawn
        if ph < 0.55 {
            0.0
        } else if ph < 0.65 {
            (ph - 0.55) / 0.1 * 0.62
        } else if ph < 0.92 {
            0.62
        } else {
            (1.0 - (ph - 0.92) / 0.08) * 0.62
        }
    }
    pub fn is_night(&self) -> bool {
        self.darkness() > 0.3
    }

    fn draw(&self, c: &mut Canvas) {
        let z = self.zoom;
        let t = self.clock;
        c.fill(0x1d3a57ff);
        c.rect(0.0, 0.0, self.w, self.h);
        let cd = self.camx / 32.0;
        let cs = self.camy / 16.0;
        let dr = (self.w * 0.5 / (32.0 * z)).ceil() as i32 + 2;
        let sr = (self.h * 0.5 / (16.0 * z)).ceil() as i32 + 2;
        let (s0, s1) = (cs.floor() as i32 - sr, cs.ceil() as i32 + sr + 6);
        let (d0, d1) = (cd.floor() as i32 - dr, cd.ceil() as i32 + dr);
        let p = &self.ents[0];
        let (ps, pd) = (p.x + p.y, p.x - p.y);
        let scr = |x: i32, y: i32| self.to_screen(x as f32, y as f32);

        // ---- pass 1: flat ground
        for s in s0..=s1 {
            for d in d0..=d1 {
                if (s + d) & 1 != 0 {
                    continue;
                }
                let (x, y) = ((s + d) / 2, (s - d) / 2);
                let (sx, sy) = scr(x, y);
                let tl = self.map.tile(x, y);
                if tl == T_MOUNT {
                    continue;
                }
                let sh = if inb(x, y) {
                    self.map.shade[idx(x, y)]
                } else {
                    ((x * 31 + y * 17) & 255) as u8
                };
                self.ground(c, x, y, sx, sy, tl, sh, z, t);
            }
        }
        if let Some((dx, dy)) = self.dest {
            let (sx, sy) = scr(dx, dy);
            let k = 0.6 + (t * 6.0).sin() * 0.15;
            c.stroke(0xfff0a0cc, 1.5);
            c.poly_s(&[
                (sx, sy - 10.0 * z * k),
                (sx + 20.0 * z * k, sy),
                (sx, sy + 10.0 * z * k),
                (sx - 20.0 * z * k, sy),
            ]);
        }

        // ---- blood and gore on the ground
        for d in self.decals.iter() {
            let (dx, dy) = self.to_screen(d.x, d.y);
            if dx < -60.0 || dx > self.w + 60.0 || dy < -60.0 || dy > self.h + 60.0 {
                continue;
            }
            let k = if d.t < 0.0 {
                ((d.t + 1.5) / 1.5).clamp(0.15, 1.0)
            } else {
                1.0
            };
            let a = ((90.0 - d.t) / 25.0).clamp(0.0, 1.0) * 0.85;
            let (rx, ry) = (d.r * 32.0 * z * k, d.r * 16.0 * z * k);
            c.fill(with_a(d.color, a));
            c.ell(dx, dy, rx, ry);
            c.fill(with_a(shade(d.color, 0.6), a));
            c.ell(dx - rx * 0.2, dy + ry * 0.1, rx * 0.55, ry * 0.5);
            let j = (d.x * 13.0).sin();
            c.fill(with_a(d.color, a));
            c.ell(
                dx + rx * (1.1 + j * 0.2),
                dy - ry * 0.4,
                rx * 0.18,
                ry * 0.18,
            );
            c.ell(
                dx - rx * 1.05,
                dy + ry * (0.3 + j * 0.2),
                rx * 0.12,
                ry * 0.12,
            );
        }

        // ---- entity buckets
        let mut order: Vec<(i32, usize)> = self
            .ents
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.dead)
            .map(|(i, e)| ((e.x + e.y).round() as i32, i))
            .collect();
        order.sort_by_key(|o| o.0);
        let mut oi = 0;
        let mut lights: Vec<(f32, f32, f32, u32)> = vec![];

        // ---- pass 2: objects + entities in depth order
        for s in s0..=s1 {
            for d in d0..=d1 {
                if (s + d) & 1 != 0 {
                    continue;
                }
                let (x, y) = ((s + d) / 2, (s - d) / 2);
                if !inb(x, y) {
                    continue;
                }
                let i = idx(x, y);
                let (sx, sy) = scr(x, y);
                let tl = self.map.t[i];
                let near_front =
                    (s as f32) > ps + 0.3 && (s as f32 - ps) < 6.0 && ((d as f32) - pd).abs() < 4.5;
                if tl == T_MOUNT {
                    let a = if near_front && (d as f32 - pd).abs() < 2.5 && (s as f32 - ps) < 3.5 {
                        0.45
                    } else {
                        1.0
                    };
                    if a < 1.0 {
                        c.alpha(a);
                    }
                    self.mountain(c, x, y, sx, sy, self.map.shade[i], z);
                    if a < 1.0 {
                        c.alpha(1.0);
                    }
                }
                let o = self.map.o[i];
                if o != O_NONE {
                    let occl = match o {
                        O_WALL => near_front,
                        O_TREE => {
                            near_front && (d as f32 - pd).abs() < 2.5 && (s as f32 - ps) < 4.0
                        }
                        _ => false,
                    };
                    if occl {
                        c.alpha(0.35);
                    }
                    c.outline(INK, self.pix.max(z));
                    self.object(
                        c,
                        o,
                        x,
                        y,
                        sx,
                        sy,
                        self.map.shade[i],
                        self.map.style[i],
                        z,
                        t,
                        &mut lights,
                    );
                    c.outline(0, 0.0);
                    if occl {
                        c.alpha(1.0);
                    }
                }
            }
            for cp in self.corpses.iter().filter(|cp| cp.x + cp.y == s) {
                let (sx, sy) = scr(cp.x, cp.y);
                self.corpse(c, cp, sx, sy, z);
            }
            while oi < order.len() && order[oi].0 <= s {
                let e = &self.ents[order[oi].1];
                let (sx, sy) = self.to_screen(e.x, e.y);
                if sx > -150.0 && sx < self.w + 150.0 && sy > -150.0 && sy < self.h + 250.0 {
                    c.outline(INK, self.pix.max(z));
                    self.entity(c, e, sx, sy, z, t);
                    c.outline(0, 0.0);
                }
                oi += 1;
            }
        }
        while oi < order.len() {
            oi += 1;
        }

        // ---- effects
        for f in self.fx.iter() {
            self.effect(c, f, z, t);
        }

        // ---- Pagan-style perpetual twilight: violet haze and a dark vignette
        c.outline(0, 0.0);
        // drifting fog banks
        for i in 0..6 {
            let fi = i as f32;
            let fx = (h2(i, 3, 1) * (self.w + 500.0) + t * (5.0 + fi)) % (self.w + 500.0) - 250.0;
            let fy = h2(i, 5, 2) * self.h;
            c.fill(0xa8a09010);
            c.ell(fx, fy, 260.0 * z, 70.0 * z);
            c.ell(fx + 90.0 * z, fy + 20.0 * z, 180.0 * z, 50.0 * z);
        }
        c.fill(0x2a120a38);
        c.rect(-10.0, -10.0, self.w + 20.0, self.h + 20.0);
        c.dark(
            self.w / 2.0,
            self.h / 2.0,
            self.w.min(self.h) * 0.28,
            self.w.max(self.h) * 0.72,
            0.72,
            0x050302ff,
        );
        // falling ash and the odd ember
        let px1 = self.pix.max(1.0);
        for i in 0..44 {
            let (a, b) = (h2(i, 11, 3), h2(i, 13, 4));
            let ax = (a * self.w + t * (6.0 + b * 10.0) + (t * 0.7 + fi32(i)).sin() * 12.0)
                .rem_euclid(self.w);
            let ay = (b * self.h + t * (10.0 + a * 14.0)).rem_euclid(self.h);
            c.fill(if i % 9 == 0 {
                with_a(0xff7a30ff, 0.5 + (t * 6.0 + fi32(i)).sin() * 0.3)
            } else {
                0x9a948870
            });
            c.rect(ax, ay, 1.5 * px1, 1.5 * px1);
        }

        // ---- night
        let dk = self.darkness();
        if dk > 0.01 {
            let (px, py) = self.to_screen(p.x, p.y);
            c.dark(px, py - 20.0 * z, 50.0 * z, 300.0 * z, dk, 0x050a1eff);
            c.alpha(dk.min(1.0) * 1.4);
            for &(x, y, r, col) in lights.iter() {
                c.glow(x, y, r * z, col);
            }
            c.alpha(1.0);
        }

        // ---- overhead texts, bars, floats (above everything)
        let tid = p.target;
        for e in self.ents.iter() {
            if e.dead {
                continue;
            }
            let (sx, sy) = self.to_screen(e.x, e.y);
            if sx < -100.0 || sx > self.w + 100.0 || sy < -100.0 || sy > self.h + 200.0 {
                continue;
            }
            let size = e.mon().map(|m| m.size).unwrap_or(1.0);
            let top = sy - (44.0 * size + 8.0) * z;
            let ts = (12.0 * z.max(0.85)).min(16.0);
            let targeted = e.id == tid && tid != 0;
            let show_bar = targeted
                || (e.hp < e.mhp && e.k != EK::Player && matches!(e.k, EK::Mon(_)))
                || (e.k == EK::Player && e.hp < e.mhp && !self.pl.ghost);
            if show_bar {
                let bw = 30.0 * z;
                c.fill(0x000000b0);
                c.rect(
                    sx - bw / 2.0 - 1.0,
                    top - 1.0,
                    bw + 2.0,
                    5.0 * z.max(0.8) + 2.0,
                );
                c.fill(0xc02020ff);
                c.rect(sx - bw / 2.0, top, bw, 5.0 * z.max(0.8));
                c.fill(if e.k == EK::Player {
                    0x3070ffff
                } else {
                    0x30d040ff
                });
                c.rect(
                    sx - bw / 2.0,
                    top,
                    bw * (e.hp.max(0) as f32 / e.mhp.max(1) as f32),
                    5.0 * z.max(0.8),
                );
            }
            if targeted {
                let col = match e.k {
                    EK::Mon(m) if MONS[m].karma < 0 => 0xff5050ff,
                    EK::Mon(_) => 0xc8c8c8ff,
                    _ => 0x60a0ffff,
                };
                c.text(sx, top - 6.0 * z, ts, 0, &e.name, 0x000000ff);
                let _ = col;
                c.stroke(with_a(col, 0.9), 2.0);
                c.ell_s(sx, sy, 16.0 * z * size.max(0.7), 7.0 * z * size.max(0.7));
            }
            if e.say_t > 0.0 && !e.say.is_empty() {
                let a = (e.say_t * 2.0).min(1.0);
                c.alpha(a);
                c.fill(e.say_col);
                c.text(
                    sx,
                    top - if targeted { 22.0 } else { 8.0 } * z,
                    ts,
                    0,
                    &e.say,
                    0x000000ff,
                );
                c.alpha(1.0);
            }
        }
        for cp in self.corpses.iter().filter(|cp| cp.player) {
            let (sx, sy) = scr(cp.x, cp.y);
            c.fill(0xffd070ff);
            c.text(
                sx,
                sy - 16.0 * z,
                (11.0 * z).max(10.0),
                0,
                "your corpse",
                0x000000ff,
            );
        }
        for f in self.floats.iter() {
            let (sx, sy) = self.to_screen(f.x, f.y);
            c.alpha((1.2 - f.t).min(1.0));
            c.fill(f.color);
            c.text(
                sx + 10.0 * z,
                sy - (50.0 + f.t * 40.0) * z,
                (14.0 * z).max(12.0),
                0,
                &f.text,
                0x000000ff,
            );
            c.alpha(1.0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ground(
        &self,
        c: &mut Canvas,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        tl: u8,
        sh: u8,
        z: f32,
        t: f32,
    ) {
        let hw = 32.0 * z + 0.6;
        let hh = 16.0 * z + 0.6;
        let base = tile_color(tl);
        let mut col = shade(base, 0.9 + sh as f32 / 255.0 * 0.16);
        if tl == T_WATER {
            col = shade(
                col,
                1.0 + (t * 1.3 + x as f32 * 0.6 + y as f32 * 0.4).sin() * 0.06,
            );
        }
        // point inside the diamond: u runs top->right, v runs top->left
        let pt = |u: f32, v: f32| (sx + (u - v) * 32.0 * z, sy - 16.0 * z + (u + v) * 16.0 * z);
        let px = self.pix.max(z);
        let speck = |c: &mut Canvas, k: i32, colr: u32, sz: f32| {
            let (u, v) = (h2(x, y, k), h2(x, y, k + 101));
            let (a, b) = pt(u * 0.9 + 0.05, v * 0.9 + 0.05);
            c.fill(colr);
            c.rect(a.floor(), b.floor(), sz * px, sz * px);
        };
        if tl == T_COBBLE {
            // flagstones: dark mortar with four stones of varied tone
            c.fill(shade(base, 0.5));
            c.poly(&[(sx, sy - hh), (sx + hw, sy), (sx, sy + hh), (sx - hw, sy)]);
            for i in 0..2 {
                for j in 0..2 {
                    let f = 0.82 + h2(x * 2 + i, y * 2 + j, 7) * 0.32;
                    let (u0, v0) = (i as f32 * 0.5 + 0.035, j as f32 * 0.5 + 0.035);
                    let (u1, v1) = (u0 + 0.43, v0 + 0.43);
                    c.fill(shade(base, f));
                    c.poly(&[pt(u0, v0), pt(u1, v0), pt(u1, v1), pt(u0, v1)]);
                    c.fill(shade(base, f * 1.15));
                    c.poly(&[
                        pt(u0, v0),
                        pt(u1, v0),
                        pt(u1, v0 + 0.06),
                        pt(u0 + 0.06, v0 + 0.06),
                        pt(u0 + 0.06, v1),
                        pt(u0, v1),
                    ]);
                }
            }
            if sh % 5 == 0 {
                c.stroke(shade(base, 0.45), px);
                let (a, b) = (pt(0.15, 0.3), pt(0.35, 0.4));
                c.line(a.0, a.1, b.0, b.1);
            }
            return;
        }
        c.fill(col);
        c.poly(&[(sx, sy - hh), (sx + hw, sy), (sx, sy + hh), (sx - hw, sy)]);
        match tl {
            T_WATER => {
                for k in 0..3 {
                    speck(c, k, shade(col, 0.8), 2.0);
                }
                if sh % 3 == 0 {
                    let o = ((t * 0.7 + sh as f32).sin()) * 5.0 * z;
                    c.stroke(0x6a80c070, px);
                    c.line(sx - 8.0 * z + o, sy, sx + 4.0 * z + o, sy - 2.0 * z);
                }
                // foam where the shore meets the water
                c.stroke(0xb8c8e080, 1.5 * px);
                if self.map.tile(x - 1, y) != T_WATER && inb(x - 1, y) {
                    let (a, b) = (pt(0.0, 0.0), pt(0.0, 1.0));
                    c.line(a.0, a.1 + 1.0, b.0 + 1.0, b.1);
                }
                if self.map.tile(x, y - 1) != T_WATER && inb(x, y - 1) {
                    let (a, b) = (pt(0.0, 0.0), pt(1.0, 0.0));
                    c.line(a.0, a.1 + 1.0, b.0 - 1.0, b.1);
                }
            }
            T_GRASS => {
                for k in 0..5 {
                    speck(
                        c,
                        k,
                        shade(col, if k % 2 == 0 { 0.72 } else { 1.22 }),
                        if k == 0 { 2.0 } else { 1.0 },
                    );
                }
                if sh > 190 {
                    c.stroke(shade(col, 0.6), px);
                    let (a, _) = pt(0.5, 0.5);
                    let ox = (sh as f32 - 220.0) * 0.3 * z;
                    c.line(a + ox, sy, a + ox - 2.0 * z, sy - 4.0 * z);
                    c.line(a + ox + 2.0 * z, sy, a + ox + 3.0 * z, sy - 4.0 * z);
                }
            }
            T_DIRT | T_CAVE => {
                for k in 0..5 {
                    speck(
                        c,
                        k,
                        shade(col, if k % 2 == 0 { 0.7 } else { 1.25 }),
                        if k < 2 { 2.0 } else { 1.0 },
                    );
                }
                if sh % 4 == 0 {
                    c.stroke(shade(col, 0.55), px);
                    let (a, b, d) = (pt(0.2, 0.5), pt(0.45, 0.42), pt(0.6, 0.62));
                    c.poly_s(&[a, b, d]);
                }
            }
            T_SAND => {
                for k in 0..4 {
                    speck(c, k, shade(col, if k % 2 == 0 { 0.82 } else { 1.18 }), 1.0);
                }
            }
            T_FLOOR | T_BRIDGE => {
                c.stroke(shade(col, 0.55), px);
                for k in [-0.5f32, 0.0, 0.5] {
                    let a = (sx - hw * 0.5 + k * hw * 0.5, sy - hh * 0.5 - k * hh * 0.5);
                    c.line(a.0, a.1, a.0 + hw, a.1 + hh);
                }
                speck(c, 3, shade(col, 0.5), 1.0);
                if tl == T_BRIDGE {
                    c.stroke(0x2e1e10ff, 2.0 * z);
                    c.line(sx - hw * 0.5, sy - hh * 0.5, sx + hw * 0.5, sy + hh * 0.5);
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn mountain(&self, c: &mut Canvas, x: i32, y: i32, sx: f32, sy: f32, sh: u8, z: f32) {
        let hw = 32.0 * z + 0.6;
        let hh = 16.0 * z + 0.6;
        // massifs rise towards their middle, like Pagan's rock cliffs
        let mut n = 0;
        for &(dx, dy) in DIRS.iter() {
            if self.map.tile(x + dx, y + dy) == T_MOUNT {
                n += 1;
            }
        }
        let hgt = (10.0 + n as f32 * 4.0 + (sh % 7) as f32) * z;
        let base = 0x5e5650ff;
        let f = 0.88 + sh as f32 / 255.0 * 0.2;
        let px = self.pix.max(z);
        c.outline(INK, px);
        c.fill(shade(base, 0.55 * f));
        c.poly(&[
            (sx - hw, sy),
            (sx, sy + hh),
            (sx, sy + hh - hgt),
            (sx - hw, sy - hgt),
        ]);
        c.fill(shade(base, 0.75 * f));
        c.poly(&[
            (sx, sy + hh),
            (sx + hw, sy),
            (sx + hw, sy - hgt),
            (sx, sy + hh - hgt),
        ]);
        c.fill(shade(base, 1.0 * f));
        c.poly(&[
            (sx, sy - hh - hgt),
            (sx + hw, sy - hgt),
            (sx, sy + hh - hgt),
            (sx - hw, sy - hgt),
        ]);
        c.outline(0, 0.0);
        // strata cracks on the cliff faces
        c.stroke(shade(base, 0.38), px);
        for (k, fr) in [0.33f32, 0.66].iter().enumerate() {
            let j = (h2(x, y, k as i32 + 3) - 0.5) * 4.0 * z;
            c.poly_s(&[
                (sx - hw, sy - hgt * fr + j),
                (sx - hw * 0.5, sy + hh * 0.5 - hgt * fr - j),
                (sx, sy + hh - hgt * fr + j),
            ]);
            c.poly_s(&[
                (sx, sy + hh - hgt * fr - j),
                (sx + hw * 0.5, sy + hh * 0.5 - hgt * fr + j),
                (sx + hw, sy - hgt * fr),
            ]);
        }
        // rock texture on top
        let top = |u: f32, v: f32| {
            (
                sx + (u - v) * 32.0 * z,
                sy - 16.0 * z + (u + v) * 16.0 * z - hgt,
            )
        };
        for k in 0..4 {
            let (a, b) = top(h2(x, y, k + 20) * 0.8 + 0.1, h2(x, y, k + 40) * 0.8 + 0.1);
            c.fill(shade(base, if k % 2 == 0 { 0.7 } else { 1.25 }));
            c.rect(a.floor(), b.floor(), 2.0 * px, px);
        }
        if sh % 5 == 0 {
            c.fill(0x4a5a2cff);
            let (a, b) = top(0.3, 0.4);
            c.ell(a, b, 7.0 * z, 3.0 * z);
        }
        if self.map.res[idx(x, y)] == 0 {
            c.fill(0x00000050);
            c.poly(&[
                (sx, sy - hh - hgt),
                (sx + hw, sy - hgt),
                (sx, sy + hh - hgt),
                (sx - hw, sy - hgt),
            ]);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn object(
        &self,
        c: &mut Canvas,
        o: u8,
        x: i32,
        y: i32,
        sx: f32,
        sy: f32,
        sh: u8,
        style: u8,
        z: f32,
        t: f32,
        lights: &mut Vec<(f32, f32, f32, u32)>,
    ) {
        let hw = 32.0 * z + 0.6;
        let hh = 16.0 * z + 0.6;
        let p = P {
            x: sx,
            y: sy,
            s: z,
            f: 1.0,
        };
        match o {
            O_TREE => {
                let depleted = self.map.res[idx(x, y)] == 0;
                c.fill(0x00000040);
                c.ell(sx, sy, 16.0 * z, 7.0 * z);
                c.fill(0x3e2a18ff);
                c.poly(&[
                    p.at(-4.0, 0.0),
                    p.at(4.0, 0.0),
                    p.at(2.5, -26.0),
                    p.at(-2.5, -26.0),
                ]);
                if style == 1 {
                    // dead, gnarled tree
                    c.outline(0, 0.0);
                    c.stroke(INK, 4.5 * z);
                    let br = [
                        (-2.0, -18.0, -14.0, -34.0),
                        (1.0, -24.0, 12.0, -40.0),
                        (0.0, -26.0, -3.0, -48.0),
                        (-8.0, -28.0, -16.0, -30.0),
                        (6.0, -32.0, 15.0, -30.0),
                    ];
                    for &(ax, ay, bx, by) in br.iter() {
                        let (a, b) = (p.at(ax, ay), p.at(bx, by));
                        c.line(a.0, a.1, b.0, b.1);
                    }
                    c.stroke(0x4a3826ff, 2.5 * z);
                    for &(ax, ay, bx, by) in br.iter() {
                        let (a, b) = (p.at(ax, ay), p.at(bx, by));
                        c.line(a.0, a.1, b.0, b.1);
                    }
                    c.outline(INK, self.pix.max(z));
                    return;
                }
                let g = if depleted { 0x4e5028ff } else { 0x263c18ff };
                let f = 0.85 + sh as f32 / 255.0 * 0.3;
                if sh % 3 == 0 {
                    for k in 0..3 {
                        let kf = k as f32;
                        c.fill(shade(g, f * (0.8 + kf * 0.12)));
                        c.poly(&[
                            p.at(-15.0 + kf * 4.0, -16.0 - kf * 12.0),
                            p.at(15.0 - kf * 4.0, -16.0 - kf * 12.0),
                            p.at(0.0, -38.0 - kf * 12.0),
                        ]);
                    }
                } else {
                    c.fill(shade(g, f * 0.75));
                    c.ell(sx, sy - 34.0 * z, 21.0 * z, 15.0 * z);
                    c.fill(shade(g, f));
                    c.ell(sx - 7.0 * z, sy - 41.0 * z, 13.0 * z, 10.0 * z);
                    c.fill(shade(g, f * 1.25));
                    c.ell(sx + 5.0 * z, sy - 46.0 * z, 9.0 * z, 7.0 * z);
                    c.outline(0, 0.0);
                    for k in 0..5 {
                        let (u, v) = (h2(x, y, k) * 34.0 - 17.0, h2(x, y, k + 9) * 22.0 - 48.0);
                        c.fill(shade(g, if k % 2 == 0 { 0.6 } else { 1.45 }));
                        c.rect(
                            sx + u * z,
                            sy + v * z,
                            2.0 * self.pix.max(z),
                            self.pix.max(z),
                        );
                    }
                    c.outline(INK, self.pix.max(z));
                }
            }
            O_WALL => {
                let col = WALL_COLORS[(style as usize).min(4)];
                let hgt = 38.0 * z;
                c.fill(shade(col, 0.62));
                c.poly(&[
                    (sx - hw, sy),
                    (sx, sy + hh),
                    (sx, sy + hh - hgt),
                    (sx - hw, sy - hgt),
                ]);
                c.fill(shade(col, 0.82));
                c.poly(&[
                    (sx, sy + hh),
                    (sx + hw, sy),
                    (sx + hw, sy - hgt),
                    (sx, sy + hh - hgt),
                ]);
                c.fill(shade(col, 1.05));
                c.poly(&[
                    (sx, sy - hh - hgt),
                    (sx + hw, sy - hgt),
                    (sx, sy + hh - hgt),
                    (sx - hw, sy - hgt),
                ]);
                c.outline(0, 0.0);
                // masonry courses
                c.stroke(shade(col, 0.45), self.pix.max(z));
                for r in 1..5 {
                    let k = hgt * r as f32 / 5.0;
                    c.line(sx - hw, sy - k, sx, sy + hh - k);
                    c.line(sx, sy + hh - k, sx + hw, sy - k);
                    let off = if r % 2 == 0 { 0.25 } else { 0.5 };
                    for q in [off, off + 0.5] {
                        if q < 1.0 {
                            let (lx, ly) = (sx - hw + hw * q, sy + hh * q - k);
                            c.line(lx, ly, lx, ly + hgt / 5.0);
                            let (rx, ry) = (sx + hw * q, sy + hh - hh * q - k);
                            c.line(rx, ry, rx, ry + hgt / 5.0);
                        }
                    }
                }
                if (x + y) % 3 == 0 && style != 4 {
                    c.fill(0x0e0c14ff);
                    c.poly(&[
                        p.at(-21.0, -12.0),
                        p.at(-12.0, -7.0),
                        p.at(-12.0, -22.0),
                        p.at(-21.0, -27.0),
                    ]);
                    c.stroke(0xc89a40a0, self.pix.max(z));
                    c.line(
                        p.at(-16.5, -9.5).0,
                        p.at(-16.5, -9.5).1,
                        p.at(-16.5, -24.5).0,
                        p.at(-16.5, -24.5).1,
                    );
                }
                c.outline(INK, self.pix.max(z));
            }
            O_DOOR => {
                c.fill(0x4a2e18ff);
                c.poly(&[
                    p.at(-30.0, -1.0),
                    p.at(-20.0, 4.0),
                    p.at(-20.0, -26.0),
                    p.at(-30.0, -31.0),
                ]);
            }
            O_ANVIL => {
                c.fill(0x00000050);
                c.ell(sx, sy, 14.0 * z, 6.0 * z);
                c.fill(0x404448ff);
                c.rect(sx - 4.0 * z, sy - 12.0 * z, 8.0 * z, 12.0 * z);
                c.fill(0x5a6066ff);
                c.poly(&[
                    p.at(-14.0, -12.0),
                    p.at(10.0, -12.0),
                    p.at(16.0, -16.0),
                    p.at(-10.0, -18.0),
                ]);
            }
            O_FORGE => {
                c.fill(0x6a3a2aff);
                c.poly(&[
                    p.at(-16.0, 0.0),
                    p.at(0.0, 8.0),
                    p.at(16.0, 0.0),
                    p.at(16.0, -18.0),
                    p.at(0.0, -10.0),
                    p.at(-16.0, -18.0),
                ]);
                c.fill(0x8a4a32ff);
                c.poly(&[
                    p.at(-16.0, -18.0),
                    p.at(0.0, -26.0),
                    p.at(16.0, -18.0),
                    p.at(0.0, -10.0),
                ]);
                let fl = 0.8 + (t * 9.0 + x as f32).sin() * 0.2;
                c.fill(0xff8a20ff);
                c.ell(sx, sy - 18.0 * z, 9.0 * z * fl, 4.0 * z);
                c.fill(0xffe060ff);
                c.ell(sx, sy - 19.0 * z, 4.0 * z * fl, 2.0 * z);
                lights.push((sx, sy - 18.0 * z, 90.0, 0xff9040ff));
            }
            O_GRAVE => {
                c.fill(0x00000040);
                c.ell(sx, sy, 10.0 * z, 4.0 * z);
                c.fill(0x8a8a88ff);
                c.rect(sx - 6.0 * z, sy - 16.0 * z, 12.0 * z, 16.0 * z);
                c.ell(sx, sy - 16.0 * z, 6.0 * z, 4.0 * z);
                c.stroke(0x505050ff, 1.2 * z);
                c.line(sx, sy - 16.0 * z, sx, sy - 6.0 * z);
                c.line(sx - 3.0 * z, sy - 12.0 * z, sx + 3.0 * z, sy - 12.0 * z);
                c.fill(0x6a5a40ff);
                c.poly(&[
                    p.at(-8.0, 4.0),
                    p.at(10.0, 10.0),
                    p.at(16.0, 6.0),
                    p.at(-2.0, 0.0),
                ]);
            }
            O_TENT => {
                c.fill(0x00000040);
                c.ell(sx, sy, 24.0 * z, 10.0 * z);
                c.fill(0x8a7050ff);
                c.poly(&[p.at(-24.0, 0.0), p.at(0.0, 10.0), p.at(0.0, -30.0)]);
                c.fill(0xa88a62ff);
                c.poly(&[p.at(0.0, 10.0), p.at(24.0, 0.0), p.at(0.0, -30.0)]);
                c.fill(0x2a2018ff);
                c.poly(&[p.at(-4.0, 8.0), p.at(4.0, 8.0), p.at(0.0, -8.0)]);
            }
            O_FIRE => {
                c.fill(0x4a3020ff);
                c.rect(sx - 10.0 * z, sy - 3.0 * z, 20.0 * z, 4.0 * z);
                for k in 0..3 {
                    let kf = k as f32;
                    let fl = (t * 10.0 + kf * 2.1).sin() * 3.0 * z;
                    c.fill(if k == 2 { 0xffe070ff } else { 0xff7a20e0 });
                    c.ell(
                        sx + (kf - 1.0) * 4.0 * z,
                        sy - 8.0 * z - fl.abs(),
                        (6.0 - kf) * z,
                        (10.0 - kf * 2.0) * z + fl,
                    );
                }
                lights.push((sx, sy - 8.0 * z, 140.0, 0xff9040ff));
            }
            O_ANKH => {
                c.fill(0x8a8478ff);
                c.poly(&[
                    p.at(-12.0, 0.0),
                    p.at(0.0, 6.0),
                    p.at(12.0, 0.0),
                    p.at(0.0, -6.0),
                ]);
                c.stroke(0xe8c040ff, 3.5 * z);
                c.ell_s(sx, sy - 40.0 * z, 5.0 * z, 7.0 * z);
                c.line(sx, sy - 33.0 * z, sx, sy - 4.0 * z);
                c.line(sx - 9.0 * z, sy - 30.0 * z, sx + 9.0 * z, sy - 30.0 * z);
                let gl = 0.5 + (t * 2.0).sin() * 0.2;
                c.fill(with_a(0xfff0a0ff, gl * 0.25));
                c.ell(sx, sy - 25.0 * z, 16.0 * z, 26.0 * z);
                lights.push((sx, sy - 25.0 * z, 110.0, 0xffe090ff));
            }
            O_LAMP => {
                // stone pedestal with an iron fire bowl
                c.fill(0x5a5650ff);
                c.poly(&[
                    p.at(-6.0, 0.0),
                    p.at(6.0, 0.0),
                    p.at(4.0, -20.0),
                    p.at(-4.0, -20.0),
                ]);
                c.fill(0x2a2420ff);
                c.ell(sx, sy - 22.0 * z, 10.0 * z, 4.5 * z);
                c.outline(0, 0.0);
                for k in 0..3 {
                    let kf = k as f32;
                    let fl = (t * 11.0 + kf * 2.3 + x as f32).sin() * 2.5 * z;
                    c.fill(if k == 2 { 0xffe070ff } else { 0xff6a18e0 });
                    c.ell(
                        sx + (kf - 1.0) * 3.5 * z,
                        sy - 27.0 * z - fl.abs(),
                        (5.0 - kf) * z,
                        (8.0 - kf * 1.5) * z + fl,
                    );
                }
                c.outline(INK, self.pix.max(z));
                lights.push((sx, sy - 26.0 * z, 150.0, 0xffa050ff));
            }
            O_MUSHROOM => {
                let big = 0.8 + h2(x, y, 5) * 0.6;
                let cap = if sh % 2 == 0 { 0x8a3a2aff } else { 0x6a5a8aff };
                c.fill(0xc8bca0ff);
                c.poly(&[
                    p.at(-4.0 * big, 0.0),
                    p.at(4.0 * big, 0.0),
                    p.at(3.0 * big, -26.0 * big),
                    p.at(-3.0 * big, -26.0 * big),
                ]);
                c.fill(cap);
                c.ell(sx, sy - 28.0 * z * big, 18.0 * z * big, 9.0 * z * big);
                c.fill(shade(cap, 1.3));
                c.ell(
                    sx - 4.0 * z * big,
                    sy - 31.0 * z * big,
                    8.0 * z * big,
                    4.0 * z * big,
                );
                c.outline(0, 0.0);
                c.fill(0xe8dcc0ff);
                for k in 0..4 {
                    let (u, v) = (h2(x, y, k) * 24.0 - 12.0, h2(x, y, k + 7) * 6.0 - 32.0);
                    c.rect(sx + u * z * big, sy + v * z * big, 2.0 * z, 1.5 * z);
                }
                c.outline(INK, self.pix.max(z));
            }
            O_BARREL => {
                c.fill(0x6a4424ff);
                c.rect(sx - 7.0 * z, sy - 18.0 * z, 14.0 * z, 16.0 * z);
                c.ell(sx, sy - 2.0 * z, 7.0 * z, 3.0 * z);
                c.fill(0x8a5a30ff);
                c.ell(sx, sy - 18.0 * z, 7.0 * z, 3.0 * z);
                c.outline(0, 0.0);
                c.stroke(0x2a2420ff, 1.5 * z);
                c.line(sx - 7.0 * z, sy - 7.0 * z, sx + 7.0 * z, sy - 7.0 * z);
                c.line(sx - 7.0 * z, sy - 14.0 * z, sx + 7.0 * z, sy - 14.0 * z);
                c.outline(INK, self.pix.max(z));
            }
            O_CRATE => {
                let hgt = 14.0 * z;
                let (w2, h2_) = (13.0 * z, 6.5 * z);
                c.fill(0x5a4026ff);
                c.poly(&[
                    (sx - w2, sy),
                    (sx, sy + h2_),
                    (sx, sy + h2_ - hgt),
                    (sx - w2, sy - hgt),
                ]);
                c.fill(0x7a5632ff);
                c.poly(&[
                    (sx, sy + h2_),
                    (sx + w2, sy),
                    (sx + w2, sy - hgt),
                    (sx, sy + h2_ - hgt),
                ]);
                c.fill(0x92683cff);
                c.poly(&[
                    (sx, sy - h2_ - hgt),
                    (sx + w2, sy - hgt),
                    (sx, sy + h2_ - hgt),
                    (sx - w2, sy - hgt),
                ]);
                c.outline(0, 0.0);
                c.stroke(0x3a2814ff, self.pix.max(z));
                c.line(sx - w2, sy - hgt, sx, sy + h2_);
                c.line(sx, sy + h2_ - hgt, sx + w2, sy);
                c.outline(INK, self.pix.max(z));
            }
            O_BONES => {
                c.outline(0, 0.0);
                c.stroke(0xd8d0b8ff, 2.0 * z);
                let a = h2(x, y, 1) * 6.0;
                c.line(sx - 8.0 * z, sy + a * z * 0.3, sx + 6.0 * z, sy - 3.0 * z);
                c.line(
                    sx - 3.0 * z,
                    sy + 4.0 * z,
                    sx + 2.0 * z,
                    sy - 4.0 * z + a * 0.3,
                );
                c.fill(0xe0d8c0ff);
                c.ell(sx + 9.0 * z, sy + 1.0 * z, 4.0 * z, 3.5 * z);
                c.fill(0x1a1410ff);
                c.rect(
                    sx + 7.5 * z,
                    sy,
                    1.2 * self.pix.max(z),
                    1.2 * self.pix.max(z),
                );
                c.rect(
                    sx + 10.0 * z,
                    sy,
                    1.2 * self.pix.max(z),
                    1.2 * self.pix.max(z),
                );
                c.outline(INK, self.pix.max(z));
            }
            O_FOUNTAIN => {
                c.fill(0x9a9a98ff);
                c.poly(&[(sx, sy - hh), (sx + hw, sy), (sx, sy + hh), (sx - hw, sy)]);
                c.fill(0x7a7a78ff);
                c.poly(&[
                    (sx - hw, sy),
                    (sx, sy + hh),
                    (sx, sy + hh - 6.0 * z),
                    (sx - hw, sy - 6.0 * z),
                ]);
                c.poly(&[
                    (sx, sy + hh),
                    (sx + hw, sy),
                    (sx + hw, sy - 6.0 * z),
                    (sx, sy + hh - 6.0 * z),
                ]);
                let w = shade(0x3a7ab0ff, 1.0 + (t * 2.0 + x as f32).sin() * 0.08);
                c.fill(w);
                c.poly(&[
                    (sx, sy - hh * 0.7 - 6.0 * z),
                    (sx + hw * 0.75, sy - 6.0 * z),
                    (sx, sy + hh * 0.7 - 6.0 * z),
                    (sx - hw * 0.75, sy - 6.0 * z),
                ]);
                if (x + y) % 2 == 0 {
                    c.fill(0xd0e8ffc0);
                    let j = (t * 8.0).sin() * 2.0 * z;
                    c.ell(sx, sy - 14.0 * z + j, 2.0 * z, 8.0 * z);
                }
            }
            O_BUSH => {
                c.fill(0x00000030);
                c.ell(sx, sy, 10.0 * z, 4.0 * z);
                c.fill(shade(0x3d7a2aff, 0.9 + sh as f32 / 900.0));
                c.ell(sx - 4.0 * z, sy - 6.0 * z, 7.0 * z, 6.0 * z);
                c.ell(sx + 4.0 * z, sy - 7.0 * z, 7.0 * z, 6.0 * z);
                c.fill(0x4f9036ff);
                c.ell(sx, sy - 10.0 * z, 6.0 * z, 5.0 * z);
            }
            O_BOULDER => {
                c.fill(0x00000040);
                c.ell(sx, sy, 14.0 * z, 6.0 * z);
                c.fill(0x7a766cff);
                c.ell(sx, sy - 8.0 * z, 13.0 * z, 10.0 * z);
                c.fill(0x9a968aff);
                c.ell(sx - 3.0 * z, sy - 12.0 * z, 6.0 * z, 4.0 * z);
            }
            O_FENCE => {
                c.stroke(0x3a3a3aff, 2.0 * z);
                let horiz = self.map.obj(x + 1, y) == O_FENCE || self.map.obj(x - 1, y) == O_FENCE;
                let (a, b) = if horiz {
                    (p.at(-16.0, -8.0), p.at(16.0, 8.0))
                } else {
                    (p.at(16.0, -8.0), p.at(-16.0, 8.0))
                };
                for k in [0.0f32, 0.5, 1.0] {
                    let (px, py) = (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
                    c.line(px, py, px, py - 16.0 * z);
                }
                c.line(a.0, a.1 - 12.0 * z, b.0, b.1 - 12.0 * z);
                c.line(a.0, a.1 - 6.0 * z, b.0, b.1 - 6.0 * z);
            }
            O_FLOWERS => {
                let cols = [0xe04060ff, 0xf0d040ff, 0xffffffff, 0x8060e0ff];
                for k in 0..5 {
                    let kf = k as f32;
                    let ox = ((sh as f32 + kf * 47.0) % 24.0 - 12.0) * z;
                    let oy = ((sh as f32 * 0.7 + kf * 31.0) % 10.0 - 5.0) * z;
                    c.fill(cols[(k + sh as usize) % 4]);
                    c.ell(sx + ox, sy + oy - 2.0 * z, 2.0 * z, 1.6 * z);
                }
            }
            O_HOARD => {
                c.fill(0xc89a28ff);
                c.ell(sx, sy - 4.0 * z, 20.0 * z, 9.0 * z);
                c.fill(0xf0c840ff);
                c.ell(sx - 3.0 * z, sy - 8.0 * z, 12.0 * z, 6.0 * z);
                let sp = (t * 3.0 + x as f32).sin();
                if sp > 0.6 {
                    c.fill(0xffffffff);
                    c.ell(sx + 5.0 * z, sy - 10.0 * z, 1.5 * z, 4.0 * z);
                    c.ell(sx + 5.0 * z, sy - 10.0 * z, 4.0 * z, 1.5 * z);
                }
                lights.push((sx, sy - 6.0 * z, 70.0, 0xffd060ff));
            }
            O_SIGN => {
                c.stroke(0x4a3020ff, 2.0 * z);
                c.line(sx, sy, sx, sy - 22.0 * z);
                c.fill(0x8a6238ff);
                c.rect(sx - 10.0 * z, sy - 30.0 * z, 20.0 * z, 10.0 * z);
                let label = self
                    .map
                    .buildings
                    .iter()
                    .find(|b| b.x + 4 == x && b.y + 6 == y)
                    .map(|b| match b.vendor {
                        V_SMITH => "SMITH",
                        V_MAGE => "MAGE",
                        V_PROV => "GOODS",
                        V_HEALER => "HEALER",
                        V_BANKER => "BANK",
                        _ => "",
                    });
                if let Some(l) = label {
                    c.fill(0xf0e0b0ff);
                    c.text(sx, sy - 22.0 * z, 6.5 * z, 0, l, 0x00000000);
                }
            }
            _ => {}
        }
    }

    fn corpse(&self, c: &mut Canvas, cp: &Corpse, sx: f32, sy: f32, z: f32) {
        let s = z * cp.size.clamp(0.5, 1.8);
        c.fill(0x00000050);
        c.ell(sx, sy, 16.0 * s, 6.0 * s);
        c.fill(shade(cp.color, 0.6));
        c.ell(sx, sy - 3.0 * s, 13.0 * s, 5.0 * s);
        c.fill(0xe0d8c0ff);
        c.ell(sx + 9.0 * s, sy - 5.0 * s, 3.5 * s, 3.0 * s);
        c.stroke(0xe0d8c0ff, 1.5 * s);
        c.line(sx - 8.0 * s, sy - 2.0 * s, sx - 2.0 * s, sy - 6.0 * s);
        if cp.player || !cp.items.is_empty() || cp.gold > 0 {
            c.fill(0xffd84080);
            c.ell(sx, sy - 3.0 * s, 3.0 * s, 1.5 * s);
        }
    }

    fn entity(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, z: f32, t: f32) {
        let moving = !e.path.is_empty();
        let wk = if moving { e.walk.sin() } else { 0.0 };
        let flash = e.hurt > 0.5;
        match e.k {
            EK::Player => {
                let pl = &self.pl;
                let body = pl
                    .armor
                    .as_ref()
                    .map(|a| a.def().color)
                    .unwrap_or(0x8a6a4aff);
                let robe = pl
                    .armor
                    .as_ref()
                    .map(|a| a.def().id == "robe")
                    .unwrap_or(false);
                let weapon = match pl.weapon.as_ref().map(|w| w.def().id) {
                    None => 0,
                    Some("hatchet") => 2,
                    Some("mace") | Some("warhammer") | Some("pickaxe") => 3,
                    Some(_) => 1,
                };
                let h = Hum {
                    body,
                    robe,
                    legs: if robe { body } else { 0x3a3040ff },
                    hair: Some(pl.hair),
                    long_hair: pl.female,
                    weapon: if pl.war || matches!(pl.action, Action::Gather { .. }) {
                        weapon
                    } else {
                        0
                    },
                    shield: pl.shield.as_ref().map(|s| s.def().color),
                    helm: if pl
                        .armor
                        .as_ref()
                        .map(|a| a.def().id == "platemail")
                        .unwrap_or(false)
                    {
                        Some(0xc8ced6ff)
                    } else {
                        None
                    },
                    ..HUM
                };
                if pl.ghost {
                    c.alpha(0.45);
                    let g = Hum {
                        skin: 0xe8f0ffff,
                        body: 0xe8f0ffff,
                        legs: 0xe8f0ffff,
                        robe: true,
                        hair: None,
                        weapon: 0,
                        shield: None,
                        helm: None,
                        ..HUM
                    };
                    self.humanoid(
                        c,
                        e,
                        sx,
                        sy - 4.0 * z - (t * 2.0).sin() * 2.0 * z,
                        z,
                        1.0,
                        &g,
                        wk,
                        false,
                    );
                    c.alpha(1.0);
                } else {
                    if pl.war {
                        c.stroke(0xd0302080, 1.5);
                        c.ell_s(sx, sy, 14.0 * z, 6.0 * z);
                    }
                    self.humanoid(c, e, sx, sy, z, 1.0, &h, wk, flash);
                }
                if let Action::Cast { .. } = pl.action {
                    c.fill(with_a(0x80b0ffff, 0.4 + (t * 12.0).sin() * 0.2));
                    let (hx, hy) = (sx + e.face * 10.0 * z, sy - 30.0 * z);
                    c.ell(hx, hy, 5.0 * z, 5.0 * z);
                }
            }
            EK::Npc(v) => {
                let vd = &VENDORS[v];
                let hair = HAIR[(e.look as usize) % HAIR.len()];
                let h = match v {
                    V_GUARD => Hum {
                        body: 0xc8ced6ff,
                        legs: 0x7a8088ff,
                        helm: Some(0xb0b8c0ff),
                        weapon: 5,
                        shield: Some(0x3050a0ff),
                        ..HUM
                    },
                    V_MAGE => Hum {
                        body: vd.robe,
                        legs: vd.robe,
                        robe: true,
                        hat: Some(vd.robe),
                        hair: Some(0xd0d0d0ff),
                        weapon: 4,
                        ..HUM
                    },
                    V_HEALER => Hum {
                        body: vd.robe,
                        legs: vd.robe,
                        robe: true,
                        hair: Some(hair),
                        long_hair: e.look % 2 == 0,
                        ..HUM
                    },
                    V_BANKER => Hum {
                        body: vd.robe,
                        legs: 0x2a2030ff,
                        hair: Some(hair),
                        ..HUM
                    },
                    V_SMITH => Hum {
                        body: vd.robe,
                        legs: 0x3a3028ff,
                        hair: Some(hair),
                        weapon: 3,
                        skin: 0xd8a070ff,
                        ..HUM
                    },
                    _ => Hum {
                        body: vd.robe,
                        legs: 0x4a3a2aff,
                        hair: Some(hair),
                        long_hair: e.look % 3 == 0,
                        ..HUM
                    },
                };
                self.humanoid(c, e, sx, sy, z, 1.0, &h, wk, false);
            }
            EK::Mon(m) => {
                let d = &MONS[m];
                let s = d.size;
                let col = if flash {
                    mix(d.color, 0xc02010ff, 0.5)
                } else {
                    d.color
                };
                match d.id {
                    "rabbit" | "rat" | "wolf" => self.quad(c, e, sx, sy, z * s, col, d.id, wk, t),
                    "mongbat" => self.mongbat(c, e, sx, sy, z * s, col, t),
                    "slime" => self.slime(c, e, sx, sy, z * s, col, t),
                    "dragon" => self.dragon(c, e, sx, sy, z * s, col, wk, t),
                    "skeleton" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: col,
                            legs: col,
                            skull: true,
                            weapon: 1,
                            shield: Some(0x6a5a40ff),
                            eyes: Some(0x000000ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "zombie" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x5a4a3aff,
                            legs: 0x3a3a30ff,
                            hair: Some(0x2a2a20ff),
                            eyes: Some(0xd0ff60ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "orc" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x6a4a2aff,
                            legs: 0x3a2a1aff,
                            weapon: 2,
                            eyes: Some(0xff3020ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "orclord" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x8a8f96ff,
                            legs: 0x3a2a1aff,
                            weapon: 2,
                            helm: Some(0x6a6058ff),
                            shield: Some(0x5a3a20ff),
                            eyes: Some(0xff3020ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "troll" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x5a4a30ff,
                            legs: col,
                            weapon: 6,
                            eyes: Some(0xffe040ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "ettin" => self.humanoid(
                        c,
                        e,
                        sx,
                        sy,
                        z,
                        s,
                        &Hum {
                            skin: col,
                            body: 0x6a5038ff,
                            legs: col,
                            weapon: 6,
                            heads: 2,
                            eyes: Some(0x000000ff),
                            ..HUM
                        },
                        wk,
                        flash,
                    ),
                    "lich" => {
                        c.fill(with_a(0x80ff80ff, 0.12 + (t * 3.0).sin() * 0.05));
                        c.ell(sx, sy - 24.0 * z * s, 22.0 * z * s, 30.0 * z * s);
                        self.humanoid(
                            c,
                            e,
                            sx,
                            sy - 3.0 * z,
                            z,
                            s,
                            &Hum {
                                skin: 0xe8e2cfff,
                                body: col,
                                legs: col,
                                robe: true,
                                skull: true,
                                weapon: 4,
                                eyes: Some(0x60ff60ff),
                                hat: Some(0x3a2a5aff),
                                ..HUM
                            },
                            wk,
                            flash,
                        )
                    }
                    _ => self.humanoid(c, e, sx, sy, z, s, &Hum { body: col, ..HUM }, wk, flash),
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn humanoid(
        &self,
        c: &mut Canvas,
        e: &Ent,
        sx: f32,
        sy: f32,
        z: f32,
        scale: f32,
        h: &Hum,
        wk: f32,
        flash: bool,
    ) {
        let t = self.clock;
        let ink = self.pix.max(z);
        let moving = !e.path.is_empty();
        let recoil = e.hurt * e.hurt * 3.0;
        let sx = sx - e.face * recoil * z;
        let breathe = if moving {
            0.0
        } else {
            (t * 2.2 + e.id as f32 * 0.7).sin() * 0.7
        };
        let bob = if moving {
            -(e.walk.sin()).abs() * 1.8
        } else {
            0.0
        };
        let up = bob + breathe * 0.5;
        let p = P {
            x: sx,
            y: sy,
            s: z * scale * 1.12,
            f: e.face,
        };
        let s = p.s;
        let tint = |col: u32| {
            if flash {
                mix(col, 0xc02010ff, 0.5)
            } else {
                col
            }
        };
        let casting = e.k == EK::Player && matches!(self.pl.action, Action::Cast { .. });

        // ---- arm & weapon kinematics (model units, facing right)
        let shoulder = (6.5, -27.0 + up);
        let rest = (8.5 - wk * 3.0, -17.0 + up);
        let windup = (1.5, -39.0 + up);
        let strike = (15.0, -12.0 + up);
        let lerp =
            |a: (f32, f32), b: (f32, f32), k: f32| (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k);
        let hand_at = |ph: f32| {
            if ph < 0.3 {
                lerp(rest, windup, ph / 0.3)
            } else if ph < 0.62 {
                lerp(windup, strike, (ph - 0.3) / 0.32)
            } else {
                lerp(strike, rest, (ph - 0.62) / 0.38)
            }
        };
        let blade_len = match h.weapon {
            4 => 22.0,
            5 => 28.0,
            _ => 15.0,
        };
        let tip_of = |hand: (f32, f32)| {
            let (dx, dy) = (
                (hand.0 - shoulder.0) * 0.6 + 0.8,
                (hand.1 - shoulder.1) * 0.6 - 0.6,
            );
            let l = (dx * dx + dy * dy).sqrt().max(0.01);
            (hand.0 + dx / l * blade_len, hand.1 + dy / l * blade_len)
        };
        let attacking = e.atk > 0.0 && !casting;
        let phase = 1.0 - e.atk;
        let hand = if casting {
            (7.0, -41.0 + up)
        } else if attacking {
            hand_at(phase)
        } else {
            rest
        };
        let back_hand = if casting {
            (-7.0, -41.0 + up)
        } else {
            (-8.0 + wk * 3.0, -16.0 + up)
        };

        let draw_weapon_arm = |c: &mut Canvas| {
            let (a, b) = (p.at(shoulder.0, shoulder.1), p.at(hand.0, hand.1));
            c.ink_line(a, b, tint(h.skin), 2.7 * s);
            if h.weapon > 0 && !casting {
                let tip = tip_of(hand);
                let wcol = match h.weapon {
                    4 | 6 => 0x5a3e22ff,
                    5 => 0x7a7e84ff,
                    _ => 0xc8ccd0ff,
                };
                let (a, b) = (p.at(hand.0, hand.1), p.at(tip.0, tip.1));
                c.ink_line(a, b, wcol, if h.weapon == 6 { 4.0 * s } else { 2.0 * s });
                match h.weapon {
                    2 | 5 => {
                        c.fill(0x9aa0a8ff);
                        c.ell(b.0, b.1, 4.0 * s, 3.0 * s);
                    }
                    3 => {
                        c.fill(0x4a5058ff);
                        c.rect(b.0 - 3.5 * s, b.1 - 3.0 * s, 7.0 * s, 6.0 * s);
                    }
                    4 => {
                        c.fill(0x60a0e0ff);
                        c.ell(b.0, b.1, 2.8 * s, 2.8 * s);
                    }
                    6 => {
                        c.fill(0x4a3018ff);
                        c.ell(b.0, b.1, 4.0 * s, 4.0 * s);
                    }
                    _ => {}
                }
            }
            if casting {
                for hp in [hand, back_hand] {
                    let (x, y) = p.at(hp.0, hp.1);
                    c.fill(with_a(0x80b0ffff, 0.5 + (t * 14.0).sin() * 0.25));
                    c.ell(x, y, 4.0 * s, 4.0 * s);
                }
            }
        };

        // shadow (no ink)
        c.outline(0, 0.0);
        c.fill(0x00000066);
        c.ell(sx, sy, 11.0 * s, 4.5 * s);
        // slash trail
        if attacking && h.weapon > 0 && phase > 0.3 && phase < 0.8 {
            let mut pts = vec![];
            let end = phase.min(0.62);
            for k in 0..7 {
                let ph = 0.3 + (end - 0.3) * k as f32 / 6.0;
                let tp = tip_of(hand_at(ph));
                pts.push(p.at(tp.0, tp.1));
            }
            c.stroke(
                with_a(0xe8e0d0ff, 0.45 * (1.0 - (phase - 0.3) / 0.5)),
                3.0 * s,
            );
            c.poly_s(&pts);
        }
        c.outline(INK, ink);

        if e.back {
            draw_weapon_arm(c);
        }
        // legs / robe
        if h.robe {
            c.fill(tint(shade(h.body, 0.8)));
            c.poly(&[
                p.at(-7.0, -27.0 + up),
                p.at(7.0, -27.0 + up),
                p.at(9.5 + wk, -1.0),
                p.at(-9.5 + wk, -1.0),
            ]);
        } else {
            let (a, b) = (p.at(-3.0, -14.0 + up), p.at(-3.0 + wk * 4.5, -0.5));
            c.ink_line(a, b, tint(h.legs), 3.4 * s);
            let (a, b) = (p.at(3.0, -14.0 + up), p.at(3.0 - wk * 4.5, -0.5));
            c.ink_line(a, b, tint(h.legs), 3.4 * s);
        }
        // torso, lit from the upper left
        let torso = [
            p.at(-7.0, -13.0 + up),
            p.at(7.0, -13.0 + up),
            p.at(6.5, -29.0 + up),
            p.at(-6.5, -29.0 + up),
        ];
        c.fill(tint(h.body));
        c.poly(&torso);
        c.outline(0, 0.0);
        c.fill(0x00000048);
        c.poly(&[
            (sx + 1.0 * s, sy + (-13.0 + up) * s),
            (sx + 7.0 * s, sy + (-13.0 + up) * s),
            (sx + 6.5 * s, sy + (-29.0 + up) * s),
            (sx + 1.5 * s, sy + (-29.0 + up) * s),
        ]);
        if h.skull {
            c.stroke(0x00000080, 1.0 * s);
            for k in 0..3 {
                let yy = -25.0 + up + k as f32 * 4.0;
                let (a, b) = (p.at(-5.0, yy), p.at(5.0, yy));
                c.line(a.0, a.1, b.0, b.1);
            }
        }
        c.fill(shade(h.body, 0.5));
        let (a, b) = (p.at(-7.0, -15.0 + up), p.at(7.0, -15.0 + up));
        c.poly(&[a, b, (b.0, b.1 + 2.0 * s), (a.0, a.1 + 2.0 * s)]);
        c.outline(INK, ink);
        // shield arm
        let sh = p.at(-6.5, -27.0 + up);
        c.ink_line(sh, p.at(back_hand.0, back_hand.1), tint(h.skin), 2.7 * s);
        if let Some(scol) = h.shield {
            if !casting {
                c.fill(scol);
                let (x, y) = p.at(-9.0, -19.0 + up);
                c.ell(x, y, 4.5 * s, 7.0 * s);
                c.fill(shade(scol, 1.4));
                c.ell(x, y, 1.5 * s, 2.0 * s);
            }
        }
        if !e.back {
            draw_weapon_arm(c);
        }
        // heads
        let offs: &[f32] = if h.heads == 2 { &[-4.5, 4.5] } else { &[0.0] };
        for &ox in offs {
            let (hx, hy) = p.at(ox, -34.0 + up);
            if h.long_hair {
                if let Some(hc) = h.hair {
                    c.fill(hc);
                    c.poly(&[
                        p.at(ox - 5.5, -36.0 + up),
                        p.at(ox + 5.5, -36.0 + up),
                        p.at(ox + 5.0, -26.0 + up),
                        p.at(ox - 6.0, -25.0 + up),
                    ]);
                }
            }
            c.fill(tint(h.skin));
            c.ell(hx, hy, 5.2 * s, 5.4 * s);
            c.outline(0, 0.0);
            c.fill(0x00000040);
            c.ell(hx + 2.0 * s, hy + 1.0 * s, 3.0 * s, 4.0 * s);
            if !e.back {
                if let Some(ec) = h.eyes {
                    c.fill(ec);
                    let (ex, ey) = p.at(ox + 2.5, -34.5 + up);
                    c.ell(ex, ey, 1.1 * s, 1.1 * s);
                    let (ex, ey) = p.at(ox - 0.5, -34.5 + up);
                    c.ell(ex, ey, 1.1 * s, 1.1 * s);
                }
            }
            c.outline(INK, ink);
            if let Some(hc) = h.hair {
                c.fill(hc);
                if e.back {
                    c.ell(hx, hy - 0.5 * s, 5.5 * s, 5.2 * s);
                } else {
                    let (x, y) = p.at(ox - 0.5, -37.5 + up);
                    c.ell(x, y, 5.6 * s, 3.2 * s);
                }
            }
            if let Some(hc) = h.helm {
                c.fill(hc);
                let (x, y) = p.at(ox, -36.5 + up);
                c.ell(x, y, 6.0 * s, 4.5 * s);
                if !e.back {
                    c.stroke(shade(hc, 0.5), 1.2 * s);
                    let (a, b) = (p.at(ox + 2.5, -37.0 + up), p.at(ox + 2.5, -32.0 + up));
                    c.line(a.0, a.1, b.0, b.1);
                }
            }
            if let Some(hc) = h.hat {
                c.fill(hc);
                c.poly(&[
                    p.at(ox - 8.0, -37.0 + up),
                    p.at(ox + 8.0, -37.0 + up),
                    p.at(ox - 2.0, -52.0 + up),
                ]);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn quad(
        &self,
        c: &mut Canvas,
        e: &Ent,
        sx: f32,
        sy: f32,
        s: f32,
        col: u32,
        kind: &str,
        wk: f32,
        _t: f32,
    ) {
        let p = P {
            x: sx,
            y: sy,
            s,
            f: e.face,
        };
        c.fill(0x00000050);
        c.ell(sx, sy, 13.0 * s, 5.0 * s);
        let leg = shade(col, 0.7);
        c.stroke(INK, 3.6 * s);
        for (lx, ph) in [(-7.0, 1.0), (-4.0, -1.0), (5.0, -1.0), (8.0, 1.0)] {
            let (a, b) = (p.at(lx, -6.0), p.at(lx + wk * ph * 3.0, 0.0));
            c.line(a.0, a.1, b.0, b.1);
        }
        c.stroke(leg, 2.2 * s);
        for (lx, ph) in [(-7.0, 1.0), (-4.0, -1.0), (5.0, -1.0), (8.0, 1.0)] {
            let (a, b) = (p.at(lx, -6.0), p.at(lx + wk * ph * 3.0, 0.0));
            c.line(a.0, a.1, b.0, b.1);
        }
        let bob = if kind == "rabbit" {
            wk.abs() * -3.0
        } else {
            0.0
        };
        // tail
        c.stroke(col, if kind == "rat" { 1.4 * s } else { 2.6 * s });
        let (a, b) = (
            p.at(-10.0, -9.0 + bob),
            p.at(
                if kind == "rat" { -20.0 } else { -16.0 },
                if kind == "wolf" { -13.0 } else { -5.0 },
            ),
        );
        if kind == "rabbit" {
            c.fill(0xf0f0f0ff);
            c.ell(a.0, a.1, 3.0 * s, 3.0 * s);
        } else {
            c.line(a.0, a.1, b.0, b.1);
        }
        c.fill(col);
        let (bx, by) = p.at(0.0, -9.0 + bob);
        c.ell(bx, by, 11.0 * s, 5.5 * s);
        c.fill(shade(col, 1.15));
        let (hx, hy) = p.at(10.0, -12.0 + bob);
        c.ell(hx, hy, 4.5 * s, 4.0 * s);
        let (nx, ny) = p.at(14.0, -11.0 + bob);
        c.ell(nx, ny, 3.0 * s, 2.0 * s);
        c.fill(0x101010ff);
        let (ex, ey) = p.at(11.5, -13.5 + bob);
        c.ell(ex, ey, 0.9 * s, 0.9 * s);
        c.fill(shade(col, 0.9));
        match kind {
            "rabbit" => {
                let (ax, ay) = p.at(8.5, -19.0 + bob);
                c.ell(ax, ay, 1.5 * s, 5.0 * s);
                let (ax, ay) = p.at(10.5, -19.5 + bob);
                c.ell(ax, ay, 1.5 * s, 5.0 * s);
            }
            "wolf" => {
                c.poly(&[
                    p.at(7.0, -15.0 + bob),
                    p.at(9.0, -20.0 + bob),
                    p.at(11.0, -15.0 + bob),
                ]);
            }
            _ => {
                let (ax, ay) = p.at(9.0, -16.0 + bob);
                c.ell(ax, ay, 2.0 * s, 2.0 * s);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn mongbat(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, s: f32, col: u32, t: f32) {
        let p = P {
            x: sx,
            y: sy,
            s,
            f: e.face,
        };
        let bob = (t * 5.0 + e.id as f32).sin() * 3.0;
        let flap = (t * 16.0 + e.id as f32).sin();
        c.fill(0x00000040);
        c.ell(sx, sy, 9.0 * s, 3.5 * s);
        c.fill(shade(col, 0.8));
        c.poly(&[
            p.at(-2.0, -24.0 + bob),
            p.at(-20.0, -32.0 + bob + flap * 9.0),
            p.at(-12.0, -20.0 + bob),
            p.at(-6.0, -18.0 + bob),
        ]);
        c.poly(&[
            p.at(2.0, -24.0 + bob),
            p.at(20.0, -32.0 + bob + flap * 9.0),
            p.at(12.0, -20.0 + bob),
            p.at(6.0, -18.0 + bob),
        ]);
        c.fill(col);
        let (x, y) = p.at(0.0, -22.0 + bob);
        c.ell(x, y, 6.0 * s, 7.0 * s);
        let (x, y) = p.at(2.0, -30.0 + bob);
        c.ell(x, y, 4.5 * s, 4.0 * s);
        c.poly(&[
            p.at(-1.0, -32.0 + bob),
            p.at(0.0, -38.0 + bob),
            p.at(2.0, -33.0 + bob),
        ]);
        c.poly(&[
            p.at(3.0, -33.0 + bob),
            p.at(5.0, -38.0 + bob),
            p.at(6.0, -32.0 + bob),
        ]);
        c.fill(0xffe040ff);
        let (x, y) = p.at(4.0, -30.0 + bob);
        c.ell(x, y, 1.0 * s, 1.0 * s);
    }

    #[allow(clippy::too_many_arguments)]
    fn slime(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, s: f32, col: u32, t: f32) {
        let q = (t * 4.0 + e.id as f32).sin() * 0.12;
        c.fill(0x00000040);
        c.ell(sx, sy, 12.0 * s, 4.5 * s);
        c.fill(with_a(col, 0.9));
        c.ell(
            sx,
            sy - 6.0 * s * (1.0 - q),
            11.0 * s * (1.0 + q),
            8.0 * s * (1.0 - q),
        );
        c.fill(0xffffff90);
        c.ell(sx - 4.0 * s, sy - 10.0 * s, 3.0 * s, 2.0 * s);
        c.fill(0x102010ff);
        c.ell(sx + e.face * 3.0 * s, sy - 8.0 * s, 1.2 * s, 1.6 * s);
        c.ell(sx + e.face * 6.5 * s, sy - 8.0 * s, 1.2 * s, 1.6 * s);
    }

    #[allow(clippy::too_many_arguments)]
    fn dragon(&self, c: &mut Canvas, e: &Ent, sx: f32, sy: f32, s: f32, col: u32, wk: f32, t: f32) {
        let p = P {
            x: sx,
            y: sy,
            s,
            f: e.face,
        };
        let flap = (t * 2.5).sin();
        c.fill(0x00000060);
        c.ell(sx, sy, 22.0 * s, 8.0 * s);
        // far wing
        c.fill(shade(col, 0.6));
        c.poly(&[
            p.at(-2.0, -22.0),
            p.at(-12.0, -46.0 - flap * 6.0),
            p.at(-26.0, -40.0 - flap * 4.0),
            p.at(-14.0, -20.0),
        ]);
        // tail
        c.fill(shade(col, 0.85));
        c.poly(&[
            p.at(-10.0, -16.0),
            p.at(-28.0, -6.0),
            p.at(-34.0, -10.0),
            p.at(-10.0, -10.0),
        ]);
        // legs
        c.stroke(shade(col, 0.7), 4.0 * s);
        for (lx, ph) in [(-8.0, 1.0), (-4.0, -1.0), (6.0, -1.0), (9.0, 1.0)] {
            let (a, b) = (p.at(lx, -10.0), p.at(lx + wk * ph * 2.0, 0.0));
            c.line(a.0, a.1, b.0, b.1);
        }
        // body
        c.fill(col);
        let (bx, by) = p.at(0.0, -15.0);
        c.ell(bx, by, 14.0 * s, 8.0 * s);
        c.fill(0xe0a060ff);
        let (bx, by) = p.at(2.0, -11.0);
        c.ell(bx, by, 9.0 * s, 3.5 * s);
        // neck & head
        c.fill(col);
        c.poly(&[
            p.at(6.0, -20.0),
            p.at(10.0, -12.0),
            p.at(16.0, -28.0),
            p.at(12.0, -32.0),
        ]);
        let (hx, hy) = p.at(16.0, -32.0);
        c.ell(hx, hy, 6.0 * s, 4.0 * s);
        let (hx, hy) = p.at(21.0, -31.0);
        c.ell(hx, hy, 4.0 * s, 2.5 * s);
        c.stroke(0xe8e0c8ff, 1.5 * s);
        let (a, b) = (p.at(14.0, -35.0), p.at(10.0, -41.0));
        c.line(a.0, a.1, b.0, b.1);
        let (a, b) = (p.at(17.0, -35.5), p.at(15.0, -42.0));
        c.line(a.0, a.1, b.0, b.1);
        c.fill(0xffe040ff);
        let (ex, ey) = p.at(17.5, -33.0);
        c.ell(ex, ey, 1.2 * s, 1.0 * s);
        // near wing
        c.fill(shade(col, 0.9));
        c.poly(&[
            p.at(0.0, -22.0),
            p.at(4.0, -50.0 - flap * 7.0),
            p.at(-16.0, -44.0 - flap * 5.0),
            p.at(-8.0, -20.0),
        ]);
        c.stroke(shade(col, 0.5), 1.0 * s);
        let (a, b) = (p.at(0.0, -22.0), p.at(-6.0, -46.0 - flap * 6.0));
        c.line(a.0, a.1, b.0, b.1);
    }

    fn effect(&self, c: &mut Canvas, f: &Fx, z: f32, t: f32) {
        let k = (f.t / f.dur).clamp(0.0, 1.0);
        let (x0, y0) = self.to_screen(f.x0, f.y0);
        let (x1, y1) = self.to_screen(f.x1, f.y1);
        let (y0, y1) = (y0 - 26.0 * z, y1 - 24.0 * z);
        let lerp = |k: f32| (x0 + (x1 - x0) * k, y0 + (y1 - y0) * k);
        match f.kind {
            FX_ARROW => {
                let (a, b) = (lerp((k - 0.15).max(0.0)), lerp(k));
                c.stroke(0xff4040ff, 3.0 * z);
                c.line(a.0, a.1, b.0, b.1);
                c.fill(0xffc0c0ff);
                c.ell(b.0, b.1, 2.5 * z, 2.5 * z);
            }
            FX_FIREBALL | FX_BREATH => {
                let n = if f.kind == FX_BREATH { 6 } else { 3 };
                for i in 0..n {
                    let kk = (k - i as f32 * 0.07).max(0.0);
                    let (x, y) = lerp(kk);
                    let j = if f.kind == FX_BREATH {
                        ((i as f32 * 2.3 + t * 9.0).sin()) * 8.0 * z
                    } else {
                        0.0
                    };
                    let r = (8.0 - i as f32 * 1.5).max(2.0) * z;
                    c.fill(0xff7020d0);
                    c.ell(x + j, y + j * 0.5, r, r);
                    c.fill(0xffe070ff);
                    c.ell(x + j, y + j * 0.5, r * 0.5, r * 0.5);
                }
            }
            FX_EBOLT => {
                let (x, y) = lerp(k);
                let (a, _) = (lerp((k - 0.2).max(0.0)), 0);
                c.stroke(0x6080ffa0, 6.0 * z);
                c.line(a.0, a.1, x, y);
                c.fill(0xa0c0ffff);
                c.ell(x, y, 7.0 * z, 7.0 * z);
                c.fill(0xffffffff);
                c.ell(x, y, 3.5 * z, 3.5 * z);
            }
            FX_LIGHTNING => {
                let mut pts = vec![];
                let top = y1 - 260.0 * z;
                for i in 0..=8 {
                    let yy = top + (y1 - top) * i as f32 / 8.0;
                    let j = if i == 0 || i == 8 {
                        0.0
                    } else {
                        ((i as f32 * 12.9898 + f.x1 * 7.0 + (t * 30.0).floor()).sin() * 43758.5)
                            .fract()
                            - 0.5
                    };
                    pts.push((x1 + j * 30.0 * z, yy));
                }
                c.stroke(0x8ab0ffc0, 6.0 * z);
                c.poly_s(&pts);
                c.stroke(0xffffffff, 2.0 * z);
                c.poly_s(&pts);
                c.fill(with_a(0xffffffff, 0.4 * (1.0 - k)));
                c.ell(x1, y1 + 20.0 * z, 20.0 * z, 9.0 * z);
            }
            FX_EXPLOSION => {
                let r = (10.0 + k * 34.0) * z;
                c.fill(with_a(0xff8030ff, 0.85 * (1.0 - k)));
                c.ell(x1, y1, r, r * 0.8);
                c.fill(with_a(0xffe080ff, 0.9 * (1.0 - k)));
                c.ell(x1, y1, r * 0.5, r * 0.4);
            }
            FX_FLAMESTRIKE => {
                for i in 0..6 {
                    let fi = i as f32;
                    let yy = y1 + 20.0 * z - fi * 12.0 * z * (0.4 + k);
                    let w = (12.0 - fi) * z * (1.0 - k * 0.5);
                    c.fill(if i % 2 == 0 {
                        with_a(0xff6020ff, 0.8 * (1.0 - k))
                    } else {
                        with_a(0xffd060ff, 0.8 * (1.0 - k))
                    });
                    c.ell(x1 + (fi * 1.7 + t * 10.0).sin() * 3.0 * z, yy, w, w * 0.9);
                }
            }
            FX_HARM => {
                c.fill(with_a(0xa040e0ff, 0.6 * (1.0 - k)));
                c.ell(x1, y1, (8.0 + k * 18.0) * z, (6.0 + k * 12.0) * z);
            }
            FX_HEAL | FX_RES | FX_SPARK => {
                let col = if f.kind == FX_RES {
                    0xffe080ff
                } else {
                    0x80ffb0ff
                };
                for i in 0..8 {
                    let a = i as f32 * 0.785 + k * 3.0;
                    let r = 12.0 * z;
                    let yy = y0 + 20.0 * z - k * 40.0 * z + (i as f32 * 1.3).sin() * 6.0 * z;
                    c.fill(with_a(col, 1.0 - k));
                    c.ell(x0 + a.cos() * r, yy, 2.2 * z, 2.2 * z);
                }
                if f.kind == FX_RES {
                    c.fill(with_a(0xfff0b0ff, 0.35 * (1.0 - k)));
                    c.rect(x0 - 14.0 * z, y0 - 200.0 * z, 28.0 * z, 230.0 * z);
                }
            }
            FX_BLOOD => {
                let (bx, by) = self.to_screen(f.x0, f.y0);
                let col = f.tgt;
                for j in 0..8 {
                    let jf = j as f32;
                    let ang = jf * 0.8 + f.x0 * 3.0;
                    let (vx, vy) = (
                        ang.cos() * (10.0 + jf * 2.0),
                        -14.0 - (jf * 1.7).sin().abs() * 10.0,
                    );
                    let x = bx + vx * k * z;
                    let y = by - 22.0 * z + (vy * k + 46.0 * k * k) * z;
                    c.fill(with_a(col, 1.0 - k * 0.6));
                    let r = (2.2 - k) * z.max(0.8);
                    c.rect(x, y, r, r);
                }
            }
            FX_FIZZLE => {
                for i in 0..5 {
                    let fi = i as f32;
                    c.fill(with_a(0x909090ff, 0.6 * (1.0 - k)));
                    c.ell(
                        x0 + (fi - 2.0) * 5.0 * z,
                        y0 - k * 20.0 * z - fi * 2.0 * z,
                        (3.0 + k * 4.0) * z,
                        (3.0 + k * 4.0) * z,
                    );
                }
            }
            _ => {}
        }
    }
}
