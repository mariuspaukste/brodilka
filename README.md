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

## The map

Tap the radar (or press M) for a map of the whole land, drawn the same way
round as the game: pinch or scroll to zoom, drag to look about, and tap a spot
to walk there. It names the places of note (the great ones always, the small
ones close up, as the world pack names them), shows folk and creatures near
the player as the radar does, and marks where they are walking and their
corpse. Its buttons open the list of places (L) and the Chronicle (C).

## Mountains and the elves

The land is 240 tiles across and 320 from north to south. A great range, rugged and high-footed, runs down
the east, and a lower one across the south; old tracks climb over both through
passes. Three elven places lie beyond them, each guarded and untouched by the
Shadow: a hidden valley in the eastern range, its pale stone halls under
silver-blue roofs with starlit spires; a town of golden trees in a clearing at
the heart of a deep wood; and a haven on the eastern shore, where a white ship
with a swan's prow lies at the quay. The elves are tall, slender and
long-haired, and speak of stars, ships and the long years. Roads with guard
posts run to each.

## The tropics

South of the southern range the land runs on into the tropics
(`src/world/south.rs`), past where the old southern shore used to be: the
land is now 240 tiles across and 320 from north to south, and the north
comes out of a seed exactly as before. The line runs along the range's foot
and bends south round the elves' wood. Below it the grass is a deep, wet
jungle green with fallen blossom, the earth is red laterite, the beaches are
wide and pale, and the trees are coconut palms, banyans hung with aerial
roots, and stands of bamboo; the lower rock is jungle-clad hills. Tigers
hunt the jungle, great spiders and slimes keep to the wet. There is no
snow: winter is dry and hazy, and summer brings the monsoon, rain and
thunderstorms most of the season.

Both old tracks over the southern range come down to its two cities, built
in the Indian style, and a road joins them:

- **Suryagarh**, a fort city of red sandstone: curtain walls with rounded
  merlons, bastions and three gates, the Raja's palace (a marble hall and a
  great white dome over a sandstone hall, the Raja on his throne inside), a
  temple, a stepped tank in the square, and the trades in havelis round it.
- **Kanakpur**, a temple town on a bay: a great temple, ghats stepping down
  into the sea, a tank, havelis and a bazaar along the cross street.

Havelis are lime-washed under flat roof terraces behind pierced parapets,
with domed kiosks (chhatris) at the corners, a stair hut, water pots and
washing on the line; their windows are arched, latticed jharokhas under
little hoods. A temple's shikhara climbs in curved courses with a rib up
each face to a ribbed amalaka, a gilt pot and a saffron flag, and inside it
the priest heals the sick and raises the dead. The streets are red
flagstone and the rugs are cotton dhurries. Its people have their own names
and talk: sipahi guards in red coats and saffron turbans with brass shields
and talwars, men in turbans and white, women in saris with the end over the
shoulder or the head, the Raja in gold, the priest in saffron.

## Light and weather

A day runs through six periods (`src/game/sky.rs`). Dusk reddens and darkens
into night, when only lamps, hearths, fires and magic give light, and dawn
brings it back. The weather turns twice a day and follows the season: showers
and thunderstorms with lightning in spring and autumn, fewer in summer, fog,
and snow in winter. It fades in and out, shows beside the hour in the HUD, and
the journal says when it turns; under a roof nothing falls. Walking into thick
forest dims the light to a green gloom, with sun coming through in patches.

## Art details

The ground is painted in two greens, a deep one and a sunlit olive, with
blades of grass, clover and the odd flower; tracks have pebbles, ruts and dry
grass at their edges; water swells and catches the sky, with a line of foam
where it meets the bank; rock is cracked and spotted with lichen. Woods mix
oaks, pines and white-barked birches, and the oaks turn amber in autumn and
stand bare in winter. Hedges are banks of leafy clumps.

Plastered houses are half-timbered; windows have frames, mullions, painted
shutters and, on the ground floor, boxes of flowers, and they glow warm after
dark. Roofs are laid tile by tile, a few darker, some green with moss, under
an eave that shades the wall below. Halls and houses have a woven rug on the
ground floor.

Timber and masonry are textured as they are baked: wood shows grain along
each board, dark seams between planks and the odd knot, and stone is laid in
courses of blocks, each its own shade, with mortar between. Houses are
furnished to suit who lives there (`src/bake/furniture.rs`): four-poster
beds under patchwork quilts, ladder-back chairs, tables laid for supper,
bookcases with gilt spines, jars, scrolls and a skull, iron-bound chests,
and chimney breasts with a mantel, candlesticks and a pot by the fire. Then
the trades: the smith shows a weapon rack and suits of mail, the mage brews
in a cauldron beside an armillary sphere, the healer hangs herbs to dry, the
banker and the librarian write at desks with pigeonholes, the pubs keep
dressers of crockery and jars of pickles behind the bar, and the small folk
have spinning wheels, wardrobes, dressers and potted plants. Iron
candle-stands light the halls, and the castle keeps its lord's arms and
armour. The burrow mounds catch the light on one side and fall into
shade on the other.

Figures are lit warm on the lit side and cool in the shade, the top edge of
each shape catches the light, and outlines take a deep shade of the colour
they border rather than plain black. Orcs and trolls have tusks and wear
pelts over their shoulders, orcs a black topknot; elves have long pointed
ears. Giant spiders hunt the deep woods: eight jointed legs walking in two
alternating sets, a cluster of red eyes, fangs that open to strike.

## The market

Duskhold's square holds a market round the fountain: six stalls under
striped awnings, each with its keeper standing at the end of the counter: the
herb-seller (garlic, ginseng, mandrake, blood moss), the baker, the
fishmonger, the clothier (robes and leather), the alewife and the
apothecary (bandages and potions). Each keeper sells only what is on their own
stall, and cries it across the square. A crowd of townsfolk, many with
baskets on their arms, goes from stall to stall: they stop at a free one,
look the wares over, haggle with the keeper, and move on to the next, or
stroll across the square. After dusk the wares are covered, a lantern is lit
on each stall and the crowd only wanders. (`game/market.rs`,
`world::Map::market`.)

## The village and the rivers

Two rivers run through the land: one from the northern hills down through the
west to the sea, one from the eastern uplands to the south-eastern shore.
Where a road crosses one there is a bridge, and fords here and there keep the
land beyond them within reach.

West of the town, along a lane, the small folk live in a village of burrow
houses: earthen walls dug into turf mounds, round doors and round lit windows,
a chimney pot smoking on each mound and a flower bed by every door. There is an
inn, a mill whose wheel turns in the river beside it, and a great tree hung
with lanterns over the tables on the green. The village is a peaceful place: a
thick hedge runs all round it, its gates are shut to beasts like any door, a
hayward keeps the gate, nothing spawns inside, and no Shadow falls there. The
small folk stand about chest-high to a man, barefoot and curly-haired, and talk
of gardens, elevenses and parties rather than war. (`world::Map::village`.)

## Doors and storeys

Doors are shut until someone opens them (`src/game/storeys.rs`). The player
and the townsfolk open a door by walking into it, and it swings shut by
itself a few seconds later once the doorway is clear; tap a door close by to
open or shut it. Beasts and monsters cannot open doors, so a shut door keeps
them out.

Some buildings stand more than one storey high: rooms over the inn and the
pubs, a ward over the healer's, a gallery over the library, and a three-storey
mage's tower in the town and in the city. Stairs in the back corner lead up;
walk onto them to climb, step off and on again to go down. Paths never take
stairs on the way somewhere else. Upstairs the view is cut away above the
player's floor: the storeys below show as a solid block under the floorboards,
the stairwell opens onto the flight below, and only those on the same floor
are seen. From outside, the walls rise a storey at a time, with a beam and
windows on each, under a roof raised to match. A building's upper floors are
`world::Floor`s (walls, furniture and stairs over its footprint); every
entity stands on a storey (`Ent::fl`), and the floor is kept in the save.

## World packs

The engine plays its own world, but a **world pack** can rename and
re-flavour it without touching the code: one text file, a small subset of
TOML. On the title screen, **World** picks one: the engine's own, a world
that ships with the game (`web/packs/`, listed in `web/packs/index.json`), or
**From a file…** for a pack on the player's device, which never has to be in
this repository. The game checks the pack, keeps the choice in this browser
and reloads into that world. Each world keeps its own saved hero.

Shipped worlds:

- **The Waning North** (`web/packs/waning-north.toml`): an old northern
  kingdom in its last age. Thornmere at the crossroads, the King's seat at
  Caer Ardun, the Howe-king in the burial mounds and the Witch-realm of
  Vathmor sending its orcs over the fells.

```toml
[pack]
id = "my-world"              # lowercase letters, digits and '-'
name = "My World"
subtitle = "A grim little realm"
seed = 1409                  # optional: a fixed world seed

[mon.wolf]                   # monsters by id: name, color ("#rrggbb"), hp
name = "a warg"

[item.katana]                # items by id: name, icon, plural
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
names, words, colours and the seed. `src/pack.rs` has the format and parser, and
`cargo run --example pack_check -- my-world.toml` checks a pack.

## The living world

The world goes on without the player (`src/sim.rs`). A calendar runs with
the day cycle, one day per six periods. Each hostile place on the map
(graveyard, orc camp, troll hills, the lair and the wilds) has a strength
from 0 to 100 that sets how many of its creatures roam and how quickly they
return. Slaying them weakens the place, and clearing it empties it; left
alone, it grows back toward the strength of the power behind it, a region
off the map that drifts stronger or weaker day by day. Crossing a threshold
posts news to the journal, and townsfolk repeat the latest as rumours. The
**Chronicle** (C, or the tab beside the map) shows the date, every region
and place, trade, who is on the roads, and the news. All of it is kept in
the save.

The roads carry the world's traffic (`src/game/travel.rs`):

- **Raids.** A place at strength 60 or more sends a warband down its road
  toward the town, up to twice a day at full strength. It stops short of the
  guards and burns the outlying farms for a minute and a half; if it lives
  through that it marches home, the place and its region grow bolder, and
  trade falls. Cut it down first and the raid is broken: the region is
  shaken, and the town speaks of the one who did it.
- **Caravans.** About once a day a merchant and a drover walk from the city
  to the town; the merchant trades on the road like any provisioner. Each
  arrival makes trade brisker, and shop prices follow trade, from ×1.3 when
  it is poor to ×0.7 when brisk. When the wilds are restless, wolves may fall
  on a caravan on the road: drive them off within a minute (the merchant pays
  for the help) or the caravan is lost.

**The Shadow** (`src/game/shadow.rs`). A region marked `shadow = true`
darkens the land around each of its places, further as the region and the
place grow strong, and draws back as they are cleared. The walls and hedges
of the town and city keep it out, road posts and waystones thin it. In it the
ground darkens and the fog and ash thicken, trees wither (and green again when
it lifts), the creatures of the dark mend their wounds, and the player's heal
slowly. The radar shows it, and the Chronicle tells how much of the land it
holds. The ground chunks under it are drawn again wherever it moves.

A world pack sets this up:

```toml
[sim]
start = "640-03-01"          # year-month-day; months are 30 days
era = "W.Y."
months = ["Frostmonth", …]   # all twelve

[region.vathmor]             # any id: name, strength, drift, min, rise, fall, shadow
name = "the Witch-realm of Vathmor"
strength = 62
drift = 4                    # tenths of a point a day
shadow = true                # its places cast the Shadow
rise = "{name} gathers its strength."

[site.orcs]                  # graveyard orcs hills lair wilds: name, region,
region = "vathmor"           # strength, base, regrow, rise, fall, and for raids
rise = "More orcs have come to {name}."   # raid, hit, broken ({town} too)
```

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
`cargo run --release --example props_sheet -- /tmp/props.rgba` bakes the prop
atlas alone and writes it as raw RGBA (it prints the width and height).

Pushes to `main` are built and deployed to GitHub Pages by
`.github/workflows/pages.yml`.
