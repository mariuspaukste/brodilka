//! The things of the deep: Cthulhu, risen to the waist from the sea, and the
//! cyclopean ruins of R'lyeh (`game/deep.rs`); the idol and the wrecks of
//! drowned Saltgrave (`world/drowned.rs`), and the great crabs of its shore.
//!
//! Cthulhu is modelled from the waterline (z = 0) up and baked with
//! `WATERLINE` set, so nothing below the surface is drawn: a scaled, hunched
//! green bulk with clawed arms, rudimentary bat wings, an octopus head with a
//! beard of feelers and burning eyes, and two great tentacles rising from the
//! water beside him, all ringed with foam. Dying, he sinks.

use super::*;
use std::f32::consts::TAU;

const SKIN: u32 = 0x3e5a3aff;
const BELLY: u32 = 0x55704aff;
const FEELER: u32 = 0x5a7e52ff;
const WINGC: u32 = 0x2a3a2aff;
const CLAW: u32 = 0x24221cff;
const FOAM: u32 = 0xa4b8b2ff;

/// a ring of foam on the water round `c`: a broken, wobbling band
fn foam(v: &mut Vec<Prim>, c: V3, r: f32, n: usize, size: f32, turn: f32) {
    let at = |k: usize| {
        let a = k as f32 / n as f32 * TAU + turn;
        let wob = 1.0 + 0.15 * (a * 3.0 + turn * 2.0).sin();
        add(c, [a.cos() * r * wob, a.sin() * r * wob, size * 0.6])
    };
    for k in 0..n {
        // a gap here and there
        if (k * 7 + (turn * 3.0) as usize) % 5 == 0 {
            continue;
        }
        prim(v, at(k), at(k + 1), size, size * 0.8, FOAM, Mat::Matte);
    }
}

/// a tentacle from `base`, rising along `dir` (radians from upright, toward
/// +x) and curling at the tip; `wave` sets its sway
fn tentacle(v: &mut Vec<Prim>, base: V3, side: f32, len: f32, r0: f32, lean: f32, wave: f32) {
    let n = 9;
    let mut p0 = base;
    for j in 0..n {
        let t = j as f32 / n as f32;
        let a = lean + t * t * 1.6 + 0.35 * (wave + t * 3.0).sin() * t;
        let seg = len / n as f32;
        let p1 = add(
            p0,
            [
                a.sin() * seg,
                side * 0.25 * seg * (wave * 0.7 + t * 4.0).sin(),
                a.cos() * seg,
            ],
        );
        let (ra, rb) = (
            r0 * (1.0 - 0.85 * t),
            r0 * (1.0 - 0.85 * (t + 1.0 / n as f32)),
        );
        prim(v, p0, p1, ra, rb, SKIN, Mat::Matte);
        p0 = p1;
    }
}

/// One frame of Cthulhu's animations.
pub(super) fn cthulhu(a: Anim, f: usize, n: usize) -> Figure {
    let mut v = vec![];
    let t = f as f32 / n.max(1) as f32;
    let p = if n > 1 {
        f as f32 / (n - 1) as f32
    } else {
        0.0
    };
    let ease = |x: f32| 0.5 - 0.5 * (x.clamp(0.0, 1.0) * std::f32::consts::PI).cos();
    let breathe = (t * TAU).sin();
    // the strike: a windup, then the blow
    let (wind, blow) = if a == Anim::Attack {
        (
            ease(p / 0.4) * (1.0 - ease((p - 0.4) / 0.3)),
            ease((p - 0.4) / 0.35),
        )
    } else {
        (0.0, 0.0)
    };
    let cast = if a == Anim::Cast { ease(p * 1.5) } else { 0.0 };
    let die = if a == Anim::Die { ease(p) } else { 0.0 };
    let recoil = if a == Anim::Hit { 1.0 - p * 0.5 } else { 0.0 };
    let wade = if a == Anim::Walk {
        (t * TAU).sin()
    } else {
        0.0
    };
    let sink = die * 2.0;
    let lunge = 0.25 * blow - 0.12 * recoil - 0.1 * wind;
    let down = |q: V3| [q[0], q[1], q[2] - sink];

    // the foam he stands in, churning as he moves or sinks
    let churn = if a == Anim::Die { 1.0 + p } else { 1.0 };
    foam(&mut v, [0.05, 0.0, 0.0], 0.4 * churn, 18, 0.045, t * 1.3);
    foam(&mut v, [0.05, 0.0, 0.0], 0.6 * churn, 16, 0.03, -t * 1.7);

    // the body: a hunched, scaled mass rising from the water
    let hips = down([0.0, 0.0, 0.1]);
    let chest = down([
        0.12 + lunge,
        0.04 * wade,
        0.8 + 0.02 * breathe + 0.08 * cast,
    ]);
    prim(&mut v, hips, chest, 0.28, 0.32, SKIN, Mat::Matte);
    prim(
        &mut v,
        add(hips, [0.12, 0.0, 0.12]),
        add(chest, [0.12, 0.0, -0.16]),
        0.2,
        0.22,
        BELLY,
        Mat::Matte,
    );
    // scales: rows of small plates down the back and flanks
    for k in 0..6 {
        let c = lerp3(hips, chest, 0.15 + k as f32 * 0.14);
        for s in [-1.0f32, 1.0] {
            let q = add(c, [-0.1, s * 0.24, 0.02]);
            prim(
                &mut v,
                q,
                add(q, [-0.04, s * 0.02, -0.05]),
                0.05,
                0.03,
                shade_col(SKIN, 0.8),
                Mat::Matte,
            );
        }
    }

    // wings, half folded on his back; spread wide when he calls on the deep
    let flap = 1.25 - 0.55 * cast - 0.3 * wind + 0.05 * breathe + 0.6 * die;
    for side in [-1.0f32, 1.0] {
        let root = add(chest, [-0.26, side * 0.16, 0.02]);
        wing(&mut v, root, side, 0.68 + 0.08 * cast, flap, WINGC);
    }

    // the head: a bulbous, swept-back mantle, burning eyes, a beard of feelers
    let head = add(
        chest,
        [0.14 + 0.06 * blow, 0.0, 0.34 + 0.06 * cast - 0.06 * blow],
    );
    prim(&mut v, head, head, 0.2, 0.2, SKIN, Mat::Matte);
    prim(
        &mut v,
        add(head, [-0.05, 0.0, 0.06]),
        add(head, [-0.24, 0.0, 0.32 + 0.04 * breathe]),
        0.21,
        0.12,
        shade_col(SKIN, 0.85),
        Mat::Matte,
    );
    for es in [-1.0f32, 1.0] {
        let e = add(head, [0.16, es * 0.09, 0.04]);
        prim(&mut v, e, e, 0.036, 0.036, 0xf0c838ff, Mat::Glow);
    }
    for k in 0..8 {
        let kf = k as f32 - 3.5;
        let mut p0 = add(head, [0.12, kf * 0.034, -0.07 - kf.abs() * 0.012]);
        for j in 0..7 {
            let jf = j as f32;
            let curl =
                0.25 + 0.14 * jf + 0.3 * (t * TAU + k as f32 * 0.9 + jf * 0.6).sin() - 0.5 * cast;
            let p1 = add(
                p0,
                [
                    curl.sin() * 0.07,
                    kf * 0.006 + 0.012 * (t * TAU + k as f32).cos(),
                    -curl.cos() * 0.07,
                ],
            );
            let r = 0.034 * (1.0 - jf / 8.0);
            prim(&mut v, p0, p1, r, r * 0.85, FEELER, Mat::Matte);
            p0 = p1;
        }
    }

    // arms: hanging forward at the waterline, swinging as he wades; the
    // right raised and brought down in the strike; both raised to call or as
    // he goes under
    for side in [-1.0f32, 1.0] {
        let sh = add(chest, [-0.02, side * 0.34, 0.04]);
        let striking = side < 0.0 && a == Anim::Attack;
        let up = (cast + die).min(1.0);
        let hang_e = [0.12 + 0.08 * wade * side, side * 0.12, -0.32];
        let hang_w = [0.24, side * 0.03, -0.12];
        let raise_e = [0.05, side * 0.3, 0.25];
        let raise_w = [0.05, side * 0.25, 0.28];
        let (mut de, mut dw) = (lerp3(hang_e, raise_e, up), lerp3(hang_w, raise_w, up));
        if striking {
            let wind_e = [-0.05, -0.12, 0.32];
            let wind_w = [0.0, -0.05, 0.32];
            let hit_e = [0.36, -0.06, 0.02];
            let hit_w = [0.36, 0.0, -0.26];
            de = lerp3(lerp3(de, wind_e, wind), hit_e, blow);
            dw = lerp3(lerp3(dw, wind_w, wind), hit_w, blow);
        }
        let elbow = add(sh, de);
        let wrist = add(elbow, dw);
        prim(&mut v, sh, sh, 0.13, 0.13, SKIN, Mat::Matte);
        prim(&mut v, sh, elbow, 0.11, 0.085, SKIN, Mat::Matte);
        prim(&mut v, elbow, wrist, 0.085, 0.065, SKIN, Mat::Matte);
        // a hand of four long, hooked claws
        let fwd = {
            let d = [
                wrist[0] - elbow[0],
                wrist[1] - elbow[1],
                wrist[2] - elbow[2],
            ];
            let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-4);
            mul(d, 1.0 / l)
        };
        for k in 0..4 {
            let spread = (k as f32 - 1.5) * 0.045;
            let k1 = add(wrist, add(mul(fwd, 0.1), [0.0, spread, 0.0]));
            let k2 = add(k1, add(mul(fwd, 0.09), [0.0, spread * 0.5, -0.05]));
            prim(&mut v, wrist, k1, 0.035, 0.026, SKIN, Mat::Matte);
            prim(&mut v, k1, k2, 0.024, 0.006, CLAW, Mat::Bone);
        }
    }

    // two great tentacles out of the sea at his sides; the right one lashes
    // forward in the strike
    for side in [-1.0f32, 1.0] {
        let base = [0.35, side * 0.78, 0.0];
        foam(&mut v, base, 0.14, 8, 0.03, t * 2.0 + side);
        let lash = if side < 0.0 {
            blow * 0.9 - wind * 0.3
        } else {
            0.2 * blow
        };
        let rise = (1.0 - die) * (0.9 + 0.1 * breathe);
        let b = down(add(base, [0.0, 0.0, -0.05]));
        tentacle(
            &mut v,
            b,
            side,
            1.35 * rise.max(0.05),
            0.1,
            0.15 + lash,
            t * TAU + side * 1.7,
        );
    }

    Figure {
        prims: v,
        boxes: vec![],
        pitch: 0.0,
        roll: 0.0,
        lift: 0.0,
    }
}

/// The ruins of R'lyeh: a leaning monolith, a great carved door between
/// two pillars, a jumble of blocks set at wrong angles. Dark green stone
/// hung with weed, standing in the foam.
pub(super) fn build(kind: usize, p: &mut Vec<Prim>, b: &mut Vec<BoxP>) -> bool {
    if !(P_RUIN..P_RUIN + 5).contains(&kind) {
        return false;
    }
    let stone = 0x3a4a3cff;
    let dark = 0x222c22ff;
    let weed = 0x4a5628ff;
    let mut slab = |c: V3, h: V3, yaw: f32, col: u32| {
        b.push(BoxP {
            c,
            h,
            yaw,
            color: col,
            mat: Mat::Stone,
        });
    };
    let mut strands: Vec<(V3, f32)> = vec![];
    match kind - P_RUIN {
        0 => {
            // a monolith, each block set askew on the one below
            slab([0.0, 0.0, 0.6], [0.36, 0.26, 0.6], 0.3, stone);
            slab(
                [0.08, 0.02, 1.55],
                [0.31, 0.23, 0.36],
                0.48,
                shade_col(stone, 0.9),
            );
            slab([0.16, 0.04, 2.15], [0.27, 0.2, 0.25], 0.66, stone);
            slab([0.2, 0.05, 2.5], [0.18, 0.14, 0.1], 0.9, dark);
            strands.extend([
                ([0.3, 0.1, 1.2], 0.9),
                ([-0.2, 0.2, 1.1], 0.8),
                ([0.25, -0.2, 2.0], 0.7),
            ]);
        }
        1 => {
            // the great door: pillars, a lintel, a carved panel and its sign
            for s in [-1.0f32, 1.0] {
                slab([0.0, s * 0.6, 0.95], [0.22, 0.2, 0.95], 0.08 * s, stone);
            }
            slab(
                [0.02, 0.0, 2.05],
                [0.3, 0.9, 0.17],
                0.14,
                shade_col(stone, 0.9),
            );
            slab([-0.04, 0.0, 0.95], [0.07, 0.4, 0.92], 0.0, dark);
            // the sign on the door: a head with feelers, in faint green fire
            let c = [0.05, 0.0, 1.25];
            prim(p, c, c, 0.07, 0.07, 0x5ac878ff, Mat::Glow);
            for k in 0..5 {
                let y = (k as f32 - 2.0) * 0.035;
                prim(
                    p,
                    add(c, [0.0, y, -0.06]),
                    add(c, [0.0, y * 1.4, -0.24]),
                    0.014,
                    0.008,
                    0x5ac878ff,
                    Mat::Glow,
                );
            }
            strands.extend([
                ([0.2, 0.7, 1.6], 1.0),
                ([0.2, -0.55, 1.4], 0.9),
                ([0.2, 0.3, 1.9], 0.6),
            ]);
        }
        3 => {
            // the idol of the sleeper on a plinth: a squatting thing with an
            // octopus head, wings folded, feelers down its chest
            slab([0.0, 0.0, 0.2], [0.42, 0.42, 0.2], 0.0, dark);
            slab([0.0, 0.0, 0.44], [0.34, 0.34, 0.05], 0.785, stone);
            let st = 0x4a5e48ff;
            prim(
                p,
                [0.0, 0.0, 0.62],
                [0.03, 0.0, 1.15],
                0.24,
                0.2,
                st,
                Mat::Matte,
            );
            for s in [-1.0f32, 1.0] {
                // haunches, knees up, and the clawed hands upon them
                prim(
                    p,
                    [0.0, s * 0.18, 0.62],
                    [0.2, s * 0.2, 0.75],
                    0.12,
                    0.09,
                    st,
                    Mat::Matte,
                );
                prim(
                    p,
                    [0.2, s * 0.2, 0.75],
                    [0.24, s * 0.2, 0.52],
                    0.08,
                    0.07,
                    st,
                    Mat::Matte,
                );
                prim(
                    p,
                    [0.05, s * 0.24, 1.05],
                    [0.22, s * 0.2, 0.82],
                    0.07,
                    0.06,
                    st,
                    Mat::Matte,
                );
                // folded wings behind
                prim(
                    p,
                    [-0.12, s * 0.12, 1.1],
                    [-0.2, s * 0.22, 1.5],
                    0.06,
                    0.02,
                    dark,
                    Mat::Matte,
                );
            }
            let head = [0.06, 0.0, 1.32];
            prim(p, head, head, 0.17, 0.17, st, Mat::Matte);
            prim(
                p,
                add(head, [-0.04, 0.0, 0.05]),
                add(head, [-0.18, 0.0, 0.3]),
                0.17,
                0.09,
                st,
                Mat::Matte,
            );
            for k in 0..6 {
                let y = (k as f32 - 2.5) * 0.045;
                prim(
                    p,
                    add(head, [0.12, y, -0.08]),
                    add(head, [0.17, y * 1.2, -0.36]),
                    0.025,
                    0.012,
                    st,
                    Mat::Matte,
                );
            }
            for s in [-1.0f32, 1.0] {
                let e = add(head, [0.14, s * 0.07, 0.03]);
                prim(p, e, e, 0.025, 0.025, 0x5ac878ff, Mat::Glow);
            }
            strands.extend([([0.38, 0.3, 0.42], 0.3), ([-0.3, -0.36, 0.42], 0.35)]);
        }
        4 => {
            // a fishing boat stove in and left on its side, mast snapped
            let wood = 0x5a4a38ff;
            let tilt = 0.35;
            for k in 0..7 {
                let x = -0.75 + k as f32 * 0.25;
                let half = 0.36 * (1.0 - (x / 0.85).powi(2)).max(0.1);
                for s in [-1.0f32, 1.0] {
                    // the ribs, and the planks of the side that still holds
                    let top = [x, s * half, 0.42 + s * tilt * 0.3];
                    let keel = [x, 0.0, 0.08];
                    if s > 0.0 || k % 3 != 1 {
                        prim(
                            p,
                            keel,
                            top,
                            0.05,
                            0.045,
                            shade_col(wood, 0.8 + 0.05 * k as f32),
                            Mat::Wood,
                        );
                    }
                }
            }
            prim(
                p,
                [-0.85, 0.0, 0.1],
                [0.9, 0.0, 0.12],
                0.06,
                0.05,
                shade_col(wood, 0.7),
                Mat::Wood,
            );
            for s in [-1.0f32, 1.0] {
                let g = |x: f32| {
                    [
                        x,
                        s * 0.36 * (1.0 - (x / 0.85).powi(2)).max(0.1),
                        0.42 + s * tilt * 0.3,
                    ]
                };
                for k in 0..6 {
                    let x0 = -0.75 + k as f32 * 0.3;
                    if s < 0.0 && k % 2 == 0 {
                        continue;
                    }
                    prim(p, g(x0), g(x0 + 0.3), 0.065, 0.065, wood, Mat::Wood);
                }
            }
            prim(
                p,
                [0.0, 0.0, 0.2],
                [0.12, 0.55, 0.65],
                0.04,
                0.03,
                wood,
                Mat::Wood,
            );
            strands.extend([([0.4, 0.3, 0.5], 0.3), ([-0.3, 0.32, 0.5], 0.25)]);
        }
        _ => {
            // blocks tumbled together at angles no mason would set
            slab([-0.2, -0.2, 0.32], [0.36, 0.3, 0.32], 0.7, stone);
            slab(
                [0.25, 0.2, 0.38],
                [0.3, 0.3, 0.38],
                -0.42,
                shade_col(stone, 0.88),
            );
            slab([0.0, 0.05, 0.9], [0.34, 0.22, 0.18], 1.1, stone);
            slab([0.05, -0.05, 1.2], [0.14, 0.14, 0.12], 0.3, dark);
            strands.extend([([0.4, 0.2, 0.7], 0.6), ([-0.1, -0.45, 0.6], 0.5)]);
        }
    }
    // weed hanging from the stone, dripping into the sea
    for (top, len) in strands {
        let mut q = top;
        for j in 0..5 {
            let q1 = add(q, [0.01 * (j as f32).sin(), 0.01, -len / 5.0]);
            prim(p, q, q1, 0.03, 0.025, weed, Mat::Hair);
            q = q1;
        }
    }
    // the stones of R'lyeh stand in the sea
    if kind < P_RUIN + 3 {
        foam(p, [0.0, 0.0, 0.0], 0.5, 16, 0.045, 0.4);
    }
    true
}

/// A great crab: a broad shell slung between eight jointed legs, eyes on
/// stalks, and two heavy claws that open and snap shut. Dying it turns
/// over, legs curling.
pub(super) fn crab(look: &Look, a: Anim, f: usize, n: usize) -> Figure {
    let mut v = vec![];
    let col = look.skin;
    let dark = shade_col(col, 0.65);
    let (ph, lunge, recoil, die, _) = creature_phase(a, f, n);
    let pi = std::f32::consts::PI;
    let bob = if a == Anim::Walk {
        0.012 * (ph * 2.0).sin()
    } else {
        0.0
    };
    let c = [lunge * 0.06 - recoil * 0.06, 0.0, 0.17 + bob];
    // the shell: broad, low, knobbed along its front edge
    prim(
        &mut v,
        add(c, [0.0, -0.12, 0.0]),
        add(c, [0.0, 0.12, 0.0]),
        0.12,
        0.12,
        col,
        Mat::Matte,
    );
    prim(
        &mut v,
        add(c, [0.02, -0.08, 0.06]),
        add(c, [0.02, 0.08, 0.06]),
        0.09,
        0.09,
        shade_col(col, 1.12),
        Mat::Matte,
    );
    for k in 0..5 {
        let y = (k as f32 - 2.0) * 0.06;
        let q = add(c, [0.11, y, 0.04]);
        prim(&mut v, q, q, 0.022, 0.022, shade_col(col, 1.2), Mat::Matte);
    }
    // eyes on stalks
    for s in [-1.0f32, 1.0] {
        let b = add(c, [0.1, s * 0.035, 0.06]);
        let e = add(b, [0.03, s * 0.01, 0.07]);
        prim(&mut v, b, e, 0.012, 0.01, dark, Mat::Matte);
        prim(&mut v, e, e, 0.018, 0.018, 0x101010ff, Mat::Hair);
    }
    // the claws, raised and snapping in the attack
    for s in [-1.0f32, 1.0] {
        let open = if a == Anim::Attack {
            0.07 * (1.0 - lunge)
        } else {
            0.02 + 0.015 * ph.sin().abs()
        };
        let up = 0.12 * lunge + if a == Anim::Stance { 0.08 } else { 0.0 };
        let sh = add(c, [0.08, s * 0.12, -0.02]);
        let el = add(sh, [0.1, s * 0.09, 0.03 + up * 0.5]);
        let base = add(el, [0.12, -s * 0.03, up]);
        prim(&mut v, sh, el, 0.035, 0.03, col, Mat::Matte);
        prim(&mut v, el, base, 0.032, 0.05, col, Mat::Matte);
        prim(
            &mut v,
            base,
            add(base, [0.13, -s * 0.01, 0.02 + open]),
            0.05,
            0.015,
            col,
            Mat::Matte,
        );
        prim(
            &mut v,
            add(base, [0.0, 0.0, -0.02]),
            add(base, [0.11, -s * 0.01, -0.03 - open]),
            0.03,
            0.01,
            dark,
            Mat::Matte,
        );
    }
    // four legs a side, walking in alternate sets, curling up as it dies
    for s in [-1.0f32, 1.0] {
        for i in 0..4 {
            let root = add(c, [0.04 - i as f32 * 0.055, s * 0.15, -0.03]);
            let set = (i % 2) as f32 + if s > 0.0 { 1.0 } else { 0.0 };
            let q = ph + set * pi;
            let (step, lift) = if a == Anim::Walk {
                (0.05 * q.sin(), 0.05 * q.cos().max(0.0))
            } else {
                (0.0, 0.0)
            };
            let yaw = 0.5 - i as f32 * 0.35;
            let out = [yaw.sin() * 0.5, s * yaw.cos(), 0.0];
            let curl = 1.0 - die * 0.7;
            let knee = add(
                root,
                [out[0] * 0.14 * curl, out[1] * 0.14 * curl, 0.1 + die * 0.05],
            );
            let foot = [
                root[0] + out[0] * 0.3 * curl + step,
                root[1] + out[1] * 0.3 * curl,
                (lift + die * 0.15 + root[2] - c[2] + 0.0).max(0.0) * 0.5,
            ];
            prim(&mut v, root, knee, 0.022, 0.018, col, Mat::Matte);
            prim(&mut v, knee, foot, 0.018, 0.007, dark, Mat::Matte);
        }
    }
    Figure {
        prims: v,
        boxes: vec![],
        pitch: 0.0,
        roll: 2.6 * die,
        lift: 0.0,
    }
}
