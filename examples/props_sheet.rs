//! Dev tool: bake the prop atlas and write it as raw RGBA for inspection.
//! cargo run --release --example props_sheet -- OUT.rgba  (prints "w h")
fn main() {
    let out = std::env::args().nth(1).unwrap_or("/tmp/props.rgba".into());
    let a = brodilka::bake::bake_props();
    std::fs::write(&out, &a.rgba).unwrap();
    println!("{} {}", a.w, a.h);
}
