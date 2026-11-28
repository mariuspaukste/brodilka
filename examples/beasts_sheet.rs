//! Dev tool: bake the beasts of the streets (and Cthulhu) and write their
//! atlases as raw RGBA for inspection.
//! cargo run --release --example beasts_sheet -- OUTDIR  (prints "name w h fw fh" per atlas)
fn main() {
    let dir = std::env::args().nth(1).unwrap_or("/tmp/beasts".into());
    std::fs::create_dir_all(&dir).unwrap();
    for id in ["pigeon", "dog", "cat", "cow", "pig"] {
        for seed in 0..4 {
            let look = brodilka::render::street_look(id, seed).unwrap();
            let a = brodilka::bake::bake(&look);
            let name = format!("{id}{seed}");
            std::fs::write(format!("{dir}/{name}.rgba"), &a.rgba).unwrap();
            println!("{name} {} {} {} {}", a.w, a.h, a.fw, a.fh);
        }
    }
    let look = brodilka::bake::Look::creature(brodilka::bake::Body::Cthulhu, 0x3e5a3aff, 220);
    let a = brodilka::bake::bake(&look);
    std::fs::write(format!("{dir}/cthulhu.rgba"), &a.rgba).unwrap();
    println!("cthulhu {} {} {} {}", a.w, a.h, a.fw, a.fh);
}
