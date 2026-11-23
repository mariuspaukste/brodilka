//! Dev tool: run the game natively and dump draw commands + UI JSON, so the
//! renderer can be checked in a browser without a wasm toolchain.
//! cargo run --example dump -- OUTDIR
use brodilka::data::*;
use brodilka::game::*;
use std::fs;

fn dump(g: &mut Game, dir: &str, name: &str) {
    g.sprites.budget = 100000;
    for _ in 0..8 {
        g.render();
    }
    while let Some((id, a)) = g.sprites.pending.pop_front() {
        fs::write(format!("{dir}/atlas{id}.rgba"), &a.rgba).unwrap();
        let mut list = fs::read_to_string(format!("{dir}/atlases.txt")).unwrap_or_default();
        list.push_str(&format!("{id} {} {}\n", a.w, a.h));
        fs::write(format!("{dir}/atlases.txt"), list).unwrap();
    }
    let b: Vec<u8> = g.cmds.cmd.iter().flat_map(|u| u.to_le_bytes()).collect();
    fs::write(format!("{dir}/{name}.cmd"), b).unwrap();
    fs::write(format!("{dir}/{name}.str"), &g.cmds.strs).unwrap();
    fs::write(format!("{dir}/{name}.hud.json"), g.hud_json()).unwrap();
    fs::write(format!("{dir}/{name}.ui.json"), g.ui_json()).unwrap();
}
fn tp(g: &mut Game, x: i32, y: i32) {
    let p = &mut g.ents[0];
    p.x = x as f32;
    p.y = y as f32;
    p.path.clear();
    g.camx = (p.x - p.y) * 32.0;
    g.camy = (p.x + p.y) * 16.0;
}
fn main() {
    let dir = std::env::args().nth(1).unwrap_or("/tmp/dump".into());
    fs::create_dir_all(&dir).unwrap();
    let (w, h) = (390.0, 780.0);
    let mut g = Game::new(1997);
    g.w = w;
    g.h = h;
    g.pix = 4.0 / 3.0;
    g.apply_zoom();
    let texs = brodilka::tex::textures();
    let mut meta = String::from("[");
    for (i, t) in texs.iter().enumerate() {
        fs::write(format!("{dir}/tex{i}.rgba"), &t.rgba).unwrap();
        meta.push_str(&format!(
            "{}[{},{}]",
            if i > 0 { "," } else { "" },
            t.w,
            t.h
        ));
    }
    meta.push(']');
    fs::write(format!("{dir}/tex.json"), meta).unwrap();
    fs::write(format!("{dir}/pal.bin"), brodilka::tex::palette()).unwrap();
    dump(&mut g, &dir, "create");
    g.create("Marius", false, 0, "warrior", [0, 0, 0]);
    for _ in 0..30 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "town");
    g.win = Win::Paperdoll;
    g.touch_ui();
    dump(&mut g, &dir, "paperdoll");
    // vendor window
    let smith = g.ents.iter().find(|e| e.k == EK::Npc(V_SMITH)).unwrap();
    let (sx, sy, sid) = (smith.x as i32, smith.y as i32, smith.id);
    tp(&mut g, sx, sy + 2);
    g.win = Win::Vendor(sid, false);
    g.touch_ui();
    for _ in 0..10 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "vendor");
    g.win = Win::None;
    // graveyard fight
    tp(&mut g, 116, 40);
    g.pl.war = true;
    for _ in 0..90 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "graveyard");
    g.zoom_mul = 2.0;
    g.apply_zoom();
    dump(&mut g, &dir, "graveyard_zoom");
    g.zoom_mul = 1.0;
    g.apply_zoom();
    // forest / wilderness
    tp(&mut g, 44, 70);
    for _ in 0..20 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "forest");
    // orc camp
    tp(&mut g, 40, 116);
    for _ in 0..40 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "orcs");
    // dragon lair entrance
    tp(&mut g, 117, 128);
    for _ in 0..10 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "lair");
    // night in town
    let (x, y) = g.map.start;
    tp(&mut g, x, y);
    g.time = 480.0 * 0.8;
    g.pl.ghost = false;
    g.ents[0].hp = 40;
    for _ in 0..5 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "night");
    eprintln!(
        "ok: kills={} hp={} ghost={}",
        g.pl.kills, g.ents[0].hp, g.pl.ghost
    );
}
