//! The monkeys of the southern cities (`game/critters.rs`): rhesus macaques,
//! brown with pink-red faces and short tails, and Hanuman langurs, pale grey
//! with black faces, hands and feet and long tails.
//!
//! A monkey goes on all fours, its back level and the tail carried behind (a
//! langur's arched up over its back), and at a run it bounds, hind feet and
//! forefeet together. Standing still it sits on its haunches with its knees
//! drawn up and its hands on them, looking about (`Idle`); `Look` is the same
//! seat with a hand going up to its mouth and down again, eating. It flinches
//! on all fours and falls on its side when it dies.
//!
//! The look's `skin` is the fur, `body` the paler belly, `patch` the bare
//! face and ears, `legs` the hands and feet, and `long_hair` gives the long
//! tail.

use super::*;
use std::f32::consts::{PI, TAU};

/// the head at `h` facing +x: a round furred skull with a bare face, a short
/// muzzle, brows, ears and eyes; `nod` tips the face down
fn head(v: &mut Vec<Prim>, h: V3, look: &Look, nod: f32) {
    let fur = look.skin;
    let face = look.patch;
    // dark eyes in a pale face, amber in a black one
    let eye = if lum(face) < 0.25 {
        0x9a7a3cff
    } else {
        0x1e1410ff
    };
    let langur = look.long_hair;
    prim(v, h, h, 0.062, 0.062, fur, Mat::Matte);
    // the crown: a langur's pale fur stands up round its face
    let crown = add(h, [-0.01, 0.0, 0.03]);
    prim(
        v,
        crown,
        crown,
        0.05,
        0.05,
        shade_col(fur, 1.06),
        Mat::Matte,
    );
    let fd = [1.0, 0.0, -nod];
    let f = add(h, mul(fd, 0.038));
    prim(
        v,
        f,
        add(f, [0.0, 0.0, -0.012]),
        0.044,
        0.04,
        face,
        Mat::Matte,
    );
    // the muzzle, longer on a macaque
    let ml = if langur { 0.055 } else { 0.07 };
    let m = add(h, [ml, 0.0, -0.028 - nod * 0.05]);
    prim(v, m, m, 0.026, 0.026, face, Mat::Matte);
    // brows over the eyes, and the eyes under them
    for s in [-1.0f32, 1.0] {
        let b = add(h, [0.05, s * 0.022, 0.022 - nod * 0.04]);
        prim(v, b, b, 0.016, 0.016, shade_col(fur, 0.92), Mat::Matte);
        let e = add(h, [0.074, s * 0.019, 0.004 - nod * 0.05]);
        prim(v, e, e, 0.009, 0.009, eye, Mat::Hair);
        // small round ears close to the head
        let ear = add(h, [-0.004, s * 0.058, 0.006]);
        prim(v, ear, ear, 0.019, 0.019, face, Mat::Matte);
    }
}

/// relative brightness of a colour, 0..1
fn lum(c: u32) -> f32 {
    let ch = |sh: u32| ((c >> sh) & 255) as f32 / 255.0;
    0.3 * ch(24) + 0.59 * ch(16) + 0.11 * ch(8)
}

/// a limb from `a` through `j` to `b`, thick at the top, and a hand or foot
/// from `b` along `dir`
#[allow(clippy::too_many_arguments)]
fn limb(v: &mut Vec<Prim>, a: V3, j: V3, b: V3, r: f32, dir: V3, len: f32, fur: u32, hand: u32) {
    prim(v, a, j, r, r * 0.8, fur, Mat::Matte);
    prim(v, j, b, r * 0.8, r * 0.62, fur, Mat::Matte);
    prim(
        v,
        b,
        add(b, mul(dir, len)),
        r * 0.6,
        r * 0.5,
        hand,
        Mat::Matte,
    );
}

/// the tail from `p0`, in `segs` pieces of `step`, each turned through
/// `turn` from the one before, starting along `ang` in the x-z plane (0 is
/// forward, PI/2 up, PI back); `sway` swings it sideways
fn tail(
    v: &mut Vec<Prim>,
    p0: V3,
    segs: usize,
    step: f32,
    ang: f32,
    turn: f32,
    sway: f32,
    col: u32,
) {
    let mut p = p0;
    let mut a = ang;
    let mut r = 0.022;
    for k in 0..segs {
        let kf = k as f32 / segs as f32;
        let q = add(p, [a.cos() * step, sway * kf * step, a.sin() * step]);
        prim(v, p, q, r, r * 0.92, col, Mat::Matte);
        r *= 0.93;
        p = q;
        a += turn;
    }
}

/// a tail lying along the ground from `p0`: back, then curling round to the
/// right side, `curl` radians over its length
fn tail_ground(v: &mut Vec<Prim>, p0: V3, segs: usize, step: f32, curl: f32, col: u32) {
    let mut p = p0;
    let mut a = PI;
    let mut r = 0.02;
    for _ in 0..segs {
        let q = [p[0] + a.cos() * step, p[1] + a.sin() * step, 0.02];
        prim(v, p, q, r, r * 0.93, col, Mat::Matte);
        r *= 0.94;
        p = q;
        a -= curl / segs as f32;
    }
}

pub(super) fn monkey(look: &Look, a: Anim, f: usize, n: usize) -> Figure {
    let mut v = vec![];
    let fur = look.skin;
    let pale = look.body;
    let hand = look.legs;
    let langur = look.long_hair;
    let (ph, _, recoil, die, _) = creature_phase(a, f, n);
    let lp = f as f32 / n.max(1) as f32;
    let (segs, step) = if langur { (12, 0.05) } else { (5, 0.045) };

    if matches!(a, Anim::Idle | Anim::Look) {
        // sitting on its haunches, knees up, hands on them
        let eat = if a == Anim::Look {
            0.5 - 0.5 * (lp * TAU).cos()
        } else {
            0.0
        };
        let rump = [-0.05, 0.0, 0.08];
        let chest = [0.0, 0.0, 0.27];
        prim(&mut v, rump, chest, 0.088, 0.074, fur, Mat::Matte);
        prim(
            &mut v,
            [-0.015, 0.0, 0.11],
            [0.012, 0.0, 0.23],
            0.064,
            0.052,
            pale,
            Mat::Matte,
        );
        // looking about while idle, down at its hand while it eats
        let turn = if a == Anim::Idle {
            [0.0, 0.016, 0.0][f.min(2)]
        } else {
            0.0
        };
        let h = [0.04 + 0.012 * eat, turn, 0.37 - 0.012 * eat];
        prim(
            &mut v,
            add(chest, [0.01, 0.0, 0.02]),
            h,
            0.05,
            0.045,
            fur,
            Mat::Matte,
        );
        head(&mut v, h, look, 0.25 * eat);
        for s in [-1.0f32, 1.0] {
            // hind legs folded, feet flat before it
            let hip = add(rump, [0.02, s * 0.055, 0.0]);
            let knee = [0.13, s * 0.078, 0.17];
            let ankle = [0.12, s * 0.062, 0.03];
            limb(
                &mut v,
                hip,
                knee,
                ankle,
                0.046,
                [1.0, 0.0, -0.1],
                0.05,
                fur,
                hand,
            );
            // arms: the left on its knee, the right up to its mouth and down
            let sh = add(chest, [0.0, s * 0.07, -0.01]);
            let t = if s > 0.0 { eat } else { 0.0 };
            let el = lerp3([0.1, s * 0.095, 0.16], [0.1, s * 0.11, 0.23], t);
            let mouth = add(h, [0.07, s * 0.012, -0.05]);
            let hd = lerp3([0.16, s * 0.05, 0.12], mouth, t);
            let d = if t > 0.5 {
                [0.0, -s * 0.6, 0.6]
            } else {
                [0.7, 0.0, -0.5]
            };
            limb(&mut v, sh, el, hd, 0.032, d, 0.022, fur, hand);
            if a == Anim::Look && s > 0.0 {
                // a morsel in its fist
                let m = add(hd, [0.015, -s * 0.012, 0.012]);
                prim(&mut v, m, m, 0.016, 0.016, 0xc89a48ff, Mat::Matte);
            }
        }
        // the tail along the ground behind, curling round to one side
        let t0 = add(rump, [-0.07, 0.0, -0.04]);
        tail_ground(&mut v, t0, segs, step, if langur { 2.4 } else { 1.2 }, fur);
        return Figure {
            prims: v,
            boxes: vec![],
            pitch: 0.0,
            roll: 0.0,
            lift: 0.0,
        };
    }

    // on all fours
    let run = a == Anim::Run;
    let gait = matches!(a, Anim::Walk | Anim::Run);
    let bob = if gait {
        (if run { 0.03 } else { 0.012 }) * (2.0 * ph).cos()
    } else {
        0.0
    };
    // a bounding run rocks the body: forequarters up, then hindquarters
    let rock = if run { 0.03 * ph.sin() } else { 0.0 };
    let dx = -recoil * 0.05;
    let hip = [-0.13 + dx, 0.0, 0.27 + bob - rock];
    let sho = [0.12 + dx, 0.0, 0.28 + bob + rock + recoil * 0.03];
    prim(&mut v, hip, sho, 0.08, 0.086, fur, Mat::Matte);
    prim(
        &mut v,
        add(hip, [0.04, 0.0, -0.035]),
        add(sho, [-0.03, 0.0, -0.04]),
        0.055,
        0.058,
        pale,
        Mat::Matte,
    );
    let h = add(sho, [0.105, 0.0, 0.065 + recoil * 0.04]);
    prim(
        &mut v,
        add(sho, [0.03, 0.0, 0.02]),
        h,
        0.052,
        0.046,
        fur,
        Mat::Matte,
    );
    head(&mut v, h, look, 0.15 - recoil * 0.3);
    // legs: diagonal pairs at a walk, hind and fore pairs together at a run
    let (stride, lift) = if run { (0.11, 0.07) } else { (0.06, 0.035) };
    for (front, side) in [(true, 1.0f32), (true, -1.0), (false, 1.0), (false, -1.0)] {
        let off = match (run, front, side > 0.0) {
            (true, true, _) => 0.0,
            (true, false, _) => PI,
            (false, true, true) | (false, false, false) => 0.0,
            _ => PI,
        };
        let s = ph + off;
        let (sx, sz) = if gait {
            (stride * s.sin(), lift * s.cos().max(0.0))
        } else {
            (0.0, 0.0)
        };
        if front {
            // the elbow bends back, the hand flat on the ground
            let top = add(sho, [0.0, side * 0.062, -0.03]);
            let foot = [sho[0] + 0.02 + sx, side * 0.066, 0.025 + sz];
            let el = add(lerp3(top, foot, 0.5), [-0.035, side * 0.008, 0.0]);
            limb(
                &mut v,
                top,
                el,
                foot,
                0.034,
                [1.0, 0.0, 0.0],
                0.03,
                fur,
                hand,
            );
        } else {
            // the knee bends forward, the long foot flat behind it
            let top = add(hip, [0.0, side * 0.058, -0.025]);
            let foot = [hip[0] - 0.035 + sx, side * 0.064, 0.03 + sz];
            let knee = add(lerp3(top, foot, 0.5), [0.06, side * 0.01, 0.0]);
            limb(
                &mut v,
                top,
                knee,
                foot,
                0.046,
                [1.0, 0.0, -0.15],
                0.06,
                fur,
                hand,
            );
        }
    }
    // the tail: a langur's arched up behind and hanging at the tip,
    // streaming out behind at a run; a macaque's short and drooping
    let sway = if gait { 0.4 * ph.sin() } else { 0.1 };
    let t0 = add(hip, [-0.07, 0.0, 0.02]);
    if langur {
        if run {
            tail(&mut v, t0, segs, step, 2.6, 0.06, sway, fur);
        } else {
            tail(&mut v, t0, segs, step, 1.75, 0.2, sway, fur);
        }
    } else {
        tail(&mut v, t0, segs, step, 3.4, 0.12, sway, fur);
    }
    Figure {
        prims: v,
        boxes: vec![],
        pitch: 0.0,
        roll: 1.45 * die,
        lift: 0.0,
    }
}
