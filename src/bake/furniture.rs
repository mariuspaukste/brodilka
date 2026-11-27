//! Furniture and house decorations: the things that stand inside houses,
//! modelled in local metres facing +x (their back to -x), like every prop.
//!
//! Timber is `Mat::Wood` and masonry `Mat::Stone`, so boards show their
//! grain and seams and a chimney its courses, the way Pagan's hand-painted
//! furniture did.

use super::*;
use std::f32::consts::{PI, TAU};

const OAK: u32 = 0x6a4628ff;
const OAK_DK: u32 = 0x3e2816ff;
const OAK_LT: u32 = 0x86603aff;
const IRON: u32 = 0x34363aff;
const PEWTER: u32 = 0x8a8e94ff;
const BRASS: u32 = 0xc89a48ff;
const LINEN: u32 = 0xd0c8b0ff;
const CLAY: u32 = 0xa0582eff;
const CANDLE: u32 = 0xe8e0c0ff;
const FLAME: u32 = 0xffc040ff;

/// a flat eight-sided disc (four boxes turned through 45° each), for
/// tops, rims and the surface of a brew
fn disc(b: &mut Vec<BoxP>, c: V3, r: f32, thick: f32, color: u32, mat: Mat) {
    for k in 0..4 {
        b.push(BoxP {
            c,
            h: [r, r * 0.4142, thick],
            yaw: k as f32 * PI / 4.0,
            color,
            mat,
        });
    }
}

/// a ring of short rods round `c`, in the plane spanned by `u` and `v`
#[allow(clippy::too_many_arguments)]
fn ring(p: &mut Vec<Prim>, c: V3, u: V3, v: V3, r: f32, th: f32, n: usize, color: u32, mat: Mat) {
    let at = |a: f32| {
        [
            c[0] + (u[0] * a.cos() + v[0] * a.sin()) * r,
            c[1] + (u[1] * a.cos() + v[1] * a.sin()) * r,
            c[2] + (u[2] * a.cos() + v[2] * a.sin()) * r,
        ]
    };
    for k in 0..n {
        let (a0, a1) = (k as f32 / n as f32 * TAU, (k + 1) as f32 / n as f32 * TAU);
        prim(p, at(a0), at(a1), th, th, color, mat);
    }
}

/// a candle standing on `base`, lit
fn candle(p: &mut Vec<Prim>, base: V3, tall: f32) {
    let top = [base[0], base[1], base[2] + tall];
    prim(p, base, top, 0.022, 0.022, CANDLE, Mat::Matte);
    prim(
        p,
        [top[0], top[1], top[2] + 0.025],
        [top[0], top[1], top[2] + 0.05],
        0.016,
        0.006,
        FLAME,
        Mat::Glow,
    );
}

/// a lathe-turned clay vessel: (height, radius) pairs from the foot up
fn vessel(p: &mut Vec<Prim>, at: V3, profile: &[(f32, f32)], color: u32, mat: Mat) {
    for w in profile.windows(2) {
        let (z0, r0) = w[0];
        let (z1, r1) = w[1];
        prim(
            p,
            [at[0], at[1], at[2] + z0],
            [at[0], at[1], at[2] + z1],
            r0,
            r1,
            color,
            mat,
        );
    }
}

/// a sword standing point down, its hilt at `top`
fn sword(p: &mut Vec<Prim>, b: &mut Vec<BoxP>, top: V3, len: f32, grip: u32) {
    let [x, y, z] = top;
    bx(
        b,
        [x, y, z - 0.16 - len * 0.5],
        [0.008, 0.028, len * 0.5],
        0xb8bcc4ff,
        Mat::Metal,
    );
    bx(
        b,
        [x, y, z - 0.15],
        [0.02, 0.09, 0.015],
        0x5a5048ff,
        Mat::Metal,
    );
    prim(
        p,
        [x, y, z - 0.14],
        [x, y, z - 0.02],
        0.022,
        0.022,
        grip,
        Mat::Matte,
    );
    prim(p, [x, y, z], [x, y, z], 0.03, 0.03, BRASS, Mat::Metal);
}

/// Builds `kind` if it is furniture or a house decoration; false otherwise.
pub(super) fn build(kind: usize, p: &mut Vec<Prim>, b: &mut Vec<BoxP>) -> bool {
    match kind {
        P_BED => {
            // a four-poster frame without the canopy: tall headboard (+x),
            // low footboard, a straw mattress under a patchwork quilt
            for (x, y, top) in [
                (0.6f32, -0.36f32, 1.02f32),
                (0.6, 0.36, 1.02),
                (-0.6, -0.36, 0.62),
                (-0.6, 0.36, 0.62),
            ] {
                bx(
                    b,
                    [x, y, top * 0.5],
                    [0.035, 0.035, top * 0.5],
                    OAK_DK,
                    Mat::Wood,
                );
                prim(
                    p,
                    [x, y, top + 0.02],
                    [x, y, top + 0.05],
                    0.045,
                    0.03,
                    OAK,
                    Mat::Wood,
                );
            }
            bx(b, [0.6, 0.0, 0.66], [0.025, 0.33, 0.28], OAK, Mat::Wood);
            bx(b, [0.6, 0.0, 0.96], [0.03, 0.35, 0.035], OAK_DK, Mat::Wood);
            bx(b, [-0.6, 0.0, 0.42], [0.025, 0.33, 0.14], OAK, Mat::Wood);
            for y in [-0.36f32, 0.36] {
                bx(b, [0.0, y, 0.24], [0.58, 0.03, 0.07], OAK_DK, Mat::Wood);
            }
            bx(
                b,
                [0.0, 0.0, 0.33],
                [0.57, 0.33, 0.07],
                0xb8a878ff,
                Mat::Cloth,
            );
            // the sheet turned down at the head, then the quilt in squares
            bx(b, [0.32, 0.0, 0.42], [0.1, 0.345, 0.025], LINEN, Mat::Cloth);
            let patch = [0x7a2420ff, 0x8a6a3aff, 0x4a3a5aff, 0x6a3020ff];
            for i in 0..3 {
                for j in 0..2 {
                    let x = -0.47 + i as f32 * 0.24;
                    let y = -0.17 + j as f32 * 0.34;
                    bx(
                        b,
                        [x, y, 0.425],
                        [0.12, 0.17, 0.028],
                        patch[(i + j * 3) % 4],
                        Mat::Cloth,
                    );
                }
            }
            // the quilt hangs over the sides
            for y in [-0.35f32, 0.35] {
                bx(
                    b,
                    [-0.23, y, 0.36],
                    [0.36, 0.02, 0.08],
                    0x5a2018ff,
                    Mat::Cloth,
                );
            }
            prim(
                p,
                [0.46, -0.16, 0.48],
                [0.46, 0.16, 0.48],
                0.075,
                0.075,
                0xe8e0d0ff,
                Mat::Cloth,
            );
        }
        P_TABLE => {
            // a plank top on an apron, turned legs joined low by stretchers
            bx(b, [0.0, 0.0, 0.74], [0.46, 0.36, 0.04], OAK, Mat::Wood);
            for y in [-0.3f32, 0.3] {
                bx(b, [0.0, y, 0.64], [0.36, 0.02, 0.06], OAK_DK, Mat::Wood);
            }
            for x in [-0.36f32, 0.36] {
                bx(b, [x, 0.0, 0.64], [0.02, 0.28, 0.06], OAK_DK, Mat::Wood);
            }
            for (x, y) in [
                (0.37f32, 0.29f32),
                (-0.37, 0.29),
                (0.37, -0.29),
                (-0.37, -0.29),
            ] {
                prim(p, [x, y, 0.0], [x, y, 0.7], 0.04, 0.034, OAK_DK, Mat::Wood);
                prim(p, [x, y, 0.36], [x, y, 0.38], 0.05, 0.05, OAK_DK, Mat::Wood);
            }
            bx(b, [0.0, 0.29, 0.14], [0.36, 0.018, 0.02], OAK_DK, Mat::Wood);
            bx(
                b,
                [0.0, -0.29, 0.14],
                [0.36, 0.018, 0.02],
                OAK_DK,
                Mat::Wood,
            );
            bx(b, [0.0, 0.0, 0.14], [0.018, 0.28, 0.02], OAK_DK, Mat::Wood);
            // supper: a pewter plate and a loaf, a clay jug, a candle in its stick
            disc(b, [0.08, 0.1, 0.785], 0.11, 0.008, PEWTER, Mat::Metal);
            prim(
                p,
                [0.04, 0.06, 0.83],
                [0.12, 0.14, 0.83],
                0.05,
                0.045,
                0xa8703aff,
                Mat::Matte,
            );
            vessel(
                p,
                [-0.2, -0.14, 0.78],
                &[(0.0, 0.035), (0.05, 0.05), (0.1, 0.03), (0.14, 0.028)],
                CLAY,
                Mat::Bone,
            );
            disc(b, [0.22, -0.18, 0.79], 0.04, 0.008, PEWTER, Mat::Metal);
            candle(p, [0.22, -0.18, 0.8], 0.12);
        }
        P_CHAIR => {
            // a ladder-back chair: seat, four legs with a rung, three slats
            bx(b, [0.0, 0.0, 0.44], [0.2, 0.2, 0.03], OAK, Mat::Wood);
            for (x, y) in [(0.16f32, 0.16f32), (0.16, -0.16)] {
                prim(
                    p,
                    [x, y, 0.0],
                    [x, y, 0.42],
                    0.028,
                    0.024,
                    OAK_DK,
                    Mat::Wood,
                );
            }
            for y in [-0.16f32, 0.16] {
                prim(
                    p,
                    [-0.17, y, 0.0],
                    [-0.17, y, 1.0],
                    0.03,
                    0.026,
                    OAK_DK,
                    Mat::Wood,
                );
                prim(
                    p,
                    [-0.17, y, 1.02],
                    [-0.17, y, 1.06],
                    0.034,
                    0.02,
                    OAK,
                    Mat::Wood,
                );
            }
            for z in [0.64f32, 0.78, 0.92] {
                bx(b, [-0.17, 0.0, z], [0.018, 0.15, 0.035], OAK, Mat::Wood);
            }
            prim(
                p,
                [0.16, -0.16, 0.16],
                [0.16, 0.16, 0.16],
                0.016,
                0.016,
                OAK_DK,
                Mat::Wood,
            );
            prim(
                p,
                [-0.17, -0.16, 0.16],
                [-0.17, 0.16, 0.16],
                0.016,
                0.016,
                OAK_DK,
                Mat::Wood,
            );
        }
        P_SHELF => {
            // a bookcase: two sides, a back, a cornice; books, a jar, scrolls, a skull
            bx(b, [-0.15, 0.0, 0.92], [0.02, 0.44, 0.9], OAK_DK, Mat::Wood);
            for y in [-0.46f32, 0.46] {
                bx(b, [0.0, y, 0.92], [0.17, 0.025, 0.92], OAK, Mat::Wood);
            }
            bx(b, [0.02, 0.0, 1.86], [0.2, 0.5, 0.04], OAK_DK, Mat::Wood);
            bx(b, [0.03, 0.0, 0.04], [0.19, 0.49, 0.04], OAK_DK, Mat::Wood);
            for row in 0..4 {
                let z = 0.28 + row as f32 * 0.42;
                bx(b, [0.0, 0.0, z - 0.05], [0.16, 0.44, 0.02], OAK, Mat::Wood);
                let mut y = -0.42;
                let mut k = 0;
                while y < 0.38 {
                    let h = |s| hsh(row * 31 + k, s);
                    let slot = h(4);
                    if slot < 0.1 && k > 0 {
                        // a clay jar
                        vessel(
                            p,
                            [0.02, y + 0.06, z - 0.03],
                            &[(0.0, 0.045), (0.06, 0.06), (0.13, 0.035), (0.16, 0.04)],
                            [CLAY, 0x5a6a5aff][k % 2],
                            Mat::Bone,
                        );
                        y += 0.14;
                    } else if slot < 0.17 && row == 3 {
                        // a skull, for this is a grim land
                        prim(
                            p,
                            [0.03, y + 0.07, z + 0.06],
                            [0.05, y + 0.07, z + 0.04],
                            0.06,
                            0.05,
                            0xd8d0b8ff,
                            Mat::Bone,
                        );
                        prim(
                            p,
                            [0.09, y + 0.05, z + 0.06],
                            [0.09, y + 0.05, z + 0.06],
                            0.014,
                            0.014,
                            0x201810ff,
                            Mat::Matte,
                        );
                        prim(
                            p,
                            [0.09, y + 0.09, z + 0.06],
                            [0.09, y + 0.09, z + 0.06],
                            0.014,
                            0.014,
                            0x201810ff,
                            Mat::Matte,
                        );
                        y += 0.15;
                    } else if slot < 0.24 {
                        // scrolls lying in a heap
                        for s in 0..3 {
                            let sz = z + if s == 2 { 0.05 } else { 0.0 };
                            let sy = y + 0.04 + s as f32 * 0.045 - if s == 2 { 0.02 } else { 0.0 };
                            prim(
                                p,
                                [-0.08, sy, sz],
                                [0.12, sy, sz],
                                0.024,
                                0.024,
                                0xd8c898ff,
                                Mat::Matte,
                            );
                        }
                        y += 0.16;
                    } else {
                        let w = 0.025 + h(1) * 0.035;
                        let hgt = 0.1 + h(2) * 0.06;
                        let col = [
                            0x7a2a20ff, 0x2a4a7aff, 0x3a6a2aff, 0x8a6a2aff, 0x5a2a5aff, 0x3a2a1aff,
                        ][(h(3) * 6.0) as usize % 6];
                        // books of every height, a gap here and there
                        bx(
                            b,
                            [0.02, y + w, z - 0.03 + hgt],
                            [0.11, w * 0.9, hgt],
                            col,
                            Mat::Cloth,
                        );
                        if h(5) < 0.12 {
                            y += 0.02;
                        }
                        // gilt bands on the spine
                        if h(6) < 0.6 {
                            for bz in [0.3f32, 1.6] {
                                bx(
                                    b,
                                    [0.131, y + w, z - 0.03 + hgt * bz],
                                    [0.003, w * 0.8, 0.008],
                                    0xc8a050ff,
                                    Mat::Metal,
                                );
                            }
                        }
                        y += w * 2.0 + 0.006;
                    }
                    k += 1;
                }
            }
        }
        P_HEARTH => {
            // a stone chimney breast on a raised hearthstone; logs burning in
            // the firebox, a black pot to one side, an oak mantel with candles
            bx(
                b,
                [-0.06, 0.0, 1.0],
                [0.26, 0.62, 1.0],
                0x6e6a62ff,
                Mat::Stone,
            );
            bx(
                b,
                [-0.1, 0.0, 1.9],
                [0.22, 0.46, 0.12],
                0x625e58ff,
                Mat::Stone,
            );
            bx(
                b,
                [0.12, 0.0, 0.05],
                [0.18, 0.66, 0.05],
                0x5a5650ff,
                Mat::Stone,
            );
            // the opening: dark soot behind, stone jambs, an oak lintel
            bx(
                b,
                [0.18, 0.0, 0.4],
                [0.04, 0.34, 0.32],
                0x0e0a08ff,
                Mat::Matte,
            );
            for y in [-0.42f32, 0.42] {
                bx(
                    b,
                    [0.21, y, 0.4],
                    [0.03, 0.08, 0.34],
                    0x7a756cff,
                    Mat::Stone,
                );
            }
            bx(b, [0.23, 0.0, 0.78], [0.05, 0.5, 0.05], OAK_DK, Mat::Wood);
            bx(b, [0.28, 0.0, 0.88], [0.08, 0.64, 0.03], OAK, Mat::Wood);
            for k in 0..3 {
                let y = -0.12 + k as f32 * 0.12;
                prim(
                    p,
                    [0.12, y - 0.12, 0.14],
                    [0.3, y + 0.08, 0.14 + k as f32 * 0.03],
                    0.045,
                    0.04,
                    0x4a3220ff,
                    Mat::Wood,
                );
            }
            for k in 0..6 {
                let kf = k as f32;
                prim(
                    p,
                    [0.24, -0.18 + kf * 0.07, 0.16],
                    [
                        0.24,
                        -0.16 + kf * 0.066,
                        0.3 + 0.08 * (kf * 1.7).sin().abs(),
                    ],
                    0.055,
                    0.015,
                    [0xff8a20ff, 0xffd050ff, 0xff6a10ff][k % 3],
                    Mat::Glow,
                );
            }
            // andirons
            for y in [-0.24f32, 0.24] {
                prim(
                    p,
                    [0.3, y, 0.1],
                    [0.3, y, 0.22],
                    0.02,
                    0.02,
                    IRON,
                    Mat::Metal,
                );
                prim(
                    p,
                    [0.3, y, 0.23],
                    [0.3, y, 0.23],
                    0.03,
                    0.03,
                    BRASS,
                    Mat::Metal,
                );
            }
            // the pot on the hearthstone
            prim(
                p,
                [0.24, 0.52, 0.2],
                [0.24, 0.52, 0.22],
                0.12,
                0.11,
                0x2a2a2cff,
                Mat::Metal,
            );
            disc(b, [0.24, 0.52, 0.3], 0.09, 0.012, 0x3a3a3eff, Mat::Metal);
            // on the mantel: two candlesticks and a jug
            for y in [-0.45f32, 0.3] {
                prim(
                    p,
                    [0.3, y, 0.91],
                    [0.3, y, 0.97],
                    0.03,
                    0.018,
                    BRASS,
                    Mat::Metal,
                );
                candle(p, [0.3, y, 0.97], 0.1);
            }
            vessel(
                p,
                [0.3, -0.05, 0.91],
                &[(0.0, 0.04), (0.06, 0.055), (0.12, 0.03)],
                0x6a7a8aff,
                Mat::Bone,
            );
        }
        P_CHEST => {
            // an iron-bound chest with a rounded lid, corner plates and a lock
            bx(b, [0.0, 0.0, 0.21], [0.26, 0.4, 0.2], OAK, Mat::Wood);
            bx(b, [0.0, 0.0, 0.02], [0.27, 0.41, 0.02], OAK_DK, Mat::Wood);
            for (z, hx, hz) in [
                (0.44f32, 0.27f32, 0.03f32),
                (0.5, 0.22, 0.03),
                (0.545, 0.13, 0.02),
            ] {
                bx(b, [0.0, 0.0, z], [hx, 0.41, hz], OAK_DK, Mat::Wood);
            }
            for y in [-0.3f32, 0.0, 0.3] {
                bx(b, [0.0, y, 0.24], [0.272, 0.025, 0.22], IRON, Mat::Metal);
                for (z, hx, hz) in [
                    (0.44f32, 0.282f32, 0.032f32),
                    (0.5, 0.232, 0.032),
                    (0.545, 0.14, 0.022),
                ] {
                    bx(b, [0.0, y, z], [hx, 0.027, hz], IRON, Mat::Metal);
                }
            }
            for y in [-0.39f32, 0.39] {
                bx(b, [0.2, y, 0.21], [0.07, 0.025, 0.2], IRON, Mat::Metal);
            }
            bx(
                b,
                [0.27, 0.15, 0.33],
                [0.012, 0.06, 0.07],
                BRASS,
                Mat::Metal,
            );
            prim(
                p,
                [0.285, 0.15, 0.33],
                [0.285, 0.15, 0.3],
                0.012,
                0.008,
                0x100c08ff,
                Mat::Matte,
            );
            // carrying handles at the ends
            for y in [-0.42f32, 0.42] {
                prim(
                    p,
                    [-0.08, y, 0.28],
                    [0.08, y, 0.28],
                    0.016,
                    0.016,
                    IRON,
                    Mat::Metal,
                );
            }
        }
        P_BARREL => {
            // an eight-sided stave barrel, bulging at the waist, three hoops,
            // a plank head and a wooden tap
            for k in 0..8 {
                let yaw = k as f32 / 8.0 * PI;
                for (z, hz, r) in [
                    (0.14f32, 0.14f32, 0.27f32),
                    (0.42, 0.15, 0.3),
                    (0.7, 0.14, 0.27),
                ] {
                    b.push(BoxP {
                        c: [0.0, 0.0, z],
                        h: [r, r * 0.414, hz],
                        yaw,
                        color: if k % 2 == 0 { OAK } else { 0x5e3e22ff },
                        mat: Mat::Wood,
                    });
                }
                for (z, r) in [(0.1f32, 0.28f32), (0.42, 0.31), (0.76, 0.28)] {
                    b.push(BoxP {
                        c: [0.0, 0.0, z],
                        h: [r, r * 0.415, 0.028],
                        yaw,
                        color: 0x2e2e30ff,
                        mat: Mat::Metal,
                    });
                }
            }
            disc(b, [0.0, 0.0, 0.84], 0.25, 0.012, OAK_LT, Mat::Wood);
            prim(
                p,
                [0.28, 0.0, 0.3],
                [0.36, 0.0, 0.3],
                0.025,
                0.02,
                OAK_DK,
                Mat::Wood,
            );
            prim(
                p,
                [0.36, 0.0, 0.3],
                [0.36, 0.0, 0.26],
                0.018,
                0.012,
                OAK_DK,
                Mat::Wood,
            );
        }
        P_CRATE => {
            // a slatted crate: corner posts, rails, slats with gaps, straw inside
            bx(
                b,
                [0.0, 0.0, 0.3],
                [0.27, 0.27, 0.28],
                0x1a120cff,
                Mat::Matte,
            );
            for (x, y) in [
                (0.27f32, 0.27f32),
                (-0.27, 0.27),
                (0.27, -0.27),
                (-0.27, -0.27),
            ] {
                bx(b, [x, y, 0.3], [0.035, 0.035, 0.3], OAK_DK, Mat::Wood);
            }
            for z in [0.07f32, 0.24, 0.41] {
                for (c, h) in [
                    ([0.28f32, 0.0f32], [0.012f32, 0.25f32]),
                    ([-0.28, 0.0], [0.012, 0.25]),
                    ([0.0, 0.28], [0.25, 0.012]),
                    ([0.0, -0.28], [0.25, 0.012]),
                ] {
                    bx(
                        b,
                        [c[0], c[1], z],
                        [h[0], h[1], 0.06],
                        0x8a6438ff,
                        Mat::Wood,
                    );
                }
            }
            for (c, h) in [
                ([0.0f32, 0.275f32], [0.3f32, 0.02f32]),
                ([0.0, -0.275], [0.3, 0.02]),
                ([0.275, 0.0], [0.02, 0.3]),
                ([-0.275, 0.0], [0.02, 0.3]),
            ] {
                bx(
                    b,
                    [c[0], c[1], 0.575],
                    [h[0], h[1], 0.025],
                    OAK_DK,
                    Mat::Wood,
                );
            }
            for k in 0..7 {
                let (x, y) = (hsh(k, 1) * 0.36 - 0.18, hsh(k, 2) * 0.36 - 0.18);
                prim(
                    p,
                    [x, y, 0.56],
                    [x + 0.06, y - 0.04, 0.6],
                    0.04,
                    0.03,
                    0xc8a850ff,
                    Mat::Hair,
                );
            }
        }
        P_WARDROBE => {
            // a tall press: plinth, carcass, two framed doors with raised
            // panels and iron strap hinges, a moulded cornice
            bx(b, [-0.02, 0.0, 1.0], [0.26, 0.46, 0.92], OAK, Mat::Wood);
            bx(b, [0.0, 0.0, 0.05], [0.29, 0.49, 0.05], OAK_DK, Mat::Wood);
            bx(b, [0.01, 0.0, 1.94], [0.3, 0.52, 0.035], OAK_DK, Mat::Wood);
            bx(
                b,
                [-0.01, 0.0, 1.87],
                [0.28, 0.49, 0.035],
                OAK_LT,
                Mat::Wood,
            );
            bx(
                b,
                [0.25, 0.0, 1.0],
                [0.025, 0.012, 0.86],
                0x140c08ff,
                Mat::Matte,
            );
            for y in [-0.23f32, 0.23] {
                bx(b, [0.255, y, 1.0], [0.02, 0.21, 0.86], OAK_DK, Mat::Wood);
                for (z, hz) in [(0.5f32, 0.3f32), (1.36, 0.42)] {
                    bx(b, [0.285, y, z], [0.02, 0.15, hz], 0x8a643eff, Mat::Wood);
                }
                for z in [0.3f32, 1.65] {
                    bx(
                        b,
                        [0.28, y + y.signum() * 0.1, z],
                        [0.012, 0.1, 0.018],
                        IRON,
                        Mat::Metal,
                    );
                }
                prim(
                    p,
                    [0.31, y * 0.2, 0.96],
                    [0.31, y * 0.2, 1.06],
                    0.016,
                    0.016,
                    BRASS,
                    Mat::Metal,
                );
            }
            bx(
                b,
                [0.29, 0.05, 1.12],
                [0.006, 0.02, 0.04],
                BRASS,
                Mat::Metal,
            );
            prim(
                p,
                [0.31, 0.0, 1.96],
                [0.31, 0.0, 1.96],
                0.045,
                0.045,
                OAK,
                Mat::Wood,
            );
        }
        P_DRESSER => {
            // a kitchen dresser: a cupboard below with drawers, an open rack
            // above with plates standing and jugs hanging from hooks
            bx(b, [0.0, 0.0, 0.42], [0.26, 0.48, 0.42], OAK_DK, Mat::Wood);
            bx(b, [0.03, 0.0, 0.86], [0.27, 0.5, 0.025], OAK, Mat::Wood);
            for y in [-0.24f32, 0.24] {
                bx(b, [0.265, y, 0.7], [0.012, 0.22, 0.08], OAK, Mat::Wood);
                prim(
                    p,
                    [0.285, y, 0.7],
                    [0.285, y, 0.7],
                    0.018,
                    0.018,
                    BRASS,
                    Mat::Metal,
                );
                bx(b, [0.265, y, 0.32], [0.012, 0.21, 0.24], OAK_LT, Mat::Wood);
                prim(
                    p,
                    [0.285, y * 0.2, 0.36],
                    [0.285, y * 0.2, 0.42],
                    0.012,
                    0.012,
                    IRON,
                    Mat::Metal,
                );
            }
            bx(b, [-0.2, 0.0, 1.38], [0.02, 0.46, 0.5], OAK_DK, Mat::Wood);
            for y in [-0.47f32, 0.47] {
                bx(b, [-0.1, y, 1.38], [0.1, 0.025, 0.5], OAK, Mat::Wood);
            }
            bx(b, [-0.08, 0.0, 1.9], [0.14, 0.52, 0.035], OAK_DK, Mat::Wood);
            for (z, kind) in [(1.18f32, 0usize), (1.56, 1)] {
                bx(b, [-0.1, 0.0, z], [0.1, 0.45, 0.018], OAK, Mat::Wood);
                // plates leaning on the back, a rail holding them
                for k in 0..4 {
                    let y = -0.34 + k as f32 * 0.225;
                    let col = if (k + kind) % 3 == 0 {
                        0x5a7aa0ff
                    } else {
                        0xd8d4c8ff
                    };
                    prim(
                        p,
                        [-0.13, y, z + 0.13],
                        [-0.12, y, z + 0.13],
                        0.1,
                        0.1,
                        col,
                        Mat::Bone,
                    );
                    prim(
                        p,
                        [-0.09, y, z + 0.13],
                        [-0.09, y, z + 0.13],
                        0.05,
                        0.05,
                        shade_col(col, 0.85),
                        Mat::Bone,
                    );
                }
                bx(
                    b,
                    [-0.02, 0.0, z + 0.06],
                    [0.01, 0.45, 0.01],
                    OAK_DK,
                    Mat::Wood,
                );
            }
            // jugs on hooks under the top shelf, a crock on the counter
            for y in [-0.3f32, 0.0, 0.3] {
                prim(
                    p,
                    [-0.03, y, 1.86],
                    [-0.03, y, 1.8],
                    0.006,
                    0.006,
                    IRON,
                    Mat::Metal,
                );
                vessel(
                    p,
                    [-0.03, y, 1.66],
                    &[(0.0, 0.035), (0.06, 0.05), (0.12, 0.035)],
                    [0xe0d8c0ff, CLAY, 0x6a7a5aff][((y + 0.3) * 4.0) as usize % 3],
                    Mat::Bone,
                );
            }
            vessel(
                p,
                [0.08, 0.3, 0.88],
                &[(0.0, 0.07), (0.1, 0.08), (0.18, 0.06), (0.2, 0.07)],
                0x8a7a5aff,
                Mat::Bone,
            );
        }
        P_RACK => {
            // a weapon rack: a trestle with swords, a spear and an axe
            for y in [-0.42f32, 0.42] {
                bx(b, [0.0, y, 0.04], [0.22, 0.04, 0.04], OAK_DK, Mat::Wood);
                bx(b, [-0.1, y, 0.75], [0.035, 0.035, 0.75], OAK_DK, Mat::Wood);
            }
            bx(b, [-0.1, 0.0, 1.32], [0.04, 0.45, 0.035], OAK, Mat::Wood);
            bx(b, [0.05, 0.0, 0.22], [0.05, 0.45, 0.03], OAK, Mat::Wood);
            for (k, y) in [-0.28f32, -0.12, 0.04].iter().enumerate() {
                sword(
                    p,
                    b,
                    [-0.04, *y, 1.45 + k as f32 * 0.03],
                    0.95,
                    [0x3a2414ff, 0x5a1a1aff, 0x2a2a3aff][k],
                );
            }
            // a spear
            prim(
                p,
                [-0.04, 0.2, 0.08],
                [-0.04, 0.2, 1.75],
                0.02,
                0.018,
                OAK_LT,
                Mat::Wood,
            );
            prim(
                p,
                [-0.04, 0.2, 1.76],
                [-0.04, 0.2, 1.92],
                0.035,
                0.004,
                0xb8bcc4ff,
                Mat::Metal,
            );
            // an axe, head up
            prim(
                p,
                [-0.04, 0.34, 0.1],
                [-0.04, 0.34, 1.3],
                0.022,
                0.02,
                OAK,
                Mat::Wood,
            );
            bx(
                b,
                [-0.04, 0.4, 1.2],
                [0.012, 0.07, 0.09],
                0x9a9ea4ff,
                Mat::Metal,
            );
        }
        P_ARMOUR => {
            // a suit of mail on a stand: cross foot and post, a mail shirt
            // belted at the waist, plate shoulders, a helm with a nasal, and
            // a round shield propped against the stand's side
            let mail = 0x6e727aff;
            let plate = 0xa8aeb6ff;
            for yaw in [0.0f32, PI / 2.0] {
                b.push(BoxP {
                    c: [0.0, 0.0, 0.03],
                    h: [0.26, 0.04, 0.03],
                    yaw,
                    color: OAK_DK,
                    mat: Mat::Wood,
                });
            }
            prim(
                p,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.8],
                0.035,
                0.035,
                OAK_DK,
                Mat::Wood,
            );
            prim(
                p,
                [0.0, 0.0, 0.72],
                [0.0, 0.0, 0.97],
                0.21,
                0.16,
                mail,
                Mat::Cloth,
            );
            prim(
                p,
                [0.0, 0.0, 1.02],
                [0.0, 0.0, 1.28],
                0.15,
                0.19,
                mail,
                Mat::Cloth,
            );
            bx(
                b,
                [0.0, 0.0, 0.99],
                [0.17, 0.17, 0.03],
                0x4a3020ff,
                Mat::Matte,
            );
            bx(
                b,
                [0.17, 0.0, 0.99],
                [0.01, 0.035, 0.035],
                BRASS,
                Mat::Metal,
            );
            for y in [-0.2f32, 0.2] {
                prim(
                    p,
                    [0.0, y * 0.8, 1.34],
                    [0.0, y * 1.2, 1.28],
                    0.1,
                    0.08,
                    plate,
                    Mat::Metal,
                );
                prim(
                    p,
                    [0.0, y * 1.2, 1.2],
                    [0.03, y * 1.25, 0.98],
                    0.06,
                    0.05,
                    mail,
                    Mat::Cloth,
                );
            }
            prim(
                p,
                [0.0, 0.0, 1.36],
                [0.0, 0.0, 1.5],
                0.04,
                0.04,
                OAK_DK,
                Mat::Wood,
            );
            prim(
                p,
                [0.0, 0.0, 1.38],
                [0.0, 0.0, 1.42],
                0.1,
                0.08,
                mail,
                Mat::Cloth,
            );
            prim(
                p,
                [0.0, 0.0, 1.58],
                [0.0, 0.0, 1.64],
                0.12,
                0.09,
                plate,
                Mat::Metal,
            );
            prim(
                p,
                [0.0, 0.0, 1.68],
                [0.0, 0.0, 1.74],
                0.035,
                0.008,
                plate,
                Mat::Metal,
            );
            bx(
                b,
                [0.115, 0.0, 1.55],
                [0.012, 0.014, 0.075],
                0x5a5e64ff,
                Mat::Metal,
            );
            // the shield on the +y side, its face outwards
            bx(
                b,
                [0.02, 0.33, 0.34],
                [0.2, 0.025, 0.2],
                0x6a1a1aff,
                Mat::Wood,
            );
            bx(
                b,
                [0.02, 0.36, 0.34],
                [0.03, 0.01, 0.17],
                0xc8b070ff,
                Mat::Matte,
            );
            bx(
                b,
                [0.02, 0.36, 0.34],
                [0.17, 0.01, 0.03],
                0xc8b070ff,
                Mat::Matte,
            );
            prim(
                p,
                [0.02, 0.37, 0.34],
                [0.02, 0.37, 0.34],
                0.045,
                0.045,
                BRASS,
                Mat::Metal,
            );
        }
        P_CAULDRON => {
            // a black iron cauldron on three legs over embers, brewing green
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + 0.3;
                prim(
                    p,
                    [a.cos() * 0.2, a.sin() * 0.2, 0.25],
                    [a.cos() * 0.3, a.sin() * 0.3, 0.0],
                    0.025,
                    0.02,
                    IRON,
                    Mat::Metal,
                );
            }
            for k in 0..4 {
                let a = k as f32 / 4.0 * TAU + 0.5;
                prim(
                    p,
                    [a.cos() * 0.28, a.sin() * 0.28, 0.03],
                    [a.cos() * 0.06, a.sin() * 0.06, 0.04],
                    0.035,
                    0.03,
                    0x3a2414ff,
                    Mat::Wood,
                );
            }
            for k in 0..4 {
                let kf = k as f32;
                prim(
                    p,
                    [0.05 * (kf - 1.5), 0.05 * (kf * 2.1).sin(), 0.06],
                    [0.04 * (kf - 1.5), 0.0, 0.14],
                    0.04,
                    0.01,
                    [0xff6a10ff, 0xffb040ff][k % 2],
                    Mat::Glow,
                );
            }
            prim(
                p,
                [0.0, 0.0, 0.42],
                [0.0, 0.0, 0.48],
                0.3,
                0.28,
                0x232326ff,
                Mat::Metal,
            );
            disc(b, [0.0, 0.0, 0.66], 0.26, 0.025, 0x2e2e32ff, Mat::Metal);
            disc(b, [0.0, 0.0, 0.672], 0.22, 0.02, 0x58b838ff, Mat::Glow);
            for (x, y, r) in [
                (0.06f32, -0.05f32, 0.035f32),
                (-0.08, 0.06, 0.028),
                (0.0, 0.1, 0.02),
            ] {
                prim(p, [x, y, 0.7], [x, y, 0.7], r, r, 0x9ae070ff, Mat::Glow);
            }
            // handles
            for y in [-0.3f32, 0.3] {
                prim(
                    p,
                    [0.0, y, 0.62],
                    [0.0, y * 1.12, 0.56],
                    0.016,
                    0.016,
                    IRON,
                    Mat::Metal,
                );
            }
        }
        k if (P_PLANT..P_PLANT + 2).contains(&k) => {
            // a clay pot with a leafy shrub, or with a tall flowering herb
            vessel(
                p,
                [0.0, 0.0, 0.0],
                &[(0.0, 0.11), (0.18, 0.15), (0.3, 0.17), (0.33, 0.19)],
                CLAY,
                Mat::Bone,
            );
            disc(b, [0.0, 0.0, 0.33], 0.15, 0.01, 0x2a1a10ff, Mat::Matte);
            if k == P_PLANT {
                for i in 0..14 {
                    let a = hsh(i, 1) * TAU;
                    let r = 0.05 + hsh(i, 2) * 0.17;
                    let z = 0.42 + hsh(i, 3) * 0.36;
                    let col = [0x2e5a22ff, 0x3e6a2aff, 0x24481cff][i % 3];
                    prim(
                        p,
                        [a.cos() * r, a.sin() * r, z],
                        [a.cos() * r, a.sin() * r, z],
                        0.1,
                        0.1,
                        col,
                        Mat::Leaf,
                    );
                }
            } else {
                for i in 0..9 {
                    let a = i as f32 / 9.0 * TAU + hsh(i, 4);
                    let lean = 0.1 + hsh(i, 5) * 0.12;
                    let top = 0.7 + hsh(i, 6) * 0.45;
                    let tip = [a.cos() * lean, a.sin() * lean, top];
                    prim(p, [0.0, 0.0, 0.34], tip, 0.03, 0.012, 0x4a7a2aff, Mat::Leaf);
                    if i % 2 == 0 {
                        prim(
                            p,
                            tip,
                            tip,
                            0.035,
                            0.035,
                            [0xb060c0ff, 0xe8d050ff][i % 4 / 2],
                            Mat::Cloth,
                        );
                    }
                }
            }
        }
        P_CANDLES => {
            // an iron candle-stand: tripod foot, a tall shaft, three cups
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU;
                prim(
                    p,
                    [0.0, 0.0, 0.14],
                    [a.cos() * 0.2, a.sin() * 0.2, 0.0],
                    0.02,
                    0.016,
                    IRON,
                    Mat::Metal,
                );
            }
            prim(
                p,
                [0.0, 0.0, 0.1],
                [0.0, 0.0, 1.3],
                0.022,
                0.018,
                IRON,
                Mat::Metal,
            );
            for z in [0.45f32, 0.9] {
                prim(
                    p,
                    [0.0, 0.0, z],
                    [0.0, 0.0, z + 0.03],
                    0.04,
                    0.04,
                    IRON,
                    Mat::Metal,
                );
            }
            prim(
                p,
                [0.0, -0.24, 1.3],
                [0.0, 0.24, 1.3],
                0.018,
                0.018,
                IRON,
                Mat::Metal,
            );
            for y in [-0.24f32, 0.0, 0.24] {
                let z = if y == 0.0 { 1.4 } else { 1.3 };
                if y == 0.0 {
                    prim(
                        p,
                        [0.0, 0.0, 1.3],
                        [0.0, 0.0, 1.4],
                        0.018,
                        0.018,
                        IRON,
                        Mat::Metal,
                    );
                }
                prim(
                    p,
                    [0.0, y, z],
                    [0.0, y, z + 0.03],
                    0.045,
                    0.05,
                    IRON,
                    Mat::Metal,
                );
                candle(p, [0.0, y, z + 0.03], 0.16 + y.abs() * 0.2);
                // wax running down
                prim(
                    p,
                    [0.02, y, z + 0.1],
                    [0.025, y, z + 0.03],
                    0.008,
                    0.008,
                    CANDLE,
                    Mat::Matte,
                );
            }
        }
        P_JARS => {
            // storage pots: a tall amphora, a lidded crock, two jugs, a basket
            vessel(
                p,
                [-0.1, -0.14, 0.0],
                &[
                    (0.0, 0.05),
                    (0.12, 0.15),
                    (0.38, 0.16),
                    (0.58, 0.08),
                    (0.7, 0.045),
                    (0.74, 0.06),
                ],
                0xb06a3aff,
                Mat::Bone,
            );
            for y in [-0.24f32, -0.04] {
                prim(
                    p,
                    [-0.1, y, 0.68],
                    [-0.1, y, 0.52],
                    0.016,
                    0.016,
                    0xa0602eff,
                    Mat::Bone,
                );
            }
            vessel(
                p,
                [0.12, 0.16, 0.0],
                &[(0.0, 0.11), (0.2, 0.15), (0.32, 0.12), (0.34, 0.13)],
                0x7a6a50ff,
                Mat::Bone,
            );
            disc(b, [0.12, 0.16, 0.36], 0.12, 0.015, 0x5a4a38ff, Mat::Matte);
            prim(
                p,
                [0.12, 0.16, 0.37],
                [0.12, 0.16, 0.4],
                0.025,
                0.02,
                0x5a4a38ff,
                Mat::Matte,
            );
            vessel(
                p,
                [0.2, -0.16, 0.0],
                &[(0.0, 0.06), (0.1, 0.09), (0.2, 0.05), (0.26, 0.04)],
                0x4a5a6aff,
                Mat::Bone,
            );
            vessel(
                p,
                [-0.2, 0.28, 0.0],
                &[(0.0, 0.05), (0.07, 0.07), (0.15, 0.04), (0.18, 0.045)],
                CLAY,
                Mat::Bone,
            );
            // a wicker basket of apples
            vessel(
                p,
                [0.24, 0.02, 0.0],
                &[(0.0, 0.09), (0.12, 0.12)],
                0xa08048ff,
                Mat::Hair,
            );
            for i in 0..4 {
                let (x, y) = (0.24 + hsh(i, 7) * 0.1 - 0.05, 0.02 + hsh(i, 8) * 0.1 - 0.05);
                prim(
                    p,
                    [x, y, 0.15],
                    [x, y, 0.15],
                    0.035,
                    0.035,
                    [0x9a2a1aff, 0x7a8a2aff][i % 2],
                    Mat::Matte,
                );
            }
        }
        P_SPINWHEEL => {
            // a spinning wheel: a slanted bench on three legs, the great
            // wheel with its spokes, the flyer, a distaff of raw wool
            b.push(BoxP {
                c: [0.0, 0.0, 0.42],
                h: [0.05, 0.42, 0.04],
                yaw: 0.0,
                color: OAK,
                mat: Mat::Wood,
            });
            for (x, y) in [(0.0f32, -0.36f32), (0.12, 0.36), (-0.12, 0.36)] {
                prim(
                    p,
                    [0.0, y * 0.9, 0.4],
                    [x, y, 0.0],
                    0.022,
                    0.02,
                    OAK_DK,
                    Mat::Wood,
                );
            }
            let hub = [0.0, -0.12, 0.78];
            for dx in [-0.03f32, 0.03] {
                prim(
                    p,
                    [dx, -0.12, 0.44],
                    [dx, -0.12, 0.8],
                    0.018,
                    0.016,
                    OAK_DK,
                    Mat::Wood,
                );
            }
            ring(
                p,
                hub,
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                0.3,
                0.02,
                24,
                OAK,
                Mat::Wood,
            );
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                prim(
                    p,
                    hub,
                    [0.0, -0.12 + a.cos() * 0.29, 0.78 + a.sin() * 0.29],
                    0.008,
                    0.008,
                    OAK_DK,
                    Mat::Wood,
                );
            }
            prim(
                p,
                [-0.04, -0.12, 0.78],
                [0.04, -0.12, 0.78],
                0.035,
                0.035,
                OAK_DK,
                Mat::Wood,
            );
            // the flyer and bobbin at the far end of the bench
            prim(
                p,
                [0.0, 0.3, 0.46],
                [0.0, 0.3, 0.62],
                0.018,
                0.018,
                OAK_DK,
                Mat::Wood,
            );
            prim(
                p,
                [0.0, 0.22, 0.62],
                [0.0, 0.42, 0.62],
                0.035,
                0.035,
                0xd8ccb0ff,
                Mat::Hair,
            );
            // drive band
            prim(
                p,
                [0.0, 0.18, 0.62],
                [0.0, 0.32, 0.62],
                0.004,
                0.004,
                0xe0d8c0ff,
                Mat::Matte,
            );
            // the distaff and its wool
            prim(
                p,
                [0.0, 0.36, 0.44],
                [0.0, 0.3, 1.05],
                0.014,
                0.012,
                OAK_LT,
                Mat::Wood,
            );
            prim(
                p,
                [0.0, 0.31, 0.92],
                [0.0, 0.3, 1.08],
                0.07,
                0.04,
                0xe8e0d0ff,
                Mat::Hair,
            );
            // the treadle
            bx(b, [0.16, 0.0, 0.04], [0.1, 0.14, 0.015], OAK, Mat::Wood);
        }
        P_DESK => {
            // a writing desk: legs, a top, a gallery of pigeonholes at the
            // back, an open book, ink and quill, a candle, a scroll
            bx(b, [0.0, 0.0, 0.74], [0.3, 0.44, 0.03], OAK, Mat::Wood);
            for (x, y) in [
                (0.25f32, 0.39f32),
                (-0.25, 0.39),
                (0.25, -0.39),
                (-0.25, -0.39),
            ] {
                prim(
                    p,
                    [x, y, 0.0],
                    [x, y, 0.72],
                    0.032,
                    0.028,
                    OAK_DK,
                    Mat::Wood,
                );
            }
            bx(b, [0.25, 0.0, 0.64], [0.02, 0.38, 0.07], OAK_DK, Mat::Wood);
            bx(b, [0.25, 0.0, 0.64], [0.025, 0.1, 0.05], OAK_LT, Mat::Wood);
            prim(
                p,
                [0.28, 0.0, 0.64],
                [0.28, 0.0, 0.64],
                0.014,
                0.014,
                BRASS,
                Mat::Metal,
            );
            // the gallery
            bx(b, [-0.24, 0.0, 0.98], [0.06, 0.44, 0.22], OAK_DK, Mat::Wood);
            for (y, z) in [
                (-0.28f32, 0.88f32),
                (0.0, 0.88),
                (0.28, 0.88),
                (-0.28, 1.08),
                (0.0, 1.08),
                (0.28, 1.08),
            ] {
                bx(
                    b,
                    [-0.18, y, z],
                    [0.012, 0.11, 0.07],
                    0x1a120cff,
                    Mat::Matte,
                );
                if hsh((y * 10.0 + z * 7.0) as usize + 3, 1) < 0.6 {
                    prim(
                        p,
                        [-0.2, y - 0.04, z - 0.04],
                        [-0.14, y + 0.03, z - 0.04],
                        0.022,
                        0.022,
                        0xd8c898ff,
                        Mat::Matte,
                    );
                }
            }
            bx(b, [-0.22, 0.0, 1.22], [0.08, 0.46, 0.025], OAK, Mat::Wood);
            // an open book
            for (y, tilt) in [(-0.09f32, -0.12f32), (0.09, 0.12)] {
                b.push(BoxP {
                    c: [0.08, y, 0.79],
                    h: [0.12, 0.085, 0.014],
                    yaw: tilt * 0.2,
                    color: 0xe8dcc0ff,
                    mat: Mat::Matte,
                });
            }
            bx(
                b,
                [0.08, 0.0, 0.775],
                [0.13, 0.19, 0.01],
                0x5a1a1aff,
                Mat::Cloth,
            );
            for k in 0..3 {
                bx(
                    b,
                    [0.08 - 0.06 + k as f32 * 0.05, -0.09, 0.806],
                    [0.006, 0.06, 0.002],
                    0x6a5a48ff,
                    Mat::Matte,
                );
            }
            // ink, quill, candle
            prim(
                p,
                [0.12, 0.3, 0.77],
                [0.12, 0.3, 0.82],
                0.035,
                0.028,
                0x14141cff,
                Mat::Metal,
            );
            prim(
                p,
                [0.12, 0.3, 0.83],
                [0.02, 0.36, 1.0],
                0.006,
                0.02,
                0xf0f0e8ff,
                Mat::Bone,
            );
            prim(
                p,
                [-0.02, -0.32, 0.77],
                [-0.02, -0.32, 0.8],
                0.04,
                0.03,
                BRASS,
                Mat::Metal,
            );
            candle(p, [-0.02, -0.32, 0.8], 0.11);
        }
        P_ORRERY => {
            // an armillary sphere: a turned stand, brass rings, the earth at its heart
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + 0.5;
                prim(
                    p,
                    [0.0, 0.0, 0.3],
                    [a.cos() * 0.24, a.sin() * 0.24, 0.0],
                    0.026,
                    0.022,
                    OAK_DK,
                    Mat::Wood,
                );
            }
            prim(
                p,
                [0.0, 0.0, 0.25],
                [0.0, 0.0, 0.72],
                0.035,
                0.03,
                OAK,
                Mat::Wood,
            );
            prim(
                p,
                [0.0, 0.0, 0.5],
                [0.0, 0.0, 0.53],
                0.05,
                0.05,
                OAK_DK,
                Mat::Wood,
            );
            let c = [0.0, 0.0, 1.0];
            // the meridian, upright
            ring(
                p,
                c,
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                0.29,
                0.018,
                28,
                BRASS,
                Mat::Metal,
            );
            // the horizon, level
            ring(
                p,
                c,
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                0.3,
                0.016,
                28,
                0xa87a38ff,
                Mat::Metal,
            );
            // the ecliptic, tilted
            let t = 0.41f32;
            ring(
                p,
                c,
                [t.cos(), 0.0, t.sin()],
                [0.0, 1.0, 0.0],
                0.25,
                0.014,
                26,
                BRASS,
                Mat::Metal,
            );
            ring(
                p,
                c,
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                0.22,
                0.012,
                22,
                0xa87a38ff,
                Mat::Metal,
            );
            prim(
                p,
                [0.0, 0.0, 0.71],
                [0.0, 0.0, 1.31],
                0.008,
                0.008,
                BRASS,
                Mat::Metal,
            );
            prim(p, c, c, 0.075, 0.075, 0x3a6a8aff, Mat::Bone);
            prim(
                p,
                [0.02, -0.02, 1.03],
                [0.03, 0.01, 1.01],
                0.035,
                0.03,
                0x5a7a3aff,
                Mat::Matte,
            );
            prim(
                p,
                [0.0, 0.0, 1.33],
                [0.0, 0.0, 1.33],
                0.025,
                0.025,
                BRASS,
                Mat::Metal,
            );
        }
        P_STOOL => {
            // a three-legged milking stool, a tankard left on it
            disc(b, [0.0, 0.0, 0.48], 0.17, 0.03, OAK, Mat::Wood);
            for k in 0..3 {
                let a = k as f32 / 3.0 * TAU + 0.2;
                prim(
                    p,
                    [a.cos() * 0.1, a.sin() * 0.1, 0.46],
                    [a.cos() * 0.2, a.sin() * 0.2, 0.0],
                    0.028,
                    0.022,
                    OAK_DK,
                    Mat::Wood,
                );
            }
            prim(
                p,
                [0.04, 0.0, 0.52],
                [0.04, 0.0, 0.62],
                0.045,
                0.045,
                PEWTER,
                Mat::Metal,
            );
            prim(
                p,
                [0.04, 0.06, 0.6],
                [0.04, 0.08, 0.55],
                0.012,
                0.012,
                PEWTER,
                Mat::Metal,
            );
        }
        P_HERBS => {
            // a drying rack: two posts and a pole hung with bundles of herbs,
            // a basket of cuttings at its foot
            for y in [-0.42f32, 0.42] {
                prim(
                    p,
                    [0.0, y, 0.0],
                    [0.0, y, 1.5],
                    0.03,
                    0.028,
                    OAK_DK,
                    Mat::Wood,
                );
                bx(b, [0.0, y, 0.03], [0.18, 0.04, 0.03], OAK_DK, Mat::Wood);
            }
            prim(
                p,
                [0.0, -0.46, 1.42],
                [0.0, 0.46, 1.42],
                0.025,
                0.025,
                OAK,
                Mat::Wood,
            );
            prim(
                p,
                [0.0, -0.42, 0.82],
                [0.0, 0.42, 0.82],
                0.018,
                0.018,
                OAK,
                Mat::Wood,
            );
            for (row, z) in [(0usize, 1.42f32), (1, 0.82)] {
                for k in 0..5 {
                    let y = -0.32 + k as f32 * 0.16;
                    let len = 0.2 + hsh(row * 7 + k, 1) * 0.15;
                    let col = [0x5a7a2aff, 0x7a6a2aff, 0x4a6a3aff, 0x8a5a3aff, 0x6a7a4aff]
                        [(k + row * 2) % 5];
                    prim(
                        p,
                        [0.0, y, z - 0.02],
                        [0.0, y, z - 0.06],
                        0.012,
                        0.012,
                        0xb09868ff,
                        Mat::Matte,
                    );
                    prim(
                        p,
                        [0.0, y, z - 0.06],
                        [0.01, y, z - 0.06 - len],
                        0.03,
                        0.06,
                        col,
                        Mat::Leaf,
                    );
                }
            }
            vessel(
                p,
                [0.18, 0.18, 0.0],
                &[(0.0, 0.08), (0.12, 0.11)],
                0xa08048ff,
                Mat::Hair,
            );
            for k in 0..4 {
                let a = k as f32 * 1.7;
                prim(
                    p,
                    [0.18, 0.18, 0.12],
                    [0.18 + a.cos() * 0.08, 0.18 + a.sin() * 0.08, 0.2],
                    0.025,
                    0.04,
                    0x4a7a2aff,
                    Mat::Leaf,
                );
            }
        }
        _ => return false,
    }
    true
}
