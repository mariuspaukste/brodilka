//! The giant pandas of the bamboo groves (`game/panda.rs`): a heavy white
//! bear with black legs, a black band over its shoulders, black ears and
//! black patches round its eyes.
//!
//! It shambles on all fours, its head low, and lopes when it runs. Standing
//! still it looks about (`Idle`). `Look` is the panda at its meal: sat back
//! on its rump with its hind legs out before it, holding a stalk of bamboo
//! to its mouth and chewing. It flinches on all fours and rolls on its side
//! when it dies.
//!
//! The look's `skin` is the white, `body` the black.

use super::*;
use std::f32::consts::{PI, TAU};

/// the head at `h` facing +x: round and white, with black ears and eye
/// patches, a white muzzle and a black nose
fn head(v: &mut Vec<Prim>, h: V3, white: u32, black: u32, nod: f32) {
    prim(v, h, h, 0.2, 0.2, white, Mat::Matte);
    // the cheeks make it broad
    for s in [-1.0f32, 1.0] {
        let c = add(h, [0.03, s * 0.08, -0.05]);
        prim(v, c, c, 0.14, 0.14, white, Mat::Matte);
        let ear = add(h, [-0.03, s * 0.15, 0.15]);
        prim(v, ear, ear, 0.065, 0.065, black, Mat::Matte);
        // the patches slant down and out from the eyes
        let p = add(h, [0.155, s * 0.075, 0.025 - nod * 0.1]);
        prim(
            v,
            p,
            add(p, [-0.01, s * 0.025, -0.03]),
            0.045,
            0.04,
            black,
            Mat::Matte,
        );
        let e = add(h, [0.19, s * 0.072, 0.035 - nod * 0.1]);
        prim(v, e, e, 0.012, 0.012, 0x2a201aff, Mat::Hair);
    }
    let m = add(h, [0.17, 0.0, -0.07 - nod * 0.1]);
    prim(v, m, m, 0.085, 0.08, white, Mat::Matte);
    let n = add(m, [0.07, 0.0, 0.025]);
    prim(v, n, n, 0.03, 0.025, black, Mat::Hair);
}

/// a thick black leg from `a` through `j` to `b`, with a paw at `b`
fn leg(v: &mut Vec<Prim>, a: V3, j: V3, b: V3, r: f32, black: u32) {
    prim(v, a, j, r, r * 0.9, black, Mat::Matte);
    prim(v, j, b, r * 0.9, r * 0.8, black, Mat::Matte);
    prim(
        v,
        b,
        add(b, [0.07, 0.0, -0.01]),
        r * 0.8,
        r * 0.7,
        black,
        Mat::Matte,
    );
}

/// a stalk of bamboo from `a` to `b`, jointed, with a few leaves at the top
fn bamboo(v: &mut Vec<Prim>, a: V3, b: V3) {
    let green = 0x7a9a3aff;
    prim(v, a, b, 0.028, 0.024, green, Mat::Matte);
    for k in 1..4 {
        let p = lerp3(a, b, k as f32 / 4.0);
        prim(v, p, p, 0.032, 0.032, 0x5a7a2aff, Mat::Matte);
    }
    for (dy, dz) in [(0.09, 0.05), (-0.08, 0.08), (0.03, 0.12)] {
        prim(
            v,
            b,
            add(b, [0.05, dy, dz]),
            0.03,
            0.012,
            0x6a9a34ff,
            Mat::Matte,
        );
    }
}

pub(super) fn panda(look: &Look, a: Anim, f: usize, n: usize) -> Figure {
    let mut v = vec![];
    let white = look.skin;
    let black = look.body;
    let (ph, _, recoil, die, _) = creature_phase(a, f, n);
    let lp = f as f32 / n.max(1) as f32;

    if a == Anim::Look {
        // sat back on its rump, eating
        let chew = 0.5 - 0.5 * (lp * TAU).cos();
        let rump = [-0.12, 0.0, 0.3];
        let chest = [0.0, 0.0, 0.74];
        prim(&mut v, rump, chest, 0.34, 0.27, white, Mat::Matte);
        // the black band over the shoulders and down into the arms
        prim(
            &mut v,
            add(chest, [-0.02, 0.0, -0.06]),
            add(chest, [0.0, 0.0, 0.05]),
            0.29,
            0.28,
            black,
            Mat::Matte,
        );
        let h = [0.06, 0.0, 1.02 - 0.02 * chew];
        prim(
            &mut v,
            add(chest, [0.02, 0.0, 0.08]),
            h,
            0.2,
            0.18,
            white,
            Mat::Matte,
        );
        head(&mut v, h, white, black, 0.2 + 0.15 * chew);
        for s in [-1.0f32, 1.0] {
            // hind legs out before it, soles up
            let hip = add(rump, [0.06, s * 0.2, -0.06]);
            let knee = [0.32, s * 0.3, 0.24];
            let foot = [0.5, s * 0.3, 0.13];
            leg(&mut v, hip, knee, foot, 0.12, black);
            // both forepaws round the stalk at its mouth
            let sh = add(chest, [0.02, s * 0.21, -0.02]);
            let el = [0.26, s * 0.26, 0.6 + 0.05 * chew];
            let paw = add(h, [0.22, s * 0.07, -0.18 - 0.06 * (1.0 - chew)]);
            prim(&mut v, sh, el, 0.11, 0.1, black, Mat::Matte);
            prim(&mut v, el, paw, 0.1, 0.085, black, Mat::Matte);
        }
        let mouth = add(h, [0.25, 0.0, -0.1]);
        bamboo(
            &mut v,
            add(mouth, [0.12, 0.04, -0.62]),
            add(mouth, [-0.02, -0.03, 0.32]),
        );
        // the little white tail on the ground behind
        prim(
            &mut v,
            add(rump, [-0.32, 0.0, -0.18]),
            add(rump, [-0.32, 0.0, -0.18]),
            0.06,
            0.06,
            white,
            Mat::Matte,
        );
        return Figure {
            prims: v,
            boxes: vec![],
            pitch: 0.0,
            roll: 0.0,
            lift: 0.0,
        };
    }

    // on all fours: a heavy shamble, a lope at a run
    let run = a == Anim::Run;
    let gait = matches!(a, Anim::Walk | Anim::Run);
    let bob = if gait {
        (if run { 0.04 } else { 0.02 }) * (2.0 * ph).cos()
    } else {
        0.0
    };
    let sway = if gait && !run { 0.03 * ph.sin() } else { 0.0 };
    let dx = -recoil * 0.08;
    let hip = [-0.36 + dx, sway, 0.6 + bob];
    let sho = [0.3 + dx, -sway, 0.62 + bob + recoil * 0.04];
    prim(&mut v, hip, sho, 0.31, 0.3, white, Mat::Matte);
    // the shoulder band
    prim(
        &mut v,
        add(sho, [-0.12, 0.0, 0.0]),
        add(sho, [0.04, 0.0, 0.0]),
        0.315,
        0.31,
        black,
        Mat::Matte,
    );
    // the head carried low, looking about when it stands
    let turn = if a == Anim::Idle {
        [0.0, 0.05, -0.05][f.min(2)]
    } else {
        0.0
    };
    let h = add(sho, [0.33, turn, 0.03 + recoil * 0.08]);
    prim(
        &mut v,
        add(sho, [0.12, 0.0, 0.0]),
        h,
        0.2,
        0.18,
        white,
        Mat::Matte,
    );
    head(&mut v, h, white, black, 0.2 - recoil * 0.3);
    let (stride, lift) = if run { (0.2, 0.12) } else { (0.12, 0.06) };
    for (front, side) in [(true, 1.0f32), (true, -1.0), (false, 1.0), (false, -1.0)] {
        // a walk in diagonal pairs; a lope, the hind and fore pairs together
        let off = match (run, front, side > 0.0) {
            (true, true, true) => 0.0,
            (true, true, false) => 0.4,
            (true, false, true) => PI,
            (true, false, false) => PI + 0.4,
            (false, true, true) | (false, false, false) => 0.0,
            _ => PI,
        };
        let s = ph + off;
        let (sx, sz) = if gait {
            (stride * s.sin(), lift * s.cos().max(0.0))
        } else {
            (0.0, 0.0)
        };
        let base = if front { sho } else { hip };
        let top = add(base, [0.0, side * 0.2, -0.12]);
        let foot = [base[0] + sx, side * 0.22, 0.07 + sz];
        let bend = if front { -0.04 } else { 0.06 };
        let j = add(lerp3(top, foot, 0.5), [bend, side * 0.01, 0.0]);
        leg(&mut v, top, j, foot, 0.12, black);
    }
    prim(
        &mut v,
        add(hip, [-0.3, 0.0, 0.05]),
        add(hip, [-0.33, 0.0, 0.02]),
        0.06,
        0.05,
        white,
        Mat::Matte,
    );
    Figure {
        prims: v,
        boxes: vec![],
        pitch: 0.0,
        roll: 1.45 * die,
        lift: 0.0,
    }
}
