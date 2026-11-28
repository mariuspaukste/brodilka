//! Terraces drawn on hex cells. The ground is square tiles, but where its
//! level changes the tops and the rock faces between them are drawn along
//! the edges of the hex cells the levels were shaped from (see
//! `world::hex_of`), so a ledge runs as one unbroken line however it lies
//! rather than in square steps. A slope's cell has its corners raised to
//! the terrace above it, with earth banks down its open sides.

use super::*;
use std::collections::HashSet;

impl Game {
    /// Is this tile's ground drawn as hex cells: land beside land of
    /// another level, and not a slope?
    pub(super) fn terraced(&self, x: i32, y: i32) -> bool {
        let t = self.map.tile(x, y);
        if !inb(x, y) || matches!(t, T_WATER | T_MOUNT | T_BRIDGE | T_VOID) {
            return false;
        }
        let l = self.map.level(x, y);
        DIRS.iter().any(|&(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            inb(nx, ny)
                && !matches!(self.map.tile(nx, ny), T_WATER | T_BRIDGE | T_VOID)
                && self.map.level(nx, ny) != l
        })
    }

    /// The level of a hex cell's corner `k`: the cell's own, or on a slope
    /// the terrace above where that corner touches it.
    fn corner_h(&self, q: i32, r: i32, k: usize) -> f32 {
        let (x, y) = hex_tile(q, r);
        let l = self.map.level(x, y) as f32;
        if !self.map.is_ramp(x, y) {
            return l;
        }
        let mut h = l;
        for nb in [HEX_NB[(k + 5) % 6], HEX_NB[k]] {
            let (nx, ny) = hex_tile(q + nb.0, r + nb.1);
            let t = self.map.tile(nx, ny);
            let nl = self.map.level(nx, ny) as f32;
            if nl == l + 1.0 && !matches!(t, T_WATER | T_MOUNT) && !self.map.is_ramp(nx, ny) {
                h = nl;
            }
        }
        h
    }

    /// The hex cells over these terraced tiles, back to front.
    pub(super) fn terraces(&self, c: &mut Canvas, tiles: &[(i32, i32)], holes: bool) {
        let z = self.zoom;
        let lz = LH * z;
        let ink = self.pix.max(z);
        let mut seen = HashSet::new();
        let mut cells = vec![];
        for &(x, y) in tiles {
            // every cell that reaches into the tile's diamond
            for (u, v) in
                (0..25).map(|k| ((k % 5) as f32 * 0.25 - 0.5, (k / 5) as f32 * 0.25 - 0.5))
            {
                let h = hex_of(x as f32 + u * 0.99, y as f32 + v * 0.99);
                if seen.insert(h) {
                    let (cx, cy) = hex_centre(h.0, h.1);
                    cells.push(((cx + cy) * 16.0, h));
                }
            }
        }
        cells.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        // the level and ground of a cell
        let cell = |q: i32, r: i32| {
            let (x, y) = hex_tile(q, r);
            (x, y, self.map.level(x, y) as f32, self.map.tile(x, y))
        };
        let scr = |p: (f32, f32), h: f32| {
            let (a, b) = self.to_screen(p.0, p.1);
            (a, b - h * lz)
        };
        for &(_, (q, r)) in &cells {
            let (x, y, lv, tl) = cell(q, r);
            let hs: Vec<f32> = (0..6).map(|k| self.corner_h(q, r, k)).collect();
            let top: Vec<(f32, f32)> = (0..6).map(|k| scr(hex_corner(q, r, k), hs[k])).collect();
            if tl == T_WATER {
                if holes {
                    c.erase(true);
                    c.fill(0xffffffff);
                    c.poly(&top);
                    c.erase(false);
                } else {
                    self.pat(c, TEX_WATER, 0.0);
                    c.poly(&top);
                }
                continue;
            }
            let tex = if tl == T_MOUNT {
                TEX_ROCK
            } else {
                self.ground_tex(x, y, tl)
            };
            self.pat(c, tex, lv * lz);
            c.poly(&top);
            let (hx, hy) = scr(hex_centre(q, r), lv);
            for k in 0..6 {
                let (p0, p1) = (top[k], top[(k + 1) % 6]);
                let (nq, nr) = (q + HEX_NB[k].0, r + HEX_NB[k].1);
                let (_, _, _, nt) = cell(nq, nr);
                // only the edges toward the viewer show a face
                if (p0.1 + p1.1) * 0.5 < hy {
                    continue;
                }
                // the neighbour's heights at the same two corners
                let (n0, n1) = (
                    self.corner_h(nq, nr, (k + 4) % 6),
                    self.corner_h(nq, nr, (k + 3) % 6),
                );
                let (h0, h1) = (hs[k], hs[(k + 1) % 6]);
                if h0 > n0 + 0.01 || h1 > n1 + 0.01 {
                    let face = [
                        p0,
                        p1,
                        (p1.0, p1.1 + (h1 - n1.min(h1)) * lz),
                        (p0.0, p0.1 + (h0 - n0.min(h0)) * lz),
                    ];
                    let e = -self.camx * z + self.w / 2.0;
                    let f = -self.camy * z + self.h / 2.0;
                    let slope = (p1.1 - p0.1) / (p1.0 - p0.0 + 0.001);
                    // a full step is rock; a slope's side is an earth bank
                    let rock = h0 > n0 + 0.01 && h1 > n1 + 0.01 && !self.map.is_ramp(x, y);
                    // faces turned to the right catch more light
                    let side = (((p0.0 + p1.0) * 0.5 - hx) / (16.0 * z)).clamp(-1.0, 1.0);
                    if rock {
                        c.outline(INK, ink);
                        c.pattern(TEX_CLIFF, z, slope * z, 0.0, z, e, f);
                        c.poly(&face);
                        c.outline(0, 0.0);
                        c.fill(with_a(0x000000ff, 0.31 - side * 0.13));
                    } else {
                        // a bank of the slope's own ground, barely shaded, so
                        // the slope reads as one surface
                        self.pat(c, tex, lv * lz);
                        c.poly(&face);
                        c.fill(with_a(0x000000ff, 0.1 - side * 0.05));
                    }
                    c.poly(&face);
                    if rock {
                        self.ledge_lip(c, x, y, &face, k as i32 + q * 7, z);
                    }
                }
                if nt == T_WATER {
                    // foam where the bank meets the water
                    let (d0, d1) = ((h0 - n0.min(h0)) * lz, (h1 - n1.min(h1)) * lz);
                    c.stroke(0xc8d4d880, 1.4 * ink);
                    c.line(p0.0, p0.1 + d0 + ink, p1.0, p1.1 + d1 + ink);
                    c.stroke(0xb8c8d038, ink);
                    c.line(p0.0, p0.1 + d0 + 4.0 * z, p1.0, p1.1 + d1 + 4.0 * z);
                }
            }
        }
    }
}
