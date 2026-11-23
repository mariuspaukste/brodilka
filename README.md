# Brodilka

A small sandbox RPG in the spirit of **Ultima Online** (1997): an isometric
world, no classes or levels — your skills rise as you use them.

Written in **Rust**, compiled to **WebAssembly** with no external crates and
no wasm-bindgen. The browser side (`web/`) only replays draw commands on a
canvas and renders the windows ("gumps") from JSON; all game logic is in Rust.

**Play:** https://mariuspaukste.github.io/brodilka/

## What's in it

- **Use-based skills** (12 of them, 0–100, 700 total cap) and stats that grow
  with the skills you use, with titles like *Journeyman Swordsman*.
- **Melee** with war mode, swing timers, hit chance from skill vs. skill,
  Tactics damage, armour and shield parrying.
- **Magery**: spellbook, reagents, mana, power words (*In Por Ylem*, *Vas Flam*…),
  fizzles; buy scrolls to learn new spells. Meditation speeds mana regen.
- **Gathering & crafting**: mine ore in the mountains, chop trees, fish,
  smelt at the forge, smith weapons and armour at the anvil (exceptional items).
- **Town** with blacksmith, mage, provisioner, healer, banker (bank box) and
  guards who strike down any monster that wanders in.
- **Death** turns you into a ghost and leaves your belongings on your corpse;
  find a healer or an ankh shrine, then run back for your things.
- **Places**: a haunted graveyard (skeletons, zombies, a lich), an orc camp,
  troll hills and a dragon's lair. Karma, fame and a day/night cycle.
- Autosaves to the browser's localStorage.

## Look

The renderer aims for Ultima VIII: Pagan — the world is drawn into a ~320-pixel
wide buffer, then a WebGL shader ordered-dithers it into a 256-colour VGA-style
palette and scales it up with nearest-neighbour pixels. Terrain uses seamless,
world-anchored textures generated in Rust (`src/tex.rs`), so no tile grid shows;
terrain types meet on ragged borders. Thick ink outlines, a grim twilight
palette with fog and ash, Voronoi flagstones, brick and timber walls, rock
cliffs, deep blue water, braziers and giant mushrooms. Characters are animated procedurally
(walk cycle, idle breathing, wind-up/slash/recover swings with trails, recoil,
spellcasting poses, front/back facing), and wounds leave blood on the ground.

## Controls

Tap the ground to walk, hold and drag to steer. Tap a creature to attack,
tap people to trade, tap yourself for the paperdoll. Trees, mountains and
water are gathered by tapping them with the right tool.
Keyboard: WASD/arrows, Tab war mode, B bandage, P pack, K skills, M map,
1–9 cast, Esc close.

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --lib
cp target/wasm32-unknown-unknown/release/brodilka.wasm web/
python3 -m http.server -d web   # open http://localhost:8000
```

`cargo test` runs world, combat, magic and save tests natively, and CI runs
`tools/smoke.mjs` against the built `.wasm` in Node before deploying.
`cargo run --example dump -- /tmp/dump` dumps draw commands for inspection.

Pushes to `main` are built and deployed to GitHub Pages by
`.github/workflows/pages.yml`.
