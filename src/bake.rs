//! Sprite baker: builds humanoid figures from 3D shapes on a jointed skeleton,
//! poses them, and renders every animation frame in 8 directions into a sprite
//! atlas — the way Ultima VIII's art was rendered from 3D models.
//!
//! Rendering is "sphere splatting": every limb is a chain of overlapping
//! spheres rasterised with a depth buffer, lit from the upper left, rendered
//! at 2x and reduced, then given a dark silhouette outline.
pub mod icons;
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
    Look,
    Walk,
    Run,
    Stance,
    Ready,
    Advance,
    Attack,
    Work,
    Cast,
    Hit,
    Block,
    Kick,
    Talk,
    Die,
}
use Anim::*;
/// Pagan-style animation sets: each kind of figure only bakes what it uses.
const PLAYER_ANIMS: &[(Anim, usize)] = &[
    (Idle, 2),
    (Look, 3),
    (Walk, 8),
    (Run, 6),
    (Stance, 2),
    (Ready, 3),
    (Advance, 4),
    (Attack, 5),
    (Block, 2),
    (Kick, 4),
    (Work, 4),
    (Cast, 4),
    (Hit, 2),
    (Die, 5),
];
const NPC_ANIMS: &[(Anim, usize)] = &[
    (Idle, 2),
    (Look, 3),
    (Talk, 3),
    (Walk, 8),
    (Stance, 2),
    (Attack, 5),
    (Hit, 2),
    (Die, 5),
];
const MON_ANIMS: &[(Anim, usize)] = &[
    (Idle, 2),
    (Look, 3),
    (Walk, 8),
    (Run, 6),
    (Stance, 2),
    (Advance, 4),
    (Attack, 5),
    (Cast, 4),
    (Hit, 2),
    (Die, 5),
];
const BEAST_ANIMS: &[(Anim, usize)] = &[
    (Idle, 2),
    (Walk, 8),
    (Stance, 2),
    (Attack, 5),
    (Cast, 4),
    (Hit, 2),
    (Die, 5),
];

pub fn anims(look: &Look) -> &'static [(Anim, usize)] {
    if look.body_type != Body::Human {
        return BEAST_ANIMS;
    }
    match look.role {
        0 => PLAYER_ANIMS,
        1 => NPC_ANIMS,
        _ => MON_ANIMS,
    }
}
pub fn frames_per_dir(look: &Look) -> usize {
    anims(look).iter().map(|a| a.1).sum()
}
pub fn total_frames(look: &Look) -> usize {
    RDIRS * frames_per_dir(look)
}
fn fallback(a: Anim) -> Anim {
    match a {
        Run | Advance => Walk,
        Look | Talk => Idle,
        Ready | Block | Cast => Stance,
        Kick | Work => Attack,
        _ => Idle,
    }
}
/// (first frame, frame count) of an animation in this look's layout, using
/// the nearest stand-in when the figure has no such animation.
pub fn slot(look: &Look, a: Anim) -> (usize, usize) {
    let mut a = a;
    for _ in 0..3 {
        let mut off = 0;
        for &(b, n) in anims(look).iter() {
            if b == a {
                return (off, n);
            }
            off += n;
        }
        a = fallback(a);
    }
    (0, 1)
}
/// atlas frame for an animation at `phase` (0..1) facing `dir8`
pub fn frame_index(look: &Look, a: Anim, phase: f32, dir8: u8) -> (usize, bool) {
    let (r, flip) = dir_frame(dir8);
    let (off, n) = slot(look, a);
    let f = ((phase.clamp(0.0, 0.999) * n as f32) as usize).min(n - 1);
    (r * frames_per_dir(look) + off + f, flip)
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
    /// 0 player, 1 townsfolk, 2 monster: decides which animations are baked
    pub role: u8,
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
    /// a cloth hood over the head and shoulders
    pub hood: u32,
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
            role: 2,
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
            hood: 0,
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
    head_yaw: f32,
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
        Anim::Run => {
            let ph = f as f32 / n as f32 * tau;
            let (s, c) = (ph.sin(), ph.cos());
            q.hip = [0.8 * s, -0.8 * s];
            q.knee = [0.2 + 1.2 * c.max(0.0), 0.2 + 1.2 * (-c).max(0.0)];
            q.sh = [-0.65 * s, 0.65 * s];
            q.el = [1.3, 1.3];
            q.roll = [0.2, 0.2];
            q.sink = 0.05 * (2.0 * ph).cos().abs();
            q.lean += 0.25;
            q.twist = 0.12 * s;
        }
        Anim::Look => {
            q.head_yaw = [0.75, 0.0, -0.75][f.min(2)];
            q.twist = q.head_yaw * 0.15;
        }
        Anim::Talk => {
            let g = [0.0f32, 1.0, 0.5][f.min(2)];
            q.sh = [0.5 + 0.6 * g, 0.15 + 0.3 * (1.0 - g)];
            q.el = [0.9 - 0.3 * g, 0.5];
            q.roll = [0.35, 0.25];
            q.head_tilt = 0.1 * g;
            q.head_yaw = 0.15 * (g - 0.5);
        }
        Anim::Ready => {
            // reach across to the hip, draw, come up to guard
            let (sh, el, roll) = match f {
                0 => (0.35, 1.7, -0.45),
                1 => (1.4, 1.1, 0.0),
                _ => (0.95, 1.05, 0.2),
            };
            q.sh = [sh, 0.6];
            q.el = [el, 0.9];
            q.roll = [roll, 0.2];
            q.wrist = 0.6;
            q.twist = -0.25 * (2 - f.min(2)) as f32;
        }
        Anim::Advance => {
            let ph = f as f32 / n as f32 * tau;
            q.sh = [0.95, 0.75];
            q.el = [1.05, 1.1];
            q.roll = [0.2, 0.25];
            q.wrist = 0.55;
            q.hip = [0.3 + 0.3 * ph.sin(), -0.15 - 0.3 * ph.sin()];
            q.knee = [
                0.4 + 0.3 * ph.cos().max(0.0),
                0.35 + 0.3 * (-ph.cos()).max(0.0),
            ];
            q.lean += 0.12;
            q.twist = -0.2;
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
        Anim::Block => {
            // shield (or forearm) up in front, blade held across as a guard
            let b = f as f32 * 0.04;
            q.sh = [1.25 + b, 1.45 + b];
            q.el = [1.35, 0.8];
            q.roll = [0.05, 0.05];
            q.wrist = 0.15;
            q.twist = 0.15;
            q.lean -= 0.06;
            q.hip = [0.3, -0.2];
            q.knee = [0.45, 0.35];
            q.head_tilt = 0.15;
        }
        Anim::Kick => {
            // chamber, strike, recoil, set down
            let (hip, knee, lean) = match f {
                0 => (0.95, 1.45, 0.0),
                1 => (1.45, 0.08, -0.28),
                2 => (1.05, 0.95, -0.12),
                _ => (0.35, 0.35, 0.0),
            };
            q.hip = [hip, -0.1];
            q.knee = [knee, 0.25];
            q.lean += lean;
            q.sh = [0.5, 0.6];
            q.roll = [0.55, 0.55];
            q.el = [0.6, 0.6];
            q.wrist = 0.8;
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
    /// foliage: lit like cloth, broken up into clumps of light and dark leaves
    Leaf,
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
    let fwd = rotz([1.0, 0.0, 0.0], q.twist * 0.5 + q.head_yaw);
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
        if look.hood != 0 {
            let c = add(hc, add(mul(fwd, -0.035), [0.0, 0.0, 0.02]));
            p(c, c, 0.128, 0.128, look.hood, Mat::Matte);
            p(
                add(hc, add(mul(fwd, -0.05), [0.0, 0.0, -0.04])),
                add(hc, add(mul(fwd, -0.06), [0.0, 0.0, -0.2])),
                0.12,
                0.16,
                look.hood,
                Mat::Matte,
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
                boxes: vec![],
                pitch: q.fall,
                roll: 0.0,
                lift: 0.0,
            }
        }
        _ => {
            let a = match a {
                Anim::Block => Anim::Stance,
                Anim::Kick => Anim::Attack,
                x => x,
            };
            match look.body_type {
                Body::Slime => slime(look, a, f, n),
                Body::Bat => bat(look, a, f, n),
                _ => quad(look, a, f, n),
            }
        }
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
        boxes: vec![],
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
        boxes: vec![],
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
        boxes: vec![],
        pitch: 0.0,
        roll: 0.0,
        lift: 0.0,
    }
}

// ------------------------------------------------------------------ props
pub const P_BED: usize = 0;
pub const P_TABLE: usize = 1;
pub const P_CHAIR: usize = 2;
pub const P_SHELF: usize = 3;
pub const P_HEARTH: usize = 4;
pub const P_CHEST: usize = 5;
pub const P_BARREL: usize = 6;
pub const P_CRATE: usize = 7;
pub const P_OAK: usize = 8; // 3 variants
pub const P_PINE: usize = 11; // 2 variants
pub const P_DEAD: usize = 13; // 2 variants
pub const P_SHROOM: usize = 15; // red, violet
pub const P_STUMP: usize = 17;
pub const P_SACKS: usize = 18;
pub const P_SKULLS: usize = 19;
pub const P_TORCH: usize = 20;
pub const P_BANNER: usize = 21; // red, green
pub const P_STATUE: usize = 23;
pub const P_WELL: usize = 24;
pub const P_CRAG: usize = 25; // 3 low, then 3 tall
pub const P_LONGTABLE: usize = 31;
pub const P_CART: usize = 32;
/// spell effects, baked as frame animations
pub const P_FX_EXPLO: usize = 33; // 8 frames
pub const P_FX_FLAME: usize = 41; // 8 frames
pub const P_FX_HEAL: usize = 49; // 8 frames
pub const P_FX_BALL: usize = 57; // 4 frames
pub const FX_FRAMES: usize = 8;
/// the rest of the world's furniture, once drawn by hand
pub const P_DOOR: usize = 61; // post on the -y side, then on the +y side
pub const P_FENCE: usize = 63;
pub const P_GRAVE: usize = 64; // headstone, cross
pub const P_TENT: usize = 66;
pub const P_FIRE: usize = 67; // 4 frames
pub const P_ANKH: usize = 71;
pub const P_LAMP: usize = 72; // 4 frames
pub const P_BONES: usize = 76;
pub const P_BUSH: usize = 77; // 2
pub const P_BOULDER: usize = 79; // 2
pub const P_FLOWERS: usize = 81; // 2
pub const P_HOARD: usize = 83;
pub const P_SIGN: usize = 84;
pub const P_ANVIL: usize = 85;
pub const P_FORGE: usize = 86; // 4 frames
pub const P_FOUNTAIN: usize = 90;
/// small things lying about on open ground
pub const P_TUFT: usize = 91; // 3
pub const P_FERN: usize = 94;
pub const P_PEBBLES: usize = 95; // 2
pub const P_TWIGS: usize = 97;
pub const N_PROPS: usize = 98;

/// flames licking up from `base`, varied by animation frame `f`
fn flames(p: &mut Vec<Prim>, base: V3, size: f32, f: usize) {
    for i in 0..7 {
        let a = i as f32 * 2.4 + f as f32 * 1.3;
        let r = size * (0.25 + hsh(i + f * 7, 70) * 0.35);
        let h = size * (0.7 + hsh(i + f * 7, 71) * 0.9);
        let b = [
            base[0] + a.cos() * r * 0.6,
            base[1] + a.sin() * r * 0.6,
            base[2],
        ];
        let tip = [
            b[0] + (hsh(i + f * 7, 72) - 0.5) * size * 0.4,
            b[1],
            b[2] + h,
        ];
        let col = if i < 3 {
            0xffc848ff
        } else if i < 5 {
            0xe86414ff
        } else {
            0xa82a0cff
        };
        prim(p, b, tip, r, 0.01, col, Mat::Glow);
    }
}
pub const PROP_FW: usize = 128;
pub const PROP_FH: usize = 128;
pub const PROP_ANCHOR: (f32, f32) = (64.0, 116.0);

fn bx(v: &mut Vec<BoxP>, c: V3, h: V3, color: u32, mat: Mat) {
    v.push(BoxP {
        c,
        h,
        yaw: 0.0,
        color,
        mat,
    });
}
fn hsh(i: usize, k: usize) -> f32 {
    let mut h = (i as u32).wrapping_mul(0x9e37_79b9) ^ (k as u32).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h & 0xffff) as f32 / 65536.0
}

/// a prop model in local metres, front facing +x
fn prop(kind: usize) -> Figure {
    let mut p: Vec<Prim> = vec![];
    let mut b: Vec<BoxP> = vec![];
    let wood = 0x6a4628ff;
    let dark = 0x3e2816ff;
    match kind {
        P_BED => {
            bx(
                &mut b,
                [0.0, 0.0, 0.18],
                [0.62, 0.36, 0.18],
                wood,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.02, 0.0, 0.4],
                [0.58, 0.33, 0.05],
                0xd0c8b0ff,
                Mat::Matte,
            );
            bx(
                &mut b,
                [-0.14, 0.0, 0.45],
                [0.44, 0.34, 0.04],
                0x7a2420ff,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.44, 0.0, 0.5],
                [0.1, 0.24, 0.05],
                0xe0d8c8ff,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.62, 0.0, 0.42],
                [0.04, 0.38, 0.42],
                dark,
                Mat::Matte,
            );
            bx(
                &mut b,
                [-0.62, 0.0, 0.28],
                [0.04, 0.38, 0.28],
                dark,
                Mat::Matte,
            );
        }
        P_TABLE => {
            bx(
                &mut b,
                [0.0, 0.0, 0.74],
                [0.46, 0.36, 0.035],
                wood,
                Mat::Matte,
            );
            for (x, y) in [
                (0.38f32, 0.28f32),
                (-0.38, 0.28),
                (0.38, -0.28),
                (-0.38, -0.28),
            ] {
                bx(&mut b, [x, y, 0.36], [0.035, 0.035, 0.36], dark, Mat::Matte);
            }
            // a candle and a tankard
            prim(
                &mut p,
                [0.1, -0.1, 0.78],
                [0.1, -0.1, 0.9],
                0.025,
                0.025,
                0xe8e0c0ff,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.1, -0.1, 0.93],
                [0.1, -0.1, 0.93],
                0.018,
                0.018,
                0xffc040ff,
                Mat::Glow,
            );
            prim(
                &mut p,
                [-0.15, 0.12, 0.79],
                [-0.15, 0.12, 0.88],
                0.045,
                0.045,
                0x8a8e94ff,
                Mat::Metal,
            );
        }
        P_CHAIR => {
            bx(&mut b, [0.0, 0.0, 0.44], [0.2, 0.2, 0.03], wood, Mat::Matte);
            for (x, y) in [
                (0.16f32, 0.16f32),
                (-0.16, 0.16),
                (0.16, -0.16),
                (-0.16, -0.16),
            ] {
                bx(&mut b, [x, y, 0.22], [0.025, 0.025, 0.22], dark, Mat::Matte);
            }
            bx(
                &mut b,
                [-0.18, 0.0, 0.76],
                [0.025, 0.2, 0.3],
                wood,
                Mat::Matte,
            );
        }
        P_SHELF => {
            bx(
                &mut b,
                [0.0, 0.0, 0.95],
                [0.17, 0.48, 0.95],
                dark,
                Mat::Matte,
            );
            for row in 0..4 {
                let z = 0.3 + row as f32 * 0.45;
                bx(
                    &mut b,
                    [0.08, 0.0, z - 0.05],
                    [0.1, 0.44, 0.02],
                    wood,
                    Mat::Matte,
                );
                let mut y = -0.42;
                let mut k = 0;
                while y < 0.4 {
                    let w = 0.03 + hsh(row * 31 + k, 1) * 0.04;
                    let hgt = 0.12 + hsh(row * 31 + k, 2) * 0.08;
                    let col = [0x7a2a20ff, 0x2a4a7aff, 0x3a6a2aff, 0x8a6a2aff, 0x5a2a5aff]
                        [(hsh(row * 31 + k, 3) * 5.0) as usize % 5];
                    bx(
                        &mut b,
                        [0.1, y + w, z + hgt],
                        [0.08, w * 0.9, hgt],
                        col,
                        Mat::Matte,
                    );
                    y += w * 2.0 + 0.005;
                    k += 1;
                }
            }
        }
        P_HEARTH => {
            let stone = 0x6a6660ff;
            bx(
                &mut b,
                [-0.05, 0.0, 0.9],
                [0.28, 0.6, 0.9],
                stone,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.2, 0.0, 0.33],
                [0.06, 0.36, 0.33],
                0x0c0806ff,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.25, 0.0, 0.72],
                [0.08, 0.64, 0.05],
                0x5a4a3aff,
                Mat::Matte,
            );
            for k in 0..5 {
                let kf = k as f32;
                prim(
                    &mut p,
                    [0.22, -0.15 + kf * 0.075, 0.08],
                    [0.22, -0.13 + kf * 0.07, 0.2 + 0.06 * (kf * 1.7).sin().abs()],
                    0.06,
                    0.02,
                    if k % 2 == 0 { 0xff8a20ff } else { 0xffd050ff },
                    Mat::Glow,
                );
            }
        }
        P_CHEST => {
            bx(&mut b, [0.0, 0.0, 0.2], [0.26, 0.4, 0.2], wood, Mat::Matte);
            bx(
                &mut b,
                [0.0, 0.0, 0.43],
                [0.27, 0.41, 0.05],
                dark,
                Mat::Matte,
            );
            for y in [-0.28f32, 0.28] {
                bx(
                    &mut b,
                    [0.0, y, 0.25],
                    [0.275, 0.03, 0.25],
                    0x5a5e66ff,
                    Mat::Metal,
                );
            }
            prim(
                &mut p,
                [0.28, 0.0, 0.34],
                [0.28, 0.0, 0.34],
                0.04,
                0.04,
                0xd8b050ff,
                Mat::Metal,
            );
        }
        P_BARREL => {
            // an eight-sided stave barrel with two iron hoops
            for k in 0..8 {
                let yaw = k as f32 / 8.0 * std::f32::consts::PI;
                b.push(BoxP {
                    c: [0.0, 0.0, 0.42],
                    h: [0.29, 0.12, 0.42],
                    yaw,
                    color: if k % 2 == 0 { wood } else { 0x5e3e22ff },
                    mat: Mat::Matte,
                });
                for z in [0.16f32, 0.68] {
                    b.push(BoxP {
                        c: [0.0, 0.0, z],
                        h: [0.3, 0.125, 0.03],
                        yaw,
                        color: 0x2e2e30ff,
                        mat: Mat::Metal,
                    });
                }
            }
        }
        P_CRATE => {
            bx(
                &mut b,
                [0.0, 0.0, 0.3],
                [0.3, 0.3, 0.3],
                0x7a5632ff,
                Mat::Matte,
            );
            for (y, z) in [(0.0f32, 0.0f32), (0.0, 0.5), (0.0, 0.25)] {
                let _ = y;
                bx(
                    &mut b,
                    [0.0, 0.0, 0.04 + z],
                    [0.305, 0.305, 0.03],
                    dark,
                    Mat::Matte,
                );
            }
        }
        k if (P_OAK..P_OAK + 3).contains(&k) => {
            let v = k - P_OAK;
            let trunk = 0x4a3220ff;
            prim(
                &mut p,
                [0.0, 0.0, 0.0],
                [0.05, 0.0, 1.5],
                0.16,
                0.1,
                trunk,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.05, 0.0, 1.1],
                [0.5, 0.25, 1.75],
                0.07,
                0.04,
                trunk,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.0, 1.2],
                [-0.4, -0.3, 1.8],
                0.07,
                0.04,
                trunk,
                Mat::Matte,
            );
            // a ragged canopy of many small leaf clumps in a few lobes
            let rad = 0.8 + v as f32 * 0.12;
            let lobes = 4 + v;
            let greens = [
                0x24381affu32,
                0x2c4620ff,
                0x35521fff,
                0x1e3016ff,
                0x3e5a24ff,
            ];
            for l in 0..lobes {
                let la = hsh(l, v * 11 + 20) * std::f32::consts::TAU;
                let lr = rad * (0.35 + hsh(l, v * 11 + 21) * 0.45);
                let lc = [
                    la.cos() * lr,
                    la.sin() * lr,
                    1.85 + hsh(l, v * 11 + 22) * 0.6,
                ];
                let lsize = 0.45 + hsh(l, v * 11 + 23) * 0.25;
                for i in 0..26 {
                    let k = l * 31 + i;
                    let (a, e, r) = (
                        hsh(k, v * 7 + 1) * std::f32::consts::TAU,
                        hsh(k, v * 7 + 2) * 2.0 - 1.0,
                        hsh(k, v * 7 + 3).sqrt() * lsize,
                    );
                    let ce = (1.0 - e * e).sqrt();
                    let c = [
                        lc[0] + a.cos() * ce * r,
                        lc[1] + a.sin() * ce * r,
                        lc[2] + e * r * 0.75,
                    ];
                    let g = greens[(hsh(k, v + 9) * 5.0) as usize % 5];
                    let sz = 0.13 + hsh(k, 5) * 0.1;
                    prim(&mut p, c, c, sz, sz, g, Mat::Leaf);
                }
            }
            // a few bare twigs poking out
            for i in 0..3 {
                let a = hsh(i, v + 40) * std::f32::consts::TAU;
                prim(
                    &mut p,
                    [a.cos() * 0.3, a.sin() * 0.3, 2.0],
                    [
                        a.cos() * (rad + 0.25),
                        a.sin() * (rad + 0.25),
                        2.3 + hsh(i, 41) * 0.4,
                    ],
                    0.025,
                    0.01,
                    trunk,
                    Mat::Matte,
                );
            }
        }
        k if (P_PINE..P_PINE + 2).contains(&k) => {
            let v = k - P_PINE;
            prim(
                &mut p,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 2.6],
                0.12,
                0.05,
                0x3e2a1aff,
                Mat::Matte,
            );
            for tier in 0..5 {
                let tf = tier as f32;
                let z = 0.6 + tf * 0.45;
                let rad = 0.95 - tf * 0.18 + v as f32 * 0.08;
                let cnt = 9 - tier;
                for j in 0..cnt {
                    let a = j as f32 / cnt as f32 * std::f32::consts::TAU + tf * 0.4;
                    let c = [a.cos() * rad * 0.7, a.sin() * rad * 0.7, z];
                    prim(
                        &mut p,
                        c,
                        [c[0] * 0.3, c[1] * 0.3, z + 0.35],
                        0.2,
                        0.12,
                        if (j + tier) % 2 == 0 {
                            0x1e3418ff
                        } else {
                            0x243c1cff
                        },
                        Mat::Leaf,
                    );
                }
            }
            prim(
                &mut p,
                [0.0, 0.0, 2.6],
                [0.0, 0.0, 2.95],
                0.12,
                0.02,
                0x243c1cff,
                Mat::Leaf,
            );
        }
        k if (P_DEAD..P_DEAD + 2).contains(&k) => {
            let v = k - P_DEAD;
            let c = 0x4a3a2aff;
            prim(
                &mut p,
                [0.0, 0.0, 0.0],
                [0.08, 0.0, 1.6],
                0.14,
                0.06,
                c,
                Mat::Matte,
            );
            for i in 0..5 + v {
                let a = hsh(i, 40 + v) * std::f32::consts::TAU;
                let z = 0.8 + hsh(i, 41) * 0.8;
                let l = 0.4 + hsh(i, 42) * 0.4;
                let e = [a.cos() * l, a.sin() * l, z + 0.3 + hsh(i, 43) * 0.3];
                prim(&mut p, [0.04, 0.0, z], e, 0.05, 0.015, c, Mat::Matte);
                prim(
                    &mut p,
                    e,
                    add(e, [a.cos() * 0.2, a.sin() * 0.2 + 0.1, 0.18]),
                    0.015,
                    0.008,
                    c,
                    Mat::Matte,
                );
            }
        }
        P_SHROOM | 16 => {
            let cap = if kind == P_SHROOM {
                0x8a2a20ffu32
            } else {
                0x5a4a8aff
            };
            prim(
                &mut p,
                [0.0, 0.0, 0.0],
                [0.03, 0.0, 0.95],
                0.13,
                0.09,
                0xc8bca0ff,
                Mat::Matte,
            );
            for j in 0..10 {
                let a = j as f32 / 10.0 * std::f32::consts::TAU;
                let c = [a.cos() * 0.38, a.sin() * 0.38, 0.98];
                prim(&mut p, c, c, 0.17, 0.17, cap, Mat::Hair);
            }
            prim(
                &mut p,
                [0.0, 0.0, 1.05],
                [0.0, 0.0, 1.05],
                0.36,
                0.36,
                cap,
                Mat::Hair,
            );
            for j in 0..6 {
                let a = hsh(j, 70) * std::f32::consts::TAU;
                let r = 0.15 + hsh(j, 71) * 0.25;
                let c = [a.cos() * r, a.sin() * r, 1.17 + 0.1 * (0.4 - r)];
                prim(&mut p, c, c, 0.045, 0.045, 0xe8e0c8ff, Mat::Matte);
            }
        }

        P_SACKS => {
            let burlap = [0x8a7450ffu32, 0x7a6644ff, 0x948058ff];
            for (k, &(x, y, lean)) in [
                (-0.12f32, -0.2f32, 0.1f32),
                (0.18, 0.12, -0.12),
                (-0.25, 0.3, 0.05),
            ]
            .iter()
            .enumerate()
            {
                let col = burlap[k];
                let s = if k == 2 { 0.8 } else { 1.0 };
                prim(
                    &mut p,
                    [x, y, 0.2 * s],
                    [x + lean, y, 0.45 * s],
                    0.26 * s,
                    0.2 * s,
                    col,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [x + lean, y, 0.5 * s],
                    [x + lean * 1.3, y, 0.6 * s],
                    0.08 * s,
                    0.06 * s,
                    shade_col(col, 0.8),
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [x + lean * 1.2, y, 0.55 * s],
                    [x + lean * 1.2, y, 0.55 * s],
                    0.075 * s,
                    0.075 * s,
                    0x3a2a18ff,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [x + lean * 1.4, y, 0.63 * s],
                    [x + lean * 1.4, y, 0.63 * s],
                    0.09 * s,
                    0.09 * s,
                    shade_col(col, 0.85),
                    Mat::Matte,
                );
            }
        }
        P_SKULLS => {
            let bone = 0xcfc4a4ff;
            let pile: [(f32, f32, f32); 10] = [
                (0.0, 0.0, 0.1),
                (0.22, 0.05, 0.1),
                (-0.2, 0.12, 0.1),
                (0.05, -0.22, 0.1),
                (-0.15, -0.18, 0.1),
                (0.2, -0.2, 0.09),
                (0.08, 0.0, 0.28),
                (-0.1, -0.05, 0.27),
                (0.0, -0.12, 0.27),
                (0.0, -0.05, 0.44),
            ];
            for (i, &(x, y, zc)) in pile.iter().enumerate() {
                let a = (hsh(i, 90) - 0.5) * 2.4; // mostly glaring at the viewer
                let (dx, dy) = (a.cos(), a.sin());
                prim(&mut p, [x, y, zc], [x, y, zc], 0.1, 0.1, bone, Mat::Bone);
                let j = [x + dx * 0.07, y + dy * 0.07, zc - 0.06];
                prim(&mut p, j, j, 0.06, 0.06, shade_col(bone, 0.9), Mat::Bone);
                for side in [-1.0f32, 1.0] {
                    let e = [
                        x + dx * 0.085 - dy * side * 0.04,
                        y + dy * 0.085 + dx * side * 0.04,
                        zc + 0.0,
                    ];
                    prim(&mut p, e, e, 0.03, 0.03, 0x120c08ff, Mat::Matte);
                }
            }
            // a few long bones
            prim(
                &mut p,
                [-0.35, 0.1, 0.03],
                [0.1, 0.38, 0.03],
                0.03,
                0.03,
                bone,
                Mat::Bone,
            );
            prim(
                &mut p,
                [0.3, -0.35, 0.03],
                [0.4, 0.1, 0.05],
                0.028,
                0.028,
                bone,
                Mat::Bone,
            );
        }
        P_TORCH => {
            // an iron bracket on the wall (behind is -x), the torch leaning out
            prim(
                &mut p,
                [-0.12, 0.0, 0.0],
                [0.06, 0.0, 0.04],
                0.025,
                0.02,
                0x2e2e30ff,
                Mat::Metal,
            );
            prim(
                &mut p,
                [-0.12, 0.0, -0.06],
                [-0.12, 0.0, 0.08],
                0.03,
                0.03,
                0x2e2e30ff,
                Mat::Metal,
            );
            prim(
                &mut p,
                [0.04, 0.0, -0.08],
                [0.12, 0.0, 0.32],
                0.03,
                0.035,
                0x4a3220ff,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.11, 0.0, 0.28],
                [0.13, 0.0, 0.36],
                0.055,
                0.05,
                0x2a1a10ff,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.13, 0.0, 0.38],
                [0.16, 0.0, 0.6],
                0.075,
                0.01,
                0xff8a20ff,
                Mat::Glow,
            );
            prim(
                &mut p,
                [0.13, 0.0, 0.38],
                [0.14, 0.0, 0.5],
                0.045,
                0.01,
                0xffe070ff,
                Mat::Glow,
            );
        }
        k if k == P_BANNER || k == P_BANNER + 1 => {
            // cloth hanging flat against a wall that faces +x
            let (cloth, mark) = if k == P_BANNER {
                (0x6a1a18ffu32, 0xc8a040ffu32)
            } else {
                (0x1e3a22ff, 0xb8b0a0ff)
            };
            prim(
                &mut p,
                [0.04, -0.3, 0.95],
                [0.04, 0.3, 0.95],
                0.025,
                0.025,
                0x3e2816ff,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.03, 0.0, 0.55],
                [0.012, 0.24, 0.38],
                cloth,
                Mat::Matte,
            );
            // swallow-tail: two narrower strips below
            bx(
                &mut b,
                [0.03, -0.14, 0.1],
                [0.012, 0.1, 0.08],
                cloth,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.03, 0.14, 0.1],
                [0.012, 0.1, 0.08],
                cloth,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.045, 0.0, 0.62],
                [0.008, 0.13, 0.02],
                mark,
                Mat::Metal,
            );
            bx(
                &mut b,
                [0.045, 0.0, 0.6],
                [0.008, 0.02, 0.2],
                mark,
                Mat::Metal,
            );
            bx(
                &mut b,
                [0.045, 0.0, 0.9],
                [0.008, 0.24, 0.02],
                mark,
                Mat::Metal,
            );
        }
        P_STATUE => {
            let stone = 0x7e7a72ff;
            let moss = 0x5a6248ff;
            bx(
                &mut b,
                [0.0, 0.0, 0.28],
                [0.42, 0.42, 0.28],
                shade_col(stone, 0.85),
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.0, 0.0, 0.6],
                [0.36, 0.36, 0.04],
                stone,
                Mat::Matte,
            );
            // a hooded knight resting both hands on a greatsword
            prim(
                &mut p,
                [0.0, 0.0, 0.64],
                [0.0, 0.0, 1.62],
                0.3,
                0.17,
                stone,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, -0.17, 1.6],
                [0.0, 0.17, 1.6],
                0.1,
                0.1,
                stone,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.0, 1.72],
                [0.0, 0.0, 1.86],
                0.12,
                0.13,
                stone,
                Mat::Matte,
            );
            prim(
                &mut p,
                [-0.03, 0.0, 1.9],
                [-0.03, 0.0, 1.9],
                0.13,
                0.13,
                shade_col(stone, 0.9),
                Mat::Matte,
            );
            for side in [-1.0f32, 1.0] {
                prim(
                    &mut p,
                    [0.0, side * 0.19, 1.58],
                    [0.18, side * 0.1, 1.3],
                    0.06,
                    0.05,
                    stone,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [0.18, side * 0.1, 1.3],
                    [0.26, side * 0.03, 1.26],
                    0.05,
                    0.045,
                    stone,
                    Mat::Matte,
                );
            }
            prim(
                &mut p,
                [0.28, 0.0, 0.66],
                [0.28, 0.0, 1.22],
                0.035,
                0.02,
                stone,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.28, -0.12, 1.22],
                [0.28, 0.12, 1.22],
                0.025,
                0.025,
                stone,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.28, 0.0, 1.22],
                [0.28, 0.0, 1.38],
                0.025,
                0.025,
                stone,
                Mat::Matte,
            );
            // weathering
            for i in 0..7 {
                let c = [
                    hsh(i, 80) * 0.5 - 0.25,
                    hsh(i, 81) * 0.5 - 0.25,
                    0.55 + hsh(i, 82) * 0.15,
                ];
                prim(&mut p, c, c, 0.05, 0.05, moss, Mat::Matte);
            }
        }
        P_WELL => {
            let stone = 0x6e6a62ff;
            for k in 0..8 {
                let yaw = k as f32 / 8.0 * std::f32::consts::PI;
                b.push(BoxP {
                    c: [0.0, 0.0, 0.3],
                    h: [0.55, 0.23, 0.3],
                    yaw,
                    color: if k % 2 == 0 {
                        stone
                    } else {
                        shade_col(stone, 0.9)
                    },
                    mat: Mat::Matte,
                });
            }
            b.push(BoxP {
                c: [0.0, 0.0, 0.605],
                h: [0.42, 0.17, 0.01],
                yaw: 0.0,
                color: 0x0a0c10ff,
                mat: Mat::Matte,
            });
            b.push(BoxP {
                c: [0.0, 0.0, 0.605],
                h: [0.42, 0.17, 0.01],
                yaw: std::f32::consts::FRAC_PI_2,
                color: 0x0a0c10ff,
                mat: Mat::Matte,
            });
            b.push(BoxP {
                c: [0.0, 0.0, 0.605],
                h: [0.3, 0.3, 0.01],
                yaw: std::f32::consts::FRAC_PI_4,
                color: 0x0a0c10ff,
                mat: Mat::Matte,
            });
            for side in [-1.0f32, 1.0] {
                bx(
                    &mut b,
                    [0.0, side * 0.5, 1.05],
                    [0.05, 0.05, 0.45],
                    wood,
                    Mat::Matte,
                );
                // a little gabled roof
                b.push(BoxP {
                    c: [side * 0.2, 0.0, 1.62],
                    h: [0.26, 0.62, 0.02],
                    yaw: 0.0,
                    color: dark,
                    mat: Mat::Matte,
                });
            }
            prim(
                &mut p,
                [-0.38, 0.0, 1.55],
                [0.38, 0.0, 1.55],
                0.02,
                0.02,
                dark,
                Mat::Matte,
            );
            prim(
                &mut p,
                [-0.45, 0.0, 1.62],
                [0.0, 0.0, 1.78],
                0.03,
                0.03,
                dark,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.45, 0.0, 1.62],
                [0.0, 0.0, 1.78],
                0.03,
                0.03,
                dark,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.0, 1.5],
                [0.0, 0.0, 1.1],
                0.012,
                0.012,
                0x8a7a5aff,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.0, 0.98],
                [0.0, 0.0, 1.1],
                0.09,
                0.1,
                wood,
                Mat::Matte,
            );
            prim(
                &mut p,
                [-0.1, 0.0, 1.03],
                [0.1, 0.0, 1.03],
                0.105,
                0.105,
                0x2e2e30ff,
                Mat::Metal,
            );
        }
        P_LONGTABLE => {
            // a trestle table two tiles long (along y), benches either side
            bx(
                &mut b,
                [0.0, 0.0, 0.74],
                [0.42, 1.25, 0.035],
                wood,
                Mat::Matte,
            );
            for y in [-1.0f32, 1.0] {
                bx(&mut b, [0.0, y, 0.37], [0.3, 0.04, 0.37], dark, Mat::Matte);
                bx(&mut b, [0.0, y, 0.05], [0.36, 0.06, 0.05], dark, Mat::Matte);
            }
            bx(&mut b, [0.0, 0.0, 0.3], [0.03, 1.0, 0.03], dark, Mat::Matte);
            for x in [-0.68f32, 0.68] {
                bx(&mut b, [x, 0.0, 0.44], [0.12, 1.15, 0.03], wood, Mat::Matte);
                for y in [-0.95f32, 0.95] {
                    bx(&mut b, [x, y, 0.21], [0.03, 0.03, 0.21], dark, Mat::Matte);
                }
            }
            // supper: tankards, a loaf, a candle
            for (i, y) in [-0.8f32, -0.2, 0.5].iter().enumerate() {
                let x = if i % 2 == 0 { 0.2 } else { -0.2 };
                prim(
                    &mut p,
                    [x, *y, 0.79],
                    [x, *y, 0.9],
                    0.045,
                    0.045,
                    0x8a8e94ff,
                    Mat::Metal,
                );
            }
            prim(
                &mut p,
                [-0.1, 0.9, 0.8],
                [0.05, 1.05, 0.82],
                0.08,
                0.07,
                0x9a6a30ff,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.1, 0.78],
                [0.0, 0.1, 0.9],
                0.025,
                0.025,
                0xe8e0c0ff,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.1, 0.93],
                [0.0, 0.1, 0.93],
                0.018,
                0.018,
                0xffc040ff,
                Mat::Glow,
            );
        }
        P_CART => {
            // a hand cart two tiles long (along y): bed, sides, two wheels, shafts
            bx(
                &mut b,
                [0.0, -0.2, 0.7],
                [0.55, 0.9, 0.05],
                wood,
                Mat::Matte,
            );
            for x in [-0.53f32, 0.53] {
                bx(&mut b, [x, -0.2, 0.88], [0.03, 0.9, 0.15], dark, Mat::Matte);
            }
            bx(
                &mut b,
                [0.0, -1.08, 0.88],
                [0.55, 0.03, 0.15],
                dark,
                Mat::Matte,
            );
            prim(
                &mut p,
                [-0.7, -0.1, 0.42],
                [0.7, -0.1, 0.42],
                0.04,
                0.04,
                dark,
                Mat::Matte,
            );
            for x in [-0.66f32, 0.66] {
                for k in 0..16 {
                    let a = k as f32 / 16.0 * std::f32::consts::TAU;
                    let a2 = (k + 1) as f32 / 16.0 * std::f32::consts::TAU;
                    prim(
                        &mut p,
                        [x, -0.1 + a.cos() * 0.4, 0.42 + a.sin() * 0.4],
                        [x, -0.1 + a2.cos() * 0.4, 0.42 + a2.sin() * 0.4],
                        0.045,
                        0.045,
                        dark,
                        Mat::Matte,
                    );
                }
                for k in 0..4 {
                    let a = k as f32 / 4.0 * std::f32::consts::PI;
                    prim(
                        &mut p,
                        [x, -0.1 + a.cos() * 0.38, 0.42 + a.sin() * 0.38],
                        [x, -0.1 - a.cos() * 0.38, 0.42 - a.sin() * 0.38],
                        0.02,
                        0.02,
                        wood,
                        Mat::Matte,
                    );
                }
                prim(
                    &mut p,
                    [x * 0.55, 0.7, 0.72],
                    [x * 0.45, 1.3, 0.05],
                    0.035,
                    0.03,
                    wood,
                    Mat::Matte,
                );
            }
            // a load of sacks and a barrel
            for (y, col) in [(-0.7f32, 0x8a7450ffu32), (-0.25, 0x7a6644ff)] {
                prim(
                    &mut p,
                    [0.1, y, 0.85],
                    [0.0, y, 1.0],
                    0.24,
                    0.18,
                    col,
                    Mat::Matte,
                );
            }
            for k in 0..8 {
                let yaw = k as f32 / 8.0 * std::f32::consts::PI;
                b.push(BoxP {
                    c: [0.0, 0.35, 1.03],
                    h: [0.24, 0.1, 0.3],
                    yaw,
                    color: if k % 2 == 0 { wood } else { 0x5e3e22ff },
                    mat: Mat::Matte,
                });
            }
        }
        k if (P_FX_EXPLO..P_FX_EXPLO + 8).contains(&k) => {
            // a burst: white heat, then orange fire, then rolling smoke
            let t = (k - P_FX_EXPLO) as f32 / 7.0;
            for i in 0..18 {
                let a = hsh(i, 50) * std::f32::consts::TAU;
                let up = hsh(i, 51) * 0.9 + 0.1;
                let reach = (0.15 + t * 1.1) * (0.5 + hsh(i, 52) * 0.6);
                let c = [
                    a.cos() * reach * (1.0 - up * 0.5),
                    a.sin() * reach * (1.0 - up * 0.5),
                    0.3 + up * reach * 1.1 + t * 0.4,
                ];
                let r = (0.32 * (1.0 - t * 0.6) + hsh(i, 53) * 0.12) * (0.6 + t * 0.8);
                let heat = (1.0 - t * 1.3 + hsh(i, 54) * 0.3).clamp(0.0, 1.0);
                let (col, mat) = if heat > 0.75 {
                    (0xffd060ffu32, Mat::Glow)
                } else if heat > 0.4 {
                    (0xe86818ff, Mat::Glow)
                } else if heat > 0.15 {
                    (0xa02a0cff, Mat::Glow)
                } else {
                    (0x3a3230ff, Mat::Matte)
                };
                prim(&mut p, c, c, r, r, col, mat);
            }
        }
        k if (P_FX_FLAME..P_FX_FLAME + 8).contains(&k) => {
            // a pillar of fire rising, roaring, then guttering out
            let f = k - P_FX_FLAME;
            let t = f as f32 / 7.0;
            let rise = (t * 3.0).min(1.0);
            let fade = if t > 0.6 { 1.0 - (t - 0.6) / 0.4 } else { 1.0 };
            for i in 0..16 {
                let fi = i as f32 / 16.0;
                let z = fi * 2.6 * rise;
                let wob = (fi * 9.0 + f as f32 * 1.7).sin() * 0.12;
                let r = (0.42 * (1.0 - fi) + 0.06) * (0.5 + fade * 0.5);
                if fi > fade + 0.1 {
                    continue;
                }
                let col = if fi < 0.25 {
                    0xffc848ffu32
                } else if fi < 0.6 {
                    0xe86414ff
                } else {
                    0xa82a0cff
                };
                for j in 0..3 {
                    let a = j as f32 * 2.1 + f as f32 * 0.9 + fi * 4.0;
                    let c = [a.cos() * r * 0.45 + wob, a.sin() * r * 0.45, z];
                    prim(&mut p, c, c, r * 0.7, r * 0.7, col, Mat::Glow);
                }
            }
        }
        k if (P_FX_HEAL..P_FX_HEAL + 8).contains(&k) => {
            // motes of light spiralling up around the figure
            let t = (k - P_FX_HEAL) as f32 / 8.0;
            for i in 0..22 {
                let fi = i as f32 / 22.0;
                let z = ((fi + t) % 1.0) * 2.2;
                let a = fi * 13.0 + t * std::f32::consts::TAU;
                let r = 0.5 - z * 0.08;
                let size = 0.05 + 0.04 * (1.0 - (z / 2.2 - 0.5).abs() * 2.0);
                let col = if i % 3 == 0 {
                    0x5a9ae0ffu32
                } else {
                    0xd8a838ff
                };
                let c = [a.cos() * r, a.sin() * r, z];
                prim(&mut p, c, c, size, size, col, Mat::Glow);
            }
            let c = [0.0, 0.0, 0.0];
            prim(&mut p, c, c, 0.01, 0.01, 0xffe8a0ff, Mat::Glow);
        }
        k if (P_FX_BALL..P_FX_BALL + 4).contains(&k) => {
            let f = k - P_FX_BALL;
            prim(
                &mut p,
                [0.0, 0.0, 0.3],
                [0.0, 0.0, 0.3],
                0.16,
                0.16,
                0xffd060ff,
                Mat::Glow,
            );
            for i in 0..9 {
                let a = i as f32 / 9.0 * std::f32::consts::TAU + f as f32 * 0.7;
                let l = 0.18 + hsh(i + f * 9, 60) * 0.12;
                let c = [a.cos() * l, a.sin() * l, 0.3 + (a * 2.0).sin() * 0.1];
                prim(
                    &mut p,
                    c,
                    c,
                    0.1,
                    0.1,
                    if i % 2 == 0 { 0xe87018ff } else { 0xb8380cff },
                    Mat::Glow,
                );
            }
        }
        k if (P_CRAG..P_CRAG + 6).contains(&k) => {
            // a mass of broken rock filling its tile
            let v = k - P_CRAG;
            let tall = v >= 3;
            let hgt = if tall { 1.25 } else { 0.7 };
            let rock = [0x52483effu32, 0x473e35ff, 0x5c5146ff, 0x3e362eff];
            bx(
                &mut b,
                [0.0, 0.0, hgt * 0.5],
                [0.72, 0.72, hgt * 0.5],
                rock[0],
                Mat::Matte,
            );
            for i in 0..(if tall { 9 } else { 7 }) {
                let (u, w) = (
                    hsh(i, v * 13 + 1) * 1.1 - 0.55,
                    hsh(i, v * 13 + 2) * 1.1 - 0.55,
                );
                let sz = 0.22 + hsh(i, v * 13 + 3) * 0.25;
                let top = hgt + hsh(i, v * 13 + 4) * (if tall { 0.75 } else { 0.4 });
                b.push(BoxP {
                    c: [u, w, top * 0.5],
                    h: [sz, sz * (0.6 + hsh(i, v * 13 + 5) * 0.5), top * 0.5],
                    yaw: hsh(i, v * 13 + 6) * std::f32::consts::PI,
                    color: rock[(hsh(i, v * 13 + 7) * 4.0) as usize % 4],
                    mat: Mat::Matte,
                });
            }
            // rounded boulders breaking up the blocky mass, and moss
            for i in 0..10 {
                let c = [
                    hsh(i, v * 13 + 8) * 1.3 - 0.65,
                    hsh(i, v * 13 + 9) * 1.3 - 0.65,
                    hsh(i, v * 13 + 10) * (hgt + 0.3),
                ];
                let r = 0.18 + hsh(i, v * 13 + 11) * 0.2;
                prim(
                    &mut p,
                    c,
                    [c[0] + 0.08, c[1], c[2] + 0.05],
                    r,
                    r * 0.9,
                    rock[i % 4],
                    Mat::Matte,
                );
            }
            for i in 0..4 {
                let c = [
                    hsh(i, v * 13 + 12) * 1.0 - 0.5,
                    hsh(i, v * 13 + 13) * 1.0 - 0.5,
                    hgt + 0.05,
                ];
                prim(&mut p, c, c, 0.12, 0.12, 0x3e4a30ff, Mat::Leaf);
            }
        }

        k if k == P_DOOR || k == P_DOOR + 1 => {
            // a doorway in a wall running along y: lintel, one post, the leaf swung inward (-x)
            let side = if k == P_DOOR { -1.0f32 } else { 1.0 };
            bx(
                &mut b,
                [0.0, 0.0, 1.17],
                [0.2, 0.72, 0.07],
                dark,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.0, side * 0.66, 0.55],
                [0.2, 0.07, 0.55],
                dark,
                Mat::Matte,
            );
            let hinge = [-0.12, side * 0.58];
            let ang = std::f32::consts::FRAC_PI_2 * 0.85;
            let dir = [-ang.sin(), -side * ang.cos()];
            let len = 0.55;
            b.push(BoxP {
                c: [
                    hinge[0] + dir[0] * len * 0.5,
                    hinge[1] + dir[1] * len * 0.5,
                    0.52,
                ],
                h: [len * 0.5, 0.035, 0.5],
                yaw: dir[1].atan2(dir[0]),
                color: wood,
                mat: Mat::Matte,
            });
            for z in [0.2f32, 0.85] {
                prim(
                    &mut p,
                    [hinge[0], hinge[1], z],
                    [hinge[0] + dir[0] * 0.3, hinge[1] + dir[1] * 0.3, z],
                    0.02,
                    0.02,
                    0x2e2e30ff,
                    Mat::Metal,
                );
            }
        }
        P_FENCE => {
            // wrought iron: spiked posts and two rails along y
            let iron = 0x34343aff;
            for y in [-0.72f32, -0.24, 0.24, 0.72] {
                prim(
                    &mut p,
                    [0.0, y, 0.0],
                    [0.0, y, 0.95],
                    0.03,
                    0.025,
                    iron,
                    Mat::Metal,
                );
                prim(
                    &mut p,
                    [0.0, y, 0.95],
                    [0.0, y, 1.08],
                    0.04,
                    0.005,
                    iron,
                    Mat::Metal,
                );
            }
            for z in [0.3f32, 0.8] {
                prim(
                    &mut p,
                    [0.0, -0.74, z],
                    [0.0, 0.74, z],
                    0.022,
                    0.022,
                    iron,
                    Mat::Metal,
                );
            }
        }
        k if k == P_GRAVE || k == P_GRAVE + 1 => {
            let stone = 0x7a7872ff;
            // the mound in front (+x), sunken earth
            prim(
                &mut p,
                [0.15, 0.0, 0.0],
                [0.75, 0.0, 0.0],
                0.2,
                0.2,
                0x4a3a28ff,
                Mat::Matte,
            );
            if k == P_GRAVE {
                bx(
                    &mut b,
                    [-0.2, 0.0, 0.38],
                    [0.07, 0.24, 0.38],
                    stone,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [-0.2, -0.17, 0.76],
                    [-0.2, 0.17, 0.76],
                    0.07,
                    0.07,
                    stone,
                    Mat::Matte,
                );
                bx(
                    &mut b,
                    [-0.2, 0.0, 0.06],
                    [0.12, 0.3, 0.06],
                    shade_col(stone, 0.85),
                    Mat::Matte,
                );
                // a carved cross and lichen
                bx(
                    &mut b,
                    [-0.125, 0.0, 0.5],
                    [0.005, 0.025, 0.14],
                    shade_col(stone, 0.6),
                    Mat::Matte,
                );
                bx(
                    &mut b,
                    [-0.125, 0.0, 0.55],
                    [0.005, 0.09, 0.025],
                    shade_col(stone, 0.6),
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [-0.13, 0.15, 0.2],
                    [-0.13, 0.15, 0.2],
                    0.05,
                    0.05,
                    0x4e5a3eff,
                    Mat::Matte,
                );
            } else {
                let w = 0x5a4430ff;
                prim(
                    &mut p,
                    [-0.2, 0.0, 0.0],
                    [-0.22, 0.03, 1.0],
                    0.045,
                    0.04,
                    w,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [-0.21, -0.28, 0.72],
                    [-0.21, 0.3, 0.74],
                    0.04,
                    0.04,
                    w,
                    Mat::Matte,
                );
            }
        }
        P_TENT => {
            // a hide A-frame along y, its mouth at +y
            let hide = 0x8a7050ff;
            let n = 14;
            for i in 0..n {
                let t = i as f32 / n as f32;
                let zt = t * 1.35;
                for s in [-1.0f32, 1.0] {
                    let x = s * (1.0 - t) * 0.85;
                    prim(
                        &mut p,
                        [x, -0.75, zt],
                        [x, 0.7, zt],
                        0.07,
                        0.07,
                        if i % 4 == 0 {
                            shade_col(hide, 0.85)
                        } else {
                            hide
                        },
                        Mat::Matte,
                    );
                }
            }
            for s in [-1.0f32, 1.0] {
                prim(
                    &mut p,
                    [s * 0.15, -0.8, 1.2],
                    [-s * 0.15, -0.8, 1.6],
                    0.035,
                    0.03,
                    dark,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [s * 0.15, 0.75, 1.2],
                    [-s * 0.15, 0.75, 1.6],
                    0.035,
                    0.03,
                    dark,
                    Mat::Matte,
                );
            }
            // the dark opening
            for i in 0..6 {
                let t = i as f32 / 6.0;
                prim(
                    &mut p,
                    [-(1.0 - t) * 0.45, 0.72, t * 0.7],
                    [(1.0 - t) * 0.45, 0.72, t * 0.7],
                    0.06,
                    0.06,
                    0x18120cff,
                    Mat::Matte,
                );
            }
        }
        k if (P_FIRE..P_FIRE + 4).contains(&k) => {
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU;
                let c = [a.cos() * 0.42, a.sin() * 0.42, 0.06];
                prim(&mut p, c, c, 0.1, 0.1, 0x5e5a54ff, Mat::Matte);
            }
            for a in [0.3f32, 1.9, 3.6] {
                prim(
                    &mut p,
                    [a.cos() * 0.35, a.sin() * 0.35, 0.05],
                    [-a.cos() * 0.1, -a.sin() * 0.1, 0.22],
                    0.06,
                    0.05,
                    0x3a2414ff,
                    Mat::Matte,
                );
            }
            prim(
                &mut p,
                [0.0, 0.0, 0.05],
                [0.0, 0.0, 0.08],
                0.22,
                0.2,
                0xb0380cff,
                Mat::Glow,
            );
            flames(&mut p, [0.0, 0.0, 0.12], 0.45, k - P_FIRE);
        }
        P_ANKH => {
            let gold = 0xd8b050ff;
            bx(
                &mut b,
                [0.0, 0.0, 0.12],
                [0.42, 0.42, 0.12],
                0x6e6a62ff,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.0, 0.0, 0.3],
                [0.3, 0.3, 0.06],
                0x7e7a72ff,
                Mat::Matte,
            );
            // the ankh faces the viewer: built in the plane through x = y
            let r2 = std::f32::consts::FRAC_1_SQRT_2;
            let q = |h: f32, v: f32| [h * r2, -h * r2, v];
            prim(
                &mut p,
                q(0.0, 0.36),
                q(0.0, 1.45),
                0.07,
                0.06,
                gold,
                Mat::Metal,
            );
            prim(
                &mut p,
                q(-0.38, 1.45),
                q(0.38, 1.45),
                0.06,
                0.06,
                gold,
                Mat::Metal,
            );
            for i in 0..14 {
                let a = i as f32 / 14.0 * std::f32::consts::TAU;
                let a2 = (i + 1) as f32 / 14.0 * std::f32::consts::TAU;
                prim(
                    &mut p,
                    q(a.sin() * 0.2, 1.72 + a.cos() * 0.25),
                    q(a2.sin() * 0.2, 1.72 + a2.cos() * 0.25),
                    0.06,
                    0.06,
                    gold,
                    Mat::Metal,
                );
            }
        }
        k if (P_LAMP..P_LAMP + 4).contains(&k) => {
            // a stone pillar carrying a shallow iron fire bowl
            let stone = 0x5e5a52ff;
            let iron = 0x2a2624ff;
            bx(
                &mut b,
                [0.0, 0.0, 0.08],
                [0.22, 0.22, 0.08],
                shade_col(stone, 0.85),
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.0, 0.15],
                [0.0, 0.0, 1.2],
                0.12,
                0.1,
                stone,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.0, 1.18],
                [0.0, 0.0, 1.26],
                0.08,
                0.14,
                iron,
                Mat::Metal,
            );
            for i in 0..12 {
                let a = i as f32 / 12.0 * std::f32::consts::TAU;
                let a2 = (i + 1) as f32 / 12.0 * std::f32::consts::TAU;
                prim(
                    &mut p,
                    [a.cos() * 0.24, a.sin() * 0.24, 1.34],
                    [a2.cos() * 0.24, a2.sin() * 0.24, 1.34],
                    0.045,
                    0.045,
                    iron,
                    Mat::Metal,
                );
                prim(
                    &mut p,
                    [a.cos() * 0.12, a.sin() * 0.12, 1.26],
                    [a.cos() * 0.24, a.sin() * 0.24, 1.34],
                    0.04,
                    0.04,
                    iron,
                    Mat::Metal,
                );
            }
            prim(
                &mut p,
                [0.0, 0.0, 1.31],
                [0.0, 0.0, 1.33],
                0.2,
                0.2,
                0xb0380cff,
                Mat::Glow,
            );
            flames(&mut p, [0.0, 0.0, 1.33], 0.38, k - P_LAMP);
        }
        P_BONES => {
            // the remains of someone who fell here: skull, ribs, spine and long bones
            let bone = 0xa89e84ff;
            let old = 0x8a8068ff;
            // spine along y, ribs curving off it
            prim(
                &mut p,
                [0.0, -0.25, 0.04],
                [0.02, 0.2, 0.04],
                0.025,
                0.022,
                old,
                Mat::Bone,
            );
            for i in 0..5 {
                let y = -0.15 + i as f32 * 0.07;
                for s in [-1.0f32, 1.0] {
                    prim(
                        &mut p,
                        [0.0, y, 0.05],
                        [s * 0.1, y + 0.02, 0.09],
                        0.014,
                        0.012,
                        bone,
                        Mat::Bone,
                    );
                    prim(
                        &mut p,
                        [s * 0.1, y + 0.02, 0.09],
                        [s * 0.16, y + 0.05, 0.03],
                        0.012,
                        0.01,
                        bone,
                        Mat::Bone,
                    );
                }
            }
            // skull beyond the neck, turned on its side
            prim(
                &mut p,
                [0.04, 0.3, 0.08],
                [0.04, 0.3, 0.08],
                0.085,
                0.085,
                bone,
                Mat::Bone,
            );
            prim(
                &mut p,
                [0.1, 0.33, 0.05],
                [0.1, 0.33, 0.05],
                0.045,
                0.045,
                old,
                Mat::Bone,
            );
            prim(
                &mut p,
                [0.11, 0.29, 0.1],
                [0.11, 0.29, 0.1],
                0.022,
                0.022,
                0x120c08ff,
                Mat::Matte,
            );
            // a thighbone and an arm flung aside
            prim(
                &mut p,
                [-0.05, -0.28, 0.03],
                [-0.3, -0.45, 0.03],
                0.026,
                0.022,
                bone,
                Mat::Bone,
            );
            prim(
                &mut p,
                [0.08, -0.3, 0.03],
                [0.2, -0.55, 0.03],
                0.024,
                0.02,
                old,
                Mat::Bone,
            );
            prim(
                &mut p,
                [0.15, 0.12, 0.03],
                [0.38, 0.05, 0.03],
                0.018,
                0.015,
                bone,
                Mat::Bone,
            );
        }
        k if k == P_BUSH || k == P_BUSH + 1 => {
            let v = k - P_BUSH;
            let leaf = [0x2e5a22ffu32, 0x3a6a2aff, 0x4a7a32ff];
            for i in 0..14 {
                let a = hsh(i, 40 + v) * std::f32::consts::TAU;
                let r = hsh(i, 42 + v) * 0.35;
                let zc = 0.18 + hsh(i, 44 + v) * 0.35 - r * 0.3;
                let c = [a.cos() * r, a.sin() * r, zc];
                prim(
                    &mut p,
                    c,
                    c,
                    0.14 + hsh(i, 46) * 0.06,
                    0.14,
                    leaf[i % 3],
                    Mat::Hair,
                );
            }
            if v == 1 {
                for i in 0..6 {
                    let a = hsh(i, 48) * std::f32::consts::TAU;
                    let c = [a.cos() * 0.3, a.sin() * 0.3, 0.3 + hsh(i, 49) * 0.2];
                    prim(&mut p, c, c, 0.035, 0.035, 0x6a1a2aff, Mat::Metal);
                }
            }
        }
        k if k == P_BOULDER || k == P_BOULDER + 1 => {
            let v = k - P_BOULDER;
            let rock = [0x6e6a62ffu32, 0x625e56ff, 0x7a766cff];
            for i in 0..(4 + v * 2) {
                let (u, w) = (hsh(i, 30 + v) * 0.6 - 0.3, hsh(i, 32 + v) * 0.6 - 0.3);
                let sz = 0.2 + hsh(i, 34 + v) * 0.2;
                b.push(BoxP {
                    c: [u, w, sz * 0.8],
                    h: [sz, sz * 0.8, sz * 0.8],
                    yaw: hsh(i, 36 + v) * 3.0,
                    color: rock[i % 3],
                    mat: Mat::Matte,
                });
            }
            prim(
                &mut p,
                [0.0, 0.0, 0.5],
                [0.0, 0.0, 0.5],
                0.08,
                0.08,
                0x4e5a3eff,
                Mat::Matte,
            );
        }
        k if k == P_FLOWERS || k == P_FLOWERS + 1 => {
            let v = k - P_FLOWERS;
            let petals = if v == 0 {
                [0xa02030ffu32, 0xc04050ff]
            } else {
                [0xc8b040ff, 0xd8d0c0ff]
            };
            for i in 0..9 {
                let (x, y) = (hsh(i, 20 + v) * 0.9 - 0.45, hsh(i, 22 + v) * 0.9 - 0.45);
                let h = 0.12 + hsh(i, 24 + v) * 0.15;
                prim(
                    &mut p,
                    [x, y, 0.0],
                    [x, y, h],
                    0.012,
                    0.01,
                    0x3a5a22ff,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [x + 0.04, y, 0.0],
                    [x + 0.08, y + 0.03, 0.06],
                    0.03,
                    0.01,
                    0x3a6a26ff,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [x, y, h],
                    [x, y, h],
                    0.04,
                    0.04,
                    petals[i % 2],
                    Mat::Hair,
                );
            }
        }
        P_HOARD => {
            // a mound of coins, a goblet and a few gems
            let gold = [0xd8b050ffu32, 0xc89a40ff, 0xe8c868ff];
            for i in 0..60 {
                let a = hsh(i, 80) * std::f32::consts::TAU;
                let r = hsh(i, 81).sqrt() * 0.7;
                let zc = (0.45 - r * 0.6).max(0.0) + 0.04;
                let c = [a.cos() * r, a.sin() * r, zc];
                prim(&mut p, c, c, 0.07, 0.07, gold[i % 3], Mat::Metal);
            }
            prim(
                &mut p,
                [0.2, -0.25, 0.4],
                [0.2, -0.25, 0.62],
                0.025,
                0.08,
                gold[0],
                Mat::Metal,
            );
            prim(
                &mut p,
                [0.2, -0.25, 0.62],
                [0.2, -0.25, 0.7],
                0.08,
                0.09,
                gold[2],
                Mat::Metal,
            );
            for (i, col) in [0xc01828ffu32, 0x2848c8ff, 0x28a048ff].iter().enumerate() {
                let c = [0.25 - i as f32 * 0.2, 0.2, 0.35 - i as f32 * 0.05];
                prim(&mut p, c, c, 0.05, 0.05, *col, Mat::Metal);
            }
        }
        P_SIGN => {
            // a signpost with a board hanging from an arm, the board facing +x
            prim(
                &mut p,
                [0.0, -0.3, 0.0],
                [0.0, -0.3, 1.5],
                0.05,
                0.045,
                dark,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, -0.3, 1.42],
                [0.0, 0.35, 1.42],
                0.035,
                0.035,
                dark,
                Mat::Matte,
            );
            for y in [-0.15f32, 0.25] {
                prim(
                    &mut p,
                    [0.0, y, 1.4],
                    [0.0, y, 1.25],
                    0.008,
                    0.008,
                    0x2e2e30ff,
                    Mat::Metal,
                );
            }
            bx(
                &mut b,
                [0.0, 0.05, 1.06],
                [0.03, 0.3, 0.19],
                0x7a5630ff,
                Mat::Matte,
            );
            bx(
                &mut b,
                [0.0, 0.05, 1.06],
                [0.035, 0.27, 0.16],
                0x8a6238ff,
                Mat::Matte,
            );
        }
        P_ANVIL => {
            // a stump with an iron anvil and a hammer left on it
            prim(
                &mut p,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.42],
                0.22,
                0.2,
                0x4a3220ff,
                Mat::Matte,
            );
            let iron = 0x4a4e56ff;
            bx(
                &mut b,
                [0.0, 0.0, 0.47],
                [0.11, 0.09, 0.06],
                iron,
                Mat::Metal,
            );
            bx(
                &mut b,
                [0.0, 0.0, 0.6],
                [0.24, 0.12, 0.07],
                iron,
                Mat::Metal,
            );
            prim(
                &mut p,
                [0.24, 0.0, 0.62],
                [0.45, 0.0, 0.64],
                0.07,
                0.01,
                iron,
                Mat::Metal,
            );
            prim(
                &mut p,
                [-0.1, -0.05, 0.69],
                [0.12, 0.1, 0.69],
                0.018,
                0.018,
                0x6a4628ff,
                Mat::Matte,
            );
            bx(
                &mut b,
                [-0.12, -0.06, 0.7],
                [0.03, 0.05, 0.03],
                0x6a6e76ff,
                Mat::Metal,
            );
        }
        k if (P_FORGE..P_FORGE + 4).contains(&k) => {
            // a brick hearth with glowing coals and a hood
            let brick = 0x6a3a2aff;
            bx(
                &mut b,
                [0.0, 0.0, 0.32],
                [0.5, 0.5, 0.32],
                brick,
                Mat::Matte,
            );
            bx(
                &mut b,
                [-0.38, 0.0, 0.9],
                [0.12, 0.5, 0.28],
                shade_col(brick, 0.85),
                Mat::Matte,
            );
            bx(
                &mut b,
                [-0.4, 0.0, 1.45],
                [0.1, 0.2, 0.3],
                shade_col(brick, 0.8),
                Mat::Matte,
            );
            for i in 0..12 {
                let c = [hsh(i, 90) * 0.6 - 0.3, hsh(i, 91) * 0.7 - 0.35, 0.66];
                let hot = (hsh(i + (k - P_FORGE) * 12, 92) * 3.0) as usize;
                let col = [0xffb040ffu32, 0xe05010ff, 0x8a2008ff][hot.min(2)];
                prim(&mut p, c, c, 0.08, 0.08, col, Mat::Glow);
            }
            flames(&mut p, [0.05, 0.0, 0.68], 0.2, k - P_FORGE);
        }
        k if (P_TUFT..P_TUFT + 3).contains(&k) => {
            // a clump of long grass blades leaning every way
            let v = k - P_TUFT;
            let greens = [0x6a8a34ffu32, 0x5a7a2cff, 0x7a9038ff, 0x8a8a44ff];
            let n = 9 + v * 3;
            for i in 0..n {
                let a = hsh(i, 60 + v) * std::f32::consts::TAU;
                let r = hsh(i, 61 + v) * 0.12;
                let b = [a.cos() * r, a.sin() * r, 0.0];
                let lean = 0.06 + hsh(i, 62 + v) * 0.12;
                let h = 0.24 + hsh(i, 63 + v) * 0.24 + v as f32 * 0.06;
                let tip = [b[0] + a.cos() * lean, b[1] + a.sin() * lean, h];
                prim(&mut p, b, tip, 0.022, 0.006, greens[i % 4], Mat::Leaf);
            }
        }
        P_FERN => {
            for i in 0..7 {
                let a = i as f32 / 7.0 * std::f32::consts::TAU + 0.3;
                let mid = [a.cos() * 0.18, a.sin() * 0.18, 0.2];
                let tip = [a.cos() * 0.38, a.sin() * 0.38, 0.1];
                prim(
                    &mut p,
                    [0.0, 0.0, 0.02],
                    mid,
                    0.035,
                    0.05,
                    0x3a5a22ff,
                    Mat::Leaf,
                );
                prim(&mut p, mid, tip, 0.05, 0.015, 0x44662aff, Mat::Leaf);
            }
        }
        k if k == P_PEBBLES || k == P_PEBBLES + 1 => {
            let v = k - P_PEBBLES;
            let rock = [0x6e6a62ffu32, 0x5a564eff, 0x7e786cff];
            for i in 0..(4 + v * 3) {
                let (x, y) = (hsh(i, 70 + v) * 0.5 - 0.25, hsh(i, 71 + v) * 0.5 - 0.25);
                let r = 0.04 + hsh(i, 72 + v) * 0.05;
                prim(
                    &mut p,
                    [x, y, r * 0.5],
                    [x + r * 0.4, y, r * 0.5],
                    r,
                    r,
                    rock[i % 3],
                    Mat::Matte,
                );
            }
        }
        P_TWIGS => {
            let w = 0x4a3420ff;
            prim(
                &mut p,
                [-0.25, -0.05, 0.02],
                [0.25, 0.1, 0.02],
                0.02,
                0.015,
                w,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.03, 0.02],
                [0.08, 0.2, 0.02],
                0.014,
                0.01,
                w,
                Mat::Matte,
            );
            prim(
                &mut p,
                [-0.1, -0.2, 0.02],
                [0.15, -0.12, 0.02],
                0.016,
                0.012,
                0x3e2c1aff,
                Mat::Matte,
            );
            for i in 0..4 {
                let c = [hsh(i, 75) * 0.5 - 0.25, hsh(i, 76) * 0.5 - 0.25, 0.015];
                prim(
                    &mut p,
                    c,
                    c,
                    0.04,
                    0.04,
                    [0x6a4a22ffu32, 0x7a5a28ff][i % 2],
                    Mat::Matte,
                );
            }
        }
        P_FOUNTAIN => {
            // an octagonal stone basin two tiles across: a raised rim around dark water,
            // a column with a shallow bowl, and water falling back into the pool
            let stone = 0x6e6a62ff;
            let water = 0x1e3a5aff;
            let n = 16;
            for i in 0..n {
                let a = (i as f32 + 0.5) / n as f32 * std::f32::consts::TAU;
                let r = 1.12;
                let half = r * (std::f32::consts::PI / n as f32).tan() + 0.02;
                b.push(BoxP {
                    c: [a.cos() * r, a.sin() * r, 0.25],
                    h: [0.12, half, 0.25],
                    yaw: a,
                    color: if i % 2 == 0 {
                        stone
                    } else {
                        shade_col(stone, 0.93)
                    },
                    mat: Mat::Matte,
                });
            }
            for i in 0..4 {
                let yaw = i as f32 / 4.0 * std::f32::consts::FRAC_PI_2;
                b.push(BoxP {
                    c: [0.0, 0.0, 0.2],
                    h: [1.05, 0.44, 0.2],
                    yaw,
                    color: water,
                    mat: Mat::Metal,
                });
            }
            prim(
                &mut p,
                [0.0, 0.0, 0.3],
                [0.0, 0.0, 1.05],
                0.14,
                0.1,
                stone,
                Mat::Matte,
            );
            for i in 0..14 {
                let a = i as f32 / 14.0 * std::f32::consts::TAU;
                let a2 = (i + 1) as f32 / 14.0 * std::f32::consts::TAU;
                prim(
                    &mut p,
                    [a.cos() * 0.4, a.sin() * 0.4, 1.12],
                    [a2.cos() * 0.4, a2.sin() * 0.4, 1.12],
                    0.05,
                    0.05,
                    stone,
                    Mat::Matte,
                );
                prim(
                    &mut p,
                    [a.cos() * 0.12, a.sin() * 0.12, 1.02],
                    [a.cos() * 0.4, a.sin() * 0.4, 1.12],
                    0.045,
                    0.045,
                    shade_col(stone, 0.9),
                    Mat::Matte,
                );
            }
            for i in 0..4 {
                let yaw = i as f32 / 4.0 * std::f32::consts::FRAC_PI_2;
                b.push(BoxP {
                    c: [0.0, 0.0, 1.1],
                    h: [0.36, 0.15, 0.015],
                    yaw,
                    color: 0x2a4e72ff,
                    mat: Mat::Metal,
                });
            }
            prim(
                &mut p,
                [0.0, 0.0, 1.1],
                [0.0, 0.0, 1.38],
                0.06,
                0.03,
                stone,
                Mat::Matte,
            );
            for i in 0..10 {
                let a = i as f32 / 10.0 * std::f32::consts::TAU + 0.2;
                prim(
                    &mut p,
                    [a.cos() * 0.44, a.sin() * 0.44, 1.1],
                    [a.cos() * 0.55, a.sin() * 0.55, 0.42],
                    0.02,
                    0.03,
                    0x7aa4c4ff,
                    Mat::Metal,
                );
            }
        }
        _ => {
            // stump
            prim(
                &mut p,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.3],
                0.2,
                0.18,
                0x4a3220ff,
                Mat::Matte,
            );
            prim(
                &mut p,
                [0.0, 0.0, 0.31],
                [0.0, 0.0, 0.32],
                0.17,
                0.17,
                0x9a7a50ff,
                Mat::Matte,
            );
        }
    }
    if kind == P_SKULLS {
        for q in p.iter_mut() {
            q.a = q.a.map(|v| v * 1.7);
            q.b = q.b.map(|v| v * 1.7);
            q.ra *= 1.7;
            q.rb *= 1.7;
        }
    }
    Figure {
        prims: p,
        boxes: b,
        pitch: 0.0,
        roll: 0.0,
        lift: 0.0,
    }
}

/// Bake every prop, facing down-right (mirror for down-left), into one atlas.
pub fn bake_props() -> Atlas {
    let (fw, fh) = (PROP_FW, PROP_FH);
    let rows = N_PROPS.div_ceil(COLS);
    let (w, h) = (fw * COLS, fh * rows);
    let mut rgba = vec![0u8; w * h * 4];
    for k in 0..N_PROPS {
        NO_INK.with(|f| f.set(k >= P_FX_EXPLO));
        render_frame(
            1.0,
            PROP_ANCHOR,
            prop(k),
            (1.0, 0.0),
            &mut rgba,
            w,
            (k % COLS) * fw,
            (k / COLS) * fh,
            fw,
            fh,
        );
    }
    NO_INK.with(|f| f.set(false));
    Atlas { w, h, fw, fh, rgba }
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

impl Raster {
    /// an oriented box: centre in pixels (+ depth), half-axes in pixels (u, v up, w)
    fn boxp(&mut self, cx: f32, cy: f32, cz: f32, ax: [[f32; 3]; 3], color: u32, mat: u8) {
        let ext_u: f32 = ax.iter().map(|a| a[0].abs()).sum();
        let ext_v: f32 = ax.iter().map(|a| a[1].abs()).sum();
        let x0 = (cx - ext_u).floor().max(0.0) as usize;
        let x1 = ((cx + ext_u).ceil().min(self.w as f32 - 1.0)).max(0.0) as usize;
        let y0 = (cy - ext_v).floor().max(0.0) as usize;
        let y1 = ((cy + ext_v).ceil().min(self.h as f32 - 1.0)).max(0.0) as usize;
        let l2: Vec<f32> = ax
            .iter()
            .map(|a| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).max(1e-6))
            .collect();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let du = x as f32 + 0.5 - cx;
                let dv = -(y as f32 + 0.5 - cy);
                let (mut lo, mut hi, mut face, mut sign) =
                    (f32::NEG_INFINITY, f32::INFINITY, 0usize, 1.0f32);
                let mut miss = false;
                for i in 0..3 {
                    let a = (du * ax[i][0] + dv * ax[i][1]) / l2[i];
                    let b = ax[i][2] / l2[i];
                    if b.abs() < 1e-7 {
                        if a.abs() > 1.0 {
                            miss = true;
                            break;
                        }
                        continue;
                    }
                    let (t1, t2) = ((-1.0 - a) / b, (1.0 - a) / b);
                    let (tl, th) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
                    lo = lo.max(tl);
                    if th < hi {
                        hi = th;
                        face = i;
                        sign = if (a + b * th) > 0.0 { 1.0 } else { -1.0 };
                    }
                }
                if miss || lo > hi || !hi.is_finite() {
                    continue;
                }
                let z = cz + hi;
                let i = y * self.w + x;
                if z > self.depth[i] {
                    let a = ax[face];
                    let m = l2[face].sqrt();
                    self.depth[i] = z;
                    self.nrm[i] = [sign * a[0] / m, sign * a[1] / m, sign * a[2] / m];
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
        6 => (0.22 + 0.95 * diff, 0.0),
        _ => (0.4 + 0.72 * diff, spec_dot.powi(8) * 0.06),
    };
    [
        base[0] * (k + rim) + spec * 255.0,
        base[1] * (k + rim) + spec * 255.0,
        base[2] * (k + rim) + spec * 240.0,
    ]
}

const SS: usize = 2;
/// Pagan ran at 320×200 shown on 4:3 screens, so every pixel stood 20% taller
/// than wide. The host stretches the picture the same way; sprites are baked
/// for it: a true orthographic camera at Pagan's steeper angle (ground at
/// 0.6, heights at 0.8), squeezed vertically by this factor.
pub const ASPECT: f32 = 1.2;
const CAM_G: f32 = 0.6;
const CAM_Z: f32 = 0.8;

thread_local! {
    static NO_INK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A posed figure ready to render: its shapes plus whole-body rotations.
/// a box (for furniture), centred at `c`, half sizes `h`, turned `yaw` about z
struct BoxP {
    c: V3,
    h: V3,
    yaw: f32,
    color: u32,
    mat: Mat,
}

struct Figure {
    prims: Vec<Prim>,
    boxes: Vec<BoxP>,
    /// rotation about the side axis (falling backwards), radians
    pitch: f32,
    /// rotation about the forward axis (falling onto the side)
    roll: f32,
    /// height above the ground after grounding (flying), metres
    lift: f32,
}

fn mat_code(m: Mat) -> u8 {
    match m {
        Mat::Matte => 0,
        Mat::Metal => 1,
        Mat::Glow => 2,
        Mat::Hair => 3,
        Mat::Bone => 4,
        Mat::Membrane => 5,
        Mat::Leaf => 6,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_frame(
    s: f32,
    anchor: (f32, f32),
    fig: Figure,
    dir: (f32, f32),
    out: &mut [u8],
    ow: usize,
    ox: usize,
    oy: usize,
    fw: usize,
    fh: usize,
) {
    let m2t = 1.25 / 1.8 * s;
    let prims = fig.prims;
    let boxes = fig.boxes;
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
    for bx in boxes.iter() {
        for k in 0..8 {
            let (sx, sy, sz) = (
                if k & 1 == 0 { -1.0 } else { 1.0 },
                if k & 2 == 0 { -1.0 } else { 1.0 },
                if k & 4 == 0 { -1.0 } else { 1.0 },
            );
            let (c, sn) = (bx.yaw.cos(), bx.yaw.sin());
            let p = [
                bx.c[0] + sx * bx.h[0] * c - sy * bx.h[1] * sn,
                bx.c[1] + sx * bx.h[0] * sn + sy * bx.h[1] * c,
                bx.c[2] + sz * bx.h[2],
            ];
            lowest = lowest.min(tf(p)[2]);
        }
    }
    if !lowest.is_finite() {
        lowest = 0.0;
    }
    let ground = -lowest + fig.lift;
    let th = dir.1.atan2(dir.0);
    let r2 = std::f32::consts::FRAC_1_SQRT_2;
    let kk = K * SS as f32;
    // rows per output pixel: the vertical supersampling also squeezes by ASPECT
    let ssv = SS as f32 * ASPECT;
    let (aw, ah) = (fw * SS, (fh as f32 * ssv).ceil() as usize + 1);
    let (ax, ay) = (anchor.0 * SS as f32, anchor.1 * ssv);
    // local metres -> view space (u right, v up, w towards the camera), in tile units
    let view = |p: V3, with_ground: bool| -> V3 {
        let x = p[0] * m2t;
        let y = p[1] * m2t;
        let wx = x * th.cos() - y * th.sin();
        let wy = x * th.sin() + y * th.cos();
        let wz = (p[2] + if with_ground { ground } else { 0.0 }) * m2t;
        [
            (wx - wy) * r2,
            -CAM_G * (wx + wy) * r2 + CAM_Z * wz,
            CAM_Z * (wx + wy) * r2 + CAM_G * wz,
        ]
    };
    let mut ras = Raster::new(aw, ah);
    for pr in prims.iter() {
        let a = tf(pr.a);
        let b = tf(pr.b);
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        let step = (pr.ra.min(pr.rb).max(0.012)) * 0.32;
        let n = ((len / step).ceil() as usize).clamp(1, 200);
        let mat = mat_code(pr.mat);
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
            let v = -CAM_G * (wx + wy) * r2 + CAM_Z * wz;
            let w = CAM_Z * (wx + wy) * r2 + CAM_G * wz;
            ras.sphere(ax + u * kk, ay - v * kk, w * kk, rr * kk, pr.color, mat);
        }
    }
    for bx in boxes.iter() {
        let c = view(tf(bx.c), true);
        let (cy, sy) = (bx.yaw.cos(), bx.yaw.sin());
        let axes = [
            view(tf([cy * bx.h[0], sy * bx.h[0], 0.0]), false),
            view(tf([-sy * bx.h[1], cy * bx.h[1], 0.0]), false),
            view(tf([0.0, 0.0, bx.h[2]]), false),
        ];
        let px = |v: V3| [v[0] * kk, v[1] * kk, v[2] * kk];
        ras.boxp(
            ax + c[0] * kk,
            ay - c[1] * kk,
            c[2] * kk,
            [px(axes[0]), px(axes[1]), px(axes[2])],
            bx.color,
            mat_code(bx.mat),
        );
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
            let (r0, r1) = (
                (y as f32 * ssv) as usize,
                (((y + 1) as f32 * ssv) as usize).min(ah),
            );
            let total = (r1 - r0) * SS;
            for ry in r0..r1 {
                for sx in 0..SS {
                    let i = ry * aw + x * SS + sx;
                    if ras.depth[i] > f32::NEG_INFINITY {
                        let mut c = shade_px(ras.col[i], ras.nrm[i], ras.mat[i]);
                        if ras.mat[i] == 6 {
                            // leaves: clumps two pixels across, some catching light, some deep
                            let (lx, ly) = ((x / 2) as u32, (ry / 5) as u32);
                            let hsh2 = |a: u32, b: u32, s: u32| {
                                let mut h =
                                    a.wrapping_mul(0x8da6_b343) ^ b.wrapping_mul(0xd816_3841) ^ s;
                                h ^= h >> 13;
                                h = h.wrapping_mul(0x5bd1_e995);
                                ((h >> 8) & 0xffff) as f32 / 65535.0
                            };
                            let f = 0.62
                                + hsh2(lx, ly, 7) * 0.5
                                + hsh2(x as u32, ry as u32 / 2, 9) * 0.16;
                            c = [c[0] * f, c[1] * f, c[2] * f];
                        }
                        acc = [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]];
                        cnt += 1;
                        if ras.depth[i] > dz {
                            dz = ras.depth[i];
                            cid[y * fw + x] = ras.col[i];
                        }
                    }
                }
            }
            if cnt * 2 >= total {
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
    // (light itself — fire, magic — has no outline)
    let inked = !NO_INK.with(|f| f.get());
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
                        if inked && alpha[j] && ((gap > thr && cid[j] != cid[i]) || gap > thr * 4.0)
                        {
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
                if edge && inked {
                    out[o..o + 4].copy_from_slice(&ink);
                }
            }
        }
    }
}

/// An empty atlas sized for `look`.
pub fn new_atlas(look: &Look) -> Atlas {
    let (fw, fh) = look.frame_size();
    let rows = total_frames(look).div_ceil(COLS);
    let (w, h) = (fw * COLS, fh * rows);
    Atlas {
        w,
        h,
        fw,
        fh,
        rgba: vec![0u8; w * h * 4],
    }
}

/// Render one frame (0..total_frames) of `look` into its atlas.
pub fn bake_frame(look: &Look, atlas: &mut Atlas, idx: usize) {
    let fpd = frames_per_dir(look);
    let r = idx / fpd;
    let mut f = idx % fpd;
    let mut anim = (Idle, 0usize, 1usize);
    for &(a, n) in anims(look).iter() {
        if f < n {
            anim = (a, f, n);
            break;
        }
        f -= n;
    }
    let fig = figure_for(look, anim.0, anim.1, anim.2);
    let (fw, fh, w) = (atlas.fw, atlas.fh, atlas.w);
    render_frame(
        look.s(),
        look.anchor(),
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
    for idx in 0..total_frames(look) {
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
        for idx in 0..total_frames(&l) {
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
        assert_eq!(slot(&l, Die).0 + slot(&l, Die).1, frames_per_dir(&l));
        assert_eq!(slot(&l, Talk), slot(&l, Idle));
        let props = bake_props();
        for k in 0..N_PROPS {
            let (ox, oy) = ((k % COLS) * props.fw, (k / COLS) * props.fh);
            let mut n = 0;
            for y in 0..props.fh {
                for x in 0..props.fw {
                    if props.rgba[((oy + y) * props.w + ox + x) * 4 + 3] > 0 {
                        n += 1;
                    }
                }
            }
            // ground clutter is meant to be small
            let min = if (P_TUFT..P_TWIGS + 1).contains(&k) {
                20
            } else {
                60
            };
            assert!(n > min, "prop {k} nearly empty ({n})");
        }
    }
}
