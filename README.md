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
- **Town** with blacksmith, mage, provisioner, healer, banker (bank box), an
  inn, a pub (ale and bread) and a library (spellbooks and scrolls, and it buys
  the scrolls you find), and guards who strike down any monster that wanders in.
- **Duskhold**, a walled city north of the town: stone walls with battlements,
  towers and three gates, a castle bailey around the keep where the lord sits on
  his throne, a pub facing the castle gate, a soldiers' camp, and a smith,
  healer, inn, mage and goods shop around the market square. Its own guards
  keep the peace inside the walls, and guard posts on the roads to it keep
  monsters off the road around them.
- **Horses**: a stablemaster east of the town sells bay, grey and black
  horses, saddled and bridled. Tap your horse to mount and yourself to get
  down (or press E); riding is half again as fast as going on foot, and your
  horse follows you about when you're on foot. A heavy blow throws you from the
  saddle, you can't kick from horseback, and if you lose your horse the
  stablemaster will send a lad to fetch it. Wild horses run the western
  meadows, and the stablemaster's own graze in his paddock.
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
corpses keep the last frame of the death fall. Horses have their own rig:
jointed legs that walk in four beats and gallop with the body rocking, a
mane and a tail that streams at speed, grazing, and saddle and bridle; a
rider is baked sitting astride the horse with the reins in hand, and the
whole figure is baked again for each coat. Four-legged animals, the
mongbat, the slime and the dragon have their own rigs (gaits, wing membranes,
squash-and-stretch). Baking is spread over several frames, a few milliseconds
each and nearest figures first, so play never stalls.

The ground under it all is drawn once into 256-pixel chunks that the browser
keeps as canvases and puts down each frame, with the water left as holes and
drawn live beneath so it still flows; the view scrolls in whole art pixels so
the chunks meet exactly. Zoomed out, that is the difference between a few
dozen images and thousands of textured polygons a frame.

The same baker renders the props — oaks, pines, dead trees, stumps, giant
mushrooms, barrels, crates, sacks, skull piles, statues, wells, wall torches,
banners, beds, tables, chairs, bookshelves, hearths, chests and the broken
rock of the mountains — so the world shares one lighting. Shops, the inn, the pub and the library are furnished,
and buildings wear hipped roofs (slate, shingle or tile, with smoking
chimneys) that lift away when you walk inside. Figures also run, glance
around, draw their weapon when combat starts, step in a fighting stance near
a foe, gesture while they talk, and turn through the in-between directions. Wounds leave blood on the ground.

Like Pagan's 320×200 on a 4:3 monitor, the picture's pixels stand 20%
taller than wide, and sprites are baked from the matching steeper camera so
figures keep their proportions. Every object in the world — doors, fences,
graves, the fountain, lamps, forges, tents, campfires — is a baked prop, and
nothing casts a drop shadow.

As in Pagan, no night falls on screen: the sky stays the same grey overcast
and fires give only a faint glow, while the day still turns through Pagan's
six periods (Bloodwatch, Firstebb, Daytide, Threemoons, Lastebb, Eventide).
Hit points and mana show as a red and a blue light that shrink toward zero.
People speak their lines over their heads, and your replies are red
keywords. With a mouse, the pointer is Pagan's direction arrow: short, medium
or long by how far it is from you.

The ground is terraced like Pagan's: land rises in up to four levels with
rock ledges between them. You can't climb a ledge; roads and scattered slopes
lead up and down, and anything standing on high ground is drawn there.
Larger objects (the inn's trestle table, hand carts) span more than one tile,
and anything tall enough to hide you turns see-through when you walk behind it.
Townsfolk differ in build, skin, hair and clothes, and commoners wander the
streets.

The interface is drawn by the game too: inventory and toolbar icons are
modelled and lit like the props, windows are painted leather in bronze
frames, words spoken in the world use a hand-made pixel font, and the big
spells (fireball, explosion, flame strike, healing) are baked frame
animations.

## Controls

Tap the ground to walk, hold and drag to steer. Tap a creature to attack,
tap people to trade, tap yourself for the paperdoll. Trees, mountains and
water are gathered by tapping them with the right tool.
Tap your horse to mount, yourself to dismount.
Keyboard: WASD/arrows, Tab war mode, Space (hold) block, F kick, E mount/dismount, B bandage, P pack, K skills, M map,
1–9 cast, Esc close.

## World packs

The engine plays its own world, but a **world pack** can rename and
re-flavour it without touching the code: one text file, a small subset of
TOML. On the title screen, **Change world…** picks a pack file; the game
checks it, keeps it in this browser and reloads into that world. Each world
keeps its own saved hero, and **Default world** goes back. Packs never need
to be in this repository: they are loaded at run time from the player's own
files.

```toml
[pack]
id = "my-world"              # lowercase letters, digits and '-'
name = "My World"
subtitle = "A grim little realm"
seed = 1409                  # optional: a fixed world seed

[mon.wolf]                   # monsters by id: name, color ("#rrggbb"), hp
name = "a warg"

[item.katana]                # items by id: name, icon
name = "long knife"

[vendor.guard]               # smith mage provisioner healer banker guard folk
title = "the watchman"       # barkeep librarian lord stablemaster:
names = ["Ned", "Tom"]       # title, names, chatter, robe
chatter = ["All quiet."]

[spell.fireball]             # spells by id: name, words
[template.warrior]           # starting templates by id: name, desc

[text]                       # exact replacements of any shown text
"Graveyard" = "Barrow-downs"

[words]                      # then replacements inside shown text, longest first
"ankh shrine" = "waystone"
```

Text spoken in the world uses the pixel font, so accented letters are shown
without their accents there (*Annúminas* → *Annuminas*). The rules —
stats, damage, prices, the map's layout — stay the engine's; a pack changes
names, words, colours and the seed. `src/pack.rs` has the format and parser.

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
