//! The trees of the north: oaks (green, and turned for the autumn), pines,
//! silver birches, dead trees and stumps.
//!
//! A tree's crown is built of clumps, each a few dozen small leaves on and
//! in a ball, and every leaf is lit as part of its clump (`Mat::Foliage`), so
//! a crown reads as a few solid masses catching the light on their upper
//! left, going deep and cool underneath, rather than a heap of bubbles. Limbs
//! fork from the trunk into the clumps and show through the gaps beneath.

use super::*;
use std::f32::consts::TAU;

/// a leaf of a clump: a small ball lit as part of the clump
fn leaf(p: &mut Vec<Prim>, c: V3, size: f32, color: u32, clump: (V3, f32)) {
    p.push(Prim {
        a: c,
        b: c,
        ra: size,
        rb: size,
        color,
        mat: Mat::Foliage,
        vol: Some(clump),
    });
}

/// A clump of `n` leaves round `c`, radius `r`, mostly on its surface so its
/// edge is ragged with single leaves, its underside flatter than its top.
fn clump(p: &mut Vec<Prim>, c: V3, r: f32, n: usize, size: f32, cols: &[u32], seed: usize) {
    for i in 0..n {
        let k = seed * 977 + i;
        let a = hsh(k, 1) * TAU;
        let e = hsh(k, 2) * 2.0 - 1.0;
        let rr = r * (0.45 + 0.55 * hsh(k, 3).powf(0.4));
        let ce = (1.0 - e * e).sqrt();
        let squash = if e < 0.0 { 0.55 } else { 0.85 };
        let pt = [
            c[0] + a.cos() * ce * rr,
            c[1] + a.sin() * ce * rr,
            c[2] + e * rr * squash,
        ];
        let sz = size * (0.75 + hsh(k, 4) * 0.5);
        // lighter leaves towards the top of the clump, darker below
        let pick = ((hsh(k, 5) * 0.7 + (e + 1.0) * 0.25) * cols.len() as f32) as usize;
        leaf(p, pt, sz, cols[pick.min(cols.len() - 1)], (c, r * 1.05));
    }
}

/// roots spreading from the foot of a trunk of radius `r`
fn roots(p: &mut Vec<Prim>, r: f32, bark: u32, seed: usize) {
    for i in 0..5 {
        let a = i as f32 / 5.0 * TAU + hsh(i, seed) * 0.8;
        let l = r * (1.8 + hsh(i, seed + 1) * 1.2);
        prim(
            p,
            [a.cos() * r * 0.4, a.sin() * r * 0.4, r * 1.4],
            [a.cos() * l, a.sin() * l, 0.0],
            r * 0.55,
            r * 0.2,
            bark,
            Mat::Wood,
        );
    }
}

/// A broadleaf tree: a flared trunk forking into limbs, each bearing a clump,
/// round a central crown. `v` varies its shape; `cols` are its leaves from
/// darkest to lightest.
fn broadleaf(p: &mut Vec<Prim>, v: usize, cols: &[u32], seed: usize) {
    let bark = 0x4a3524ff;
    let h = 1.25 + v as f32 * 0.1;
    let lean = [0.06 + v as f32 * 0.03, 0.02 * v as f32 - 0.02];
    let fork = [lean[0], lean[1], h];
    roots(p, 0.16, bark, seed + 3);
    prim(p, [0.0, 0.0, 0.0], fork, 0.17, 0.12, bark, Mat::Wood);
    let crown_z = h + 1.05;
    let n = 4 + v % 2;
    let mut clumps = vec![];
    for l in 0..n {
        let a = l as f32 / n as f32 * TAU + hsh(l, seed + 7) * 0.9 + v as f32;
        let d = 0.62 + hsh(l, seed + 8) * 0.3;
        let z = crown_z - 0.25 + hsh(l, seed + 9) * 0.55;
        let r = 0.48 + hsh(l, seed + 10) * 0.18;
        clumps.push(([fork[0] + a.cos() * d, fork[1] + a.sin() * d, z], r));
    }
    // a high clump over the middle, and the crown's heart
    clumps.push(([fork[0] - 0.05, fork[1], crown_z + 0.55], 0.5));
    clumps.push(([fork[0], fork[1], crown_z + 0.05], 0.62));
    // limbs from the fork out under each outer clump, a twig or two beyond
    for (k, &(c, r)) in clumps.iter().enumerate().take(n) {
        let end = [
            fork[0] + (c[0] - fork[0]) * 0.8,
            fork[1] + (c[1] - fork[1]) * 0.8,
            c[2] - r * 0.35,
        ];
        let mid = [
            fork[0] + (end[0] - fork[0]) * 0.5,
            fork[1] + (end[1] - fork[1]) * 0.5,
            fork[2] + (end[2] - fork[2]) * 0.35 + 0.08,
        ];
        prim(
            p,
            [fork[0], fork[1], fork[2] - 0.15],
            mid,
            0.085,
            0.06,
            bark,
            Mat::Wood,
        );
        prim(p, mid, end, 0.06, 0.03, bark, Mat::Wood);
        if hsh(k, seed + 11) > 0.5 {
            let a = hsh(k, seed + 12) * TAU;
            prim(
                p,
                end,
                [
                    end[0] + a.cos() * 0.35,
                    end[1] + a.sin() * 0.35,
                    end[2] + 0.45,
                ],
                0.025,
                0.01,
                bark,
                Mat::Wood,
            );
        }
    }
    for (k, &(c, r)) in clumps.iter().enumerate() {
        let leaves = (r * r * 230.0) as usize;
        clump(p, c, r, leaves, 0.085, cols, seed * 31 + k);
    }
}

pub(super) fn build(kind: usize, p: &mut Vec<Prim>, _b: &mut Vec<BoxP>) -> bool {
    match kind {
        k if (P_OAK..P_OAK + 3).contains(&k) => {
            let cols = [
                0x1e3016ffu32,
                0x26391aff,
                0x2e4520ff,
                0x375224ff,
                0x42602aff,
            ];
            broadleaf(p, k - P_OAK, &cols, 100 + k);
        }
        k if (P_AUTUMN..P_AUTUMN + 2).contains(&k) => {
            // an oak turned for the autumn, leaves already down round its foot
            let v = k - P_AUTUMN;
            let cols = [
                0x6a2410ffu32,
                0x8a3a14ff,
                0xa85a1aff,
                0xb87020ff,
                0xc89030ff,
            ];
            broadleaf(p, v, &cols, 200 + k);
            for i in 0..16 {
                let (a, d) = (hsh(i, 70 + v) * TAU, 0.3 + hsh(i, 71 + v) * 0.9);
                let (x, y) = (a.cos() * d, a.sin() * d);
                prim(
                    p,
                    [x, y, 0.01],
                    [x + 0.05, y, 0.01],
                    0.045,
                    0.04,
                    cols[i % 5],
                    Mat::Leaf,
                );
            }
        }
        k if (P_PINE..P_PINE + 2).contains(&k) => {
            // a pine: a straight trunk and tiers of drooping boughs, wide
            // at the foot and narrowing to a spire, each tier lit on top
            // and dark beneath
            let v = k - P_PINE;
            let bark = 0x3e2a1aff;
            let top = 3.3 + v as f32 * 0.3;
            roots(p, 0.12, bark, 40 + v);
            prim(
                p,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, top],
                0.13,
                0.03,
                bark,
                Mat::Wood,
            );
            let cols = [
                0x162a1effu32,
                0x1c3424ff,
                0x223e2aff,
                0x2a4a30ff,
                0x355838ff,
            ];
            let tiers = 7;
            for t in 0..tiers {
                let tf = t as f32 / (tiers - 1) as f32;
                let z = 0.75 + tf * (top - 1.0);
                let rad = (1.05 - tf * 0.85) * (1.0 + v as f32 * 0.08);
                let boughs = 9 - t;
                let mass = ([0.0, 0.0, z - rad * 0.15], rad * 0.95);
                for j in 0..boughs {
                    let a = j as f32 / boughs as f32 * TAU + t as f32 * 0.7 + hsh(j, t + 50) * 0.3;
                    let tip = [a.cos() * rad, a.sin() * rad, z - rad * 0.35];
                    // needles in sprays along the bough, drooping at the tip
                    let n = 6 + (rad * 8.0) as usize;
                    for i in 0..n {
                        let s = (i as f32 + 0.5) / n as f32;
                        let jitter = (hsh(i + j * 17, t + 60) - 0.5) * 0.14;
                        let pt = [
                            tip[0] * s - a.sin() * jitter,
                            tip[1] * s + a.cos() * jitter,
                            z + (tip[2] - z) * s * s + (hsh(i, j + 70) - 0.3) * 0.12,
                        ];
                        let sz = 0.11 - s * 0.04;
                        let pick = ((hsh(i * 7 + j, t + 80) * 0.6 + tf * 0.3 + (1.0 - s) * 0.2)
                            * 5.0) as usize;
                        leaf(p, pt, sz, cols[pick.min(4)], mass);
                    }
                }
            }
            leaf(
                p,
                [0.0, 0.0, top + 0.05],
                0.08,
                cols[3],
                ([0.0, 0.0, top - 0.2], 0.3),
            );
        }
        k if (P_BIRCH..P_BIRCH + 2).contains(&k) => {
            // a silver birch: a slender white trunk banded black, thin limbs
            // and a light crown of small leaves, its ends hanging
            let v = k - P_BIRCH;
            let bark = 0xd8d4c8ff;
            let top = [0.08, 0.03, 2.4];
            prim(p, [0.0, 0.0, 0.0], top, 0.09, 0.045, bark, Mat::Matte);
            if v == 1 {
                prim(
                    p,
                    [0.04, 0.0, 0.25],
                    [-0.3, 0.18, 2.0],
                    0.06,
                    0.035,
                    bark,
                    Mat::Matte,
                );
            }
            for i in 0..8 {
                let z = 0.15 + i as f32 * 0.26 + hsh(i, 90 + v) * 0.1;
                let x = top[0] * z / top[2];
                prim(
                    p,
                    [x, 0.01, z],
                    [x, 0.01, z + 0.04 + hsh(i, 91) * 0.03],
                    0.093 - z * 0.019,
                    0.09 - z * 0.019,
                    0x2a2620ff,
                    Mat::Matte,
                );
            }
            let cols = [
                0x3e5a22ffu32,
                0x4e6e28ff,
                0x5e8030ff,
                0x6e9238ff,
                0x84a646ff,
            ];
            let n = 5;
            for l in 0..n {
                let a = l as f32 / n as f32 * TAU + v as f32 * 0.5;
                let d = 0.3 + hsh(l, 95 + v) * 0.15;
                let z = 2.0 + hsh(l, 96 + v) * 0.6;
                let c = [top[0] + a.cos() * d, top[1] + a.sin() * d, z];
                prim(
                    p,
                    [top[0], top[1], z - 0.5],
                    c,
                    0.03,
                    0.012,
                    bark,
                    Mat::Matte,
                );
                clump(p, c, 0.32, 34, 0.065, &cols, 300 + l + v * 10);
                // a few strands hanging from the clump
                for s in 0..3 {
                    let b = hsh(s + l * 5, 97 + v) * TAU;
                    let mut pt = [c[0] + b.cos() * 0.28, c[1] + b.sin() * 0.28, c[2] - 0.1];
                    for _ in 0..4 {
                        pt[2] -= 0.1;
                        leaf(p, pt, 0.05, cols[1 + s % 3], (c, 0.45));
                    }
                }
            }
            clump(p, [top[0], top[1], 2.75], 0.36, 40, 0.065, &cols, 350 + v);
        }
        k if (P_DEAD..P_DEAD + 2).contains(&k) => {
            // a dead tree, grey and gnarled: limbs forking twice, crooked
            let v = k - P_DEAD;
            let c = 0x4e4438ff;
            roots(p, 0.14, c, 60 + v);
            let top = [0.1, 0.0, 1.7];
            prim(p, [0.0, 0.0, 0.0], top, 0.15, 0.07, c, Mat::Wood);
            for i in 0..4 + v {
                let a = hsh(i, 40 + v) * TAU;
                let z = 0.9 + hsh(i, 41) * 0.8;
                let base = [top[0] * z / top[2], 0.0, z];
                let l = 0.45 + hsh(i, 42) * 0.35;
                let e = [
                    base[0] + a.cos() * l,
                    base[1] + a.sin() * l,
                    z + 0.35 + hsh(i, 43) * 0.3,
                ];
                prim(p, base, e, 0.055, 0.025, c, Mat::Wood);
                for j in 0..2 {
                    let b = a + (j as f32 - 0.5) * 1.3 + hsh(i * 3 + j, 44) * 0.4;
                    let f = [
                        e[0] + b.cos() * 0.28,
                        e[1] + b.sin() * 0.28,
                        e[2] + 0.12 + hsh(j, i + 45) * 0.2,
                    ];
                    prim(p, e, f, 0.022, 0.008, c, Mat::Wood);
                }
            }
            prim(
                p,
                top,
                [top[0] - 0.1, 0.05, top[2] + 0.45],
                0.06,
                0.015,
                c,
                Mat::Wood,
            );
        }
        P_STUMP => {
            // a felled tree's stump: roots, bark, and the pale cut with its rings
            let bark = 0x4a3220ff;
            roots(p, 0.15, bark, 77);
            prim(
                p,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.3],
                0.21,
                0.18,
                bark,
                Mat::Wood,
            );
            prim(
                p,
                [0.0, 0.0, 0.31],
                [0.0, 0.0, 0.32],
                0.17,
                0.17,
                0x9a7a50ff,
                Mat::Matte,
            );
            prim(
                p,
                [0.0, 0.0, 0.325],
                [0.0, 0.0, 0.33],
                0.1,
                0.1,
                0x7a5a38ff,
                Mat::Matte,
            );
            prim(
                p,
                [0.0, 0.0, 0.333],
                [0.0, 0.0, 0.336],
                0.04,
                0.04,
                0xa88a58ff,
                Mat::Matte,
            );
        }
        _ => return false,
    }
    true
}
