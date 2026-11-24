//! The worlds that ship in web/packs: each must read cleanly, use only what
//! the engine knows, and be listed in index.json under its own id.
use brodilka::{data, pack::Pack};

#[test]
fn every_bundled_pack_is_clean_and_listed() {
    let index = std::fs::read_to_string("web/packs/index.json").unwrap();
    let mut n = 0;
    for e in std::fs::read_dir("web/packs").unwrap() {
        let path = e.unwrap().path();
        if path.extension().and_then(|x| x.to_str()) != Some("toml") {
            continue;
        }
        let file = path.file_name().unwrap().to_str().unwrap().to_string();
        let p = Pack::parse(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        let w = data::pack_warnings(&p);
        assert!(w.is_empty(), "{file}: {w:?}");
        assert!(
            index.contains(&format!("\"id\": \"{}\", \"file\": \"{file}\"", p.id)),
            "{file} is not listed in index.json as id {}",
            p.id
        );
        n += 1;
    }
    assert!(n > 0);
}
