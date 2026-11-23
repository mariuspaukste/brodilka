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
  Tactics damage, armour and shield parrying, plus Pagan's moves: **hold
  Block** to catch blows, **Kick** to shove a foe back (it may go down), and
  heavy blows can **knock you down** until you get back up.
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
cliffs, deep blue water, braziers and giant mushrooms. Every person and creature is a **baked sprite**, as Pagan's were rendered
from 3D models: `src/bake.rs` builds each figure from 3D shapes on a jointed
skeleton, poses it, lights it from the upper left, outlines the silhouette and
renders every frame in 8 directions (5 rendered, 3 mirrored) into an atlas the
first time that figure appears on screen. Animations follow Pagan's set: idle,
walk, combat stance, attack, block, kick, work (mining/chopping/fishing),
cast, hit, die, and falling down and getting back up;
corpses keep the last frame of the death fall. Four-legged animals, the
mongbat, the slime and the dragon have their own rigs (gaits, wing membranes,
squash-and-stretch). Baking is spread over several frames so play never stalls.

The same baker renders the props — oaks, pines, dead trees, stumps, giant
mushrooms, barrels, crates, beds, tables, chairs, bookshelves, hearths and
chests — so the world shares one lighting. Shops and the inn are furnished,
and buildings wear hipped roofs (slate, shingle or tile, with smoking
chimneys) that lift away when you walk inside. Figures also run, glance
around, draw their weapon when combat starts, step in a fighting stance near
a foe, gesture while they talk, and turn through the in-between directions. Wounds leave blood on the ground.

## Controls

Tap the ground to walk, hold and drag to steer. Tap a creature to attack,
tap people to trade, tap yourself for the paperdoll. Trees, mountains and
water are gathered by tapping them with the right tool.
Keyboard: WASD/arrows, Tab war mode, Space (hold) block, F kick, B bandage, P pack, K skills, M map,
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
