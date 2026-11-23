//! Brodilka — an Ultima Online-flavoured sandbox RPG.
//! All game logic lives in Rust; the browser host (web/main.js) only replays
//! draw commands on a canvas and renders HTML windows from JSON.
pub mod data;
pub mod game;
pub mod render;
mod rng;
pub mod save;
pub mod ui;
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
    g().pix = p.max(1.0);
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
