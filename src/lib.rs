//! Brodilka — an Ultima Online-flavoured sandbox RPG.
//! All game logic lives in Rust; the browser host (web/main.js) only replays
//! draw commands on a canvas and renders HTML windows from JSON.
pub mod bake;
pub mod data;
pub mod game;
pub mod pack;
pub mod render;
mod rng;
pub mod save;
pub mod sim;
pub mod tex;
pub mod ui;
pub mod uiart;
pub mod world;

use game::Game;

static mut GAME: Option<Game> = None;
static mut INBUF: [u8; 1 << 16] = [0; 1 << 16];
static mut OUT: String = String::new();

fn g() -> &'static mut Game {
    unsafe { (&mut *(&raw mut GAME)).as_mut().expect("init() not called") }
}
fn out(s: String) -> usize {
    unsafe {
        *(&raw mut OUT) = s;
        (&*(&raw const OUT)).len()
    }
}
fn input(len: usize) -> String {
    unsafe {
        let b = &(&*(&raw const INBUF))[..len.min(1 << 16)];
        String::from_utf8_lossy(b).into_owned()
    }
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn js_log(p: *const u8, l: usize);
    fn js_now() -> f64;
}
/// a clock in milliseconds, for keeping work inside a frame
pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js_now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        use std::time::Instant;
        static T0: OnceLock<Instant> = OnceLock::new();
        T0.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    }
}
pub fn log(s: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        js_log(s.as_ptr(), s.len())
    }
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{s}");
}

#[no_mangle]
pub extern "C" fn init(seed: u32) {
    std::panic::set_hook(Box::new(|i| log(&format!("panic: {i}"))));
    let (w, h, zm, pix) = unsafe {
        (&*(&raw const GAME))
            .as_ref()
            .map(|g| (g.w, g.h, g.zoom_mul, g.pix))
            .unwrap_or((800.0, 600.0, 1.0, 1.0))
    };
    // a world pack may fix the seed, so its places always lie where it says
    let seed = pack::get().seed.unwrap_or(seed);
    let mut game = Game::new(seed);
    game.w = w;
    game.h = h;
    game.zoom_mul = zm;
    game.pix = pix;
    game.apply_zoom();
    unsafe {
        *(&raw mut GAME) = Some(game);
    }
}
#[no_mangle]
pub extern "C" fn resize(w: f32, h: f32) {
    let g = g();
    g.w = w;
    g.h = h;
    g.apply_zoom();
}
/// CSS pixels per art pixel used by the host's low-resolution buffer
#[no_mangle]
pub extern "C" fn set_pix(p: f32) {
    let g = g();
    g.pix = p.max(0.25);
    g.apply_zoom();
}
#[no_mangle]
pub extern "C" fn frame(dt: f32) {
    let g = g();
    g.update(dt.clamp(0.0, 0.1));
    g.render();
}
#[no_mangle]
pub extern "C" fn cmd_ptr() -> *const u32 {
    g().cmds.cmd.as_ptr()
}
#[no_mangle]
pub extern "C" fn cmd_len() -> usize {
    g().cmds.cmd.len()
}
#[no_mangle]
pub extern "C" fn str_ptr() -> *const u8 {
    g().cmds.strs.as_ptr()
}
#[no_mangle]
pub extern "C" fn str_len() -> usize {
    g().cmds.strs.len()
}
#[no_mangle]
pub extern "C" fn ptr_down(x: f32, y: f32) {
    let g = g();
    g.ptr_down = true;
    g.ptr_t = 0.0;
    g.ptr_x0 = x;
    g.ptr_y0 = y;
    g.ptr_x = x;
    g.ptr_y = y;
    g.steering = false;
}
#[no_mangle]
pub extern "C" fn ptr_move(x: f32, y: f32) {
    let g = g();
    g.ptr_x = x;
    g.ptr_y = y;
}
#[no_mangle]
pub extern "C" fn ptr_up(x: f32, y: f32) {
    let g = g();
    if g.ptr_down && !g.steering {
        g.tap(x, y);
    }
    g.ptr_down = false;
    g.steering = false;
    g.touch_ui();
}
#[no_mangle]
pub extern "C" fn ptr_cancel() {
    let g = g();
    g.ptr_down = false;
    g.steering = false;
}
/// bit 1 up, 2 right, 4 down, 8 left
#[no_mangle]
pub extern "C" fn keys(bits: u32) {
    g().keys = bits as u8;
}
#[no_mangle]
pub extern "C" fn zoom(f: f32) {
    g().zoom_by(f);
}
#[no_mangle]
pub extern "C" fn inbuf() -> *mut u8 {
    unsafe { (&mut *(&raw mut INBUF)).as_mut_ptr() }
}
#[no_mangle]
pub extern "C" fn ui_cmd(len: usize) {
    let s = input(len);
    g().ui_cmd(&s);
}
#[no_mangle]
pub extern "C" fn hud_ver() -> u32 {
    g().hud_ver
}
#[no_mangle]
pub extern "C" fn ui_ver() -> u32 {
    g().ui_ver
}
#[no_mangle]
pub extern "C" fn hud_json() -> usize {
    out(g().hud_json())
}
#[no_mangle]
pub extern "C" fn ui_json() -> usize {
    out(g().ui_json())
}
#[no_mangle]
pub extern "C" fn out_ptr() -> *const u8 {
    unsafe { (&*(&raw const OUT)).as_ptr() }
}
#[no_mangle]
pub extern "C" fn save() -> usize {
    let g = g();
    if !g.created {
        return 0;
    }
    out(g.save())
}
/// Load a save from the input buffer; recreates the world with the saved seed.
#[no_mangle]
pub extern "C" fn load(len: usize) -> u32 {
    let s = input(len);
    let seed = save::save_seed(&s).unwrap_or(g().seed);
    init(seed);
    g().load(&s) as u32
}
#[no_mangle]
pub extern "C" fn mini_ptr() -> *const u8 {
    g().mini.as_ptr()
}
/// bumped when the minimap changed (the Shadow moving): redraw the radar
#[no_mangle]
pub extern "C" fn mini_ver() -> u32 {
    g().mini_ver
}
#[no_mangle]
pub extern "C" fn map_w() -> i32 {
    world::W
}
#[no_mangle]
pub extern "C" fn px() -> f32 {
    g().ents[0].x
}
#[no_mangle]
pub extern "C" fn py() -> f32 {
    g().ents[0].y
}
/// the avatar's chest on screen, in the game's (squeezed) screen units
#[no_mangle]
pub extern "C" fn avatar_sx() -> f32 {
    let g = g();
    g.scr_h(g.ents[0].x, g.ents[0].y, g.ents[0].anim.vz).0
}
#[no_mangle]
pub extern "C" fn avatar_sy() -> f32 {
    let g = g();
    g.scr_h(g.ents[0].x, g.ents[0].y, g.ents[0].anim.vz).1 - 20.0 * render::ZS * g.zoom
}
/// -1 when no player corpse exists
#[no_mangle]
pub extern "C" fn corpse_x() -> f32 {
    g().corpses
        .iter()
        .find(|c| c.player)
        .map(|c| c.x as f32)
        .unwrap_or(-1.0)
}
#[no_mangle]
pub extern "C" fn corpse_y() -> f32 {
    g().corpses
        .iter()
        .find(|c| c.player)
        .map(|c| c.y as f32)
        .unwrap_or(-1.0)
}
/// number of hostile monsters near the player, for the radar (packed list in OUT)
#[no_mangle]
pub extern "C" fn radar_dots() -> usize {
    let g = g();
    let (px, py) = (g.ents[0].x, g.ents[0].y);
    let mut s = String::new();
    for e in g.ents.iter().skip(1) {
        if e.dead || e.dist(px, py) > 40.0 {
            continue;
        }
        let k = match e.k {
            game::EK::Mon(m) if data::MONS[m].passive => 'a',
            game::EK::Mon(_) => 'm',
            _ => 'n',
        };
        s.push_str(&format!("{}{:.1},{:.1};", k, e.x, e.y));
    }
    out(s)
}

/// What the big map shows besides the land: the places of note (named as the
/// world pack names them, the larger ones marked), where the player is, the
/// way they are walking and their corpse. JSON in OUT.
#[no_mangle]
pub extern "C" fn map_marks() -> usize {
    let g = g();
    const MAJOR: [&str; 8] = [
        "Town",
        "Duskhold",
        "Village",
        "Graveyard",
        "Orc Camp",
        "Troll Hills",
        "Dragon Lair",
        "West river",
    ];
    let pois: Vec<String> = g
        .map
        .pois
        .iter()
        .map(|&(n, x, y)| {
            format!(
                "[{},{},{},{}]",
                ui::esc(&pack::t(n)),
                x,
                y,
                (MAJOR.contains(&n) || n == "East river") as u8
            )
        })
        .collect();
    let p = &g.ents[0];
    let dest = match g.dest {
        Some((x, y)) => format!("[{x},{y}]"),
        None => "null".into(),
    };
    let corpse = match g.corpses.iter().find(|c| c.player) {
        Some(c) => format!("[{},{}]", c.x, c.y),
        None => "null".into(),
    };
    out(format!(
        "{{\"pois\":[{}],\"me\":[{:.2},{:.2}],\"dest\":{},\"corpse\":{}}}",
        pois.join(","),
        p.x,
        p.y,
        dest,
        corpse
    ))
}

// ---------------------------------------------------------------- world packs
static mut PACK_BUF: Vec<u8> = Vec::new();
/// Room for a world pack's text of `len` bytes; the host writes it there.
#[no_mangle]
pub extern "C" fn pack_buf(len: usize) -> *mut u8 {
    unsafe {
        let b = &mut *(&raw mut PACK_BUF);
        *b = vec![0; len];
        b.as_mut_ptr()
    }
}
fn pack_text(len: usize) -> String {
    unsafe {
        let b = &*(&raw const PACK_BUF);
        String::from_utf8_lossy(&b[..len.min(b.len())]).into_owned()
    }
}
fn pack_report(r: Result<pack::Pack, String>) -> String {
    match r {
        Ok(p) => {
            let w: Vec<String> = data::pack_warnings(&p).iter().map(|s| ui::esc(s)).collect();
            format!(
                "{{\"ok\":true,\"pack\":{},\"warnings\":[{}]}}",
                p.info_json(),
                w.join(",")
            )
        }
        Err(e) => format!("{{\"ok\":false,\"error\":{}}}", ui::esc(&e)),
    }
}
/// Read the pack in the pack buffer without using it: a JSON report in OUT.
#[no_mangle]
pub extern "C" fn pack_check(len: usize) -> usize {
    out(pack_report(pack::Pack::parse(&pack_text(len))))
}
/// Put the pack in the pack buffer in play. Call before `init`: the game's
/// tables are built from it once. A JSON report in OUT.
#[no_mangle]
pub extern "C" fn pack_set(len: usize) -> usize {
    let src = pack_text(len);
    let rep = pack_report(pack::Pack::parse(&src));
    if let Err(e) = pack::set(&src) {
        return out(format!("{{\"ok\":false,\"error\":{}}}", ui::esc(&e)));
    }
    out(rep)
}
/// The pack in play, as JSON in OUT.
#[no_mangle]
pub extern "C" fn pack_info() -> usize {
    out(pack::get().info_json())
}

// ---------------------------------------------------------------- art assets
static ART: std::sync::OnceLock<(Vec<tex::Texture>, Vec<u8>)> = std::sync::OnceLock::new();
fn art() -> &'static (Vec<tex::Texture>, Vec<u8>) {
    ART.get_or_init(|| (tex::textures(), tex::palette()))
}
#[no_mangle]
pub extern "C" fn tex_count() -> usize {
    art().0.len()
}
#[no_mangle]
pub extern "C" fn tex_w(i: usize) -> usize {
    art().0[i].w
}
#[no_mangle]
pub extern "C" fn tex_h(i: usize) -> usize {
    art().0[i].h
}
#[no_mangle]
pub extern "C" fn tex_ptr(i: usize) -> *const u8 {
    art().0[i].rgba.as_ptr()
}
/// 256 RGB triples
#[no_mangle]
pub extern "C" fn pal_ptr() -> *const u8 {
    art().1.as_ptr()
}

// ---------------------------------------------------------------- interface art
/// 0 icon sheet, 1 leather panel, 2 bronze frame, 3 pixel font; plus font advances
struct UiArt {
    imgs: Vec<(Vec<u8>, usize, usize)>,
    adv: Vec<u8>,
}
static UI_ART: std::sync::OnceLock<UiArt> = std::sync::OnceLock::new();
fn ui_art() -> &'static UiArt {
    UI_ART.get_or_init(|| {
        let icons = bake::icons::bake_icons(|n| {
            data::ITEMS
                .iter()
                .find(|d| d.id == n)
                .map(|d| d.color)
                .unwrap_or(0x9a9a9aff)
        });
        let (frgba, fw, fh, adv) = uiart::font();
        UiArt {
            imgs: vec![
                (icons.rgba, icons.w, icons.h),
                uiart::panel(),
                uiart::frame(),
                (frgba, fw, fh),
            ],
            adv,
        }
    })
}
#[no_mangle]
pub extern "C" fn ui_img_w(i: usize) -> usize {
    ui_art().imgs[i].1
}
#[no_mangle]
pub extern "C" fn ui_img_h(i: usize) -> usize {
    ui_art().imgs[i].2
}
#[no_mangle]
pub extern "C" fn ui_img_ptr(i: usize) -> *const u8 {
    ui_art().imgs[i].0.as_ptr()
}
/// advance widths of the 95 printable ASCII glyphs
#[no_mangle]
pub extern "C" fn font_adv_ptr() -> *const u8 {
    ui_art().adv.as_ptr()
}

// ---------------------------------------------------------------- baked sprites
/// Hand the next freshly baked atlas to the host: returns its id or -1.
#[no_mangle]
pub extern "C" fn atlas_next() -> i32 {
    let g = g();
    g.sprites.current = g.sprites.pending.pop_front();
    g.sprites
        .current
        .as_ref()
        .map(|(id, _)| *id as i32)
        .unwrap_or(-1)
}
#[no_mangle]
pub extern "C" fn atlas_ptr() -> *const u8 {
    g().sprites
        .current
        .as_ref()
        .map(|(_, a)| a.rgba.as_ptr())
        .unwrap_or(std::ptr::null())
}
#[no_mangle]
pub extern "C" fn atlas_w() -> usize {
    g().sprites.current.as_ref().map(|(_, a)| a.w).unwrap_or(0)
}
#[no_mangle]
pub extern "C" fn atlas_h() -> usize {
    g().sprites.current.as_ref().map(|(_, a)| a.h).unwrap_or(0)
}
/// An atlas the host may drop: its id or -1.
#[no_mangle]
pub extern "C" fn atlas_evicted() -> i32 {
    g().sprites
        .evicted
        .pop_front()
        .map(|i| i as i32)
        .unwrap_or(-1)
}
