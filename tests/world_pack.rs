//! A world pack in play: its own process, since a pack is set once.
use brodilka::data::*;
use brodilka::game::Game;
use brodilka::pack;

const PACK: &str = r##"
[pack]
id = "test-realm"
name = "Test Realm"
seed = 4242

[mon.wolf]
name = "a warg"

[vendor.lord]
title = "of Testhold"

[template.warrior]
name = "Ranger"

[text]
"Graveyard" = "Barrow-downs"
"Daytide" = "Morning"

[words]
"Ankh shrine" = "Waystone"
"Duskhold" = "Testhold"
"##;

#[test]
fn a_pack_renames_the_world_it_is_played_in() {
    pack::set(PACK).unwrap();
    // a second pack is refused once one is in play
    assert!(pack::set(PACK).is_err());
    assert_eq!(pack::get().seed, Some(4242));

    assert_eq!(MONS[mon_idx("wolf")].name, "a warg");
    assert_eq!(VENDORS[V_LORD].title, "of Testhold");

    let mut g = Game::new(4242);
    g.w = 390.0;
    g.h = 650.0;
    g.apply_zoom();
    let u = g.ui_json();
    assert!(u.contains("\"Ranger\""), "templates on the create screen");
    g.ui_cmd("create:Tester:m:0:warrior");
    for _ in 0..60 {
        g.update(1.0 / 30.0);
    }
    g.ui_cmd("open:map");
    let u = g.ui_json();
    assert!(u.contains("Barrow-downs"), "{u}");
    assert!(u.contains("Waystone"), "{u}");
    assert!(u.contains("Testhold"), "{u}");
    assert!(!u.contains("Duskhold"), "{u}");
    assert!(!u.contains("Ankh shrine"), "{u}");
}
