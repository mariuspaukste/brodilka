//! Procedural textures and the 256-colour palette, generated at start-up.
//! Ground textures are 128x64 and periodic, which matches the 2:1 diamond
//! lattice, so the host can use them as world-anchored repeating patterns and
//! no tile grid is visible (as in Ultima VIII's rendered terrain).

pub const TEX_GRASS: u32 = 0;
pub const TEX_DIRT: u32 = 1;
pub const TEX_SAND: u32 = 2;
pub const TEX_FLAG: u32 = 3;
pub const TEX_FLOOR: u32 = 4;
pub const TEX_WATER: u32 = 5;
pub const TEX_CAVE: u32 = 6;
pub const TEX_ROCK: u32 = 7;
pub const TEX_BRIDGE: u32 = 8;
pub const TEX_CLIFF: u32 = 9;
pub const TEX_BRICK: u32 = 10;
pub const TEX_WOODWALL: u32 = 11;
pub const TEX_PLASTER: u32 = 12;

pub struct Texture {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

fn hash(x: i32, y: i32, s: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d)
        ^ (y as u32).wrapping_mul(0x1656_67b1)
        ^ s.wrapping_mul(0x9e37_79b9);
    h = (h ^ (h >> 15)).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    (h & 0xffff) as f32 / 65536.0
}

/// periodic value noise over a w x h domain with the given cell size
fn pnoise(x: f32, y: f32, cell: f32, w: usize, h: usize, s: u32) -> f32 {
    let gw = (w as f32 / cell).round().max(1.0) as i32;
    let gh = (h as f32 / cell).round().max(1.0) as i32;
    let (fx, fy) = (x / cell, y / cell);
    let (xi, yi) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - xi as f32, fy - yi as f32);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let g = |a: i32, b: i32| hash(a.rem_euclid(gw), b.rem_euclid(gh), s);
    let top = g(xi, yi) + (g(xi + 1, yi) - g(xi, yi)) * sx;
    let bot = g(xi, yi + 1) + (g(xi + 1, yi + 1) - g(xi, yi + 1)) * sx;
    top + (bot - top) * sy
}
fn fbm(x: f32, y: f32, w: usize, h: usize, s: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut norm = 0.0;
    for (o, cell) in [32.0, 16.0, 8.0, 4.0].iter().enumerate() {
        sum += pnoise(x, y, *cell, w, h, s + o as u32 * 77) * amp;
        norm += amp;
        amp *= 0.55;
    }
    sum / norm
}

fn rgb(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 255) as f32,
        ((c >> 8) & 255) as f32,
        (c & 255) as f32,
    ]
}

struct Img {
    w: usize,
    h: usize,
    px: Vec<[f32; 3]>,
}
impl Img {
    fn new(w: usize, h: usize) -> Img {
        Img {
            w,
            h,
            px: vec![[0.0; 3]; w * h],
        }
    }
    fn at(&mut self, x: i32, y: i32) -> &mut [f32; 3] {
        let (x, y) = (
            x.rem_euclid(self.w as i32) as usize,
            y.rem_euclid(self.h as i32) as usize,
        );
        &mut self.px[y * self.w + x]
    }
    fn mul(&mut self, x: i32, y: i32, k: f32) {
        let p = self.at(x, y);
        for c in p.iter_mut() {
            *c *= k;
        }
    }
    fn set(&mut self, x: i32, y: i32, c: [f32; 3]) {
        *self.at(x, y) = c;
    }
    fn fill_noise(&mut self, base: u32, lo: f32, hi: f32, s: u32) {
        let b = rgb(base);
        for y in 0..self.h {
            for x in 0..self.w {
                let n = fbm(x as f32, y as f32, self.w, self.h, s);
                let k = lo + (hi - lo) * n;
                self.px[y * self.w + x] = [b[0] * k, b[1] * k, b[2] * k];
            }
        }
    }
    fn done(self) -> Texture {
        let mut rgba = Vec::with_capacity(self.w * self.h * 4);
        for p in self.px {
            rgba.extend_from_slice(&[
                p[0].clamp(0.0, 255.0) as u8,
                p[1].clamp(0.0, 255.0) as u8,
                p[2].clamp(0.0, 255.0) as u8,
                255,
            ]);
        }
        Texture {
            w: self.w,
            h: self.h,
            rgba,
        }
    }
}

fn grass() -> Texture {
    let mut m = Img::new(128, 64);
    m.fill_noise(0x434826, 0.7, 1.25, 1);
    for i in 0..420 {
        let (x, y) = (
            (hash(i, 1, 2) * 128.0) as i32,
            (hash(i, 2, 2) * 64.0) as i32,
        );
        let len = 2 + (hash(i, 3, 2) * 3.0) as i32;
        let k = if i % 3 == 0 { 1.35 } else { 0.62 };
        for j in 0..len {
            m.mul(x + (j / 2) * if i % 2 == 0 { 1 } else { -1 }, y - j, k);
        }
    }
    for i in 0..18 {
        let (x, y) = (
            (hash(i, 9, 3) * 128.0) as i32,
            (hash(i, 8, 3) * 64.0) as i32,
        );
        m.set(x, y, rgb(if i % 2 == 0 { 0x6a5a30 } else { 0x5a3a2a }));
    }
    m.done()
}

fn dirt(base: u32, s: u32) -> Texture {
    let mut m = Img::new(128, 64);
    m.fill_noise(base, 0.72, 1.22, s);
    for i in 0..60 {
        let (x, y) = (
            (hash(i, 1, s) * 128.0) as i32,
            (hash(i, 2, s) * 64.0) as i32,
        );
        let k = 1.25 + hash(i, 3, s) * 0.3;
        m.mul(x, y, k);
        m.mul(x + 1, y, k);
        m.mul(x, y + 1, 0.55);
        m.mul(x + 1, y + 1, 0.55);
    }
    for i in 0..5 {
        let (mut x, mut y) = (
            (hash(i, 5, s) * 128.0) as i32,
            (hash(i, 6, s) * 64.0) as i32,
        );
        for j in 0..14 {
            m.mul(x, y, 0.6);
            x += 1;
            if hash(i, j, s + 9) < 0.4 {
                y += if hash(i, j, s + 7) < 0.5 { 1 } else { -1 };
            }
        }
    }
    m.done()
}

fn sand() -> Texture {
    let mut m = Img::new(128, 64);
    m.fill_noise(0x7a6a4c, 0.85, 1.15, 5);
    for y in 0..64 {
        for x in 0..128 {
            // ripples running along the iso x axis
            let r = ((y as f32 - x as f32 * 0.5) * std::f32::consts::PI / 8.0).sin();
            m.mul(x, y, 1.0 + r * 0.06);
            if hash(x, y, 6) < 0.03 {
                m.mul(x, y, 1.2);
            }
        }
    }
    m.done()
}

/// irregular flagstones from periodic Voronoi cells
fn flag() -> Texture {
    let (w, h) = (128usize, 64usize);
    let mut m = Img::new(w, h);
    let pts: Vec<(f32, f32, f32)> = (0..18)
        .map(|i| {
            (
                hash(i, 1, 11) * w as f32,
                hash(i, 2, 11) * h as f32,
                0.8 + hash(i, 3, 11) * 0.35,
            )
        })
        .collect();
    let b = rgb(0x5c5a58);
    for y in 0..h {
        for x in 0..w {
            let (mut d1, mut d2, mut best) = (1e9f32, 1e9f32, 0usize);
            for (k, &(px, py, _)) in pts.iter().enumerate() {
                for ox in [-1.0f32, 0.0, 1.0] {
                    for oy in [-1.0f32, 0.0, 1.0] {
                        let dx = x as f32 - (px + ox * w as f32);
                        let dy = (y as f32 - (py + oy * h as f32)) * 2.0;
                        let d = (dx * dx + dy * dy).sqrt();
                        if d < d1 {
                            d2 = d1;
                            d1 = d;
                            best = k;
                        } else if d < d2 {
                            d2 = d;
                        }
                    }
                }
            }
            let n = fbm(x as f32, y as f32, w, h, 12);
            let edge = d2 - d1;
            let mut k = pts[best].2 * (0.85 + n * 0.3);
            if edge < 1.6 {
                k = 0.5;
            } else if edge < 2.8 {
                k *= 1.1;
            }
            m.px[y * w + x] = [b[0] * k, b[1] * k, b[2] * k];
        }
    }
    m.done()
}

fn planks(base: u32, along_x: bool, s: u32) -> Texture {
    let mut m = Img::new(128, 64);
    let b = rgb(base);
    for y in 0..64i32 {
        for x in 0..128i32 {
            let v = if along_x {
                y as f32 - x as f32 * 0.5
            } else {
                y as f32 + x as f32 * 0.5
            };
            let idx = (v / 8.0).floor() as i32;
            let within = v.rem_euclid(8.0);
            let grain = pnoise(x as f32, y as f32 * 4.0, 8.0, 128, 256, s) * 0.25;
            let mut k = 0.8 + hash(idx.rem_euclid(8), 0, s) * 0.3 + grain;
            if within < 1.0 {
                k = 0.4;
            } else if within < 2.0 {
                k *= 1.12;
            }
            *m.at(x, y) = [b[0] * k, b[1] * k, b[2] * k];
        }
    }
    m.done()
}

fn water() -> Texture {
    let mut m = Img::new(128, 64);
    m.fill_noise(0x111b40, 0.75, 1.2, 21);
    for i in 0..26 {
        let (x, y) = (
            (hash(i, 1, 22) * 128.0) as i32,
            (hash(i, 2, 22) * 64.0) as i32,
        );
        let len = 3 + (hash(i, 3, 22) * 6.0) as i32;
        for j in 0..len {
            m.set(
                x + j,
                y,
                rgb(if j == len / 2 { 0x4a64a0 } else { 0x2a3a70 }),
            );
        }
    }
    m.done()
}

fn rock() -> Texture {
    let mut m = Img::new(128, 64);
    m.fill_noise(0x5e5650, 0.65, 1.25, 31);
    for i in 0..40 {
        let (x, y) = (
            (hash(i, 1, 32) * 128.0) as i32,
            (hash(i, 2, 32) * 64.0) as i32,
        );
        m.set(x, y, rgb(if i % 4 == 0 { 0x4a5a2c } else { 0x7a7268 }));
        m.mul(x + 1, y + 1, 0.6);
    }
    m.done()
}

fn cliff() -> Texture {
    let (w, h) = (64usize, 64usize);
    let mut m = Img::new(w, h);
    let b = rgb(0x524a44);
    for y in 0..h {
        for x in 0..w {
            let band = pnoise(x as f32 * 0.3, y as f32, 8.0, w, h, 41);
            let n = fbm(x as f32, y as f32, w, h, 42);
            let mut k = 0.7 + band * 0.35 + n * 0.25;
            if (y as f32 + pnoise(x as f32, 0.0, 16.0, w, h, 43) * 6.0).rem_euclid(13.0) < 1.0 {
                k *= 0.55;
            }
            m.px[y * w + x] = [b[0] * k, b[1] * k, b[2] * k];
        }
    }
    for i in 0..10 {
        let (mut x, mut y) = (
            (hash(i, 1, 44) * 64.0) as i32,
            (hash(i, 2, 44) * 64.0) as i32,
        );
        for j in 0..9 {
            m.mul(x, y, 0.5);
            y += 1;
            if hash(i, j, 45) < 0.3 {
                x += 1;
            }
        }
    }
    m.done()
}

fn brick(base: u32, s: u32) -> Texture {
    let (w, h) = (64usize, 64usize);
    let mut m = Img::new(w, h);
    let b = rgb(base);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let row = y / 8;
            let off = if row % 2 == 0 { 0 } else { 8 };
            let col = (x + off) / 16;
            let (bx, by) = ((x + off) % 16, y % 8);
            let n = fbm(x as f32, y as f32, w, h, s);
            let mut k = 0.78 + hash(col.rem_euclid(4), row, s) * 0.3 + n * 0.2;
            if bx == 0 || by == 0 {
                k = 0.42;
            } else if by == 1 || bx == 1 {
                k *= 1.12;
            } else if by == 7 {
                k *= 0.8;
            }
            *m.at(x, y) = [b[0] * k, b[1] * k, b[2] * k];
        }
    }
    m.done()
}

fn woodwall() -> Texture {
    let (w, h) = (64usize, 64usize);
    let mut m = Img::new(w, h);
    let b = rgb(0x5e4228);
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let col = x / 8;
            let grain = pnoise(x as f32 * 4.0, y as f32, 8.0, 256, 64, 51) * 0.25;
            let mut k = 0.78 + hash(col, 0, 52) * 0.25 + grain;
            if x % 8 == 0 {
                k = 0.4;
            }
            *m.at(x, y) = [b[0] * k, b[1] * k, b[2] * k];
        }
    }
    m.done()
}

pub fn textures() -> Vec<Texture> {
    vec![
        grass(),
        dirt(0x58402e, 2),
        sand(),
        flag(),
        planks(0x583a22, true, 3),
        water(),
        dirt(0x241c18, 7),
        rock(),
        planks(0x5a3e22, false, 8),
        cliff(),
        brick(0x6a6660, 61),
        woodwall(),
        brick(0x7e7462, 62),
    ]
}

/// A Pagan-ish 256-colour VGA palette built from hue ramps.
pub fn palette() -> Vec<u8> {
    let ramps: [(u32, usize); 14] = [
        (0x8a8a8a, 24), // neutral greys
        (0x7a7468, 20), // warm stone
        (0x6a7484, 16), // cool steel
        (0x8a5a34, 24), // browns / wood
        (0xa8843a, 16), // ochre / gold
        (0x6a7a34, 22), // olive / foliage
        (0x3a6a2a, 12), // deep green
        (0x2a3c86, 20), // pagan blues
        (0x9a1a10, 18), // blood
        (0xc89070, 16), // skin
        (0xe06a1a, 16), // fire
        (0x6a4a8a, 14), // violet
        (0x2a7a7a, 10), // teal
        (0xd8d0b8, 28), // bone / parchment highlights
    ];
    let mut pal = vec![];
    for &(c, n) in ramps.iter() {
        let b = rgb(c);
        for i in 0..n {
            let t = (i as f32 + 0.5) / n as f32;
            // dark end goes to near-black, top end lifts towards a pale tint
            let (k, lift) = if t < 0.75 {
                (0.06 + t / 0.75 * 0.94, 0.0)
            } else {
                (1.0, (t - 0.75) / 0.25 * 0.45)
            };
            for ch in b.iter() {
                let v = ch * k + (235.0 - ch) * lift;
                pal.push(v.clamp(0.0, 255.0) as u8);
            }
        }
    }
    while pal.len() < 768 {
        let i = pal.len() / 3;
        let v = ((i * 37) % 64) as u8;
        pal.extend_from_slice(&[v, v, v]);
    }
    pal.truncate(768);
    pal
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textures_are_periodic_sized() {
        let t = textures();
        assert_eq!(t.len(), 13);
        for x in t.iter() {
            assert_eq!(x.rgba.len(), x.w * x.h * 4);
            assert_eq!(x.w % 64, 0);
            assert_eq!(x.h % 32, 0);
        }
        assert_eq!(palette().len(), 768);
    }
}
