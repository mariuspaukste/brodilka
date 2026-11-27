//! Dev tool: run the game natively and dump draw commands + UI JSON, so the
//! renderer can be checked in a browser without a wasm toolchain.
//! cargo run --example dump -- OUTDIR
use brodilka::data::*;
use brodilka::game::*;
use std::fs;

/// dump without running the game, so effects stay as placed
fn dump_nofx_update(g: &mut Game, dir: &str, name: &str) {
    dump(g, dir, name)
}
fn dump(g: &mut Game, dir: &str, name: &str) {
    g.sprites.budget = 100000;
    g.sprites.bake_ms = f64::INFINITY;
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
    g.snap_camera();
}
fn main() {
    let dir = std::env::args().nth(1).unwrap_or("/tmp/dump".into());
    fs::create_dir_all(&dir).unwrap();
    let (w, h) = (390.0, 780.0 / brodilka::bake::ASPECT);
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
    // interface art, laid out as the host would read it from wasm memory
    {
        let icons = brodilka::bake::icons::bake_icons(|n| {
            brodilka::data::ITEMS
                .iter()
                .find(|d| d.id == n)
                .map(|d| d.color)
                .unwrap_or(0x9a9a9aff)
        });
        let (fr, fw, fh, adv) = brodilka::uiart::font();
        let imgs = [
            (icons.rgba, icons.w, icons.h),
            brodilka::uiart::panel(),
            brodilka::uiart::frame(),
            (fr, fw, fh),
        ];
        let mut meta = String::from("[");
        for (i, (r, w, h)) in imgs.iter().enumerate() {
            fs::write(format!("{dir}/ui{i}.rgba"), r).unwrap();
            meta.push_str(&format!("{}[{w},{h}]", if i > 0 { "," } else { "" }));
        }
        meta.push(']');
        fs::write(format!("{dir}/ui.json"), meta).unwrap();
        fs::write(format!("{dir}/uiadv.bin"), adv).unwrap();
    }
    dump(&mut g, &dir, "create");
    g.create("Marius", false, 0, "warrior", [0, 0, 0]);
    for _ in 0..30 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "town");
    g.zoom_mul = 0.55;
    g.apply_zoom();
    tp(&mut g, 80, 94);
    dump(&mut g, &dir, "town_wide");
    g.zoom_mul = 1.0;
    g.apply_zoom();
    tp(&mut g, 91, 68);
    dump(&mut g, &dir, "inside");
    tp(&mut g, 91, 83);
    dump(&mut g, &dir, "inside_inn");
    let (x, y) = g.map.start;
    tp(&mut g, x, y);
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
    g.arrive(brodilka::game::Pending::Talk(sid));
    for _ in 0..3 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "talk");
    g.win = Win::None;
    // a shore near town, for water and bank art
    {
        use brodilka::world::*;
        let (sx, sy) = g.map.start;
        let mut best = (i32::MAX, sx, sy);
        for y in 2..H - 2 {
            for x in 2..W - 2 {
                let mut wet = 0;
                for dy in 0..6 {
                    for dx in 0..6 {
                        if g.map.tile(x + dx, y + dy) == T_WATER {
                            wet += 1;
                        }
                    }
                }
                if g.map.passable(x, y) && g.map.reach[idx(x, y)] && (16..=26).contains(&wet) {
                    let d = (x - sx).abs() + (y - sy).abs();
                    if d < best.0 {
                        best = (d, x, y);
                    }
                }
            }
        }
        tp(&mut g, best.1, best.2);
    }
    g.zoom_mul = 1.3;
    g.apply_zoom();
    for _ in 0..10 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "coast");
    g.zoom_mul = 1.0;
    g.apply_zoom();
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
    // Pagan's combat moves, close up
    g.zoom_mul = 2.0;
    g.apply_zoom();
    g.ents.iter_mut().skip(1).for_each(|e| {
        e.anim.atk = 0.0;
        e.anim.hurt = 0.0;
    });
    g.ents[0].anim.hurt = 0.0;
    g.ents[0].anim.atk = 0.0;
    g.ents[0].path.clear();
    g.ents[0].dir8 = 2;
    g.pl.blocking = true;
    dump(&mut g, &dir, "move_block");
    g.pl.blocking = false;
    g.ents[0].anim.kick = 0.55;
    dump(&mut g, &dir, "move_kick");
    g.ents[0].anim.kick = 0.0;
    g.ents[0].down_total = 1.9;
    g.ents[0].down_t = 1.2;
    dump(&mut g, &dir, "move_down");
    g.ents[0].down_t = 0.0;
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
    // new props up close: town square statue and well, lair skulls
    let (x0, y0, _, _) = g.map.town;
    tp(&mut g, x0 + 28, y0 + 11);
    g.pl.war = false;
    g.zoom_mul = 1.6;
    g.apply_zoom();
    for _ in 0..10 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "props");
    tp(&mut g, 124, 131);
    for _ in 0..10 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "lair_zoom");
    tp(&mut g, x0 + 13, y0 + 26);
    for _ in 0..10 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "cart");
    // the townsfolk lined up
    tp(&mut g, x0 + 18, y0 + 14);
    let mut k = 0.0;
    for e in g.ents.iter_mut() {
        if matches!(e.k, brodilka::game::EK::Npc(_)) {
            e.x = x0 as f32 + 14.0 + k;
            e.y = y0 as f32 + 11.0 + k * 0.3;
            e.path.clear();
            k += 1.0;
        }
    }
    g.zoom_mul = 1.3;
    g.apply_zoom();
    for _ in 0..3 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "folk");
    g.zoom_mul = 2.0;
    g.apply_zoom();
    for _ in 0..3 {
        g.update(1.0 / 30.0);
    }
    dump(&mut g, &dir, "folk_zoom");
    // the big spells, caught mid-cast
    tp(&mut g, 70, 100);
    g.zoom_mul = 1.3;
    g.apply_zoom();
    let (px, py) = (g.ents[0].x, g.ents[0].y);
    for (kind, dx, dy, k) in [
        (brodilka::data::FX_EXPLOSION, 1.5, -2.5, 0.25),
        (brodilka::data::FX_FLAMESTRIKE, 2.0, 2.5, 0.35),
        (brodilka::data::FX_HEAL, 0.0, 0.0, 0.4),
        (brodilka::data::FX_FIREBALL, -3.0, 0.0, 0.5),
    ] {
        g.fx.push(brodilka::game::Fx {
            kind,
            x0: px
                + if kind == brodilka::data::FX_FIREBALL {
                    -1.0
                } else {
                    dx
                },
            y0: py + dy
                - if kind == brodilka::data::FX_FIREBALL {
                    3.0
                } else {
                    0.0
                },
            x1: px + dx,
            y1: py + dy,
            tgt: 0,
            t: k * 1.0,
            dur: 1.0,
            dmg: 0,
            by_player: false,
        });
    }
    let n = g.fx.len();
    g.sprites.budget = 100000;
    g.sprites.bake_ms = f64::INFINITY;
    g.render();
    g.fx.truncate(n);
    dump_nofx_update(&mut g, &dir, "spells");
    g.zoom_mul = 1.0;
    g.apply_zoom();
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
