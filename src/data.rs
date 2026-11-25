//! Static game data: skills, items, spells, monsters, vendors, templates, recipes.

pub const SWORDS: usize = 0;
pub const MACE: usize = 1;
pub const WRESTLING: usize = 2;
pub const TACTICS: usize = 3;
pub const PARRY: usize = 4;
pub const MAGERY: usize = 5;
pub const MEDITATION: usize = 6;
pub const HEALING: usize = 7;
pub const MINING: usize = 8;
pub const LUMBER: usize = 9;
pub const SMITHING: usize = 10;
pub const FISHING: usize = 11;
pub const N_SKILLS: usize = 12;

pub const SKILL_ID: [&str; N_SKILLS] = [
    "swords",
    "mace",
    "wrestling",
    "tactics",
    "parry",
    "magery",
    "meditation",
    "healing",
    "mining",
    "lumber",
    "smithing",
    "fishing",
];
pub const SKILL_NAME: [&str; N_SKILLS] = [
    "Swordsmanship",
    "Mace Fighting",
    "Wrestling",
    "Tactics",
    "Parrying",
    "Magery",
    "Meditation",
    "Healing",
    "Mining",
    "Lumberjacking",
    "Blacksmithy",
    "Fishing",
];
/// Primary stat trained by the skill: 0 str, 1 dex, 2 int.
pub const SKILL_STAT: [usize; N_SKILLS] = [0, 0, 1, 0, 1, 2, 2, 2, 0, 0, 0, 1];
pub const SKILL_TITLE: [&str; N_SKILLS] = [
    "Swordsman",
    "Armsman",
    "Wrestler",
    "Tactician",
    "Duelist",
    "Mage",
    "Stoic",
    "Healer",
    "Miner",
    "Woodsman",
    "Blacksmith",
    "Fisherman",
];
pub const STAT_NAME: [&str; 3] = ["strength", "dexterity", "intelligence"];
/// Skill values are stored in tenths: 1000 = 100.0
pub const SKILL_CAP: i32 = 7000;
pub const STAT_CAP: i32 = 225;

pub fn skill_rank(v: i32) -> &'static str {
    match v {
        1000.. => "Grandmaster",
        900.. => "Master",
        800.. => "Adept",
        700.. => "Expert",
        600.. => "Journeyman",
        500.. => "Apprentice",
        400.. => "Novice",
        300.. => "Neophyte",
        _ => "",
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum K {
    Gold,
    Weapon,
    Armor,
    Shield,
    Tool,
    Reagent,
    Resource,
    Consumable,
    Book,
    Scroll,
    Gem,
    /// a riding horse: bought from the stablemaster, it is led out, not packed
    Mount,
}

#[derive(Clone)]
pub struct ItemDef {
    pub id: &'static str,
    pub name: &'static str,
    pub k: K,
    pub skill: usize,
    pub dmg: (i32, i32),
    pub speed: f32,
    pub ar: i32,
    pub price: i32,
    /// skill this works as a tool for, or -1
    pub tool: i32,
    pub two: bool,
    pub stack: bool,
    pub icon: &'static str,
    /// spell index for scrolls, or -1
    pub spell: i32,
    /// colour used when worn (0xRRGGBBAA)
    pub color: u32,
    /// the name of several, when adding "s" will not do (from a world pack)
    pub plural: &'static str,
}

const B: ItemDef = ItemDef {
    id: "",
    name: "",
    k: K::Resource,
    skill: WRESTLING,
    dmg: (0, 0),
    speed: 2.0,
    ar: 0,
    price: 0,
    tool: -1,
    two: false,
    stack: false,
    icon: "",
    spell: -1,
    color: 0x9a9a9aff,
    plural: "",
};

static ITEMS_DEF: &[ItemDef] = &[
    ItemDef {
        id: "fists",
        name: "fists",
        k: K::Weapon,
        skill: WRESTLING,
        dmg: (1, 6),
        speed: 1.8,
        ..B
    },
    ItemDef {
        id: "dagger",
        name: "dagger",
        k: K::Weapon,
        skill: SWORDS,
        dmg: (2, 8),
        speed: 1.7,
        price: 21,
        icon: "🗡️",
        ..B
    },
    ItemDef {
        id: "katana",
        name: "katana",
        k: K::Weapon,
        skill: SWORDS,
        dmg: (5, 13),
        speed: 2.2,
        price: 33,
        icon: "⚔️",
        ..B
    },
    ItemDef {
        id: "broadsword",
        name: "broadsword",
        k: K::Weapon,
        skill: SWORDS,
        dmg: (6, 17),
        speed: 2.8,
        price: 35,
        icon: "⚔️",
        ..B
    },
    ItemDef {
        id: "longsword",
        name: "longsword",
        k: K::Weapon,
        skill: SWORDS,
        dmg: (8, 19),
        speed: 3.1,
        price: 55,
        icon: "⚔️",
        ..B
    },
    ItemDef {
        id: "hatchet",
        name: "hatchet",
        k: K::Weapon,
        skill: SWORDS,
        dmg: (3, 11),
        speed: 2.6,
        price: 25,
        tool: LUMBER as i32,
        icon: "🪓",
        ..B
    },
    ItemDef {
        id: "mace",
        name: "mace",
        k: K::Weapon,
        skill: MACE,
        dmg: (4, 12),
        speed: 2.3,
        price: 28,
        icon: "🔨",
        ..B
    },
    ItemDef {
        id: "warhammer",
        name: "war hammer",
        k: K::Weapon,
        skill: MACE,
        dmg: (10, 24),
        speed: 3.6,
        price: 70,
        two: true,
        icon: "🔨",
        ..B
    },
    ItemDef {
        id: "pickaxe",
        name: "pickaxe",
        k: K::Weapon,
        skill: MACE,
        dmg: (1, 12),
        speed: 3.0,
        price: 25,
        tool: MINING as i32,
        icon: "⛏️",
        ..B
    },
    ItemDef {
        id: "smithhammer",
        name: "smith's hammer",
        k: K::Tool,
        price: 23,
        tool: SMITHING as i32,
        icon: "🔨",
        ..B
    },
    ItemDef {
        id: "fishingpole",
        name: "fishing pole",
        k: K::Tool,
        price: 15,
        tool: FISHING as i32,
        icon: "🎣",
        ..B
    },
    ItemDef {
        id: "robe",
        name: "robe",
        k: K::Armor,
        ar: 2,
        price: 12,
        icon: "👘",
        color: 0x3a4fa0ff,
        ..B
    },
    ItemDef {
        id: "leather",
        name: "leather tunic",
        k: K::Armor,
        ar: 12,
        price: 40,
        icon: "🦺",
        color: 0x7a5230ff,
        ..B
    },
    ItemDef {
        id: "ringmail",
        name: "ringmail tunic",
        k: K::Armor,
        ar: 22,
        price: 90,
        icon: "🛡️",
        color: 0x8a8f96ff,
        ..B
    },
    ItemDef {
        id: "chainmail",
        name: "chainmail tunic",
        k: K::Armor,
        ar: 28,
        price: 140,
        icon: "🛡️",
        color: 0xa0a6aeff,
        ..B
    },
    ItemDef {
        id: "platemail",
        name: "platemail",
        k: K::Armor,
        ar: 38,
        price: 250,
        icon: "🛡️",
        color: 0xd0d6dcff,
        ..B
    },
    ItemDef {
        id: "buckler",
        name: "buckler",
        k: K::Shield,
        ar: 7,
        price: 30,
        icon: "🛡️",
        color: 0x9a7a40ff,
        ..B
    },
    ItemDef {
        id: "heater",
        name: "heater shield",
        k: K::Shield,
        ar: 14,
        price: 60,
        icon: "🛡️",
        color: 0xb0b8c0ff,
        ..B
    },
    ItemDef {
        id: "bandage",
        name: "bandage",
        k: K::Consumable,
        price: 2,
        stack: true,
        icon: "🩹",
        ..B
    },
    ItemDef {
        id: "healpotion",
        name: "heal potion",
        k: K::Consumable,
        price: 20,
        stack: true,
        icon: "🧪",
        ..B
    },
    ItemDef {
        id: "fish",
        name: "raw fish",
        k: K::Consumable,
        price: 6,
        stack: true,
        icon: "🐟",
        ..B
    },
    ItemDef {
        id: "ale",
        name: "tankard of ale",
        k: K::Consumable,
        price: 3,
        stack: true,
        icon: "🍺",
        ..B
    },
    ItemDef {
        id: "bread",
        name: "loaf of bread",
        k: K::Consumable,
        price: 5,
        stack: true,
        icon: "🍞",
        ..B
    },
    ItemDef {
        id: "ore",
        name: "iron ore",
        k: K::Resource,
        price: 6,
        stack: true,
        icon: "🪨",
        ..B
    },
    ItemDef {
        id: "ingot",
        name: "iron ingot",
        k: K::Resource,
        price: 8,
        stack: true,
        icon: "🧱",
        ..B
    },
    ItemDef {
        id: "logs",
        name: "log",
        k: K::Resource,
        price: 4,
        stack: true,
        icon: "🪵",
        ..B
    },
    ItemDef {
        id: "spellbook",
        name: "spellbook",
        k: K::Book,
        price: 25,
        icon: "📖",
        ..B
    },
    ItemDef {
        id: "ruby",
        name: "ruby",
        k: K::Gem,
        price: 120,
        stack: true,
        icon: "🔴",
        ..B
    },
    ItemDef {
        id: "sapphire",
        name: "sapphire",
        k: K::Gem,
        price: 100,
        stack: true,
        icon: "🔷",
        ..B
    },
    ItemDef {
        id: "diamond",
        name: "diamond",
        k: K::Gem,
        price: 240,
        stack: true,
        icon: "💎",
        ..B
    },
    ItemDef {
        id: "bp",
        name: "black pearl",
        k: K::Reagent,
        price: 5,
        stack: true,
        icon: "⚫",
        ..B
    },
    ItemDef {
        id: "bm",
        name: "blood moss",
        k: K::Reagent,
        price: 5,
        stack: true,
        icon: "🟥",
        ..B
    },
    ItemDef {
        id: "ga",
        name: "garlic",
        k: K::Reagent,
        price: 3,
        stack: true,
        icon: "🧄",
        ..B
    },
    ItemDef {
        id: "gi",
        name: "ginseng",
        k: K::Reagent,
        price: 3,
        stack: true,
        icon: "🌱",
        ..B
    },
    ItemDef {
        id: "mr",
        name: "mandrake root",
        k: K::Reagent,
        price: 3,
        stack: true,
        icon: "🥕",
        ..B
    },
    ItemDef {
        id: "ns",
        name: "nightshade",
        k: K::Reagent,
        price: 3,
        stack: true,
        icon: "🍇",
        ..B
    },
    ItemDef {
        id: "ss",
        name: "spiders' silk",
        k: K::Reagent,
        price: 3,
        stack: true,
        icon: "🕸️",
        ..B
    },
    ItemDef {
        id: "sa",
        name: "sulfurous ash",
        k: K::Reagent,
        price: 3,
        stack: true,
        icon: "🟨",
        ..B
    },
    ItemDef {
        id: "scroll_magicarrow",
        name: "Magic Arrow scroll",
        k: K::Scroll,
        price: 12,
        stack: true,
        icon: "📜",
        spell: 0,
        ..B
    },
    ItemDef {
        id: "scroll_heal",
        name: "Heal scroll",
        k: K::Scroll,
        price: 12,
        stack: true,
        icon: "📜",
        spell: 1,
        ..B
    },
    ItemDef {
        id: "scroll_harm",
        name: "Harm scroll",
        k: K::Scroll,
        price: 18,
        stack: true,
        icon: "📜",
        spell: 2,
        ..B
    },
    ItemDef {
        id: "scroll_fireball",
        name: "Fireball scroll",
        k: K::Scroll,
        price: 28,
        stack: true,
        icon: "📜",
        spell: 3,
        ..B
    },
    ItemDef {
        id: "scroll_greaterheal",
        name: "Greater Heal scroll",
        k: K::Scroll,
        price: 40,
        stack: true,
        icon: "📜",
        spell: 4,
        ..B
    },
    ItemDef {
        id: "scroll_lightning",
        name: "Lightning scroll",
        k: K::Scroll,
        price: 40,
        stack: true,
        icon: "📜",
        spell: 5,
        ..B
    },
    ItemDef {
        id: "scroll_energybolt",
        name: "Energy Bolt scroll",
        k: K::Scroll,
        price: 90,
        stack: true,
        icon: "📜",
        spell: 6,
        ..B
    },
    ItemDef {
        id: "scroll_explosion",
        name: "Explosion scroll",
        k: K::Scroll,
        price: 90,
        stack: true,
        icon: "📜",
        spell: 7,
        ..B
    },
    ItemDef {
        id: "scroll_flamestrike",
        name: "Flamestrike scroll",
        k: K::Scroll,
        price: 140,
        stack: true,
        icon: "📜",
        spell: 8,
        ..B
    },
    ItemDef {
        id: "horse_bay",
        name: "bay horse",
        k: K::Mount,
        price: 550,
        icon: "🐎",
        ..B
    },
    ItemDef {
        id: "horse_grey",
        name: "grey horse",
        k: K::Mount,
        price: 620,
        icon: "🐎",
        ..B
    },
    ItemDef {
        id: "horse_black",
        name: "black horse",
        k: K::Mount,
        price: 750,
        icon: "🐎",
        ..B
    },
];

// ---------------------------------------------------------------- horses
/// Horse coats and their manes: bay, grey, black (sold at the stables), then
/// chestnut and dun (wild).
pub const HORSE_COATS: [(u32, u32); 5] = [
    (0x6a4028ff, 0x1a1410ff),
    (0x8a8680ff, 0xc8c4bcff),
    (0x2a2522ff, 0x151210ff),
    (0x7a4a2aff, 0x4a2a18ff),
    (0x8a7050ff, 0x3a2a1aff),
];
/// the coat a horse bought as `item` wears
pub fn horse_coat_of(item: &str) -> u8 {
    match item {
        "horse_grey" => 1,
        "horse_black" => 2,
        _ => 0,
    }
}
pub const HORSE_NAMES: [&str; 10] = [
    "Thistle", "Cinder", "Ash", "Bramble", "Mire", "Soot", "Raven", "Gloam", "Nettle", "Rook",
];

pub fn item_idx(id: &str) -> usize {
    ITEMS
        .iter()
        .position(|d| d.id == id)
        .unwrap_or_else(|| panic!("unknown item {id}"))
}
pub fn try_item_idx(id: &str) -> Option<usize> {
    ITEMS.iter().position(|d| d.id == id)
}

pub const REAGENTS: [&str; 8] = ["bp", "bm", "ga", "gi", "mr", "ns", "ss", "sa"];
pub const GEMS: [&str; 3] = ["ruby", "sapphire", "diamond"];

// ---------------------------------------------------------------- spells
pub const FX_ARROW: u8 = 0;
pub const FX_FIREBALL: u8 = 1;
pub const FX_EBOLT: u8 = 2;
pub const FX_LIGHTNING: u8 = 3;
pub const FX_EXPLOSION: u8 = 4;
pub const FX_FLAMESTRIKE: u8 = 5;
pub const FX_HARM: u8 = 6;
pub const FX_HEAL: u8 = 7;
pub const FX_FIZZLE: u8 = 8;
pub const FX_BREATH: u8 = 9;
pub const FX_RES: u8 = 10;
pub const FX_SPARK: u8 = 11;
pub const FX_BLOOD: u8 = 12;

#[derive(Clone)]
pub struct SpellDef {
    pub id: &'static str,
    pub name: &'static str,
    pub circle: usize,
    pub words: &'static str,
    pub regs: &'static [&'static str],
    pub hostile: bool,
    pub amt: (i32, i32),
    pub fx: u8,
    pub range: f32,
}

static SPELLS_DEF: &[SpellDef] = &[
    SpellDef {
        id: "magicarrow",
        name: "Magic Arrow",
        circle: 1,
        words: "In Por Ylem",
        regs: &["sa"],
        hostile: true,
        amt: (4, 8),
        fx: FX_ARROW,
        range: 11.0,
    },
    SpellDef {
        id: "heal",
        name: "Heal",
        circle: 1,
        words: "In Mani",
        regs: &["ga", "gi", "ss"],
        hostile: false,
        amt: (6, 12),
        fx: FX_HEAL,
        range: 11.0,
    },
    SpellDef {
        id: "harm",
        name: "Harm",
        circle: 2,
        words: "An Mani",
        regs: &["ns", "ss"],
        hostile: true,
        amt: (6, 11),
        fx: FX_HARM,
        range: 4.0,
    },
    SpellDef {
        id: "fireball",
        name: "Fireball",
        circle: 3,
        words: "Vas Flam",
        regs: &["bp"],
        hostile: true,
        amt: (9, 16),
        fx: FX_FIREBALL,
        range: 11.0,
    },
    SpellDef {
        id: "greaterheal",
        name: "Greater Heal",
        circle: 4,
        words: "In Vas Mani",
        regs: &["ga", "gi", "mr", "ss"],
        hostile: false,
        amt: (20, 34),
        fx: FX_HEAL,
        range: 11.0,
    },
    SpellDef {
        id: "lightning",
        name: "Lightning",
        circle: 4,
        words: "Por Ort Grav",
        regs: &["mr", "sa"],
        hostile: true,
        amt: (13, 22),
        fx: FX_LIGHTNING,
        range: 11.0,
    },
    SpellDef {
        id: "energybolt",
        name: "Energy Bolt",
        circle: 6,
        words: "Corp Por",
        regs: &["bp", "ns"],
        hostile: true,
        amt: (22, 34),
        fx: FX_EBOLT,
        range: 11.0,
    },
    SpellDef {
        id: "explosion",
        name: "Explosion",
        circle: 6,
        words: "Vas Ort Flam",
        regs: &["bm", "mr"],
        hostile: true,
        amt: (20, 32),
        fx: FX_EXPLOSION,
        range: 11.0,
    },
    SpellDef {
        id: "flamestrike",
        name: "Flamestrike",
        circle: 7,
        words: "Kal Vas Flam",
        regs: &["ss", "sa"],
        hostile: true,
        amt: (28, 42),
        fx: FX_FLAMESTRIKE,
        range: 11.0,
    },
];
pub const CIRCLE_MANA: [i32; 9] = [0, 4, 6, 9, 11, 14, 20, 40, 50];
/// minimum Magery (tenths) per circle
pub const CIRCLE_MIN: [i32; 9] = [0, 0, 100, 250, 350, 500, 600, 750, 850];

pub fn spell_idx(id: &str) -> Option<usize> {
    SPELLS.iter().position(|s| s.id == id)
}

// ---------------------------------------------------------------- monsters
#[derive(Clone)]
pub struct MonDef {
    pub id: &'static str,
    pub name: &'static str,
    pub hp: i32,
    pub dmg: (i32, i32),
    pub skill: i32,
    pub speed: f32,
    pub mv: f32,
    pub passive: bool,
    pub gold: (i32, i32),
    pub fame: i32,
    pub karma: i32,
    pub size: f32,
    pub color: u32,
    pub ar: i32,
    pub regen: i32,
    pub caster: &'static [usize],
    pub magery: i32,
    pub breath: bool,
    pub regs: bool,
    pub drops: &'static [&'static str],
    pub flies: bool,
}

const MB: MonDef = MonDef {
    id: "",
    name: "",
    hp: 10,
    dmg: (1, 2),
    skill: 20,
    speed: 2.2,
    mv: 3.0,
    passive: false,
    gold: (0, 0),
    fame: 0,
    karma: 0,
    size: 1.0,
    color: 0x888888ff,
    ar: 0,
    regen: 0,
    caster: &[],
    magery: 0,
    breath: false,
    regs: false,
    drops: &[],
    flies: false,
};

static MONS_DEF: &[MonDef] = &[
    MonDef {
        id: "rabbit",
        name: "a rabbit",
        hp: 6,
        dmg: (1, 2),
        skill: 10,
        speed: 2.0,
        mv: 4.5,
        passive: true,
        size: 0.45,
        color: 0xa07850ff,
        ..MB
    },
    MonDef {
        id: "rat",
        name: "a giant rat",
        hp: 14,
        dmg: (2, 4),
        skill: 25,
        mv: 3.5,
        gold: (4, 14),
        fame: 10,
        size: 0.6,
        color: 0x6d5a4aff,
        ..MB
    },
    MonDef {
        id: "mongbat",
        name: "a mongbat",
        hp: 10,
        dmg: (1, 5),
        skill: 25,
        speed: 1.8,
        mv: 4.0,
        gold: (5, 12),
        fame: 10,
        karma: -10,
        size: 0.55,
        color: 0x7a5d7aff,
        flies: true,
        ..MB
    },
    MonDef {
        id: "slime",
        name: "a slime",
        hp: 24,
        dmg: (2, 6),
        skill: 30,
        speed: 2.4,
        mv: 2.0,
        gold: (8, 18),
        fame: 15,
        karma: -15,
        size: 0.65,
        color: 0x6cbf3cff,
        ..MB
    },
    MonDef {
        id: "wolf",
        name: "a grey wolf",
        hp: 28,
        dmg: (3, 8),
        skill: 40,
        speed: 1.9,
        mv: 4.6,
        fame: 30,
        size: 0.75,
        color: 0x8d8d8dff,
        ..MB
    },
    MonDef {
        id: "skeleton",
        name: "a skeleton",
        hp: 34,
        dmg: (3, 8),
        skill: 45,
        mv: 3.2,
        gold: (20, 45),
        fame: 45,
        karma: -45,
        size: 0.9,
        color: 0xe8e2cfff,
        regs: true,
        ..MB
    },
    MonDef {
        id: "zombie",
        name: "a zombie",
        hp: 44,
        dmg: (3, 7),
        skill: 35,
        speed: 2.6,
        mv: 2.2,
        gold: (18, 40),
        fame: 40,
        karma: -40,
        size: 0.95,
        color: 0x7b8f62ff,
        ..MB
    },
    MonDef {
        id: "orc",
        name: "an orc",
        hp: 55,
        dmg: (5, 11),
        skill: 55,
        speed: 2.4,
        mv: 3.4,
        gold: (30, 70),
        fame: 60,
        karma: -60,
        size: 1.0,
        color: 0x5f8a3aff,
        ar: 10,
        drops: &["ore", "logs", "bandage"],
        ..MB
    },
    MonDef {
        id: "orclord",
        name: "an orc lord",
        hp: 95,
        dmg: (8, 15),
        skill: 70,
        speed: 2.4,
        mv: 3.4,
        gold: (80, 160),
        fame: 150,
        karma: -150,
        size: 1.1,
        color: 0x486e2aff,
        ar: 18,
        drops: &["ringmail", "katana", "healpotion"],
        ..MB
    },
    MonDef {
        id: "troll",
        name: "a troll",
        hp: 110,
        dmg: (8, 15),
        skill: 70,
        speed: 2.6,
        mv: 3.0,
        gold: (90, 180),
        fame: 200,
        karma: -200,
        size: 1.25,
        color: 0x7a6e52ff,
        ar: 15,
        regen: 2,
        drops: &["healpotion"],
        ..MB
    },
    MonDef {
        id: "ettin",
        name: "an ettin",
        hp: 140,
        dmg: (9, 18),
        skill: 75,
        speed: 2.9,
        mv: 2.8,
        gold: (110, 220),
        fame: 260,
        karma: -260,
        size: 1.4,
        color: 0xa3866aff,
        ar: 14,
        drops: &["gems", "warhammer"],
        ..MB
    },
    MonDef {
        id: "lich",
        name: "a lich",
        hp: 120,
        dmg: (6, 12),
        skill: 70,
        speed: 2.5,
        mv: 2.8,
        gold: (180, 360),
        fame: 400,
        karma: -400,
        size: 1.05,
        color: 0x6a4c93ff,
        caster: &[0, 3, 5, 6],
        magery: 900,
        regs: true,
        drops: &["gems", "scroll"],
        ..MB
    },
    MonDef {
        id: "horse",
        name: "a wild horse",
        hp: 45,
        dmg: (2, 6),
        skill: 30,
        mv: 4.2,
        passive: true,
        fame: 20,
        size: 1.3,
        color: 0x7a4a2aff,
        ..MB
    },
    MonDef {
        id: "dragon",
        name: "a dragon",
        hp: 650,
        dmg: (16, 26),
        skill: 100,
        speed: 2.4,
        mv: 3.6,
        gold: (900, 1600),
        fame: 2500,
        karma: -2500,
        size: 2.3,
        color: 0xb8352bff,
        ar: 45,
        caster: &[3, 5],
        magery: 1000,
        breath: true,
        drops: &["gems", "gems", "gems", "platemail", "scroll", "scroll"],
        ..MB
    },
];

pub fn mon_idx(id: &str) -> usize {
    MONS.iter()
        .position(|m| m.id == id)
        .unwrap_or_else(|| panic!("unknown monster {id}"))
}

// ---------------------------------------------------------------- vendors
pub const V_SMITH: usize = 0;
pub const V_MAGE: usize = 1;
pub const V_PROV: usize = 2;
pub const V_HEALER: usize = 3;
pub const V_BANKER: usize = 4;
pub const V_GUARD: usize = 5;
/// townsfolk with no trade, wandering the streets
pub const V_FOLK: usize = 6;
pub const V_BARKEEP: usize = 7;
pub const V_LIBRARIAN: usize = 8;
/// the lord of Duskhold castle, on the throne in the keep
pub const V_LORD: usize = 9;
/// the stablemaster by the town's east side: sells horses, fetches lost ones
pub const V_STABLE: usize = 10;
/// the small folk of the village in the west: no trade, a quiet life
pub const V_VILLAGER: usize = 11;
/// the elves of the valley, the wood and the haven: no trade, long memories
pub const V_ELF: usize = 12;

#[derive(Clone)]
pub struct VendorDef {
    pub title: &'static str,
    pub names: &'static [&'static str],
    pub sells: &'static [&'static str],
    pub buys: &'static [K],
    pub buys_ids: &'static [&'static str],
    pub robe: u32,
    pub chatter: &'static [&'static str],
}

static VENDORS_DEF: &[VendorDef] = &[
    VendorDef {
        title: "the blacksmith",
        names: &["Gareth", "Bryn", "Hilda"],
        sells: &[
            "dagger",
            "katana",
            "broadsword",
            "longsword",
            "mace",
            "warhammer",
            "pickaxe",
            "smithhammer",
            "buckler",
            "heater",
            "ringmail",
            "chainmail",
            "platemail",
            "ingot",
        ],
        buys: &[K::Weapon, K::Armor, K::Shield],
        buys_ids: &["ingot", "ore"],
        robe: 0x5a3a22ff,
        chatter: &[
            "Fine steel, forged this very morn!",
            "Bring me ingots and I shall pay fair coin.",
            "Smelt thine ore at my forge, friend.",
        ],
    },
    VendorDef {
        title: "the mage",
        names: &["Mirelle", "Aldous", "Theron"],
        sells: &[
            "spellbook",
            "bp",
            "bm",
            "ga",
            "gi",
            "mr",
            "ns",
            "ss",
            "sa",
            "scroll_magicarrow",
            "scroll_heal",
            "scroll_harm",
            "scroll_fireball",
            "scroll_greaterheal",
            "scroll_lightning",
            "scroll_energybolt",
            "scroll_explosion",
            "scroll_flamestrike",
        ],
        buys: &[K::Reagent, K::Scroll, K::Gem],
        buys_ids: &[],
        robe: 0x2e3f9aff,
        chatter: &[
            "Reagents! Fresh reagents!",
            "The arcane arts reward the patient.",
            "Practise thy spells and thy Magery shall grow.",
        ],
    },
    VendorDef {
        title: "the provisioner",
        names: &["Tobin", "Wenna", "Corwin"],
        sells: &[
            "bandage",
            "hatchet",
            "fishingpole",
            "leather",
            "robe",
            "fish",
        ],
        buys: &[K::Gem, K::Tool],
        buys_ids: &["logs", "fish"],
        robe: 0x3f6a34ff,
        chatter: &[
            "Bandages, tools, all a traveller needs.",
            "The woods are thick with timber.",
            "Fish from the shore fetch a good price.",
        ],
    },
    VendorDef {
        title: "the healer",
        names: &["Sister Ana", "Brother Ewan"],
        sells: &["bandage", "healpotion", "ga", "gi"],
        buys: &[],
        buys_ids: &[],
        robe: 0xa8a090ff,
        chatter: &[
            "Should death find thee, seek me out.",
            "Bandages mend the flesh, if thy hand be steady.",
            "The ankh shrines can also restore the dead.",
        ],
    },
    VendorDef {
        title: "the banker",
        names: &["Oswin", "Marta"],
        sells: &[],
        buys: &[],
        buys_ids: &[],
        robe: 0x6b2d6bff,
        chatter: &[
            "Thy gold is safe with me.",
            "Carry less, lose less, as they say.",
            "Deposits and withdrawals, friend.",
        ],
    },
    VendorDef {
        title: "the guard",
        names: &["Sir Roland", "Dame Ysolde", "Sir Cedric"],
        sells: &[],
        buys: &[],
        buys_ids: &[],
        robe: 0x8a8e96ff,
        chatter: &[
            "All is well.",
            "Keep thy blade sheathed within the town.",
            "Undead stir in the graveyard to the north-east.",
            "Orcs have made camp in the south-west.",
            "Trolls roam the rocky hills to the north-west.",
            "They say a dragon sleeps in the south-east.",
        ],
    },
    VendorDef {
        title: "",
        names: &[
            "Old Tam",
            "Mirela",
            "Joss",
            "Widow Agna",
            "Pell",
            "Hanne",
            "Corwin",
            "Ilse",
            "Brother Aldo",
            "Fen",
        ],
        sells: &[],
        buys: &[],
        buys_ids: &[],
        robe: 0x5a4a38ff,
        chatter: &[
            "The bell tolls for another burial today.",
            "Bar thy door after dark, stranger.",
            "My son went to the hills. He has not come back.",
            "The well water tastes of iron of late.",
            "Bread costs twice what it did last winter.",
            "The dead walk in the graveyard. I have seen them.",
            "Stay off the roads at night.",
            "Pray, and keep thy coin close.",
        ],
    },
    VendorDef {
        title: "the barkeep",
        names: &["Old Hob", "Greta", "Dunstan"],
        sells: &["ale", "bread", "fish"],
        buys: &[],
        buys_ids: &["fish"],
        robe: 0x6a4a2aff,
        chatter: &[
            "Ale's a copper or three. Sit, warm thy bones.",
            "A drover swore he saw the dead walking the north road.",
            "The orcs pay no tithe, and want no ale. Ill-mannered folk.",
            "Last fool who went for the dragon's gold drank here first.",
            "Pay before thou drinkest. House rule.",
        ],
    },
    VendorDef {
        title: "the librarian",
        names: &["Master Quill", "Sister Hesper", "Anselm"],
        sells: &[
            "spellbook",
            "scroll_magicarrow",
            "scroll_heal",
            "scroll_harm",
            "scroll_greaterheal",
        ],
        buys: &[K::Scroll],
        buys_ids: &[],
        robe: 0x3a3430ff,
        chatter: &[
            "Hush. This is a house of learning.",
            "The lich was a scholar once, they say. Mind thy studies.",
            "The chronicles name six periods of the day, Bloodwatch to Eventide.",
            "Bring me scrolls thou hast found. I pay for knowledge.",
            "A spell learnt from a scroll is never forgot.",
        ],
    },
    VendorDef {
        title: "of Duskhold",
        names: &["Lord Aldric", "Lady Morwen", "Lord Ulfgar"],
        sells: &[],
        buys: &[],
        buys_ids: &[],
        robe: 0x5a1a1aff,
        chatter: &[
            "Kneel, or state thy business.",
            "The dead stir in the graveyard east of my walls. Put them down.",
            "Orcs raid my roads from the south-west. I would see their lord's head.",
            "My guards keep the peace within these walls. Beyond them, thou art alone.",
            "Slay the dragon, and Duskhold will sing thy name.",
            "The trolls in the hills grow bold. Thin their number.",
        ],
    },
    VendorDef {
        title: "the stablemaster",
        names: &["Edda", "Garric", "Wynn"],
        sells: &["horse_bay", "horse_grey", "horse_black"],
        buys: &[],
        buys_ids: &[],
        robe: 0x5a4a32ff,
        chatter: &[
            "A good horse outruns any wolf on the road.",
            "Tap thy horse to mount. Tap thyself to get down.",
            "Lost thy horse? I'll send the lad to fetch it home.",
            "Wild horses run the western meadows. Leave them be.",
            "A heavy blow will throw thee from the saddle. Hold on tight.",
        ],
    },
    VendorDef {
        title: "",
        names: &[
            "Pip Thistlebrook",
            "Daisy Hollowell",
            "Barnaby Mossfoot",
            "Clover Puddleby",
            "Tobin Fernbank",
            "Wren Applethorn",
            "Bramble Sweetwater",
            "Nell Underbough",
            "Ollie Hedgerow",
            "Poppy Fairbrook",
        ],
        sells: &[],
        buys: &[],
        buys_ids: &[],
        robe: 0x5a6a32ff,
        chatter: &[
            "Fine weather for the garden.",
            "The mill-wheel's been turning since before my grandmother's time.",
            "Mind the cabbages, if you please!",
            "Nothing ever happens here, and that's just how we like it.",
            "Big Folk from the town? You'll be wanting the inn, I expect.",
            "There's to be a party under the great tree come midsummer.",
            "The hedge keeps out the wolves, and the hayward keeps out the rest.",
            "Elevenses! Is it elevenses already?",
            "A pipe, a good chair and a window on the river. What more is there?",
            "Adventures are for Big Folk. We have a garden to see to.",
        ],
    },
    VendorDef {
        title: "",
        names: &[
            "Elaith", "Varneth", "Ilyrin", "Sorael", "Caelir", "Liriel", "Faelan", "Ithriel",
            "Aelor", "Myrren",
        ],
        sells: &[],
        buys: &[],
        buys_ids: &[],
        robe: 0x6a7a70ff,
        chatter: &[
            "The stars were brighter when I was young. That was long ago.",
            "Be welcome, traveller. Rest; no shadow comes here.",
            "The ships leave for the west, one by one. One day I shall go too.",
            "Your kind live so briefly, and do so much.",
            "Listen: the river is singing. It sings of the snows.",
            "We remember the kingdoms of men when they were young and bright.",
            "Walk softly in the wood. The trees are older than your cities.",
            "Our lamps burn without smoke and our songs without end.",
        ],
    },
];

// ---------------------------------------------------------------- templates
#[derive(Clone)]
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub skills: [(usize, i32); 3],
    pub stats: (i32, i32, i32),
    pub items: &'static [(&'static str, u32)],
    pub spells: u32,
}

static TEMPLATES_DEF: &[Template] = &[
    Template {
        id: "warrior",
        name: "Warrior",
        desc: "Swordsmanship, Tactics, Healing. Strong and sturdy.",
        skills: [(SWORDS, 500), (TACTICS, 300), (HEALING, 200)],
        stats: (45, 35, 10),
        items: &[("broadsword", 1), ("leather", 1), ("bandage", 15)],
        spells: 0,
    },
    Template {
        id: "mage",
        name: "Mage",
        desc: "Magery, Meditation, Wrestling. Fragile but deadly.",
        skills: [(MAGERY, 500), (MEDITATION, 300), (WRESTLING, 200)],
        stats: (25, 20, 45),
        items: &[
            ("spellbook", 1),
            ("robe", 1),
            ("dagger", 1),
            ("bp", 10),
            ("ga", 15),
            ("gi", 15),
            ("ss", 15),
            ("sa", 25),
            ("mr", 10),
            ("ns", 8),
        ],
        spells: 0b1111,
    },
    Template {
        id: "blacksmith",
        name: "Blacksmith",
        desc: "Blacksmithy, Mining, Mace Fighting. Forge thine own arms.",
        skills: [(SMITHING, 500), (MINING, 300), (MACE, 200)],
        stats: (45, 20, 15),
        items: &[
            ("pickaxe", 1),
            ("smithhammer", 1),
            ("leather", 1),
            ("ingot", 12),
            ("bandage", 5),
        ],
        spells: 0,
    },
    Template {
        id: "woodsman",
        name: "Woodsman",
        desc: "Lumberjacking, Swordsmanship, Fishing. Lives off the land.",
        skills: [(LUMBER, 500), (SWORDS, 300), (FISHING, 200)],
        stats: (40, 30, 10),
        items: &[
            ("hatchet", 1),
            ("fishingpole", 1),
            ("leather", 1),
            ("bandage", 8),
        ],
        spells: 0,
    },
];

// ---------------------------------------------------------------- crafting
/// (item id, ingots, minimum skill in tenths)
pub static RECIPES: &[(&str, u32, i32)] = &[
    ("dagger", 3, 0),
    ("buckler", 5, 50),
    ("mace", 6, 100),
    ("pickaxe", 4, 100),
    ("broadsword", 10, 300),
    ("katana", 8, 350),
    ("heater", 12, 400),
    ("ringmail", 18, 400),
    ("longsword", 12, 450),
    ("chainmail", 20, 550),
    ("warhammer", 16, 600),
    ("platemail", 25, 700),
];

pub const HAIR: [u32; 6] = [
    0x3b2414ff, 0x8a5a2bff, 0xd8b45aff, 0x1a1a1aff, 0xa83a1eff, 0xd0d0d0ff,
];

// ---------------------------------------------------------------- world packs
/// Vendor ids as a world pack names them in `[vendor.<id>]`, by V_* index.
pub const VENDOR_IDS: [&str; 13] = [
    "smith",
    "mage",
    "provisioner",
    "healer",
    "banker",
    "guard",
    "folk",
    "barkeep",
    "librarian",
    "lord",
    "stablemaster",
    "villager",
    "elf",
];

/// A game table built once from the engine's data and the active world pack
/// (see `pack.rs`), then read like a plain slice.
pub struct Table<T: 'static> {
    cell: std::sync::OnceLock<Vec<T>>,
    init: fn() -> Vec<T>,
}
impl<T> Table<T> {
    const fn new(init: fn() -> Vec<T>) -> Table<T> {
        Table {
            cell: std::sync::OnceLock::new(),
            init,
        }
    }
}
impl<T> std::ops::Deref for Table<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.cell.get_or_init(self.init)
    }
}

pub static ITEMS: Table<ItemDef> = Table::new(|| items_for(crate::pack::get()));
pub static SPELLS: Table<SpellDef> = Table::new(|| spells_for(crate::pack::get()));
pub static MONS: Table<MonDef> = Table::new(|| mons_for(crate::pack::get()));
pub static VENDORS: Table<VendorDef> = Table::new(|| vendors_for(crate::pack::get()));
pub static TEMPLATES: Table<Template> = Table::new(|| templates_for(crate::pack::get()));

use crate::pack::{leak, leak_list, Pack};

fn name_of(p: &Pack, sec: &str, key: &str, dflt: &'static str) -> &'static str {
    p.str(sec, key).map(|s| leak(s.to_string())).unwrap_or(dflt)
}

pub fn items_for(p: &Pack) -> Vec<ItemDef> {
    ITEMS_DEF
        .iter()
        .map(|d| {
            let sec = format!("item.{}", d.id);
            ItemDef {
                name: name_of(p, &sec, "name", d.name),
                icon: name_of(p, &sec, "icon", d.icon),
                plural: name_of(p, &sec, "plural", d.plural),
                ..d.clone()
            }
        })
        .collect()
}
pub fn spells_for(p: &Pack) -> Vec<SpellDef> {
    SPELLS_DEF
        .iter()
        .map(|d| {
            let sec = format!("spell.{}", d.id);
            SpellDef {
                name: name_of(p, &sec, "name", d.name),
                words: name_of(p, &sec, "words", d.words),
                ..d.clone()
            }
        })
        .collect()
}
pub fn mons_for(p: &Pack) -> Vec<MonDef> {
    MONS_DEF
        .iter()
        .map(|d| {
            let sec = format!("mon.{}", d.id);
            MonDef {
                name: name_of(p, &sec, "name", d.name),
                color: p.color(&sec, "color").unwrap_or(d.color),
                hp: p
                    .int(&sec, "hp")
                    .map(|n| n.clamp(1, 100_000) as i32)
                    .unwrap_or(d.hp),
                ..d.clone()
            }
        })
        .collect()
}
pub fn vendors_for(p: &Pack) -> Vec<VendorDef> {
    VENDORS_DEF
        .iter()
        .zip(VENDOR_IDS)
        .map(|(d, id)| {
            let sec = format!("vendor.{id}");
            VendorDef {
                title: name_of(p, &sec, "title", d.title),
                names: p.strs(&sec, "names").map(leak_list).unwrap_or(d.names),
                chatter: p.strs(&sec, "chatter").map(leak_list).unwrap_or(d.chatter),
                robe: p.color(&sec, "robe").unwrap_or(d.robe),
                ..d.clone()
            }
        })
        .collect()
}
pub fn templates_for(p: &Pack) -> Vec<Template> {
    TEMPLATES_DEF
        .iter()
        .map(|d| {
            let sec = format!("template.{}", d.id);
            Template {
                name: name_of(p, &sec, "name", d.name),
                desc: name_of(p, &sec, "desc", d.desc),
                ..d.clone()
            }
        })
        .collect()
}

/// What in a pack the engine will not use: unknown ids, sections and keys.
pub fn pack_warnings(p: &Pack) -> Vec<String> {
    use crate::sim::{REGION_KEYS, SIM_KEYS, SITE_IDS, SITE_KEYS};
    let known: [(&str, Vec<&str>, &[&str]); 6] = [
        (
            "mon",
            MONS_DEF.iter().map(|d| d.id).collect(),
            &["name", "color", "hp"],
        ),
        (
            "item",
            ITEMS_DEF.iter().map(|d| d.id).collect(),
            &["name", "icon", "plural"],
        ),
        (
            "spell",
            SPELLS_DEF.iter().map(|d| d.id).collect(),
            &["name", "words"],
        ),
        (
            "template",
            TEMPLATES_DEF.iter().map(|d| d.id).collect(),
            &["name", "desc"],
        ),
        (
            "vendor",
            VENDOR_IDS.to_vec(),
            &["title", "names", "chatter", "robe"],
        ),
        ("site", SITE_IDS.to_vec(), &SITE_KEYS),
    ];
    let mut w = vec![];
    for sec in p.sections.keys() {
        let (kind, id) = sec.split_once('.').unwrap_or((sec.as_str(), ""));
        // [sim] and any [region.<id>] have keys to check but no fixed ids
        let free: Option<&[&str]> = match (kind, id) {
            ("sim", "") => Some(&SIM_KEYS),
            ("region", r) if !r.is_empty() => Some(&REGION_KEYS),
            _ => None,
        };
        if id.is_empty() && free.is_none() {
            if !matches!(kind, "pack" | "text" | "words") {
                w.push(format!("[{sec}] is not a section the engine reads"));
            }
            continue;
        }
        let (ids, keys): (Vec<&str>, &[&str]) = match free {
            Some(keys) => (vec![id], keys),
            None => match known.iter().find(|(k, _, _)| *k == kind) {
                Some((_, ids, keys)) => (ids.clone(), keys),
                None => {
                    w.push(format!("[{sec}] is not a section the engine reads"));
                    continue;
                }
            },
        };
        if kind == "site" {
            if let Some(r) = p.str(sec, "region") {
                if !p.sections.contains_key(&format!("region.{r}")) {
                    w.push(format!("[{sec}] region: no [region.{r}] in this pack"));
                }
            }
        }
        if kind == "sim" {
            if let Some(d) = p.str(sec, "start") {
                if crate::sim::parse_date(d).is_none() {
                    w.push(format!(
                        "[sim] start: \"{d}\" is not a date like \"640-03-01\""
                    ));
                }
            }
            if p.strs(sec, "months").is_some_and(|m| m.len() != 12) {
                w.push("[sim] months: needs all twelve".into());
            }
        }
        if !ids.contains(&id) {
            w.push(format!("[{sec}]: no {kind} with id \"{id}\""));
            continue;
        }
        let mut ks: Vec<&String> = p.sections[sec].keys().collect();
        ks.sort();
        for k in ks {
            if !keys.contains(&k.as_str()) {
                w.push(format!(
                    "[{sec}] {k}: not used (known: {})",
                    keys.join(", ")
                ));
            }
        }
    }
    w.sort();
    w
}

#[cfg(test)]
mod pack_tests {
    use super::*;

    #[test]
    fn a_pack_renames_without_touching_rules() {
        let p = Pack::parse(
            r##"[pack]
id = "t"
[mon.wolf]
name = "a warg"
color = "#102030"
hp = 40
[vendor.guard]
title = "the watchman"
names = ["Ned"]
[item.katana]
name = "long knife"
[spell.fireball]
words = "Naur"
[template.mage]
name = "Loremaster"
[mon.balrog]
name = "nope"
[vendor.guard2]
title = "x"
[mon.rat]
speed = 3
"##,
        )
        .unwrap();
        let m = mons_for(&p);
        let w = &m[MONS_DEF.iter().position(|d| d.id == "wolf").unwrap()];
        assert_eq!((w.name, w.color, w.hp), ("a warg", 0x102030ff, 40));
        assert_eq!(w.dmg, (3, 8));
        let v = vendors_for(&p);
        assert_eq!(v[V_GUARD].title, "the watchman");
        assert_eq!(v[V_GUARD].names, &["Ned"]);
        assert_eq!(v[V_GUARD].chatter, VENDORS_DEF[V_GUARD].chatter);
        let it = items_for(&p);
        assert_eq!(
            it.iter().find(|d| d.id == "katana").unwrap().name,
            "long knife"
        );
        let sp = spells_for(&p);
        assert_eq!(
            sp.iter().find(|d| d.id == "fireball").unwrap().words,
            "Naur"
        );
        assert_eq!(templates_for(&p)[1].name, "Loremaster");
        let warn = pack_warnings(&p);
        assert_eq!(warn.len(), 3, "{warn:?}");
        assert!(warn.iter().any(|s| s.contains("balrog")));
        assert!(warn.iter().any(|s| s.contains("guard2")));
        assert!(warn.iter().any(|s| s.contains("speed")));
    }

    #[test]
    fn the_engine_world_is_unchanged_without_a_pack() {
        let p = crate::pack::builtin();
        assert_eq!(mons_for(&p).len(), MONS_DEF.len());
        assert_eq!(vendors_for(&p)[V_LORD].title, "of Duskhold");
        assert!(pack_warnings(&p).is_empty());
    }
}
