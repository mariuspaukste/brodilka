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
}

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
};

pub static ITEMS: &[ItemDef] = &[
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

pub static SPELLS: &[SpellDef] = &[
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

pub static MONS: &[MonDef] = &[
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

pub struct VendorDef {
    pub title: &'static str,
    pub names: &'static [&'static str],
    pub sells: &'static [&'static str],
    pub buys: &'static [K],
    pub buys_ids: &'static [&'static str],
    pub robe: u32,
    pub chatter: &'static [&'static str],
}

pub static VENDORS: &[VendorDef] = &[
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
        robe: 0xe8e0c0ff,
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
        robe: 0xc8ced6ff,
        chatter: &[
            "All is well.",
            "Keep thy blade sheathed within the town.",
            "Undead stir in the graveyard to the north-east.",
            "Orcs have made camp in the south-west.",
            "Trolls roam the rocky hills to the north-west.",
            "They say a dragon sleeps in the south-east.",
        ],
    },
];

// ---------------------------------------------------------------- templates
pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub skills: [(usize, i32); 3],
    pub stats: (i32, i32, i32),
    pub items: &'static [(&'static str, u32)],
    pub spells: u32,
}

pub static TEMPLATES: &[Template] = &[
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
