//! Dev tool: dump the draw commands of a few chosen views, for checking art.
//! cargo run --release --example shot -- OUTDIR "name x y zoom [floor] [hour]" ...
//! (x, y in tiles; with `-` for x and y, the view is of the town's start)
use std::fs;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().unwrap_or("/tmp/shot".into());
    fs::create_dir_all(&dir).unwrap();
    let (w, h) = (390.0, 780.0 / brodilka::bake::ASPECT);
    let mut g = brodilka::game::Game::new(1997);
    g.w = w;
    g.h = h;
    g.pix = 4.0 / 3.0;
    g.ground.enabled = false;
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
    let _ = fs::remove_file(format!("{dir}/atlases.txt"));
    if let Ok(v) = std::env::var("LEVELS") {
        let p: Vec<i32> = v.split(',').map(|s| s.parse().unwrap()).collect();
        for y in p[1] - 5..p[1] + 5 {
            let mut row = String::new();
            for x in p[0] - 5..p[0] + 5 {
                let c = g.map.corners(x, y);
                row.push_str(&format!(
                    " {}{}{}",
                    g.map.level(x, y),
                    if g.map.is_ramp(x, y) { 'r' } else { '.' },
                    if c.iter().any(|&k| k != c[0]) {
                        '*'
                    } else {
                        ' '
                    }
                ));
            }
            eprintln!("{y}:{row}");
        }
    }
    if std::env::var("LIST").is_ok() {
        for b in &g.map.buildings {
            eprintln!(
                "bld {} {} {}x{} style {} vendor {} floors {}",
                b.x,
                b.y,
                b.w,
                b.h,
                b.style,
                b.vendor,
                b.floors.len()
            );
        }
    }
    eprintln!(
        "town {:?} city {:?} start {:?}",
        g.map.town, g.map.city, g.map.start
    );
    g.create("Marius", false, 0, "warrior", [0, 0, 0]);
    for _ in 0..10 {
        g.update(1.0 / 30.0);
    }
    for spec in args {
        let f: Vec<&str> = spec.split_whitespace().collect();
        let name = f[0];
        let (x, y) = if f[1] == "-" {
            g.map.start
        } else {
            (f[1].parse().unwrap(), f[2].parse().unwrap())
        };
        g.zoom_mul = f.get(3).map(|s| s.parse().unwrap()).unwrap_or(1.0);
        let fl: u8 = f.get(4).map(|s| s.parse().unwrap()).unwrap_or(0);
        if let Some(hr) = f.get(5) {
            g.time = 480.0 * hr.parse::<f32>().unwrap();
        }
        g.apply_zoom();
        {
            let p = &mut g.ents[0];
            p.x = x as f32;
            p.y = y as f32;
            p.fl = fl;
            p.path.clear();
            g.camx = (p.x - p.y) * 32.0;
            g.camy = (p.x + p.y) * 16.0;
        }
        g.sprites.budget = 100000;
        g.sprites.bake_ms = f64::INFINITY;
        for _ in 0..4 {
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
    }
}
