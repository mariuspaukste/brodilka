//! Trees of the tropics: coconut palms, the banyan and bamboo.

use super::*;
use std::f32::consts::{PI, TAU};

/// a point along a curved trunk, t in 0..1, leaning `lean` metres at the top
fn bend(t: f32, height: f32, lean: f32, dir: f32) -> V3 {
    let off = lean * t * t;
    [off * dir.cos(), off * dir.sin(), height * t]
}

pub(super) fn build(kind: usize, p: &mut Vec<Prim>, _b: &mut Vec<BoxP>) -> bool {
    match kind {
        k if (P_PALM..P_PALM + 2).contains(&k) => {
            // a coconut palm: a ringed trunk curving up to a crown of
            // arching, drooping fronds, a cluster of nuts beneath
            let v = k - P_PALM;
            let (height, lean, dir) = if v == 0 {
                (4.0, 0.8, 0.6)
            } else {
                (3.2, 1.0, 0.75)
            };
            let n = 14;
            for i in 0..n {
                let (t0, t1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
                let r = 0.13 - t0 * 0.05;
                let col = if i % 2 == 0 { 0x6a5a40ff } else { 0x5a4a34ff };
                prim(
                    p,
                    bend(t0, height, lean, dir),
                    bend(t1, height, lean, dir),
                    r,
                    r - 0.004,
                    col,
                    Mat::Matte,
                );
            }
            let crown = bend(1.0, height, lean, dir);
            prim(
                p,
                crown,
                add(crown, [0.0, 0.0, 0.1]),
                0.13,
                0.1,
                0x4a5a24ff,
                Mat::Leaf,
            );
            for (dx, dy) in [(0.1f32, 0.05f32), (-0.05, 0.1), (0.02, -0.11)] {
                let c = add(crown, [dx, dy, -0.12]);
                prim(p, c, c, 0.08, 0.08, 0x5a4020ff, Mat::Matte);
            }
            let fronds = 9;
            for f in 0..fronds {
                let a = f as f32 / fronds as f32 * TAU + hsh(f, v + 3) * 0.5;
                let len = 1.25 + hsh(f, v + 4) * 0.4;
                let rise = 0.35 + hsh(f, v + 5) * 0.3;
                let mut prev = add(crown, [0.0, 0.0, 0.05]);
                let segs = 7;
                for s in 1..=segs {
                    let t = s as f32 / segs as f32;
                    // up a little, then arching over and down
                    let h = rise * (t * PI * 0.9).sin() - t * t * 0.55;
                    let pt = add(crown, [a.cos() * len * t, a.sin() * len * t, h]);
                    let w = 0.15 * (1.0 - t * 0.7);
                    let col = [0x2e5a1eff, 0x3a6a24ff, 0x24481aff][(f + s) % 3];
                    prim(p, prev, pt, w, w * 0.8, col, Mat::Leaf);
                    // leaflets hanging from the rib on both sides
                    if s > 1 {
                        for side in [-1.0f32, 1.0] {
                            let sa = a + side * PI / 2.0;
                            let tip = add(
                                pt,
                                [
                                    sa.cos() * w * 1.8,
                                    sa.sin() * w * 1.8,
                                    -0.12 * (1.0 - t * 0.4),
                                ],
                            );
                            prim(p, pt, tip, w * 0.45, 0.015, col, Mat::Leaf);
                        }
                    }
                    prev = pt;
                }
            }
        }
        P_BANYAN => {
            // the banyan: a knot of trunks, a broad low crown, and roots
            // hanging from its boughs down to the ground
            let bark = 0x5a5448ff;
            for (dx, dy, r) in [
                (0.0f32, 0.0f32, 0.26f32),
                (0.3, 0.2, 0.14),
                (-0.25, 0.22, 0.13),
                (0.1, -0.3, 0.12),
            ] {
                prim(
                    p,
                    [dx, dy, 0.0],
                    [dx * 0.4, dy * 0.4, 1.9],
                    r,
                    r * 0.7,
                    bark,
                    Mat::Matte,
                );
            }
            for (a, len) in [(0.3f32, 1.5f32), (2.0, 1.4), (3.6, 1.6), (5.0, 1.3)] {
                prim(
                    p,
                    [0.0, 0.0, 1.7],
                    [a.cos() * len * 0.8 - 0.3, a.sin() * len * 0.8 - 0.3, 2.3],
                    0.09,
                    0.05,
                    bark,
                    Mat::Matte,
                );
            }
            let greens = [0x1e3a1aff, 0x2a4a1eff, 0x34561fff, 0x1a3216ff, 0x3e5a24ff];
            for i in 0..150 {
                let (a, r) = (hsh(i, 61) * TAU, hsh(i, 62).sqrt() * 1.4);
                let z = 2.3 + hsh(i, 63) * 0.7 - (r / 1.4).powi(2) * 0.4;
                let c = [a.cos() * r - 0.3, a.sin() * r * 0.9 - 0.3, z];
                let sz = 0.15 + hsh(i, 64) * 0.1;
                prim(
                    p,
                    c,
                    c,
                    sz,
                    sz,
                    greens[(hsh(i, 65) * 5.0) as usize % 5],
                    Mat::Leaf,
                );
            }
            // aerial roots
            for i in 0..14 {
                let (a, r) = (hsh(i, 71) * TAU, 0.6 + hsh(i, 72) * 0.8);
                let top = [a.cos() * r - 0.3, a.sin() * r * 0.9 - 0.3, 2.15];
                let reach = hsh(i, 73);
                let bot = [
                    top[0] * 1.03,
                    top[1] * 1.03,
                    if reach < 0.5 { 0.0 } else { 0.5 + reach },
                ];
                prim(p, top, bot, 0.022, 0.016, 0x6a6050ff, Mat::Matte);
            }
        }
        P_BAMBOO => {
            // a clump of bamboo: tall jointed culms leaning outward, with
            // sprays of narrow leaves up high
            for i in 0..16 {
                let (a, r) = (hsh(i, 81) * TAU, hsh(i, 82) * 0.32);
                let base = [a.cos() * r, a.sin() * r, 0.0];
                let top_h = 2.8 + hsh(i, 83) * 1.4;
                let out = 0.4 + hsh(i, 84) * 0.6;
                let top = [a.cos() * (r + out), a.sin() * (r + out), top_h];
                let col = if i % 3 == 0 { 0x8a9a3aff } else { 0x5a7a2aff };
                let n = 6;
                for s in 0..n {
                    let (t0, t1) = (s as f32 / n as f32, (s + 1) as f32 / n as f32);
                    let a0 = [
                        base[0] + (top[0] - base[0]) * t0,
                        base[1] + (top[1] - base[1]) * t0,
                        top_h * t0,
                    ];
                    let a1 = [
                        base[0] + (top[0] - base[0]) * t1,
                        base[1] + (top[1] - base[1]) * t1,
                        top_h * t1,
                    ];
                    prim(p, a0, a1, 0.05, 0.045, col, Mat::Matte);
                    // the node ring
                    prim(p, a1, a1, 0.046, 0.046, shade_col(col, 0.7), Mat::Matte);
                }
                for l in 0..7 {
                    let t = 0.45 + l as f32 * 0.08;
                    let at = [
                        base[0] + (top[0] - base[0]) * t,
                        base[1] + (top[1] - base[1]) * t,
                        top_h * t,
                    ];
                    let la = hsh(i * 7 + l, 85) * TAU;
                    let tip = add(at, [la.cos() * 0.35, la.sin() * 0.35, -0.12]);
                    prim(
                        p,
                        at,
                        tip,
                        0.08,
                        0.025,
                        [0x3a6a24ff, 0x4a7a2aff, 0x2e5a1eff][l % 3],
                        Mat::Leaf,
                    );
                }
            }
        }
        _ => return false,
    }
    true
}
