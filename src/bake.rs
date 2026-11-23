//! Sprite baker: builds humanoid figures from 3D shapes on a jointed skeleton,
//! poses them, and renders every animation frame in 8 directions into a sprite
//! atlas — the way Ultima VIII's art was rendered from 3D models.
//!
//! Rendering is "sphere splatting": every limb is a chain of overlapping
//! spheres rasterised with a depth buffer, lit from the upper left, rendered
//! at 2x and reduced, then given a dark silhouette outline.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// art pixels per tile unit (a floor tile is 64 px wide)
pub const K: f32 = 45.254_834;
pub const FW: usize = 64;
pub const FH: usize = 72;
pub const AX: f32 = 32.0;
pub const AY: f32 = 64.0;
pub const COLS: usize = 16;
/// directions actually rendered; the other three are mirrored
pub const RDIRS: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anim {
    Idle,
    Walk,
    Stance,
    Attack,
    Work,
    Cast,
    Hit,
    Die,
}
pub const ANIMS: [(Anim, usize); 8] = [
    (Anim::Idle, 2),
    (Anim::Walk, 8),
    (Anim::Stance, 2),
    (Anim::Attack, 5),
    (Anim::Work, 4),
    (Anim::Cast, 4),
    (Anim::Hit, 2),
    (Anim::Die, 5),
];
pub const FRAMES_PER_DIR: usize = 32;

/// (first frame, frame count) of an animation within one direction
pub fn anim_range(a: Anim) -> (usize, usize) {
    let mut off = 0;
    for &(b, n) in ANIMS.iter() {
        if b == a {
            return (off, n);
        }
        off += n;
    }
    (0, 1)
}

/// The tile-space direction table used by the game (N, NE, E, SE, S, SW, W, NW).
/// Rendered directions are SE, E, NE, N, NW; S, SW, W are mirrors of E, NE, N.
pub fn dir_frame(dir8: u8) -> (usize, bool) {
    match dir8 & 7 {
        3 => (0, false),
        2 => (1, false),
        1 => (2, false),
        0 => (3, false),
        7 => (4, false),
        4 => (1, true),
        5 => (2, true),
        _ => (3, true),
    }
}
const RENDER_DIRS: [(f32, f32); RDIRS] = [
    (1.0, 1.0),
    (1.0, 0.0),
    (1.0, -1.0),
    (0.0, -1.0),
    (-1.0, -1.0),
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Body {
    Human,
    Rabbit,
    Rat,
    Wolf,
    Bat,
    Slime,
    Dragon,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Weapon {
    None,
    Sword,
    Axe,
    Mace,
    Staff,
    Halberd,
    Club,
    Pick,
    Pole,
}

/// Everything that decides how a figure looks. Colours are 0xRRGGBBAA, 0 = absent.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Look {
    pub body_type: Body,
    pub scale: u16,
    pub skin: u32,
    pub body: u32,
    pub legs: u32,
    pub boots: u32,
    pub metal_body: bool,
    pub robe: bool,
    pub skirt: bool,
    pub hair: u32,
    pub long_hair: bool,
    pub helm: u32,
    pub hat: u32,
    pub weapon: Weapon,
    pub shield: u32,
    pub heads: u8,
    pub skeleton: bool,
    pub eyes: u32,
    pub bulk: u8,
    pub hunch: u8,
    pub zombie: bool,
    pub female: bool,
}

impl Look {
    pub fn human(skin: u32, body: u32, legs: u32) -> Look {
        Look {
            body_type: Body::Human,
            scale: 100,
            skin,
            body,
            legs,
            boots: 0x3a2a1aff,
            metal_body: false,
            robe: false,
            skirt: false,
            hair: 0,
            long_hair: false,
            helm: 0,
            hat: 0,
            weapon: Weapon::None,
            shield: 0,
            heads: 1,
            skeleton: false,
            eyes: 0,
            bulk: 0,
            hunch: 0,
            zombie: false,
            female: false,
        }
    }
    pub fn creature(body: Body, color: u32, scale: u16) -> Look {
        let mut l = Look::human(color, color, color);
        l.body_type = body;
        l.scale = scale;
        l
    }
    pub fn key(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }
    pub fn s(&self) -> f32 {
        self.scale as f32 / 100.0
    }
    /// extra frame room for wide creatures
    fn room(&self) -> (f32, f32) {
        match self.body_type {
            Body::Dragon => (1.55, 1.3),
            Body::Bat => (1.2, 1.0),
            _ => (1.0, 1.0),
        }
    }
    pub fn frame_size(&self) -> (usize, usize) {
        let s = self.s();
        let (mx, my) = self.room();
        (
            (FW as f32 * s * mx).ceil() as usize,
            (FH as f32 * s * my).ceil() as usize,
        )
    }
    pub fn anchor(&self) -> (f32, f32) {
        let (fw, fh) = self.frame_size();
        (fw as f32 / 2.0, fh as f32 - 8.0 * self.s())
    }
}

pub struct Atlas {
    pub w: usize,
    pub h: usize,
    pub fw: usize,
    pub fh: usize,
    pub rgba: Vec<u8>,
}

// ------------------------------------------------------------------ pose
#[derive(Clone, Copy, Default, Debug)]
struct Pose {
    lean: f32,
    twist: f32,
    fall: f32,
    sink: f32,
    /// 0 = right (weapon) side, 1 = left (shield) side
    sh: [f32; 2],
    roll: [f32; 2],
    el: [f32; 2],
    hip: [f32; 2],
    knee: [f32; 2],
    wrist: f32,
    head_tilt: f32,
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn mixf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn pose_for(a: Anim, f: usize, n: usize, look: &Look) -> Pose {
    let tau = std::f32::consts::TAU;
    let p = if n > 1 {
        f as f32 / (n - 1) as f32
    } else {
        0.0
    };
    let mut q = Pose {
        sh: [0.12, 0.12],
        roll: [0.14, 0.14],
        el: [0.3, 0.3],
        wrist: 1.1,
        ..Default::default()
    };
    let hunch = look.hunch as f32 / 100.0;
    q.lean = hunch;
    match a {
        Anim::Idle => {
            let b = (f as f32 / n as f32 * tau).sin();
            q.sink = 0.012 * b;
            q.sh = [0.1 + 0.03 * b, 0.1 + 0.03 * b];
        }
        Anim::Walk => {
            let ph = f as f32 / n as f32 * tau;
            let (s, c) = (ph.sin(), ph.cos());
            q.hip = [0.48 * s, -0.48 * s];
            q.knee = [0.12 + 0.8 * c.max(0.0), 0.12 + 0.8 * (-c).max(0.0)];
            q.sh = [-0.38 * s, 0.38 * s];
            q.el = [0.35 + 0.2 * s.max(0.0), 0.35 + 0.2 * (-s).max(0.0)];
            q.sink = 0.025 * (2.0 * ph).cos().abs();
            q.lean += 0.06;
            q.twist = 0.08 * s;
        }
        Anim::Stance => {
            let b = (f as f32 / n as f32 * tau).sin();
            q.sh = [0.95, 0.75];
            q.el = [1.05, 1.1];
            q.roll = [0.2, 0.25];
            q.wrist = 0.55;
            q.hip = [0.28, -0.18];
            q.knee = [0.35, 0.3];
            q.lean += 0.1;
            q.twist = -0.2;
            q.sink = 0.01 * b;
        }
        Anim::Attack => {
            // wind-up, strike, recover
            let (sh, el, tw, ln) = if p < 0.35 {
                let t = ease(p / 0.35);
                (
                    mixf(0.95, 2.7, t),
                    mixf(1.05, 1.35, t),
                    mixf(-0.2, -0.55, t),
                    0.05,
                )
            } else if p < 0.7 {
                let t = ease((p - 0.35) / 0.35);
                (
                    mixf(2.7, 0.55, t),
                    mixf(1.35, 0.15, t),
                    mixf(-0.55, 0.45, t),
                    mixf(0.05, 0.3, t),
                )
            } else {
                let t = ease((p - 0.7) / 0.3);
                (
                    mixf(0.55, 0.95, t),
                    mixf(0.15, 1.05, t),
                    mixf(0.45, -0.2, t),
                    mixf(0.3, 0.1, t),
                )
            };
            q.sh = [sh, 0.7];
            q.el = [el, 1.1];
            q.roll = [0.15, 0.3];
            q.twist = tw;
            q.lean += ln;
            q.wrist = 0.35;
            q.hip = [0.4 * ln / 0.3, -0.2];
            q.knee = [0.35, 0.25];
        }
        Anim::Work => {
            // two-handed overhead chop / pick swing
            let up = (p * std::f32::consts::PI).sin();
            let sh = mixf(0.7, 2.9, up);
            q.sh = [sh, sh];
            q.el = [mixf(0.2, 0.9, up), mixf(0.2, 0.9, up)];
            q.roll = [-0.05, -0.05];
            q.lean += mixf(0.45, 0.05, up);
            q.wrist = 0.25;
            q.hip = [0.25, -0.2];
            q.knee = [0.35, 0.3];
        }
        Anim::Cast => {
            let t = ease(p);
            q.sh = [mixf(2.6, 1.55, t), mixf(2.6, 1.55, t)];
            q.el = [mixf(0.5, 0.05, t), mixf(0.5, 0.05, t)];
            q.roll = [0.35, 0.35];
            q.lean += mixf(-0.12, 0.12, t);
            q.head_tilt = mixf(-0.3, 0.1, t);
            q.wrist = 1.3;
        }
        Anim::Hit => {
            let t = if f == 0 { 1.0 } else { 0.55 };
            q.lean += -0.32 * t;
            q.sh = [0.5 * t, 0.6 * t];
            q.roll = [0.7 * t, 0.7 * t];
            q.el = [0.6, 0.6];
            q.hip = [-0.25 * t, 0.15 * t];
            q.knee = [0.3 * t, 0.2];
            q.head_tilt = -0.4 * t;
        }
        Anim::Die => {
            let t = ease(p);
            let buckle = (t * 2.2).min(1.0);
            q.knee = [
                mixf(0.0, 1.4, buckle) * (1.0 - t * 0.85),
                mixf(0.0, 1.2, buckle) * (1.0 - t * 0.85),
            ];
            q.hip = [
                mixf(0.0, 0.9, buckle) * (1.0 - t * 0.8),
                mixf(0.0, 0.7, buckle) * (1.0 - t * 0.8),
            ];
            q.fall = -1.5 * t;
            q.sh = [mixf(0.2, 2.2, t), mixf(0.2, 2.0, t)];
            q.roll = [mixf(0.2, 0.9, t), mixf(0.2, 1.0, t)];
            q.el = [0.4, 0.5];
            q.head_tilt = -0.3 * t;
        }
    }
    if look.zombie && !matches!(a, Anim::Attack | Anim::Die | Anim::Work) {
        q.sh = [1.4, 1.35];
        q.el = [0.15, 0.2];
        q.roll = [0.05, 0.05];
    }
    q
}

// ------------------------------------------------------------------ geometry
type V3 = [f32; 3];
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn rotz(v: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]]
}

#[derive(Clone, Copy)]
enum Mat {
    Matte,
    Metal,
    Glow,
    Bone,
    Hair,
    Membrane,
}

/// a primitive in local model space (metres): a tapered chain of spheres
struct Prim {
    a: V3,
    b: V3,
    ra: f32,
    rb: f32,
    color: u32,
    mat: Mat,
}

/// lateral unit vector pointing to the figure's right, after torso twist
fn lat(twist: f32) -> V3 {
    rotz([0.0, 1.0, 0.0], twist)
}
/// limb direction: angle from straight down, swung forward, rolled outwards
fn limb_dir(angle: f32, roll: f32, side: f32, twist: f32) -> V3 {
    rotz(
        [
            angle.sin() * roll.cos(),
            side * roll.sin(),
            -angle.cos() * roll.cos(),
        ],
        twist,
    )
}

fn build(look: &Look, q: &Pose) -> Vec<Prim> {
    let mut v = vec![];
    let bulk = 1.0 + look.bulk as f32 / 100.0;
    let thin = if look.skeleton { 0.55 } else { 1.0 };
    let shoulder_w = if look.female { 0.17 } else { 0.19 } * bulk;
    let pel = [0.0, 0.0, 0.95 - q.sink];
    let sd = [q.lean.sin(), 0.0, q.lean.cos()];
    let chest = add(pel, mul(sd, 0.47));
    let la = lat(q.twist);
    let head_c = add(chest, add(mul(sd, 0.25), [q.head_tilt * 0.06, 0.0, 0.0]));
    let mut p = |a: V3, b: V3, ra: f32, rb: f32, color: u32, mat: Mat| {
        v.push(Prim {
            a,
            b,
            ra,
            rb,
            color,
            mat,
        })
    };
    let skin_mat = if look.skeleton { Mat::Bone } else { Mat::Matte };
    let body_mat = if look.metal_body {
        Mat::Metal
    } else if look.skeleton {
        Mat::Bone
    } else {
        Mat::Matte
    };

    // legs
    for (i, side) in [(0usize, 1.0f32), (1, -1.0)] {
        let hip = add(pel, [0.0, side * 0.1 * bulk, -0.06]);
        let d1 = limb_dir(q.hip[i], 0.03, side, 0.0);
        let knee = add(hip, mul(d1, 0.44));
        let d2 = limb_dir(q.hip[i] - q.knee[i], 0.03, side, 0.0);
        let ankle = add(knee, mul(d2, 0.44));
        let leg_col = if look.robe { look.body } else { look.legs };
        p(
            hip,
            knee,
            0.08 * bulk * thin,
            0.06 * bulk * thin,
            leg_col,
            if look.skeleton { Mat::Bone } else { Mat::Matte },
        );
        p(
            knee,
            ankle,
            0.058 * bulk * thin,
            0.045 * bulk * thin,
            leg_col,
            if look.skeleton { Mat::Bone } else { Mat::Matte },
        );
        let toe = add(
            ankle,
            [0.17 * (q.hip[i] - q.knee[i]).cos().max(0.3), 0.0, -0.03],
        );
        p(
            ankle,
            toe,
            0.05 * bulk,
            0.04 * bulk,
            if look.skeleton { look.skin } else { look.boots },
            if look.skeleton { Mat::Bone } else { Mat::Matte },
        );
    }
    // pelvis and torso
    let hipl = add(pel, mul(la, -0.1 * bulk));
    let hipr = add(pel, mul(la, 0.1 * bulk));
    p(
        hipl,
        hipr,
        0.12 * bulk * thin.max(0.8),
        0.12 * bulk * thin.max(0.8),
        if look.robe { look.body } else { look.legs },
        body_mat,
    );
    let belly = add(pel, mul(sd, 0.06));
    if look.skeleton {
        // spine and a rib cage
        p(belly, chest, 0.04, 0.05, look.body, Mat::Bone);
        for k in 0..4 {
            let c = add(pel, mul(sd, 0.22 + k as f32 * 0.08));
            p(
                add(c, mul(la, -0.1)),
                add(c, mul(la, 0.1)),
                0.035,
                0.035,
                look.body,
                Mat::Bone,
            );
        }
    } else {
        p(belly, chest, 0.15 * bulk, 0.17 * bulk, look.body, body_mat);
    }
    let shl = add(chest, mul(la, -shoulder_w));
    let shr = add(chest, mul(la, shoulder_w));
    p(
        shl,
        shr,
        0.08 * bulk * thin.max(0.6),
        0.08 * bulk * thin.max(0.6),
        look.body,
        body_mat,
    );
    if look.robe {
        let hem = [pel[0] - 0.02, pel[1], 0.06];
        p(
            add(pel, [0.0, 0.0, 0.05]),
            hem,
            0.17 * bulk,
            0.3 * bulk,
            look.body,
            Mat::Matte,
        );
    } else if look.skirt {
        p(
            add(pel, [0.0, 0.0, 0.02]),
            add(pel, [0.0, 0.0, -0.24]),
            0.16 * bulk,
            0.2 * bulk,
            look.body,
            body_mat,
        );
    }
    // belt
    if !look.skeleton && !look.robe {
        p(
            add(pel, mul(la, -0.13 * bulk)),
            add(pel, mul(la, 0.13 * bulk)),
            0.045,
            0.045,
            0x2a1c10ff,
            Mat::Matte,
        );
    }
    // arms (0 = right / weapon)
    let mut hands = [[0.0f32; 3]; 2];
    let mut fdir = [[0.0f32; 3]; 2];
    for (i, side) in [(0usize, 1.0f32), (1, -1.0)] {
        let s = if i == 0 { shr } else { shl };
        let d1 = limb_dir(q.sh[i], q.roll[i], side, q.twist);
        let el = add(s, mul(d1, 0.29));
        let d2 = limb_dir(q.sh[i] + q.el[i], q.roll[i], side, q.twist);
        let wr = add(el, mul(d2, 0.27));
        let sleeve = if look.robe || look.metal_body {
            look.body
        } else {
            look.skin
        };
        p(
            s,
            el,
            0.06 * bulk * thin,
            0.05 * bulk * thin,
            sleeve,
            if look.metal_body {
                Mat::Metal
            } else {
                skin_mat
            },
        );
        p(
            el,
            wr,
            0.05 * bulk * thin,
            0.04 * bulk * thin,
            look.skin,
            skin_mat,
        );
        p(wr, wr, 0.048 * bulk, 0.048 * bulk, look.skin, skin_mat);
        hands[i] = wr;
        fdir[i] = d2;
    }
    // shield on the left forearm
    if look.shield != 0 {
        let c = add(hands[1], mul(fdir[1], -0.1));
        let out = mul(la, -1.0);
        let up = [0.0, 0.0, 1.0];
        let fw = rotz([1.0, 0.0, 0.0], q.twist);
        for k in 0..13 {
            let (du, dv) = if k == 0 {
                (0.0f32, 0.0f32)
            } else {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                (
                    a.cos() * if k % 2 == 0 { 0.13 } else { 0.07 },
                    a.sin() * if k % 2 == 0 { 0.15 } else { 0.08 },
                )
            };
            let pc = add(add(add(c, mul(out, 0.07)), mul(fw, du)), mul(up, dv));
            p(pc, pc, 0.07, 0.07, look.shield, Mat::Metal);
        }
        let boss = add(c, mul(out, 0.12));
        p(boss, boss, 0.035, 0.035, 0xd8c070ff, Mat::Metal);
    }
    // weapon in the right hand
    let aw = q.sh[0] + q.el[0] + q.wrist;
    let wd = limb_dir(aw, q.roll[0] * 0.5, 1.0, q.twist);
    let h = hands[0];
    let grip = add(h, mul(wd, -0.07));
    let steel = 0xc8ccd2ff;
    let wood = 0x5a3c22ff;
    match look.weapon {
        Weapon::None => {}
        Weapon::Sword => {
            let guard = add(h, mul(wd, 0.06));
            p(grip, guard, 0.025, 0.025, 0x3a2a1aff, Mat::Matte);
            let cl = mul(rotz([0.0, 1.0, 0.0], q.twist), 0.07);
            p(
                add(guard, cl),
                add(guard, mul(cl, -1.0)),
                0.022,
                0.022,
                0xb89a50ff,
                Mat::Metal,
            );
            p(
                guard,
                add(guard, mul(wd, 0.72)),
                0.03,
                0.012,
                steel,
                Mat::Metal,
            );
        }
        Weapon::Axe | Weapon::Pick => {
            let top = add(h, mul(wd, 0.55));
            p(grip, top, 0.025, 0.025, wood, Mat::Matte);
            let back = rotz([-1.0, 0.0, 0.0], q.twist);
            let fwd = rotz([1.0, 0.0, 0.0], q.twist);
            let side = if look.weapon == Weapon::Pick {
                [mul(fwd, 0.2), mul(back, 0.2)]
            } else {
                [mul(fwd, 0.13), mul(fwd, 0.0)]
            };
            p(
                add(top, side[0]),
                add(top, side[1]),
                0.05,
                0.03,
                steel,
                Mat::Metal,
            );
        }
        Weapon::Mace => {
            let top = add(h, mul(wd, 0.45));
            p(grip, top, 0.025, 0.025, wood, Mat::Matte);
            p(top, top, 0.08, 0.08, 0x8a8e94ff, Mat::Metal);
        }
        Weapon::Club => {
            p(
                grip,
                add(h, mul(wd, 0.6)),
                0.04,
                0.09,
                0x4a3018ff,
                Mat::Matte,
            );
        }
        Weapon::Staff => {
            let lo = add(h, mul(wd, -0.6));
            let top = add(h, mul(wd, 0.75));
            p(lo, top, 0.025, 0.025, wood, Mat::Matte);
            p(top, top, 0.055, 0.055, 0x80c0ffff, Mat::Glow);
        }
        Weapon::Pole => {
            p(
                add(h, mul(wd, -0.12)),
                add(h, mul(wd, 1.25)),
                0.018,
                0.008,
                wood,
                Mat::Matte,
            );
        }
        Weapon::Halberd => {
            let lo = add(h, mul(wd, -0.7));
            let top = add(h, mul(wd, 0.95));
            p(lo, top, 0.022, 0.022, wood, Mat::Matte);
            let fwd = rotz([1.0, 0.0, 0.0], q.twist);
            p(
                add(top, mul(wd, -0.2)),
                add(add(top, mul(wd, -0.08)), mul(fwd, 0.14)),
                0.06,
                0.03,
                steel,
                Mat::Metal,
            );
            p(
                top,
                add(top, mul(wd, 0.12)),
                0.025,
                0.008,
                steel,
                Mat::Metal,
            );
        }
    }
    // neck and head(s)
    let neck_top = add(chest, mul(sd, 0.12));
    p(
        chest,
        neck_top,
        0.05 * thin.max(0.8),
        0.05 * thin.max(0.8),
        look.skin,
        skin_mat,
    );
    let heads: &[f32] = if look.heads == 2 {
        &[-0.12, 0.12]
    } else {
        &[0.0]
    };
    let fwd = rotz([1.0, 0.0, 0.0], q.twist * 0.5);
    for &hy in heads {
        let hc = add(head_c, mul(la, hy * bulk));
        p(
            add(hc, [0.0, 0.0, -0.015]),
            add(hc, [0.0, 0.0, 0.02]),
            0.1,
            0.1,
            look.skin,
            skin_mat,
        );
        if look.hair != 0 {
            let back = add(hc, add(mul(fwd, -0.03), [0.0, 0.0, 0.035]));
            p(back, back, 0.105, 0.105, look.hair, Mat::Hair);
            if look.long_hair {
                p(
                    add(hc, mul(fwd, -0.05)),
                    add(add(hc, mul(fwd, -0.07)), [0.0, 0.0, -0.22]),
                    0.09,
                    0.07,
                    look.hair,
                    Mat::Hair,
                );
            }
        }
        if look.eyes != 0 {
            for es in [-0.04f32, 0.04] {
                let e = add(add(hc, mul(fwd, 0.085)), add(mul(la, es), [0.0, 0.0, 0.01]));
                p(e, e, 0.022, 0.022, look.eyes, Mat::Glow);
            }
        }
        if look.helm != 0 {
            let c = add(hc, [0.0, 0.0, 0.03]);
            p(c, c, 0.115, 0.115, look.helm, Mat::Metal);
            p(
                add(c, mul(fwd, 0.1)),
                add(add(c, mul(fwd, 0.1)), [0.0, 0.0, -0.08]),
                0.018,
                0.018,
                look.helm,
                Mat::Metal,
            );
        }
        if look.hat != 0 {
            let base = add(hc, [0.0, 0.0, 0.07]);
            p(base, base, 0.06, 0.06, look.hat, Mat::Matte);
            p(
                add(base, [0.0, 0.0, 0.02]),
                add(base, [-0.04, 0.0, 0.3]),
                0.12,
                0.01,
                look.hat,
                Mat::Matte,
            );
            p(base, base, 0.0, 0.0, look.hat, Mat::Matte);
            // brim
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                let e = add(base, [a.cos() * 0.13, a.sin() * 0.13, -0.01]);
                p(e, e, 0.045, 0.045, look.hat, Mat::Matte);
            }
        }
    }
    v
}

// ------------------------------------------------------------------ creatures
fn figure_for(look: &Look, a: Anim, f: usize, n: usize) -> Figure {
    match look.body_type {
        Body::Human => {
            let q = pose_for(a, f, n, look);
            Figure {
                prims: build(look, &q),
                pitch: q.fall,
                roll: 0.0,
                lift: 0.0,
            }
        }
        Body::Slime => slime(look, a, f, n),
        Body::Bat => bat(look, a, f, n),
        _ => quad(look, a, f, n),
    }
}

fn prim(v: &mut Vec<Prim>, a: V3, b: V3, ra: f32, rb: f32, color: u32, mat: Mat) {
    v.push(Prim {
        a,
        b,
        ra,
        rb,
        color,
        mat,
    });
}
fn shade_col(c: u32, k: f32) -> u32 {
    let ch = |sh: u32| ((((c >> sh) & 255) as f32 * k).clamp(0.0, 255.0) as u32) << sh;
    ch(24) | ch(16) | ch(8) | (c & 255)
}

/// shared timing: (gait phase, lunge 0..1, recoil 0..1, death 0..1, crouch 0..1)
fn creature_phase(a: Anim, f: usize, n: usize) -> (f32, f32, f32, f32, f32) {
    let tau = std::f32::consts::TAU;
    let p = if n > 1 {
        f as f32 / (n - 1) as f32
    } else {
        0.0
    };
    match a {
        Anim::Walk => (f as f32 / n as f32 * tau, 0.0, 0.0, 0.0, 0.0),
        Anim::Stance => (0.0, 0.0, 0.0, 0.0, 1.0),
        Anim::Attack => (0.0, (p * std::f32::consts::PI).sin(), 0.0, 0.0, 0.6),
        Anim::Cast => (0.0, 0.0, 0.0, 0.0, -ease(p)),
        Anim::Hit => (0.0, 0.0, if f == 0 { 1.0 } else { 0.5 }, 0.0, 0.0),
        Anim::Die => (0.0, 0.0, 0.0, ease(p), 0.0),
        _ => (f as f32 * 0.6, 0.0, 0.0, 0.0, 0.0),
    }
}

/// a membrane wing: a fan of finger bones filled with small spheres
fn wing(v: &mut Vec<Prim>, root: V3, side: f32, span: f32, flap: f32, color: u32) {
    let (fs, fc) = flap.sin_cos();
    let elbow = add(
        root,
        [
            -0.12 * span,
            side * 0.5 * span * fc,
            0.5 * span * fs + 0.08 * span,
        ],
    );
    let tips: Vec<V3> = (0..4)
        .map(|k| {
            let kf = k as f32;
            add(
                elbow,
                [
                    -0.15 * span - 0.28 * span * kf,
                    side * 0.55 * span * fc * (1.0 - kf * 0.12),
                    0.55 * span * fs - 0.12 * span * kf,
                ],
            )
        })
        .collect();
    let bone = shade_col(color, 0.6);
    let r = 0.02 * span.max(0.4);
    prim(v, root, elbow, r * 1.4, r, bone, Mat::Matte);
    for t in tips.iter() {
        prim(v, elbow, *t, r, r * 0.6, bone, Mat::Matte);
    }
    // membrane between root, elbow and the finger tips
    let mut tri = |a: V3, b: V3, c: V3| {
        let steps = 9;
        for i in 0..=steps {
            for j in 0..=(steps - i) {
                let (u, w) = (i as f32 / steps as f32, j as f32 / steps as f32);
                let pt = add(add(mul(a, 1.0 - u - w), mul(b, u)), mul(c, w));
                prim(v, pt, pt, 0.05 * span, 0.05 * span, color, Mat::Membrane);
            }
        }
    };
    for k in 0..3 {
        tri(elbow, tips[k], tips[k + 1]);
    }
    tri(root, elbow, tips[3]);
}

fn quad(look: &Look, a: Anim, f: usize, n: usize) -> Figure {
    let mut v = vec![];
    let col = look.skin;
    let dark = shade_col(col, 0.7);
    let (ph, lunge, recoil, die, crouch) = creature_phase(a, f, n);
    let kind = look.body_type;
    // body length, shoulder height, body radii, leg radius, neck length, head radius
    let (len, hgt, rb, rf, lr, neck, hr) = match kind {
        Body::Rabbit => (0.22, 0.12, 0.1, 0.08, 0.025, 0.06, 0.07),
        Body::Rat => (0.3, 0.1, 0.08, 0.07, 0.022, 0.05, 0.06),
        Body::Dragon => (1.3, 0.75, 0.38, 0.33, 0.09, 0.75, 0.2),
        _ => (0.62, 0.42, 0.15, 0.16, 0.045, 0.18, 0.11),
    };
    let crouch_k = 1.0 - 0.12 * crouch.max(0.0);
    let bob = if a == Anim::Walk {
        0.04 * hgt * (2.0 * ph).cos()
    } else {
        0.0
    };
    let dx = lunge * 0.3 * len - recoil * 0.15 * len;
    let hip = [-len / 2.0 + dx, 0.0, hgt * crouch_k + bob];
    let rear_up = (-crouch).max(0.0);
    let sho = [
        len / 2.0 + dx,
        0.0,
        hgt * crouch_k * 1.05 + bob + rear_up * 0.35 * len,
    ];
    prim(&mut v, hip, sho, rb, rf, col, Mat::Matte);
    if kind == Body::Dragon {
        // pale belly
        prim(
            &mut v,
            add(hip, [0.1, 0.0, -0.12]),
            add(sho, [-0.1, 0.0, -0.12]),
            rb * 0.75,
            rf * 0.7,
            0xd89a5aff,
            Mat::Matte,
        );
    }
    // legs: diagonal pairs (rabbits hop with both hind legs)
    let width = rb * 0.75;
    for (front, side, off) in [
        (true, 1.0f32, 0.0f32),
        (true, -1.0, 1.0),
        (false, 1.0, 1.0),
        (false, -1.0, 0.0),
    ] {
        let base = if front { sho } else { hip };
        let o = if kind == Body::Rabbit {
            if front {
                0.0
            } else {
                0.5
            }
        } else {
            off
        };
        let phase = ph + o * std::f32::consts::PI;
        let swing = if a == Anim::Walk {
            0.5 * phase.sin()
        } else {
            0.0
        };
        let bend = if a == Anim::Walk {
            0.6 * phase.cos().max(0.0)
        } else {
            0.25 + 0.4 * crouch.max(0.0)
        };
        let top = add(base, [0.0, side * width, -rb * 0.4]);
        let l1 = (base[2] - rb * 0.4) * 0.55;
        let l2 = (base[2] - rb * 0.4) * 0.52;
        let d1 = [swing.sin(), 0.0, -swing.cos()];
        let knee = add(top, mul(d1, l1));
        let b2 = if front { swing - bend } else { swing + bend };
        let foot = add(knee, mul([b2.sin(), 0.0, -b2.cos()], l2));
        prim(&mut v, top, knee, lr * 1.3, lr, col, Mat::Matte);
        prim(&mut v, knee, foot, lr, lr * 0.85, dark, Mat::Matte);
        prim(
            &mut v,
            foot,
            add(foot, [lr * 1.6, 0.0, 0.0]),
            lr * 0.9,
            lr * 0.7,
            dark,
            Mat::Matte,
        );
    }
    // neck and head
    let np = match a {
        Anim::Stance => 0.15,
        Anim::Attack => 0.5 - 0.6 * lunge,
        Anim::Cast => 0.9,
        Anim::Hit => 0.9,
        Anim::Die => 0.2 - 0.6 * die,
        _ => 0.55 + 0.05 * (ph * 0.5).sin(),
    };
    let neck_top = add(
        sho,
        [neck * np.cos() + lunge * 0.1 * len, 0.0, neck * np.sin()],
    );
    prim(&mut v, sho, neck_top, rf * 0.6, hr * 0.8, col, Mat::Matte);
    let head = add(neck_top, [hr * 0.5, 0.0, 0.0]);
    prim(&mut v, head, head, hr, hr, col, Mat::Matte);
    let (snout, snr) = match kind {
        Body::Rabbit => (0.05, 0.045),
        Body::Rat => (0.09, 0.035),
        Body::Dragon => (0.32, 0.12),
        _ => (0.14, 0.05),
    };
    let jaw_open = if kind == Body::Dragon && (a == Anim::Attack || a == Anim::Cast) {
        0.35
    } else {
        0.0
    };
    let tip = add(head, [hr * 0.6 + snout, 0.0, -0.02]);
    prim(
        &mut v,
        add(head, [hr * 0.4, 0.0, 0.0]),
        tip,
        snr * 1.2,
        snr * 0.8,
        col,
        Mat::Matte,
    );
    if jaw_open > 0.0 {
        prim(
            &mut v,
            add(head, [hr * 0.3, 0.0, -hr * 0.4]),
            add(tip, [-0.05, 0.0, -jaw_open * snout]),
            snr,
            snr * 0.6,
            dark,
            Mat::Matte,
        );
        prim(
            &mut v,
            add(tip, [0.02, 0.0, -0.05]),
            add(tip, [0.02, 0.0, -0.05]),
            snr * 0.7,
            snr * 0.7,
            0xffa030ff,
            Mat::Glow,
        );
    }
    // eyes
    for es in [-1.0f32, 1.0] {
        let e = add(head, [hr * 0.75, es * hr * 0.5, hr * 0.3]);
        let ec = if kind == Body::Dragon {
            0xffd040ff
        } else {
            0x100808ff
        };
        prim(
            &mut v,
            e,
            e,
            hr * 0.18,
            hr * 0.18,
            ec,
            if kind == Body::Dragon {
                Mat::Glow
            } else {
                Mat::Hair
            },
        );
    }
    // ears / horns
    for es in [-1.0f32, 1.0] {
        let base = add(head, [-hr * 0.2, es * hr * 0.5, hr * 0.6]);
        match kind {
            Body::Rabbit => prim(
                &mut v,
                base,
                add(base, [-0.04, es * 0.02, 0.13]),
                0.022,
                0.016,
                col,
                Mat::Matte,
            ),
            Body::Rat => prim(
                &mut v,
                base,
                base,
                0.025,
                0.025,
                shade_col(col, 1.3),
                Mat::Matte,
            ),
            Body::Dragon => prim(
                &mut v,
                base,
                add(base, [-0.25, es * 0.08, 0.18]),
                0.05,
                0.01,
                0xe0d8c0ff,
                Mat::Bone,
            ),
            _ => prim(
                &mut v,
                base,
                add(base, [-0.01, 0.0, 0.08]),
                0.035,
                0.008,
                dark,
                Mat::Matte,
            ),
        }
    }
    // tail
    let wag = (ph * 1.3).sin()
        * if a == Anim::Walk || a == Anim::Idle {
            0.12
        } else {
            0.0
        };
    match kind {
        Body::Rabbit => {
            let t = add(hip, [-0.07, 0.0, 0.03]);
            prim(&mut v, t, t, 0.04, 0.04, 0xe8e0d0ff, Mat::Matte);
        }
        Body::Rat => {
            let mut p0 = add(hip, [-0.05, 0.0, 0.0]);
            for k in 0..6 {
                let kf = k as f32;
                let p1 = add(
                    p0,
                    [-0.06, wag * (kf * 0.7).sin() * 0.3, -0.012 + 0.004 * kf],
                );
                prim(&mut v, p0, p1, 0.014, 0.01, 0xb08a80ff, Mat::Matte);
                p0 = p1;
            }
        }
        Body::Dragon => {
            let mut p0 = add(hip, [-0.1, 0.0, 0.0]);
            let mut r = rb * 0.8;
            for k in 0..8 {
                let kf = k as f32;
                let p1 = add(
                    p0,
                    [
                        -0.22,
                        (wag + 0.1) * (kf * 0.6).sin() * 0.5,
                        -0.06 + 0.01 * kf,
                    ],
                );
                prim(&mut v, p0, p1, r, r * 0.8, col, Mat::Matte);
                r *= 0.8;
                p0 = p1;
            }
        }
        _ => {
            let t0 = add(hip, [-0.05, 0.0, 0.03]);
            prim(
                &mut v,
                t0,
                add(t0, [-0.3, wag, -0.18]),
                0.05,
                0.035,
                dark,
                Mat::Matte,
            );
        }
    }
    if kind == Body::Dragon {
        let flap = match a {
            Anim::Walk => 0.9 + 0.35 * (ph * 2.0).sin(),
            Anim::Cast | Anim::Attack => 1.2,
            Anim::Die => 0.2 - 0.5 * die,
            _ => 1.05 + 0.08 * ph.sin(),
        };
        for side in [-1.0f32, 1.0] {
            let root = add(sho, [-0.25, side * rf * 0.6, rf * 0.6]);
            wing(&mut v, root, side, 0.85, flap, shade_col(col, 0.75));
        }
    }
    Figure {
        prims: v,
        pitch: 0.0,
        roll: 1.45 * die,
        lift: 0.0,
    }
}

fn bat(look: &Look, a: Anim, f: usize, n: usize) -> Figure {
    let mut v = vec![];
    let col = look.skin;
    let (ph, lunge, recoil, die, _) = creature_phase(a, f, n);
    let tau = std::f32::consts::TAU;
    let flap = if a == Anim::Die {
        0.2 - 0.9 * die
    } else {
        0.6 * ((f as f32 / n.max(2) as f32) * tau * 2.0).sin() + 0.2 + 0.0 * ph
    };
    let c = [lunge * 0.12 - recoil * 0.06, 0.0, 0.0];
    let body = add(c, [0.0, 0.0, 0.12]);
    prim(
        &mut v,
        body,
        add(body, [0.03, 0.0, 0.09]),
        0.1,
        0.085,
        col,
        Mat::Matte,
    );
    let head = add(body, [0.07, 0.0, 0.17]);
    prim(&mut v, head, head, 0.07, 0.07, col, Mat::Matte);
    for es in [-1.0f32, 1.0] {
        let e = add(head, [0.05, es * 0.03, 0.02]);
        prim(&mut v, e, e, 0.016, 0.016, 0xffd040ff, Mat::Glow);
        let ear = add(head, [-0.01, es * 0.035, 0.05]);
        prim(
            &mut v,
            ear,
            add(ear, [-0.02, es * 0.02, 0.07]),
            0.025,
            0.006,
            col,
            Mat::Matte,
        );
    }
    for side in [-1.0f32, 1.0] {
        wing(
            &mut v,
            add(body, [0.0, side * 0.06, 0.08]),
            side,
            0.55,
            flap,
            shade_col(col, 0.8),
        );
    }
    // dangling feet
    for side in [-1.0f32, 1.0] {
        let hip = add(body, [-0.04, side * 0.04, -0.05]);
        prim(
            &mut v,
            hip,
            add(hip, [-0.02, 0.0, -0.08]),
            0.02,
            0.012,
            shade_col(col, 0.6),
            Mat::Matte,
        );
    }
    let lift = if a == Anim::Die {
        0.75 * (1.0 - die)
    } else {
        0.75 + 0.06 * (f as f32 * 1.7).sin()
    };
    Figure {
        prims: v,
        pitch: 0.0,
        roll: 1.2 * die,
        lift,
    }
}

fn slime(look: &Look, a: Anim, f: usize, n: usize) -> Figure {
    let mut v = vec![];
    let col = look.skin;
    let (_, lunge, recoil, die, _) = creature_phase(a, f, n);
    let tau = std::f32::consts::TAU;
    let wob = (f as f32 / n.max(1) as f32 * tau).sin();
    let squash = match a {
        Anim::Walk | Anim::Idle | Anim::Stance => 0.15 * wob,
        Anim::Attack => -0.3 * lunge,
        Anim::Hit => 0.3 * recoil,
        Anim::Die => 0.7 * die,
        _ => 0.0,
    };
    let (hk, wk) = (1.0 - squash, 1.0 + squash * 0.7);
    let fwd = lunge * 0.12;
    prim(
        &mut v,
        [fwd, 0.0, 0.2 * hk],
        [fwd, 0.0, 0.2 * hk],
        0.22 * wk.min(1.4),
        0.22 * wk.min(1.4),
        col,
        Mat::Metal,
    );
    for k in 0..8 {
        let ang = k as f32 / 8.0 * tau;
        let pt = [fwd + ang.cos() * 0.17 * wk, ang.sin() * 0.17 * wk, 0.1 * hk];
        prim(&mut v, pt, pt, 0.13, 0.13, col, Mat::Metal);
    }
    let top = [fwd - 0.02, 0.0, 0.34 * hk];
    prim(
        &mut v,
        top,
        top,
        0.12 * wk.min(1.3),
        0.12,
        shade_col(col, 1.1),
        Mat::Metal,
    );
    if a != Anim::Die {
        for es in [-1.0f32, 1.0] {
            let e = [fwd + 0.2 * wk, es * 0.06, 0.24 * hk];
            prim(&mut v, e, e, 0.03, 0.03, 0x102008ff, Mat::Hair);
        }
    }
    Figure {
        prims: v,
        pitch: 0.0,
        roll: 0.0,
        lift: 0.0,
    }
}

// ------------------------------------------------------------------ raster

struct Raster {
    w: usize,
    h: usize,
    depth: Vec<f32>,
    nrm: Vec<[f32; 3]>,
    col: Vec<u32>,
    mat: Vec<u8>,
}

impl Raster {
    fn new(w: usize, h: usize) -> Raster {
        Raster {
            w,
            h,
            depth: vec![f32::NEG_INFINITY; w * h],
            nrm: vec![[0.0; 3]; w * h],
            col: vec![0; w * h],
            mat: vec![0; w * h],
        }
    }
    /// c = (pixel x, pixel y, depth in px), r = radius in px
    fn sphere(&mut self, cx: f32, cy: f32, cz: f32, r: f32, color: u32, mat: u8) {
        if r < 0.3 {
            return;
        }
        let x0 = ((cx - r).floor().max(0.0)) as usize;
        let x1 = ((cx + r).ceil().min(self.w as f32 - 1.0)).max(0.0) as usize;
        let y0 = ((cy - r).floor().max(0.0)) as usize;
        let y1 = ((cy + r).ceil().min(self.h as f32 - 1.0)).max(0.0) as usize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32 + 0.5 - cx) / r;
                let dy = (y as f32 + 0.5 - cy) / r;
                let d2 = dx * dx + dy * dy;
                if d2 > 1.0 {
                    continue;
                }
                let dz = (1.0 - d2).sqrt();
                let z = cz + dz * r;
                let i = y * self.w + x;
                if z > self.depth[i] {
                    self.depth[i] = z;
                    self.nrm[i] = [dx, -dy, dz];
                    self.col[i] = color;
                    self.mat[i] = mat;
                }
            }
        }
    }
}

fn shade_px(color: u32, n: [f32; 3], mat: u8) -> [f32; 3] {
    let base = [
        ((color >> 24) & 255) as f32,
        ((color >> 16) & 255) as f32,
        ((color >> 8) & 255) as f32,
    ];
    let l = [-0.52f32, 0.6, 0.61];
    let diff = (n[0] * l[0] + n[1] * l[1] + n[2] * l[2]).max(0.0);
    let hv = {
        let h = [l[0], l[1], l[2] + 1.0];
        let m = (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt();
        [h[0] / m, h[1] / m, h[2] / m]
    };
    let spec_dot = (n[0] * hv[0] + n[1] * hv[1] + n[2] * hv[2]).max(0.0);
    let rim = (1.0 - n[2]).powi(2) * 0.1;
    let (k, spec) = match mat {
        1 => (0.28 + 0.6 * diff, spec_dot.powi(18) * 0.95),
        2 => (1.3, 0.0),
        5 => (0.62, 0.0),
        3 => (0.4 + 0.62 * diff, spec_dot.powi(10) * 0.2),
        4 => (0.42 + 0.66 * diff, spec_dot.powi(12) * 0.3),
        _ => (0.4 + 0.72 * diff, spec_dot.powi(8) * 0.06),
    };
    [
        base[0] * (k + rim) + spec * 255.0,
        base[1] * (k + rim) + spec * 255.0,
        base[2] * (k + rim) + spec * 240.0,
    ]
}

const SS: usize = 2;

/// A posed figure ready to render: its shapes plus whole-body rotations.
struct Figure {
    prims: Vec<Prim>,
    /// rotation about the side axis (falling backwards), radians
    pitch: f32,
    /// rotation about the forward axis (falling onto the side)
    roll: f32,
    /// height above the ground after grounding (flying), metres
    lift: f32,
}

fn render_frame(
    look: &Look,
    fig: Figure,
    dir: (f32, f32),
    out: &mut [u8],
    ow: usize,
    ox: usize,
    oy: usize,
    fw: usize,
    fh: usize,
) {
    let s = look.s();
    let m2t = 1.25 / 1.8 * s;
    let prims = fig.prims;
    // falling rotations about the feet, then ground the figure
    let (fs, fc) = fig.pitch.sin_cos();
    let (rs, rc) = fig.roll.sin_cos();
    let tf = |v: V3| -> V3 {
        let p = [v[0] * fc + v[2] * fs, v[1], -v[0] * fs + v[2] * fc];
        [p[0], p[1] * rc - p[2] * rs, p[1] * rs + p[2] * rc]
    };
    let mut lowest = f32::INFINITY;
    for pr in prims.iter() {
        for (pt, r) in [(pr.a, pr.ra), (pr.b, pr.rb)] {
            let z = tf(pt)[2] - r;
            lowest = lowest.min(z);
        }
    }
    let ground = -lowest + fig.lift;
    let th = dir.1.atan2(dir.0);
    let r2 = std::f32::consts::FRAC_1_SQRT_2;
    let kk = K * SS as f32;
    let (aw, ah) = (fw * SS, fh * SS);
    let (ax, ay) = {
        let (x, y) = look.anchor();
        (x * SS as f32, y * SS as f32)
    };
    let mut ras = Raster::new(aw, ah);
    for pr in prims.iter() {
        let a = tf(pr.a);
        let b = tf(pr.b);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        let step = (pr.ra.min(pr.rb).max(0.012)) * 0.32;
        let n = ((len / step).ceil() as usize).clamp(1, 200);
        let mat = match pr.mat {
            Mat::Matte => 0,
            Mat::Metal => 1,
            Mat::Glow => 2,
            Mat::Hair => 3,
            Mat::Bone => 4,
            Mat::Membrane => 5,
        };
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let pl = [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t + ground,
            ];
            let rr = (pr.ra + (pr.rb - pr.ra) * t) * m2t;
            // local -> tile space (y = right-hand side), then the iso camera
            let x = pl[0] * m2t;
            let y = pl[1] * m2t;
            let wx = x * th.cos() - y * th.sin();
            let wy = x * th.sin() + y * th.cos();
            let wz = pl[2] * m2t;
            let u = (wx - wy) * r2;
            let v = -0.5 * (wx + wy) * r2 + 0.866 * wz;
            let w = 0.866 * (wx + wy) * r2 + 0.5 * wz;
            ras.sphere(ax + u * kk, ay - v * kk, w * kk, rr * kk, pr.color, mat);
        }
    }
    // shade at 2x, reduce to 1x with a hard alpha edge
    let mut alpha = vec![false; fw * fh];
    let mut depth = vec![f32::NEG_INFINITY; fw * fh];
    let mut rgb = vec![[0.0f32; 3]; fw * fh];
    let mut cid = vec![0u32; fw * fh];
    for y in 0..fh {
        for x in 0..fw {
            let mut acc = [0.0f32; 3];
            let mut cnt = 0;
            let mut dz = f32::NEG_INFINITY;
            for sy in 0..SS {
                for sx in 0..SS {
                    let i = (y * SS + sy) * aw + x * SS + sx;
                    if ras.depth[i] > f32::NEG_INFINITY {
                        let c = shade_px(ras.col[i], ras.nrm[i], ras.mat[i]);
                        acc = [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]];
                        cnt += 1;
                        if ras.depth[i] > dz {
                            dz = ras.depth[i];
                            cid[y * fw + x] = ras.col[i];
                        }
                    }
                }
            }
            if cnt * 2 >= SS * SS {
                let i = y * fw + x;
                alpha[i] = true;
                depth[i] = dz / SS as f32;
                rgb[i] = [
                    acc[0] / cnt as f32,
                    acc[1] / cnt as f32,
                    acc[2] / cnt as f32,
                ];
            }
        }
    }
    // inner contour where a nearer part overlaps a farther one, then the ink outline
    let ink = [20u8, 12, 8, 255];
    let thr = 2.2 * s.max(1.0);
    for y in 0..fh {
        for x in 0..fw {
            let i = y * fw + x;
            let o = ((oy + y) * ow + ox + x) * 4;
            if alpha[i] {
                let mut k = 1.0;
                for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < fw && (ny as usize) < fh {
                        let j = ny as usize * fw + nx as usize;
                        let gap = depth[j] - depth[i];
                        if alpha[j] && ((gap > thr && cid[j] != cid[i]) || gap > thr * 4.0) {
                            k = 0.5;
                        }
                    }
                }
                let c = rgb[i];
                out[o] = (c[0] * k).clamp(0.0, 255.0) as u8;
                out[o + 1] = (c[1] * k).clamp(0.0, 255.0) as u8;
                out[o + 2] = (c[2] * k).clamp(0.0, 255.0) as u8;
                out[o + 3] = 255;
            } else {
                let mut edge = false;
                for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0
                        && ny >= 0
                        && (nx as usize) < fw
                        && (ny as usize) < fh
                        && alpha[ny as usize * fw + nx as usize]
                    {
                        edge = true;
                    }
                }
                if edge {
                    out[o..o + 4].copy_from_slice(&ink);
                }
            }
        }
    }
}

/// An empty atlas sized for `look`.
pub fn new_atlas(look: &Look) -> Atlas {
    let (fw, fh) = look.frame_size();
    let rows = (RDIRS * FRAMES_PER_DIR).div_ceil(COLS);
    let (w, h) = (fw * COLS, fh * rows);
    Atlas {
        w,
        h,
        fw,
        fh,
        rgba: vec![0u8; w * h * 4],
    }
}
pub const TOTAL_FRAMES: usize = RDIRS * FRAMES_PER_DIR;

/// Render one frame (0..TOTAL_FRAMES) of `look` into its atlas.
pub fn bake_frame(look: &Look, atlas: &mut Atlas, idx: usize) {
    let r = idx / FRAMES_PER_DIR;
    let mut f = idx % FRAMES_PER_DIR;
    let mut anim = (Anim::Idle, 0usize, 1usize);
    for &(a, n) in ANIMS.iter() {
        if f < n {
            anim = (a, f, n);
            break;
        }
        f -= n;
    }
    let fig = figure_for(look, anim.0, anim.1, anim.2);
    let (fw, fh, w) = (atlas.fw, atlas.fh, atlas.w);
    render_frame(
        look,
        fig,
        RENDER_DIRS[r],
        &mut atlas.rgba,
        w,
        (idx % COLS) * fw,
        (idx / COLS) * fh,
        fw,
        fh,
    );
}

/// Bake every animation frame in the 5 rendered directions into one atlas.
pub fn bake(look: &Look) -> Atlas {
    let mut a = new_atlas(look);
    for idx in 0..TOTAL_FRAMES {
        bake_frame(look, &mut a, idx);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bakes_every_frame() {
        let mut l = Look::human(0xe0b090ff, 0x7a5230ff, 0x3a3040ff);
        l.weapon = Weapon::Sword;
        l.shield = 0x9a7a40ff;
        l.hair = 0x3b2414ff;
        let t = std::time::Instant::now();
        let a = bake(&l);
        let ms = t.elapsed().as_millis();
        assert_eq!(a.w, FW * COLS);
        // every frame has a figure in it
        for idx in 0..RDIRS * FRAMES_PER_DIR {
            let (ox, oy) = ((idx % COLS) * a.fw, (idx / COLS) * a.fh);
            let mut n = 0;
            for y in 0..a.fh {
                for x in 0..a.fw {
                    if a.rgba[((oy + y) * a.w + ox + x) * 4 + 3] > 0 {
                        n += 1;
                    }
                }
            }
            assert!(n > 150, "frame {idx} nearly empty ({n})");
        }
        eprintln!("bake took {ms} ms");
        assert_eq!(
            anim_range(Anim::Die).0 + anim_range(Anim::Die).1,
            FRAMES_PER_DIR
        );
    }
}
