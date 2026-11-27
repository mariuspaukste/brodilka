//! A look at the world on hex tiles, for deciding whether to move to them.
//! Not part of the game: `Game::render_hex` draws the real map, snapped to a
//! hex grid, into the command buffer in place of the usual frame.
//!
//! Each hex takes the ground, level and object of the square tile under its
//! centre, so the land keeps its shape. Hexes cover one tile's area each.
//! Houses keep their footprints but their walls run along hex edges, cut away
//! to one storey with the roofs off. Two grids:
//!
//! - `0`: hexes laid square to the world's x and y, so on screen they lean
//!   with the isometric view;
//! - `1`: hexes laid square to the screen (flat top), squashed by the view
//!   as the usual isometric hex games draw them.

use super::*;

impl Game {
    /// Draw the view on hex tiles (`grid` 0: square to the world, 1: square
    /// to the screen).
    pub fn render_hex(&mut self, grid: u8) {
        self.prebake();
        let mut c = std::mem::take(&mut self.cmds);
        c.clear();
        c.ink = self.pix.max(1.0);
        self.draw_hex(&mut c, grid);
        self.cmds = c;
    }

    fn draw_hex(&self, c: &mut Canvas, grid: u8) {
        let z = self.zoom;
        let t = self.clock;
        let ink = self.pix.max(z);
        let lz = LH * z;
        c.fill(BG);
        c.rect(0.0, 0.0, self.w, self.h);

        // the hex frame: world = a * u + b * v
        let s2 = std::f32::consts::FRAC_1_SQRT_2;
        let (a, b) = if grid == 0 {
            ((1.0f32, 0.0f32), (0.0f32, 1.0f32))
        } else {
            ((s2, -s2), (s2, s2))
        };
        let to_world = |u: f32, v: f32| (a.0 * u + b.0 * v, a.1 * u + b.1 * v);
        let to_local = |x: f32, y: f32| (a.0 * x + a.1 * y, b.0 * x + b.1 * y);
        // flat-topped hexes of one tile's area
        let r = (2.0 / (3.0 * 3f32.sqrt())).sqrt();
        let centre = |q: i32, rr: i32| {
            let u = r * 1.5 * q as f32;
            let v = r * 3f32.sqrt() * (rr as f32 + q as f32 * 0.5);
            to_world(u, v)
        };
        let hex_of = |x: f32, y: f32| -> (i32, i32) {
            let (u, v) = to_local(x, y);
            let qf = u / (1.5 * r);
            let rf = v / (3f32.sqrt() * r) - qf * 0.5;
            // cube rounding
            let (cx, cz) = (qf, rf);
            let cy = -cx - cz;
            let (mut rx, ry, mut rz) = (cx.round(), cy.round(), cz.round());
            let (dx, dy, dz) = ((rx - cx).abs(), (ry - cy).abs(), (rz - cz).abs());
            if dx > dy && dx > dz {
                rx = -ry - rz;
            } else if dy <= dz {
                rz = -rx - ry;
            }
            (rx as i32, rz as i32)
        };
        let corner = |q: i32, rr: i32, k: usize| {
            let (cx, cy) = centre(q, rr);
            let ang = std::f32::consts::PI / 3.0 * k as f32;
            let (du, dv) = (r * ang.cos(), r * ang.sin());
            let (dx, dy) = (a.0 * du + b.0 * dv, a.1 * du + b.1 * dv);
            (cx + dx, cy + dy)
        };
        // neighbour across edge k (between corners k and k+1)
        const NB: [(i32, i32); 6] = [(1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1), (1, -1)];

        let tile = |q: i32, rr: i32| {
            let (x, y) = centre(q, rr);
            (x.round() as i32, y.round() as i32)
        };
        let level = |q: i32, rr: i32| {
            let (x, y) = tile(q, rr);
            self.map.level(x, y) as f32
        };
        let scr = |wx: f32, wy: f32, h: f32| {
            let (sx, sy) = self.to_screen(wx, wy);
            (sx, sy - h * lz)
        };
        // a texture on an upright face running along screen direction (dx, dy)
        let face_pat = |c: &mut Canvas, tex: u32, dx: f32, dy: f32| {
            let e = -self.camx * z + self.w / 2.0;
            let f = -self.camy * z + self.h / 2.0;
            let k = if dx.abs() < 0.01 { 0.0 } else { dy / dx };
            c.pattern(tex, z, k * z, 0.0, z, e, f);
        };

        // the hexes on screen, back to front
        let (cx, cy) = (
            (self.camx / 32.0 + self.camy / 16.0) * 0.5,
            (self.camy / 16.0 - self.camx / 32.0) * 0.5,
        );
        let reach = (self.w.max(self.h) / (24.0 * z)).ceil() as i32 + 6;
        let (q0, r0) = hex_of(cx, cy);
        let mut hexes: Vec<(f32, i32, i32)> = vec![];
        for dq in -reach..=reach {
            for dr in -reach..=reach {
                let (q, rr) = (q0 + dq, r0 + dr);
                let (x, y) = centre(q, rr);
                if !inb(x.round() as i32, y.round() as i32) {
                    continue;
                }
                let (sx, sy) = self.to_screen(x, y);
                if sx < -80.0 || sx > self.w + 80.0 || sy < -80.0 || sy > self.h + 160.0 {
                    continue;
                }
                hexes.push((sy, q, rr));
            }
        }
        hexes.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());

        // ---- the ground: tops, and rock faces down to lower neighbours
        for &(_, q, rr) in &hexes {
            let (x, y) = tile(q, rr);
            let tl = self.map.tile(x, y);
            let lv = level(q, rr);
            let top: Vec<(f32, f32)> = (0..6)
                .map(|k| {
                    let (wx, wy) = corner(q, rr, k);
                    scr(wx, wy, lv)
                })
                .collect();
            let tex = if tl == T_MOUNT {
                TEX_ROCK
            } else {
                self.ground_tex(x, y, tl)
            };
            self.pat(c, tex, lv * lz);
            c.poly(&top);
            for k in 0..6 {
                let (nq, nr) = (q + NB[k].0, rr + NB[k].1);
                let nl = level(nq, nr);
                let (p0, p1) = (top[k], top[(k + 1) % 6]);
                // only the edges facing the viewer show a face
                let my = (p0.1 + p1.1) * 0.5;
                let hy = scr(centre(q, rr).0, centre(q, rr).1, lv).1;
                if my < hy || nl >= lv {
                    continue;
                }
                let d = (lv - nl) * lz;
                let face = [p0, p1, (p1.0, p1.1 + d), (p0.0, p0.1 + d)];
                c.outline(INK, ink);
                face_pat(c, TEX_CLIFF, p1.0 - p0.0, p1.1 - p0.1);
                c.poly(&face);
                c.outline(0, 0.0);
                // lit by its slant: faces turned to the right are lighter
                let lit = ((p1.1 - p0.1) / (p1.0 - p0.0 + 0.01)).clamp(-1.0, 1.0);
                c.fill(with_a(0x000000ff, 0.28 - lit * 0.12));
                c.poly(&face);
            }
            if tl == T_WATER {
                c.fill(0xffffff10);
                c.poly(&top);
            }
        }
        // the hex grid itself, faintly, so the cells can be judged
        c.outline(0, 0.0);
        c.stroke(0x00000058, ink);
        for &(_, q, rr) in &hexes {
            let lv = level(q, rr);
            for k in 0..3 {
                let (w0, w1) = (corner(q, rr, k), corner(q, rr, k + 1));
                let (p0, p1) = (scr(w0.0, w0.1, lv), scr(w1.0, w1.1, lv));
                c.line(p0.0, p0.1, p1.0, p1.1);
            }
        }

        // ---- objects, walls and people, back to front
        let mut objs: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        for &(_, q, rr) in &hexes {
            let (x, y) = tile(q, rr);
            let o = self.map.obj(x, y);
            if o != O_NONE && o != O_PART && o != O_WALL && o != O_DOOR {
                objs.insert((q, rr), (x, y));
            }
        }
        let mut people: Vec<(f32, usize, f32, f32)> = vec![];
        for (i, e) in self.ents.iter().enumerate() {
            if e.dead || e.fl != 0 {
                continue;
            }
            let (q, rr) = hex_of(e.x, e.y);
            let (wx, wy) = centre(q, rr);
            let (sx, sy) = scr(wx, wy, level(q, rr));
            if sx < -100.0 || sx > self.w + 100.0 || sy < -100.0 || sy > self.h + 200.0 {
                continue;
            }
            people.push((self.to_screen(wx, wy).1, i, sx, sy));
        }
        people.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
        let mut pi = 0;
        let mut lights = vec![];
        let bld = |q: i32, rr: i32| {
            let (x, y) = tile(q, rr);
            self.map.building_at(x, y)
        };
        for &(hy, q, rr) in &hexes {
            let (x, y) = tile(q, rr);
            let lv = level(q, rr);
            let (wx, wy) = centre(q, rr);
            let (sx, sy) = scr(wx, wy, lv);
            // walls on this hex's edges that lie behind its centre
            for k in 0..6 {
                let (nq, nr) = (q + NB[k].0, rr + NB[k].1);
                let (here, there) = (bld(q, rr), bld(nq, nr));
                if here == there {
                    continue;
                }
                let w0 = corner(q, rr, k);
                let w1 = corner(q, rr, (k + 1) % 6);
                let (m0, m1) = (scr(w0.0, w0.1, lv), scr(w1.0, w1.1, lv));
                if (m0.1 + m1.1) * 0.5 > sy {
                    continue;
                }
                // a doorway stays open
                let (nx, ny) = tile(nq, nr);
                if self.map.obj(x, y) == O_DOOR || self.map.obj(nx, ny) == O_DOOR {
                    continue;
                }
                let style = match (here, there) {
                    (Some(b), _) | (None, Some(b)) => self.map.buildings[b].style,
                    _ => continue,
                };
                if !self.map.buildings[here.or(there).unwrap()].thin() {
                    // thick-walled halls are left out of this look
                    continue;
                }
                let hgt = STOREY_PX * z * wall_h(style);
                let face = [m0, m1, (m1.0, m1.1 - hgt), (m0.0, m0.1 - hgt)];
                c.outline(INK, ink);
                face_pat(c, wall_tex(style), m1.0 - m0.0, m1.1 - m0.1);
                c.poly(&face);
                c.outline(0, 0.0);
                let lit = ((m1.1 - m0.1) / (m1.0 - m0.0 + 0.01)).clamp(-1.0, 1.0);
                c.fill(with_a(0x000000ff, 0.32 - lit * 0.14));
                c.poly(&face);
                c.stroke(0x2a1a10ff, 2.0 * ink);
                c.line(m0.0, m0.1 - hgt, m1.0, m1.1 - hgt);
            }
            if let Some(&(ox, oy)) = objs.get(&(q, rr)) {
                let o = self.map.obj(ox, oy);
                let i = idx(ox, oy);
                c.outline(INK, ink);
                if self.map.tile(ox, oy) == T_MOUNT {
                    self.mountain(c, ox, oy, sx, sy, self.map.shade[i], z);
                } else {
                    self.object(
                        c,
                        o,
                        ox,
                        oy,
                        sx,
                        sy,
                        self.map.shade[i],
                        self.map.style[i],
                        z,
                        t,
                        &mut lights,
                    );
                }
                c.outline(0, 0.0);
            } else if self.map.tile(x, y) == T_MOUNT {
                self.mountain(c, x, y, sx, sy, self.map.shade[idx(x, y)], z);
            } else if self.map.obj(x, y) == O_NONE {
                self.clutter(c, x, y, sx, sy, self.map.tile(x, y), z);
            }
            while pi < people.len() && people[pi].0 <= hy + 0.5 {
                let (_, i, px, py) = people[pi];
                c.outline(INK, ink);
                self.entity(c, &self.ents[i], px, py, z, t);
                c.outline(0, 0.0);
                pi += 1;
            }
        }
        c.dark(
            self.w / 2.0,
            self.h / 2.0,
            self.w.min(self.h) * 0.34,
            self.w.max(self.h) * 0.75,
            0.38,
            0x050302ff,
        );
    }
}
