//! Toolbar and inventory icons, modelled and lit like everything else in the
//! world, then cropped and centred into small square cells.

use super::*;

pub const ICON: usize = 48;

/// Icons in sheet order. The first eleven are fixed for the toolbar and HUD.
pub const ICONS: &[&str] = &[
    "war",
    "status",
    "pack",
    "skills",
    "spells",
    "last",
    "bandage",
    "potion",
    "log",
    "help",
    "gold",
    "dagger",
    "katana",
    "broadsword",
    "longsword",
    "hatchet",
    "mace",
    "warhammer",
    "pickaxe",
    "smithhammer",
    "fishingpole",
    "robe",
    "leather",
    "ringmail",
    "chainmail",
    "platemail",
    "buckler",
    "heater",
    "fish",
    "ore",
    "ingot",
    "logs",
    "ruby",
    "sapphire",
    "diamond",
    "bp",
    "bm",
    "ga",
    "gi",
    "mr",
    "ns",
    "ss",
    "sa",
    "scroll",
    "spellbook",
    "healpotion",
    "ale",
    "bread",
];

pub fn icon_index(id: &str) -> Option<usize> {
    let id = if id.starts_with("scroll_") {
        "scroll"
    } else {
        id
    };
    ICONS.iter().position(|&n| n == id)
}

const R2: f32 = std::f32::consts::FRAC_1_SQRT_2;
const STEEL: u32 = 0xb8bcc4ff;
const IRON: u32 = 0x5a5e66ff;
const WOOD: u32 = 0x6a4628ff;
const LEATHER: u32 = 0x7a5030ff;
const GOLD: u32 = 0xd8b050ff;

/// a point in the picture plane: h to the right, v up, d towards the viewer
fn ip(h: f32, v: f32, d: f32) -> V3 {
    [(h + d) * R2, (d - h) * R2, v]
}

struct Ic {
    p: Vec<Prim>,
    b: Vec<BoxP>,
}
impl Ic {
    fn seg(&mut self, a: (f32, f32), b: (f32, f32), ra: f32, rb: f32, col: u32, mat: Mat) {
        prim(
            &mut self.p,
            ip(a.0, a.1, 0.0),
            ip(b.0, b.1, 0.0),
            ra,
            rb,
            col,
            mat,
        );
    }
    fn seg3(&mut self, a: V3, b: V3, ra: f32, rb: f32, col: u32, mat: Mat) {
        prim(
            &mut self.p,
            ip(a[0], a[1], a[2]),
            ip(b[0], b[1], b[2]),
            ra,
            rb,
            col,
            mat,
        );
    }
    fn ball(&mut self, c: (f32, f32), r: f32, col: u32, mat: Mat) {
        self.seg(c, c, r, r, col, mat);
    }
    /// a flat shape facing the viewer, filled with horizontal strokes;
    /// `half(v)` gives its half width at height v
    fn plate(&mut self, v0: f32, v1: f32, half: impl Fn(f32) -> f32, col: u32, mat: Mat, th: f32) {
        let n = ((v1 - v0) / (th * 0.8)).ceil() as usize;
        for i in 0..=n {
            let v = v0 + (v1 - v0) * i as f32 / n as f32;
            let w = half(v);
            if w > 0.0 {
                self.seg((-w, v), (w, v), th, th, col, mat);
            }
        }
    }
    /// a box lying in the picture plane, h/v half sizes, d half depth
    fn slab(&mut self, c: (f32, f32, f32), hh: f32, hv: f32, hd: f32, col: u32, mat: Mat) {
        // boxes only turn about z: the picture plane's h axis is a 45° yaw
        self.b.push(BoxP {
            c: ip(c.0, c.1, c.2),
            h: [hh, hd, hv],
            yaw: -std::f32::consts::FRAC_PI_4,
            color: col,
            mat,
        });
    }
    /// a blade along a diagonal from hilt to tip, with guard, grip and pommel
    fn sword(&mut self, base: (f32, f32), dir: (f32, f32), len: f32, wid: f32, curve: f32) {
        let tip = (base.0 + dir.0 * len, base.1 + dir.1 * len);
        let mid = (
            base.0 + dir.0 * len * 0.55 - dir.1 * curve,
            base.1 + dir.1 * len * 0.55 + dir.0 * curve,
        );
        self.seg(base, mid, wid, wid * 0.85, STEEL, Mat::Metal);
        self.seg(mid, tip, wid * 0.85, 0.008, STEEL, Mat::Metal);
        let (px, py) = (-dir.1, dir.0);
        self.seg(
            (base.0 + px * 0.13, base.1 + py * 0.13),
            (base.0 - px * 0.13, base.1 - py * 0.13),
            0.03,
            0.03,
            GOLD,
            Mat::Metal,
        );
        let g = (base.0 - dir.0 * 0.16, base.1 - dir.1 * 0.16);
        self.seg(base, g, 0.03, 0.03, 0x3a2416ff, Mat::Matte);
        self.ball(
            (g.0 - dir.0 * 0.03, g.1 - dir.1 * 0.03),
            0.045,
            GOLD,
            Mat::Metal,
        );
    }
    fn haft(&mut self, a: (f32, f32), b: (f32, f32)) {
        self.seg(a, b, 0.032, 0.028, WOOD, Mat::Matte);
    }
    fn torso(&mut self, col: u32, mat: Mat, robe: bool) {
        if robe {
            self.plate(-0.5, 0.32, |v| 0.17 + (0.32 - v) * 0.28, col, mat, 0.06);
        } else {
            self.plate(-0.28, 0.32, |v| 0.2 + (v + 0.28) * 0.06, col, mat, 0.07);
        }
        for s in [-1.0f32, 1.0] {
            self.seg(
                (s * 0.24, 0.3),
                (s * 0.34, if robe { -0.1 } else { 0.05 }),
                0.08,
                0.07,
                col,
                mat,
            );
        }
        self.seg(
            (-0.08, 0.36),
            (0.08, 0.36),
            0.05,
            0.05,
            shade_col(col, 0.7),
            Mat::Matte,
        );
    }
}

fn model(name: &str, color: u32) -> (Figure, f32) {
    let mut m = Ic {
        p: vec![],
        b: vec![],
    };
    let mut scale = 1.27;
    match name {
        "war" => {
            m.sword((-0.3, -0.3), (R2, R2), 0.75, 0.04, 0.0);
            m.sword((0.3, -0.3), (-R2, R2), 0.75, 0.04, 0.0);
        }
        "status" => {
            let mut l = Look::human(0xe0b090ff, 0x8a6a4aff, 0x3a3040ff);
            l.hair = 0x3b2414ff;
            let f = figure_for(&l, Anim::Idle, 0, 1);
            return (f, 0.78);
        }
        "pack" => {
            m.plate(
                -0.35,
                0.25,
                |v| 0.26 - (v - 0.25).abs().powi(3) * 0.4,
                LEATHER,
                Mat::Matte,
                0.12,
            );
            m.plate(
                0.05,
                0.3,
                |v| 0.27 - (v - 0.05) * 0.1,
                shade_col(LEATHER, 0.8),
                Mat::Matte,
                0.13,
            );
            m.seg3(
                [0.0, 0.1, 0.15],
                [0.0, 0.0, 0.16],
                0.04,
                0.04,
                GOLD,
                Mat::Metal,
            );
            m.seg3(
                [-0.18, 0.3, -0.05],
                [-0.1, 0.45, -0.05],
                0.03,
                0.03,
                shade_col(LEATHER, 0.6),
                Mat::Matte,
            );
            m.seg3(
                [0.18, 0.3, -0.05],
                [0.1, 0.45, -0.05],
                0.03,
                0.03,
                shade_col(LEATHER, 0.6),
                Mat::Matte,
            );
            m.seg3(
                [-0.1, 0.45, -0.05],
                [0.1, 0.45, -0.05],
                0.03,
                0.03,
                shade_col(LEATHER, 0.6),
                Mat::Matte,
            );
            m.plate(
                -0.3,
                -0.12,
                |_| 0.15,
                shade_col(LEATHER, 0.9),
                Mat::Matte,
                0.1,
            );
        }
        "skills" | "scroll" => {
            let paper = 0xd8c8a0ff;
            m.plate(-0.2, 0.22, |_| 0.3, paper, Mat::Matte, 0.03);
            for v in [-0.24f32, 0.26] {
                m.seg(
                    (-0.36, v),
                    (0.36, v),
                    0.07,
                    0.07,
                    shade_col(paper, 0.85),
                    Mat::Matte,
                );
            }
            for i in 0..4 {
                let v = 0.12 - i as f32 * 0.08;
                m.seg3(
                    [-0.2, v, 0.04],
                    [0.15 - (i % 2) as f32 * 0.1, v, 0.04],
                    0.012,
                    0.012,
                    0x3a2a1aff,
                    Mat::Matte,
                );
            }
            let seal = if name == "scroll" {
                0x2a4aa0ff
            } else {
                0x8a1a18ff
            };
            m.seg3(
                [0.18, -0.12, 0.05],
                [0.18, -0.12, 0.06],
                0.06,
                0.06,
                seal,
                Mat::Hair,
            );
        }
        "spells" | "spellbook" | "log" => {
            let cover = match name {
                "log" => 0x5a3a22ff,
                _ => 0x6a1a20ff,
            };
            m.slab((0.0, 0.0, 0.09), 0.3, 0.38, 0.022, cover, Mat::Matte);
            m.slab((0.0, 0.0, -0.09), 0.3, 0.38, 0.022, cover, Mat::Matte);
            m.slab(
                (-0.29, 0.0, 0.0),
                0.03,
                0.38,
                0.1,
                shade_col(cover, 0.8),
                Mat::Matte,
            );
            m.slab((0.01, 0.0, 0.0), 0.27, 0.35, 0.07, 0xe0d4b0ff, Mat::Matte);
            m.seg3(
                [-0.3, 0.0, 0.11],
                [-0.3, 0.0, 0.11],
                0.05,
                0.05,
                GOLD,
                Mat::Metal,
            );
            if name != "log" {
                m.seg3(
                    [-0.12, 0.12, 0.11],
                    [0.12, -0.12, 0.11],
                    0.02,
                    0.02,
                    GOLD,
                    Mat::Metal,
                );
                m.seg3(
                    [0.12, 0.12, 0.11],
                    [-0.12, -0.12, 0.11],
                    0.02,
                    0.02,
                    GOLD,
                    Mat::Metal,
                );
            } else {
                m.seg3(
                    [-0.15, 0.2, 0.11],
                    [0.2, 0.2, 0.11],
                    0.015,
                    0.015,
                    GOLD,
                    Mat::Metal,
                );
            }
        }
        "last" => {
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                let l = if k % 2 == 0 { 0.45 } else { 0.25 };
                m.seg(
                    (0.0, 0.0),
                    (a.cos() * l, a.sin() * l),
                    0.07,
                    0.01,
                    0xffd060ff,
                    Mat::Glow,
                );
            }
            m.ball((0.0, 0.0), 0.12, 0xfff0c0ff, Mat::Glow);
            m.ball((0.3, 0.3), 0.04, 0x80c0ffff, Mat::Glow);
            m.ball((-0.28, -0.32), 0.035, 0x80c0ffff, Mat::Glow);
        }
        "bandage" => {
            m.seg3(
                [-0.22, 0.0, 0.0],
                [0.18, 0.0, 0.0],
                0.22,
                0.22,
                0xe8e4d8ff,
                Mat::Matte,
            );
            m.seg3(
                [0.19, 0.0, 0.0],
                [0.2, 0.0, 0.0],
                0.1,
                0.1,
                0xb8b0a0ff,
                Mat::Matte,
            );
            m.seg3(
                [0.0, -0.2, 0.06],
                [0.35, -0.42, 0.12],
                0.05,
                0.04,
                0xe8e4d8ff,
                Mat::Matte,
            );
            m.seg3(
                [-0.1, 0.05, 0.22],
                [0.05, 0.05, 0.22],
                0.025,
                0.025,
                0x9a1a14ff,
                Mat::Matte,
            );
        }
        "potion" | "healpotion" => {
            let liquid = if name == "potion" || name == "healpotion" {
                0xc02818ff
            } else {
                color
            };
            m.ball((0.0, -0.15), 0.27, liquid, Mat::Metal);
            m.seg((0.0, 0.08), (0.0, 0.3), 0.08, 0.07, 0x9ab0b0ff, Mat::Metal);
            m.seg((0.0, 0.3), (0.0, 0.42), 0.075, 0.07, 0x8a6a40ff, Mat::Matte);
            m.ball((-0.09, -0.06), 0.05, 0xffd0c0ff, Mat::Glow);
        }
        "help" => {
            // a lantern
            m.seg((0.0, 0.32), (0.0, 0.45), 0.05, 0.03, IRON, Mat::Metal);
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                let a2 = (k + 1) as f32 / 8.0 * std::f32::consts::TAU;
                m.seg(
                    (a.cos() * 0.1, 0.5 + a.sin() * 0.1),
                    (a2.cos() * 0.1, 0.5 + a2.sin() * 0.1),
                    0.02,
                    0.02,
                    IRON,
                    Mat::Metal,
                );
            }
            m.ball((0.0, 0.0), 0.2, 0xffb040ff, Mat::Glow);
            m.ball((0.0, 0.0), 0.11, 0xfff0b0ff, Mat::Glow);
            for h in [-0.2f32, 0.2] {
                m.seg((h, -0.3), (h, 0.3), 0.03, 0.03, IRON, Mat::Metal);
            }
            m.plate(0.28, 0.34, |_| 0.24, IRON, Mat::Metal, 0.04);
            m.plate(-0.36, -0.28, |_| 0.25, IRON, Mat::Metal, 0.04);
        }
        "gold" => {
            for (i, (h, v, d)) in [
                (0.0f32, -0.2f32, 0.0f32),
                (0.0, -0.13, 0.0),
                (0.0, -0.06, 0.0),
                (0.0, 0.01, 0.0),
                (0.22, -0.25, 0.2),
                (-0.25, -0.24, 0.15),
            ]
            .iter()
            .enumerate()
            {
                for k in 0..8 {
                    m.b.push(BoxP {
                        c: ip(*h, *v, *d),
                        h: [0.2, 0.083, 0.03],
                        yaw: k as f32 / 8.0 * std::f32::consts::PI,
                        color: if i % 2 == 0 { GOLD } else { 0xc89a40ff },
                        mat: Mat::Metal,
                    });
                }
            }
            scale = 1.1;
        }
        "dagger" => m.sword((-0.2, -0.2), (R2, R2), 0.45, 0.04, 0.0),
        "katana" => m.sword((-0.33, -0.33), (R2, R2), 0.85, 0.028, 0.05),
        "broadsword" => m.sword((-0.3, -0.3), (R2, R2), 0.72, 0.06, 0.0),
        "longsword" => m.sword((-0.35, -0.35), (R2, R2), 0.9, 0.045, 0.0),
        "hatchet" | "pickaxe" | "smithhammer" | "warhammer" | "mace" => {
            let (a, b) = ((-0.35, -0.38), (0.25, 0.32));
            m.haft(a, b);
            match name {
                "hatchet" => {
                    // a bearded blade fanning out from the haft, drawn in vertical strokes
                    for i in 0..10 {
                        let h = i as f32 * 0.026;
                        let hh = 0.05 + h * h * 2.6;
                        let c = (0.25 + h * R2 * 1.2, 0.32 - h * R2 * 0.6);
                        let col = if i >= 8 { STEEL } else { IRON };
                        m.seg(
                            (c.0, c.1 - hh),
                            (c.0, c.1 + hh * 0.7),
                            0.03,
                            0.03,
                            col,
                            Mat::Metal,
                        );
                    }
                }
                "pickaxe" => {
                    m.seg((0.25, 0.32), (0.5, 0.15), 0.06, 0.015, IRON, Mat::Metal);
                    m.seg((0.25, 0.32), (0.05, 0.5), 0.06, 0.015, IRON, Mat::Metal);
                }
                "mace" => {
                    m.ball((0.27, 0.34), 0.15, IRON, Mat::Metal);
                    for k in 0..6 {
                        let a = k as f32 / 6.0 * std::f32::consts::TAU;
                        m.seg(
                            (0.27, 0.34),
                            (0.27 + a.cos() * 0.22, 0.34 + a.sin() * 0.22),
                            0.05,
                            0.01,
                            IRON,
                            Mat::Metal,
                        );
                    }
                }
                _ => {
                    let w = if name == "warhammer" { 0.17 } else { 0.12 };
                    m.seg(
                        (0.25 - w * R2, 0.32 + w * R2),
                        (0.25 + w * R2, 0.32 - w * R2),
                        0.09,
                        0.09,
                        IRON,
                        Mat::Metal,
                    );
                }
            }
        }
        "fishingpole" => {
            m.seg((-0.4, -0.4), (0.4, 0.42), 0.03, 0.01, WOOD, Mat::Matte);
            m.seg(
                (0.4, 0.42),
                (0.42, -0.1),
                0.006,
                0.006,
                0xc0c0b0ff,
                Mat::Matte,
            );
            m.ball((0.42, -0.12), 0.05, 0xc02020ff, Mat::Hair);
            m.ball((-0.25, -0.24), 0.06, IRON, Mat::Metal);
        }
        "robe" => m.torso(color, Mat::Matte, true),
        "leather" => m.torso(color, Mat::Hair, false),
        "ringmail" | "chainmail" => {
            m.torso(color, Mat::Metal, false);
            for i in 0..5 {
                let v = 0.22 - i as f32 * 0.11;
                m.seg3(
                    [-0.2, v, 0.08],
                    [0.2, v, 0.08],
                    0.012,
                    0.012,
                    shade_col(color, 0.6),
                    Mat::Metal,
                );
            }
        }
        "platemail" => {
            m.torso(color, Mat::Metal, false);
            m.seg3(
                [0.0, 0.3, 0.1],
                [0.0, -0.22, 0.1],
                0.03,
                0.03,
                shade_col(color, 1.15),
                Mat::Metal,
            );
        }
        "buckler" => {
            m.plate(
                -0.36,
                0.36,
                |v| (0.36f32 * 0.36 - v * v).max(0.0).sqrt(),
                color,
                Mat::Matte,
                0.04,
            );
            m.seg3(
                [0.0, 0.0, 0.02],
                [0.0, 0.0, 0.08],
                0.1,
                0.08,
                STEEL,
                Mat::Metal,
            );
            for k in 0..12 {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                let a2 = (k + 1) as f32 / 12.0 * std::f32::consts::TAU;
                m.seg3(
                    [a.cos() * 0.35, a.sin() * 0.35, 0.03],
                    [a2.cos() * 0.35, a2.sin() * 0.35, 0.03],
                    0.025,
                    0.025,
                    IRON,
                    Mat::Metal,
                );
            }
        }
        "heater" => {
            m.plate(
                -0.45,
                0.38,
                |v| {
                    if v > 0.0 {
                        0.32
                    } else {
                        0.32 * (1.0 - (v / -0.45).powf(1.6))
                    }
                },
                color,
                Mat::Matte,
                0.04,
            );
            m.seg3(
                [0.0, 0.34, 0.04],
                [0.0, -0.4, 0.04],
                0.03,
                0.02,
                GOLD,
                Mat::Metal,
            );
            m.seg3(
                [-0.28, 0.12, 0.04],
                [0.28, 0.12, 0.04],
                0.03,
                0.03,
                GOLD,
                Mat::Metal,
            );
        }
        "fish" => {
            let c = 0x8a9aa0ff;
            m.seg((-0.3, 0.0), (0.0, 0.03), 0.06, 0.15, c, Mat::Metal);
            m.seg((0.0, 0.03), (0.28, 0.0), 0.15, 0.06, c, Mat::Metal);
            m.seg((-0.3, 0.0), (-0.45, 0.13), 0.04, 0.07, c, Mat::Metal);
            m.seg((-0.3, 0.0), (-0.45, -0.13), 0.04, 0.07, c, Mat::Metal);
            m.seg3(
                [0.22, 0.04, 0.06],
                [0.22, 0.04, 0.07],
                0.025,
                0.025,
                0x101010ff,
                Mat::Matte,
            );
        }
        "ale" => {
            // a pewter tankard with a wooden handle and a head of foam
            let pewter = 0x8a8e94ff;
            m.seg((0.0, -0.32), (0.0, 0.22), 0.2, 0.18, pewter, Mat::Metal);
            for v in [-0.26f32, 0.14] {
                m.seg3(
                    [0.0, v, 0.0],
                    [0.0, v + 0.03, 0.0],
                    0.215,
                    0.205,
                    shade_col(pewter, 0.7),
                    Mat::Metal,
                );
            }
            m.seg((0.16, 0.12), (0.34, 0.06), 0.035, 0.035, WOOD, Mat::Matte);
            m.seg((0.34, 0.06), (0.32, -0.2), 0.035, 0.035, WOOD, Mat::Matte);
            m.seg((0.32, -0.2), (0.16, -0.22), 0.035, 0.035, WOOD, Mat::Matte);
            m.seg(
                (-0.08, 0.27),
                (0.08, 0.27),
                0.14,
                0.14,
                0xe8dcc0ff,
                Mat::Matte,
            );
            m.ball((-0.12, 0.3), 0.07, 0xf0e8d0ff, Mat::Matte);
        }
        "bread" => {
            // a dark round loaf, scored across the top
            let crust = 0x7a4a22ff;
            m.seg((-0.3, -0.08), (0.3, -0.08), 0.2, 0.2, crust, Mat::Matte);
            m.ball((0.0, 0.0), 0.22, shade_col(crust, 1.1), Mat::Matte);
            for h in [-0.16f32, 0.0, 0.16] {
                m.seg3(
                    [h - 0.06, 0.1, 0.16],
                    [h + 0.06, 0.16, 0.14],
                    0.02,
                    0.02,
                    0xc8a060ff,
                    Mat::Matte,
                );
            }
        }
        "ore" | "sa" => {
            let (c, fleck) = if name == "ore" {
                (0x4a4642ffu32, 0xc08a50ffu32)
            } else {
                (0x8a8470ff, 0xd8c860ff)
            };
            for k in 0..7 {
                let h = (hsh(k, 3) - 0.5) * 0.55;
                let v = -0.2 + hsh(k, 4) * 0.3 - h.abs() * 0.3;
                let r = 0.15 + hsh(k, 5) * 0.09;
                m.ball(
                    (h, v),
                    r,
                    if k % 3 == 0 { shade_col(c, 1.2) } else { c },
                    Mat::Matte,
                );
                m.seg3(
                    [h + 0.04, v + 0.04, r * 0.8],
                    [h + 0.04, v + 0.04, r * 0.85],
                    0.03,
                    0.03,
                    fleck,
                    Mat::Metal,
                );
            }
        }
        "ingot" => {
            m.b.push(BoxP {
                c: [0.0, 0.0, 0.0],
                h: [0.4, 0.18, 0.08],
                yaw: -0.4,
                color: 0xa8acb4ff,
                mat: Mat::Metal,
            });
            m.b.push(BoxP {
                c: [0.0, 0.0, 0.11],
                h: [0.34, 0.13, 0.04],
                yaw: -0.4,
                color: 0xc0c4ccff,
                mat: Mat::Metal,
            });
        }
        "logs" => {
            for (v, d) in [(-0.15f32, 0.14f32), (-0.15, -0.14), (0.08, 0.0)] {
                m.seg3(
                    [-0.4, v, d],
                    [0.3, v + 0.04, d],
                    0.13,
                    0.13,
                    0x5a3a20ff,
                    Mat::Matte,
                );
                m.seg3(
                    [0.31, v + 0.04, d],
                    [0.32, v + 0.04, d],
                    0.11,
                    0.11,
                    0xb08a58ff,
                    Mat::Matte,
                );
            }
        }
        "ruby" | "sapphire" | "diamond" => {
            let c = match name {
                "ruby" => 0xc01828ffu32,
                "sapphire" => 0x2848c8ff,
                _ => 0xd8e8f0ff,
            };
            m.plate(
                -0.3,
                0.12,
                |v| {
                    if v > 0.0 {
                        0.28 - v
                    } else {
                        0.3 * (1.0 + v / 0.3)
                    }
                },
                c,
                Mat::Metal,
                0.07,
            );
            m.ball((-0.08, 0.02), 0.04, 0xffffffff, Mat::Glow);
        }
        "bp" => {
            m.ball((0.0, -0.05), 0.22, 0x1a1a22ff, Mat::Metal);
            m.ball((-0.06, 0.02), 0.04, 0xa0a0c0ff, Mat::Glow);
        }
        "bm" | "ns" => {
            let leaf = if name == "bm" { 0x6a2a1aff } else { 0x2a4a22ff };
            for k in 0..9 {
                let a = hsh(k, 11) * std::f32::consts::TAU;
                let r = hsh(k, 12) * 0.25;
                m.ball(
                    (a.cos() * r, -0.15 + a.sin() * r * 0.6),
                    0.1,
                    leaf,
                    Mat::Hair,
                );
            }
            if name == "ns" {
                for k in 0..5 {
                    m.ball(
                        ((hsh(k, 13) - 0.5) * 0.4, -0.05 + hsh(k, 14) * 0.25),
                        0.05,
                        0x4a1a5aff,
                        Mat::Metal,
                    );
                }
            }
        }
        "ga" => {
            for k in 0..5 {
                let a = (k as f32 - 2.0) * 0.12;
                m.seg(
                    (a, -0.25),
                    (a * 0.5, 0.05),
                    0.12,
                    0.06,
                    0xe8e0d0ff,
                    Mat::Matte,
                );
            }
            m.seg((0.0, 0.05), (0.03, 0.3), 0.04, 0.02, 0xb8b090ff, Mat::Matte);
        }
        "gi" | "mr" => {
            let c = if name == "gi" {
                0xc8a070ffu32
            } else {
                0x8a6a40ff
            };
            m.seg((0.0, 0.2), (0.0, -0.05), 0.09, 0.08, c, Mat::Matte);
            m.seg((0.0, -0.05), (-0.18, -0.4), 0.07, 0.02, c, Mat::Matte);
            m.seg((0.0, -0.05), (0.16, -0.38), 0.07, 0.02, c, Mat::Matte);
            if name == "mr" {
                for s in [-1.0f32, 0.0, 1.0] {
                    m.seg(
                        (0.0, 0.25),
                        (s * 0.18, 0.45),
                        0.06,
                        0.02,
                        0x3a6a2aff,
                        Mat::Matte,
                    );
                }
            }
        }
        "ss" => {
            for k in 0..6 {
                let a = k as f32 / 6.0 * std::f32::consts::PI;
                m.seg(
                    (a.cos() * 0.35, a.sin() * 0.35 - 0.1),
                    (-a.cos() * 0.35, -a.sin() * 0.35 - 0.1),
                    0.012,
                    0.012,
                    0xe8e8e0ff,
                    Mat::Glow,
                );
            }
            for r in [0.12f32, 0.24] {
                for k in 0..6 {
                    let a = k as f32 / 6.0 * std::f32::consts::TAU;
                    let a2 = (k + 1) as f32 / 6.0 * std::f32::consts::TAU;
                    m.seg(
                        (a.cos() * r, a.sin() * r - 0.1),
                        (a2.cos() * r, a2.sin() * r - 0.1),
                        0.01,
                        0.01,
                        0xd8d8d0ff,
                        Mat::Glow,
                    );
                }
            }
        }
        _ => {
            m.ball((0.0, 0.0), 0.25, color, Mat::Matte);
        }
    }
    (
        Figure {
            prims: m.p,
            boxes: m.b,
            pitch: 0.0,
            roll: 0.0,
            lift: 0.0,
        },
        scale,
    )
}

/// Bake every icon, cropped to its pixels and centred in a 48×48 cell.
pub fn bake_icons(color_of: impl Fn(&str) -> u32) -> Atlas {
    let n = ICONS.len();
    let cols = 16;
    let rows = n.div_ceil(cols);
    let (w, h) = (ICON * cols, ICON * rows);
    let mut rgba = vec![0u8; w * h * 4];
    let (bw, bh) = (128usize, 128usize);
    for (k, name) in ICONS.iter().enumerate() {
        let mut s = 0.0;
        let mut buf = vec![0u8; bw * bh * 4];
        let mut bb = (0, 0, 0, 0);
        for pass in 0..2 {
            let (fig, s0) = model(name, color_of(name));
            if pass == 0 {
                s = s0;
            }
            buf.iter_mut().for_each(|b| *b = 0);
            render_frame(
                s,
                (64.0, 110.0),
                fig,
                (1.0, 0.0),
                &mut buf,
                bw,
                0,
                0,
                bw,
                bh,
            );
            let (mut x0, mut y0, mut x1, mut y1) = (bw, bh, 0, 0);
            for y in 0..bh {
                for x in 0..bw {
                    if buf[(y * bw + x) * 4 + 3] > 0 {
                        x0 = x0.min(x);
                        y0 = y0.min(y);
                        x1 = x1.max(x + 1);
                        y1 = y1.max(y + 1);
                    }
                }
            }
            bb = (x0, y0, x1, y1);
            let big = (x1.saturating_sub(x0)).max(y1.saturating_sub(y0));
            if big <= ICON - 4 || big == 0 {
                break;
            }
            s *= (ICON - 5) as f32 / big as f32;
        }
        let (x0, y0, x1, y1) = bb;
        if x1 <= x0 {
            continue;
        }
        let (cw, ch) = ((x1 - x0).min(ICON), (y1 - y0).min(ICON));
        let (ox, oy) = (
            (k % cols) * ICON + (ICON - cw) / 2,
            (k / cols) * ICON + (ICON - ch) / 2,
        );
        for y in 0..ch {
            for x in 0..cw {
                let si = ((y0 + y) * bw + x0 + x) * 4;
                let di = ((oy + y) * w + ox + x) * 4;
                rgba[di..di + 4].copy_from_slice(&buf[si..si + 4]);
            }
        }
    }
    Atlas {
        w,
        h,
        fw: ICON,
        fh: ICON,
        rgba,
    }
}
