// BiomeCompass: craft a Biome Compass, pick a biome, follow the needle.
//
// Original implementation for Horizon (a horizon-mixins native mod). This file is
// both the standalone, unit-tested Cargo library and the injected
// `horizon_client::biomecompass` module (in the game's library, so that hooks in
// the library, the crafting menu and the command tree, reach it too). It has no
// Horizon dependency: the search only hands out block positions to sample and
// takes back the biome names the world generator gives for them, and the item's
// components, the recipe and the picker screen are plain values; `native.rs` and
// `item.rs` turn them into the game's.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

pub type Position = [i32; 3];
/// A player's UUID as the game keeps it.
pub type Actor = [i32; 4];

/// The mod's id: its row in Options > Mods and its settings folder
/// (`horizon-mods/.horizon-mod-data/biomecompass/`).
pub const ID: &str = "biomecompass";
pub const VERSION: &str = "0.2.2";
pub const DESCRIPTION: &str = "A craftable Biome Compass: right-click it, pick a biome, and its needle points to the nearest one.";
/// The settings file, in the mod's settings folder.
pub const CONFIG_FILE: &str = "config.txt";
/// The chat command (without its slash), and its short form.
pub const COMMAND: &str = "biomecompass";
pub const ALIAS: &str = "bcompass";
pub const USAGE: &str = "Usage: /biomecompass [pick [filter]|list [filter]|find <biome or #tag>|status|stop|give|tp|reload|help]";
/// The most of a server tick a search takes, whatever `samples-per-tick`
/// says (a tick is 50 ms).
pub const TICK_TIME_MS: u64 = 10;
/// Samples asked of the generator at once (a batch locks it once).
pub const BATCH: usize = 256;
/// Blocks between the heights sampled in a column (the Overworld's caves).
pub const LAYER_STEP: i32 = 64;
/// A long search tells its player how far it got this often (ticks).
pub const PROGRESS_TICKS: u32 = 200;
/// How long the HUD keeps showing a search that found nothing (ticks).
pub const MISSING_TICKS: u32 = 200;
/// The permission level `/biomecompass tp` and `reload` need (operators).
pub const OPERATOR: u8 = 2;
/// Items that show the needle when `hud-needs-compass` is on.
pub const COMPASSES: [&str; 2] = ["minecraft:compass", "minecraft:recovery_compass"];
/// How often the Biome Compasses in players' inventories are brought up to
/// date (their needle, their lore) and the action bar shown (ticks).
pub const COMPASS_TICKS: u32 = 5;
/// Ticks between two right-clicks that open the picker (holding the button
/// down repeats the click every 4 ticks).
pub const USE_COOLDOWN: u32 = 10;

/// BiomeCompass is on in Options > Mods, as the game last saw it. The hooks
/// in the game's library (the crafting menu, the command tree) have no game
/// to ask; the game's own hooks keep this up to date.
static ENABLED: AtomicBool = AtomicBool::new(true);
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

thread_local! {
    /// This thread's game is a client of another computer's server, as it
    /// last looked.
    static CLIENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// Whether a game is a client of another computer's server: joined to a
/// server (`remote`) that isn't its own (`own_server`: single player's
/// server thread, or the LAN world it hosts, in this process). Its own
/// server runs this BiomeCompass with the window's settings, so the window
/// shows what that server's crafting grid makes; another computer's server
/// may run no BiomeCompass at all, so a client of one predicts no Biome
/// Compass.
pub fn foreign_client(remote: bool, own_server: bool) -> bool {
    remote && !own_server
}
/// A client of another computer's server only predicts clicks; that server
/// decides what a crafting grid makes (and one without BiomeCompass makes no
/// Biome Compass).
pub fn set_client_thread(client: bool) {
    CLIENT.with(|c| c.set(client));
}
pub fn client_thread() -> bool {
    CLIENT.with(|c| c.get())
}

// ---- settings ------------------------------------------------------------

/// A corner of the screen for the HUD box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
impl Corner {
    const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];
    fn key(self) -> &'static str {
        match self {
            Corner::TopLeft => "top-left",
            Corner::TopRight => "top-right",
            Corner::BottomLeft => "bottom-left",
            Corner::BottomRight => "bottom-right",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Corner::TopLeft => "top left",
            Corner::TopRight => "top right",
            Corner::BottomLeft => "bottom left",
            Corner::BottomRight => "bottom right",
        }
    }
}

/// Which heights of a column a search samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heights {
    /// Every `LAYER_STEP` blocks, nearest the player's height first: finds
    /// the Overworld's cave biomes too (as `/locate biome` does).
    All,
    /// The player's height only: a quarter to a sixth of the samples.
    Player,
}

/// Who may use `/biomecompass tp`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Teleport {
    Operators,
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Blocks from the player a search covers (a circle).
    pub radius: u32,
    /// Blocks between two sampled columns.
    pub spacing: u32,
    /// Samples a tick, shared by every search running.
    pub samples_per_tick: u32,
    pub heights: Heights,
    /// Where the HUD box goes; None hides it.
    pub hud: Option<Corner>,
    /// The HUD shows the needle only while a compass is in either hand.
    pub hud_needs_compass: bool,
    /// The action bar shows the biome and its distance while a Biome
    /// Compass is in either hand.
    pub action_bar: bool,
    pub teleport: Teleport,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            radius: 6400,
            spacing: 32,
            samples_per_tick: 1024,
            heights: Heights::All,
            hud: Some(Corner::TopRight),
            hud_needs_compass: false,
            action_bar: true,
            teleport: Teleport::Operators,
        }
    }
}

pub const RADIUS_RANGE: (u32, u32) = (64, 100_000);
pub const SPACING_RANGE: (u32, u32) = (4, 512);
pub const SAMPLES_RANGE: (u32, u32) = (16, 65_536);
pub const RADIUS_STEPS: [u32; 9] = [500, 1000, 2000, 4000, 6400, 10_000, 16_000, 25_000, 50_000];
pub const SPACING_STEPS: [u32; 8] = [8, 16, 24, 32, 48, 64, 96, 128];
pub const SAMPLE_STEPS: [u32; 8] = [128, 256, 512, 1024, 2048, 4096, 8192, 16_384];

/// The first step above `now`, else the first: a value typed by hand in the
/// file moves to the next step up.
fn next_step(steps: &[u32], now: u32) -> u32 {
    steps.iter().copied().find(|&s| s > now).unwrap_or(steps[0])
}

fn parse_bool(v: &str) -> Option<bool> {
    match v {
        "true" | "on" | "yes" => Some(true),
        "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

impl Config {
    /// The settings file; a line it can't read is reported and the setting
    /// keeps its default.
    pub fn parse(text: &str) -> (Config, Vec<String>) {
        let mut c = Config::default();
        let mut problems = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                problems.push(format!("line {}: expected key = value", n + 1));
                continue;
            };
            let (key, value) = (key.trim(), value.trim().to_ascii_lowercase());
            let number = |(lo, hi): (u32, u32)| {
                value
                    .replace('_', "")
                    .parse::<u32>()
                    .ok()
                    .filter(|v| (lo..=hi).contains(v))
            };
            let ok = match key {
                "radius" => number(RADIUS_RANGE).map(|v| c.radius = v),
                "spacing" => number(SPACING_RANGE).map(|v| c.spacing = v),
                "samples-per-tick" => number(SAMPLES_RANGE).map(|v| c.samples_per_tick = v),
                "heights" => match value.as_str() {
                    "all" => Some(Heights::All),
                    "player" => Some(Heights::Player),
                    _ => None,
                }
                .map(|v| c.heights = v),
                "hud" => match value.as_str() {
                    "off" => Some(None),
                    v => Corner::ALL.into_iter().find(|k| k.key() == v).map(Some),
                }
                .map(|v| c.hud = v),
                "hud-needs-compass" => parse_bool(&value).map(|v| c.hud_needs_compass = v),
                "action-bar" => parse_bool(&value).map(|v| c.action_bar = v),
                "teleport" => match value.as_str() {
                    "operators" => Some(Teleport::Operators),
                    "off" => Some(Teleport::Off),
                    _ => None,
                }
                .map(|v| c.teleport = v),
                _ => {
                    problems.push(format!("line {}: unknown setting `{key}`", n + 1));
                    continue;
                }
            };
            if ok.is_none() {
                problems.push(format!("line {}: bad value for `{key}`: {value}", n + 1));
            }
        }
        (c, problems)
    }

    /// The settings file, every setting explained.
    pub fn to_file(&self) -> String {
        let hud = self.hud.map_or("off", Corner::key);
        let heights = match self.heights {
            Heights::All => "all",
            Heights::Player => "player",
        };
        let teleport = match self.teleport {
            Teleport::Operators => "operators",
            Teleport::Off => "off",
        };
        format!(
            "# BiomeCompass settings. Options > Mods > BiomeCompass > Settings writes\n\
             # this file. Edited by hand, it is read again by `/biomecompass reload`\n\
             # (operators) or the page's \"Read config.txt again\" button.\n\
             \n\
             # Blocks from you a search covers, {}..{}.\n\
             radius = {}\n\
             # Blocks between two sampled columns, {}..{}. Smaller finds small\n\
             # biomes (rivers, beaches) more surely and takes longer.\n\
             spacing = {}\n\
             # Samples a tick for all searches together, {}..{} (a search also\n\
             # stops each tick's share after {} ms).\n\
             samples-per-tick = {}\n\
             # all: every {} blocks of height (cave biomes too); player: your height only.\n\
             heights = {heights}\n\
             # top-left, top-right, bottom-left, bottom-right or off.\n\
             hud = {hud}\n\
             # true: the needle shows only while you hold a compass.\n\
             hud-needs-compass = {}\n\
             # true: the biome and its distance show over the hotbar while you\n\
             # hold a Biome Compass.\n\
             action-bar = {}\n\
             # operators (permission level {}) or off.\n\
             teleport = {teleport}\n",
            RADIUS_RANGE.0,
            RADIUS_RANGE.1,
            self.radius,
            SPACING_RANGE.0,
            SPACING_RANGE.1,
            self.spacing,
            SAMPLES_RANGE.0,
            SAMPLES_RANGE.1,
            TICK_TIME_MS,
            self.samples_per_tick,
            LAYER_STEP,
            self.hud_needs_compass,
            self.action_bar,
            OPERATOR,
        )
    }

    /// The settings page's buttons, in order.
    pub fn labels(&self) -> Vec<String> {
        let on_off = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Search radius: {} blocks", self.radius),
            format!("Sample spacing: {} blocks", self.spacing),
            format!("Samples per tick: {}", self.samples_per_tick),
            match self.heights {
                Heights::All => format!("Heights searched: every {LAYER_STEP} blocks (caves too)"),
                Heights::Player => "Heights searched: yours only".to_owned(),
            },
            format!("Compass HUD: {}", self.hud.map_or("off", Corner::label)),
            format!(
                "Only while holding a compass: {}",
                on_off(self.hud_needs_compass)
            ),
            format!(
                "Action bar while holding the Biome Compass: {}",
                on_off(self.action_bar)
            ),
            match self.teleport {
                Teleport::Operators => "Teleport (/biomecompass tp): operators".to_owned(),
                Teleport::Off => "Teleport (/biomecompass tp): off".to_owned(),
            },
            "Reset to defaults".to_owned(),
            format!("Read {CONFIG_FILE} again"),
        ]
    }
}

/// What a click on the settings page does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Click {
    /// Keep these settings and put them in effect.
    Save(Config),
    /// Read the file again.
    Reload,
}

/// Button `button` of the settings page clicked with `config` in effect.
pub fn click(config: &Config, button: usize) -> Option<Click> {
    let mut c = config.clone();
    match button {
        0 => c.radius = next_step(&RADIUS_STEPS, c.radius),
        1 => c.spacing = next_step(&SPACING_STEPS, c.spacing),
        2 => c.samples_per_tick = next_step(&SAMPLE_STEPS, c.samples_per_tick),
        3 => {
            c.heights = match c.heights {
                Heights::All => Heights::Player,
                Heights::Player => Heights::All,
            }
        }
        4 => {
            c.hud = match c.hud {
                None => Some(Corner::TopLeft),
                Some(Corner::BottomRight) => None,
                Some(k) => {
                    let i = Corner::ALL.iter().position(|x| *x == k).unwrap_or(0);
                    Some(Corner::ALL[i + 1])
                }
            }
        }
        5 => c.hud_needs_compass = !c.hud_needs_compass,
        6 => c.action_bar = !c.action_bar,
        7 => {
            c.teleport = match c.teleport {
                Teleport::Operators => Teleport::Off,
                Teleport::Off => Teleport::Operators,
            }
        }
        8 => c = Config::default(),
        9 => return Some(Click::Reload),
        _ => return None,
    }
    Some(Click::Save(c))
}

// ---- commands ------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    /// The biomes of the dimension, those matching the filter.
    List(Option<String>),
    /// Start a search for a biome (or `#tag`).
    Find(String),
    Status,
    Stop,
    Teleport,
    Reload,
    /// The biome picker (the Biome Compass's screen), its list filtered.
    Pick(Option<String>),
    /// A Biome Compass (operators, or creative).
    Give,
}

/// A chat command without its slash: None when it isn't BiomeCompass's,
/// else the command or its usage line.
pub fn parse_command(line: &str) -> Option<Result<Command, String>> {
    let line = line.trim().trim_start_matches('/');
    let mut words = line.split_whitespace();
    let head = words.next()?;
    if head != COMMAND && head != ALIAS {
        return None;
    }
    let sub = words.next().map(str::to_ascii_lowercase);
    let rest = words.collect::<Vec<_>>().join(" ");
    let alone = |c: Command| {
        if rest.is_empty() {
            Ok(c)
        } else {
            Err(USAGE.to_owned())
        }
    };
    Some(match sub.as_deref() {
        None | Some("status") => alone(Command::Status),
        Some("help") => alone(Command::Help),
        Some("list") => Ok(Command::List((!rest.is_empty()).then_some(rest.clone()))),
        Some("find" | "search" | "locate") if rest.is_empty() => {
            Err(format!("Usage: /{COMMAND} find <biome or #tag>"))
        }
        Some("find" | "search" | "locate") => Ok(Command::Find(rest.clone())),
        Some("stop" | "cancel" | "clear") => alone(Command::Stop),
        Some("tp" | "teleport") => alone(Command::Teleport),
        Some("reload") => alone(Command::Reload),
        Some("pick" | "menu" | "choose") => Ok(Command::Pick(unquote(&rest))),
        Some("give") => alone(Command::Give),
        Some(_) => Err(USAGE.to_owned()),
    })
}

/// A filter as the picker's search button sends it: in double quotes (a
/// quoted phrase, so an empty filter still parses), with `"`, `'` and `\`
/// escaped by a backslash; or bare, as typed. None when empty.
pub fn unquote(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let inner = match raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        Some(inner) => {
            let mut out = String::new();
            let mut chars = inner.chars();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => out.extend(chars.next()),
                    c => out.push(c),
                }
            }
            out
        }
        None => raw.to_owned(),
    };
    let inner = inner.trim();
    (!inner.is_empty()).then(|| inner.to_owned())
}

/// Whether a chat command is BiomeCompass's.
pub fn is_command(line: &str) -> bool {
    parse_command(line).is_some()
}

/// The help lines.
pub fn help() -> Vec<String> {
    vec![
        format!("BiomeCompass {VERSION}: find the nearest biome of a kind."),
        format!("Craft a Biome Compass: {RECIPE_TEXT}. Right-click it to pick a biome."),
        format!("/{COMMAND} pick [filter]: the biome picker, as the Biome Compass opens it."),
        format!("/{COMMAND} list [filter]: this dimension's biomes; click one to search."),
        format!(
            "/{COMMAND} find <biome or #tag>: search around you (cherry_grove, \"cherry grove\", #is_forest)."
        ),
        format!("/{COMMAND} status: how far the search got, or where the biome is."),
        format!("/{COMMAND} stop: stop searching and put the needle away."),
        format!("/{COMMAND} give: a Biome Compass (operators, or in creative)."),
        format!("/{COMMAND} tp: teleport to the biome found (operators)."),
        format!("/{COMMAND} reload: read {CONFIG_FILE} again (operators)."),
    ]
}

// ---- biome names -----------------------------------------------------------

/// `cherry_grove` -> `minecraft:cherry_grove`; ids are lower case.
pub fn full_id(name: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    if name.contains(':') {
        name
    } else {
        format!("minecraft:{name}")
    }
}

/// The language key of a biome's name: `biome.minecraft.cherry_grove`.
pub fn translation_key(id: &str) -> String {
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    format!("biome.{ns}.{path}")
}

/// A name made from the id when the language has none: `Cherry Grove`.
pub fn pretty(id: &str) -> String {
    let path = id.rsplit(':').next().unwrap_or(id);
    path.split(['_', '/'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(first) => first.to_uppercase().chain(c).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The biome tag listing a dimension's biomes.
pub fn dimension_tag(dimension: &str) -> Option<&'static str> {
    match dimension {
        "minecraft:overworld" => Some("minecraft:is_overworld"),
        "minecraft:the_nether" => Some("minecraft:is_nether"),
        "minecraft:the_end" => Some("minecraft:is_end"),
        _ => None,
    }
}

/// `the Overworld`, `the Nether`, `the End`, else the id.
pub fn dimension_name(dimension: &str) -> String {
    match dimension {
        "minecraft:overworld" => "the Overworld".to_owned(),
        "minecraft:the_nether" => "the Nether".to_owned(),
        "minecraft:the_end" => "the End".to_owned(),
        other => other.to_owned(),
    }
}

/// A biome as the list and the search know it: id and shown name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub name: String,
}

/// What a typed biome is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    Biome(String),
    /// A biome tag, with its namespace (`minecraft:is_forest`).
    Tag(String),
    /// Several biomes match a part of a name.
    Several(Vec<String>),
    Unknown,
}

/// Lower case, spaces and dashes as underscores: `Cherry Grove` -> `cherry_grove`.
fn squash(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .split([' ', '-', '_'])
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// The biome (or tag) `input` names among `entries`: an id with or without
/// its namespace, a shown name, or a part of either that only one has.
pub fn resolve(input: &str, entries: &[Entry]) -> Resolved {
    let input = input.trim();
    if let Some(tag) = input.strip_prefix('#') {
        return if tag.trim().is_empty() {
            Resolved::Unknown
        } else {
            Resolved::Tag(full_id(tag))
        };
    }
    let key = squash(input);
    if key.is_empty() {
        return Resolved::Unknown;
    }
    let full = full_id(input);
    let path = |e: &Entry| e.id.rsplit(':').next().unwrap_or(&e.id).to_owned();
    if let Some(e) = entries
        .iter()
        .find(|e| e.id == full || path(e) == key || squash(&e.name) == key)
    {
        return Resolved::Biome(e.id.clone());
    }
    let mut partial: Vec<String> = entries
        .iter()
        .filter(|e| path(e).contains(&key) || squash(&e.name).contains(&key))
        .map(|e| e.id.clone())
        .collect();
    partial.sort();
    partial.dedup();
    match partial.len() {
        0 => Resolved::Unknown,
        1 => Resolved::Biome(partial.remove(0)),
        _ => Resolved::Several(partial),
    }
}

/// The entries whose id or name contains `filter` (all without one), by name.
pub fn filter_entries(entries: &[Entry], filter: Option<&str>) -> Vec<Entry> {
    let key = filter.map(squash).unwrap_or_default();
    let mut out: Vec<Entry> = entries
        .iter()
        .filter(|e| {
            key.is_empty() || squash(&e.id).contains(&key) || squash(&e.name).contains(&key)
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    out
}

// ---- the search ----------------------------------------------------------

/// The heights a column is sampled at: the player's (within the dimension)
/// first, then every `step` blocks above and below it, nearest first. The
/// Nether's and the End's biomes don't change with height: one each.
pub fn layers(
    dimension: &str,
    origin_y: i32,
    min_y: i32,
    max_y: i32,
    heights: Heights,
) -> Vec<i32> {
    let start = origin_y.clamp(min_y, max_y);
    if heights == Heights::Player || dimension != "minecraft:overworld" {
        return vec![start];
    }
    let mut out = vec![start];
    let mut k = 1;
    loop {
        let (up, down) = (start + k * LAYER_STEP, start - k * LAYER_STEP);
        let (has_up, has_down) = (up <= max_y, down >= min_y);
        if !has_up && !has_down {
            return out;
        }
        if has_up {
            out.push(up);
        }
        if has_down {
            out.push(down);
        }
        k += 1;
    }
}

/// What a search looks for, where and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// The biome ids that end it.
    pub wanted: BTreeSet<String>,
    /// What the player asked for, as shown (`Cherry Grove`, `#minecraft:is_forest`).
    pub label: String,
    pub dimension: String,
    pub origin: Position,
    pub radius: u32,
    pub spacing: u32,
    /// Heights sampled in each column, in order.
    pub layers: Vec<i32>,
}

/// A wanted biome found: which, where, and how far from the origin
/// (squared, horizontally).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub biome: String,
    pub pos: Position,
    pub distance2: i64,
}

/// Column `step` of square ring `ring` around the origin, in columns: ring 0
/// is the origin, ring `k` the `8k` columns `k` away (Chebyshev), clockwise
/// from its north-west corner.
pub fn ring_offset(ring: u32, step: u32) -> [i32; 2] {
    if ring == 0 {
        return [0, 0];
    }
    let k = ring as i32;
    let side = 2 * k;
    let j = step as i32 % side;
    match step as i32 / side {
        0 => [-k + j, -k],
        1 => [k, -k + j],
        2 => [k - j, k],
        _ => [-k, k - j],
    }
}

/// A search spread over ticks: square rings of columns outward from the
/// origin, `spacing` blocks apart, within the `radius` circle. It hands out
/// columns to sample and takes their biomes back; once a wanted biome turns
/// up it finishes the rings that could still hold a nearer one, so the
/// answer is the nearest sample, not merely the first ring's.
#[derive(Clone, Debug)]
pub struct Search {
    pub spec: Spec,
    ring: u32,
    step: u32,
    max_ring: u32,
    /// Samples taken so far.
    pub samples: u64,
    best: Option<Hit>,
    done: bool,
}

impl Search {
    pub fn new(mut spec: Spec) -> Self {
        spec.spacing = spec.spacing.max(1);
        if spec.layers.is_empty() {
            spec.layers.push(spec.origin[1]);
        }
        let max_ring = spec.radius / spec.spacing;
        Self {
            spec,
            ring: 0,
            step: 0,
            max_ring,
            samples: 0,
            best: None,
            done: false,
        }
    }
    pub fn finished(&self) -> bool {
        self.done
    }
    /// The nearest wanted biome found so far.
    pub fn best(&self) -> Option<&Hit> {
        self.best.as_ref()
    }
    /// The next columns (block x, z) to sample, about `samples` samples'
    /// worth (at least one column); none once the search is done.
    pub fn next_columns(&mut self, samples: usize) -> Vec<[i32; 2]> {
        let want = (samples / self.spec.layers.len()).max(1);
        let spacing = i64::from(self.spec.spacing);
        let radius2 = i64::from(self.spec.radius).pow(2);
        let mut out = Vec::new();
        while out.len() < want && !self.done {
            if self.step == 0 {
                // A new ring: past the radius, or no nearer than the best.
                let reach = i64::from(self.ring) * spacing;
                let beaten = self
                    .best
                    .as_ref()
                    .is_some_and(|b| reach * reach > b.distance2);
                if self.ring > self.max_ring || beaten {
                    self.done = true;
                    break;
                }
            }
            let [cx, cz] = ring_offset(self.ring, self.step);
            let length = if self.ring == 0 { 1 } else { 8 * self.ring };
            self.step += 1;
            if self.step >= length {
                self.step = 0;
                self.ring += 1;
            }
            let (dx, dz) = (i64::from(cx) * spacing, i64::from(cz) * spacing);
            if dx * dx + dz * dz > radius2 {
                continue;
            }
            let at = |o: i32, d: i64| (i64::from(o) + d).clamp(-30_000_000, 30_000_000) as i32;
            out.push([at(self.spec.origin[0], dx), at(self.spec.origin[2], dz)]);
        }
        out
    }
    /// The block positions to sample for `columns`: each column at every
    /// layer, in order.
    pub fn probes(&self, columns: &[[i32; 2]]) -> Vec<Position> {
        columns
            .iter()
            .flat_map(|&[x, z]| self.spec.layers.iter().map(move |&y| [x, y, z]))
            .collect()
    }
    /// The biomes the generator gave for `probes(columns)`, in the same
    /// order (None where it gave none).
    pub fn feed(&mut self, columns: &[[i32; 2]], biomes: &[Option<String>]) {
        let per = self.spec.layers.len();
        for (i, &[x, z]) in columns.iter().enumerate() {
            let (dx, dz) = (
                i64::from(x) - i64::from(self.spec.origin[0]),
                i64::from(z) - i64::from(self.spec.origin[2]),
            );
            let distance2 = dx * dx + dz * dz;
            for (j, &y) in self.spec.layers.iter().enumerate() {
                let Some(Some(name)) = biomes.get(i * per + j) else {
                    continue;
                };
                self.samples += 1;
                if !self.spec.wanted.contains(name) {
                    continue;
                }
                if self.best.as_ref().is_none_or(|b| distance2 < b.distance2) {
                    self.best = Some(Hit {
                        biome: name.clone(),
                        pos: [x, y, z],
                        distance2,
                    });
                }
                break;
            }
        }
    }
    /// Blocks from the origin searched all around.
    pub fn searched(&self) -> u32 {
        if self.done {
            return self.spec.radius;
        }
        (self.ring.saturating_sub(1) * self.spec.spacing).min(self.spec.radius)
    }
    /// 0 to 1.
    pub fn progress(&self) -> f32 {
        if self.spec.radius == 0 {
            return 1.0;
        }
        (self.searched() as f32 / self.spec.radius as f32).clamp(0.0, 1.0)
    }
}

// ---- what the HUD shows ----------------------------------------------------

/// A player's search as the HUD and `/biomecompass status` show it.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Searching {
        progress: f32,
        radius: u32,
    },
    /// `name`: the biome found, as shown.
    Found {
        name: String,
        pos: Position,
    },
    Missing {
        radius: u32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub label: String,
    pub dimension: String,
    pub phase: Phase,
}

/// The player looking at the HUD.
#[derive(Clone, Debug, PartialEq)]
pub struct Viewer {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Degrees; 0 faces south, 90 west.
    pub yaw: f32,
    pub dimension: String,
}

/// The yaw (degrees, 0 south, 90 west) that faces from `from` to `to` (x, z).
pub fn bearing(from: [f64; 2], to: [f64; 2]) -> f32 {
    let (dx, dz) = (to[0] - from[0], to[1] - from[1]);
    ((-dx).atan2(dz).to_degrees() as f32).rem_euclid(360.0)
}

/// How far to turn from `yaw` to face `bearing`: -180 (left) to 180 (right).
pub fn relative(bearing: f32, yaw: f32) -> f32 {
    let r = (bearing - yaw).rem_euclid(360.0);
    if r > 180.0 { r - 360.0 } else { r }
}

/// The compass point of a bearing: `N`, `NE`...
pub fn compass_point(bearing: f32) -> &'static str {
    const POINTS: [&str; 8] = ["S", "SW", "W", "NW", "N", "NE", "E", "SE"];
    let i = (bearing.rem_euclid(360.0) / 45.0).round() as usize % 8;
    POINTS[i]
}

/// Horizontal blocks between two points.
pub fn distance(from: [f64; 2], to: [f64; 2]) -> f64 {
    ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt()
}

/// A position's block centre (x, z).
pub fn centre(pos: Position) -> [f64; 2] {
    [f64::from(pos[0]) + 0.5, f64::from(pos[2]) + 0.5]
}

pub const WHITE: u32 = 0xFFFFFF;
pub const YELLOW: u32 = 0xFFFF55;
pub const GRAY: u32 = 0xAAAAAA;
pub const GREEN: u32 = 0x55FF55;
pub const RED: u32 = 0xFF5555;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Needle {
    Hidden,
    /// Searching: the needle turns.
    Spin,
    /// Degrees clockwise from straight ahead (up on the screen).
    Point(f32),
}

/// The HUD box: the needle and the text lines `(text, 0xRRGGBB)`.
#[derive(Clone, Debug, PartialEq)]
pub struct HudView {
    pub needle: Needle,
    pub lines: Vec<(String, u32)>,
}

/// What the HUD shows of `track` to `viewer`.
pub fn hud_view(track: &Track, viewer: &Viewer) -> HudView {
    match &track.phase {
        Phase::Searching { progress, radius } => HudView {
            needle: Needle::Spin,
            lines: vec![
                (format!("Searching: {}", track.label), WHITE),
                (
                    format!("{}% of {radius} blocks", (progress * 100.0).floor() as u32),
                    GRAY,
                ),
            ],
        },
        Phase::Missing { radius } => HudView {
            needle: Needle::Hidden,
            lines: vec![
                (format!("No {}", track.label), RED),
                (format!("within {radius} blocks"), GRAY),
            ],
        },
        Phase::Found { name, pos } => {
            if viewer.dimension != track.dimension {
                return HudView {
                    needle: Needle::Hidden,
                    lines: vec![
                        (name.clone(), WHITE),
                        (format!("in {}", dimension_name(&track.dimension)), GRAY),
                    ],
                };
            }
            let here = [viewer.x, viewer.z];
            let there = centre(*pos);
            let d = distance(here, there);
            if d < 3.0 {
                return HudView {
                    needle: Needle::Hidden,
                    lines: vec![(name.clone(), WHITE), ("You are there".to_owned(), GREEN)],
                };
            }
            let b = bearing(here, there);
            let mut line = format!("{} blocks {}", d.round() as i64, compass_point(b));
            if (f64::from(pos[1]) - viewer.y).abs() > 16.0 {
                line.push_str(&format!(", y {}", pos[1]));
            }
            HudView {
                needle: Needle::Point(relative(b, viewer.yaw)),
                lines: vec![(name.clone(), WHITE), (line, YELLOW)],
            }
        }
    }
}

/// A compass needle at `centre`, turned `angle` degrees clockwise from up:
/// the half toward the target and the other half, each a triangle given as
/// a quad (a vertex repeated) of screen points.
pub fn needle(
    angle: f32,
    centre: [f32; 2],
    length: f32,
    width: f32,
) -> ([[f32; 2]; 4], [[f32; 2]; 4]) {
    let (s, c) = angle.to_radians().sin_cos();
    let along = [s, -c];
    let across = [c, s];
    let at = |a: f32, b: f32| {
        [
            centre[0] + along[0] * a + across[0] * b,
            centre[1] + along[1] * a + across[1] * b,
        ]
    };
    let (tip, tail) = (at(length, 0.0), at(-length, 0.0));
    let (left, right) = (at(0.0, -width), at(0.0, width));
    let mid = at(0.0, 0.0);
    ([tip, right, mid, left], [tail, left, mid, right])
}

// ---- the HUD box beside F3 ----------------------------------------------------

/// Whether F3's debug screen (`f3`) hides the box: F3 has a column in each
/// top corner, and between them, at the top centre, other mods' boxes go
/// (LookAt's, whose height changes with what you look at, placed without
/// `Corners`), so a top-left or top-right box hides while F3 is open; the
/// Biome Compass's needle and the action bar still show the search. A
/// bottom corner keeps its place.
pub fn hud_hidden_by_f3(place: Corner, f3: bool) -> bool {
    f3 && matches!(place, Corner::TopLeft | Corner::TopRight)
}

// ---- chat lines -------------------------------------------------------------

pub fn started_message(label: &str, radius: u32, dimension: &str) -> String {
    format!(
        "BiomeCompass: searching for {label} within {radius} blocks of you in {}...",
        dimension_name(dimension)
    )
}

pub fn progress_message(label: &str, searched: u32, radius: u32) -> String {
    format!("BiomeCompass: still searching for {label}: {searched} of {radius} blocks done.")
}

pub fn missing_message(label: &str, radius: u32) -> String {
    format!("BiomeCompass: no {label} within {radius} blocks.")
}

/// The words before and after the found position's coordinates.
pub fn found_message(name: &str, label: &str, pos: Position, from: [f64; 2]) -> (String, String) {
    let there = centre(pos);
    let d = distance(from, there).round() as i64;
    let point = compass_point(bearing(from, there));
    let what = if label.starts_with('#') {
        format!("{name} ({label})")
    } else {
        name.to_owned()
    };
    (
        format!("BiomeCompass: found {what} at "),
        format!(", {d} blocks {point}."),
    )
}

/// `/biomecompass status` without a running search.
pub fn status_message(track: Option<&Track>, from: [f64; 2], dimension: &str) -> String {
    let Some(track) = track else {
        return format!(
            "BiomeCompass: no search. Try /{COMMAND} list, then /{COMMAND} find <biome>."
        );
    };
    match &track.phase {
        Phase::Searching { progress, radius } => format!(
            "BiomeCompass: searching for {}: {}% of {radius} blocks.",
            track.label,
            (progress * 100.0).floor() as u32
        ),
        Phase::Missing { radius } => missing_message(&track.label, *radius),
        Phase::Found { name, pos } if track.dimension == dimension => {
            let there = centre(*pos);
            format!(
                "BiomeCompass: {name} at {} {} {}, {} blocks {}.",
                pos[0],
                pos[1],
                pos[2],
                distance(from, there).round() as i64,
                compass_point(bearing(from, there))
            )
        }
        Phase::Found { name, pos } => format!(
            "BiomeCompass: {name} at {} {} {} in {}.",
            pos[0],
            pos[1],
            pos[2],
            dimension_name(&track.dimension)
        ),
    }
}

// ---- the Biome Compass ------------------------------------------------------

/// The item a Biome Compass is: a vanilla compass with BiomeCompass's mark,
/// so vanilla clients, servers and saves know it as what it is.
pub const ITEM: &str = "minecraft:compass";
/// Its name (`minecraft:custom_name` and `minecraft:item_name`; a compass
/// with a lodestone tracker is named "Lodestone Compass" unless it has a
/// custom name).
pub const ITEM_NAME: &str = "Biome Compass";
/// BiomeCompass's mark: `minecraft:custom_data` `{biomecompass: 1b}`.
pub const MARK: &str = "biomecompass";
pub const CUSTOM_DATA: &str = "minecraft:custom_data";
pub const ITEM_NAME_COMPONENT: &str = "minecraft:item_name";
pub const LORE: &str = "minecraft:lore";
pub const TRACKER: &str = "minecraft:lodestone_tracker";
/// The item tag the recipe takes saplings from.
pub const SAPLINGS: &str = "minecraft:saplings";
/// The recipe in words.
pub const RECIPE_TEXT: &str = "a compass in the middle of a crafting table and a sapling (any kind) on each of its four sides";

/// An NBT value as BiomeCompass writes them; the adapter turns it into the
/// game's (`item.rs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Nbt {
    Byte(i8),
    Int(i32),
    String(String),
    IntArray(Vec<i32>),
    List(Vec<Nbt>),
    Compound(Vec<(String, Nbt)>),
}
impl Nbt {
    pub fn compound<const N: usize>(entries: [(&str, Nbt); N]) -> Self {
        Nbt::Compound(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        )
    }
    pub fn string(s: impl Into<String>) -> Self {
        Nbt::String(s.into())
    }
    /// A compound's entry.
    pub fn get(&self, key: &str) -> Option<&Nbt> {
        match self {
            Nbt::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    /// A string's value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Nbt::String(s) => Some(s),
            _ => None,
        }
    }
}

/// A text component: plain text in a colour, not italic (lore is italic
/// unless it says otherwise).
fn styled(text: &str, color: &str) -> Nbt {
    Nbt::compound([
        ("text", Nbt::string(text)),
        ("color", Nbt::string(color)),
        ("italic", Nbt::Byte(0)),
    ])
}

/// `minecraft:custom_data` with BiomeCompass's mark.
pub fn custom_data() -> Nbt {
    Nbt::compound([(MARK, Nbt::Byte(1))])
}

/// `minecraft:item_name`.
pub fn item_name() -> Nbt {
    Nbt::string(ITEM_NAME)
}

/// What a player's Biome Compasses point at: their search.
#[derive(Clone, Debug, PartialEq)]
pub enum Pointing {
    /// No search: a plain compass (the world spawn).
    Idle,
    /// The needle spins.
    Searching { label: String },
    /// The needle points at the biome found.
    Found {
        name: String,
        dimension: String,
        pos: Position,
    },
    /// Nothing found: a plain compass again.
    Missing { label: String },
}

/// A player's search as their Biome Compasses show it.
pub fn pointing(track: Option<&Track>) -> Pointing {
    let Some(track) = track else {
        return Pointing::Idle;
    };
    match &track.phase {
        Phase::Searching { .. } => Pointing::Searching {
            label: track.label.clone(),
        },
        Phase::Found { name, pos } => Pointing::Found {
            name: name.clone(),
            dimension: track.dimension.clone(),
            pos: *pos,
        },
        Phase::Missing { .. } => Pointing::Missing {
            label: track.label.clone(),
        },
    }
}

/// `minecraft:lodestone_tracker` for what the compass points at (None: the
/// component goes, and the compass points at the world spawn).
/// `tracked: false`: no lodestone is needed at the target, so the game never
/// takes it away. Without a target the needle spins, as a lodestone
/// compass's whose lodestone is gone.
pub fn tracker(pointing: &Pointing) -> Option<Nbt> {
    match pointing {
        Pointing::Idle | Pointing::Missing { .. } => None,
        Pointing::Searching { .. } => Some(Nbt::compound([("tracked", Nbt::Byte(0))])),
        Pointing::Found { dimension, pos, .. } => Some(Nbt::compound([
            (
                "target",
                Nbt::compound([
                    ("dimension", Nbt::string(dimension.as_str())),
                    ("pos", Nbt::IntArray(pos.to_vec())),
                ]),
            ),
            ("tracked", Nbt::Byte(0)),
        ])),
    }
}

/// `minecraft:lore`: how to use it, and what it points at.
pub fn lore(pointing: &Pointing) -> Nbt {
    let mut lines = vec![styled("Right-click to choose a biome", "gray")];
    match pointing {
        Pointing::Idle => {}
        Pointing::Searching { label } => {
            lines.push(styled(&format!("Searching: {label}"), "yellow"));
        }
        Pointing::Found {
            name, dimension, ..
        } => {
            lines.push(styled(&format!("Points to: {name}"), "green"));
            lines.push(styled(
                &format!("in {}", dimension_name(dimension)),
                "dark_gray",
            ));
        }
        Pointing::Missing { label } => {
            lines.push(styled(&format!("No {label} found"), "red"));
        }
    }
    Nbt::List(lines)
}

/// Whether a stack's components (by component id) are a Biome Compass's:
/// BiomeCompass's mark, or its item name (a client that never saw the mark,
/// a creative player's copy, keeps the name; the mark is put back).
pub fn is_biome_compass(item: &str, component: &dyn Fn(&str) -> Option<Nbt>) -> bool {
    if item != ITEM {
        return false;
    }
    component(CUSTOM_DATA).is_some_and(|c| c.get(MARK).is_some())
        || component(ITEM_NAME_COMPONENT) == Some(item_name())
}

/// The recipe, on a crafting grid of `size` by `size` (row-major item ids):
/// a 3x3 grid with a compass in the middle, a sapling on each of its four
/// sides (any kinds, mixed) and the corners empty. Like a vanilla shaped
/// recipe, any compass will do, a Biome Compass included.
pub fn recipe_matches(
    grid: &[Option<&str>],
    size: usize,
    is_sapling: &dyn Fn(&str) -> bool,
) -> bool {
    if size != 3 || grid.len() != 9 {
        return false;
    }
    grid.iter().enumerate().all(|(i, cell)| match (i, cell) {
        (4, Some(item)) => *item == ITEM,
        (1 | 3 | 5 | 7, Some(item)) => is_sapling(item),
        (0 | 2 | 6 | 8, None) => true,
        _ => false,
    })
}

/// A sapling by its id, for a data pack with no `#minecraft:saplings`.
pub fn looks_like_sapling(id: &str) -> bool {
    id.ends_with("_sapling") || id.ends_with("_propagule") || id.ends_with("azalea")
}

// ---- the biome picker --------------------------------------------------------

/// The picker's text box, and the command its Search button runs (the
/// filter in double quotes: a quoted phrase, empty or not).
pub const FILTER_KEY: &str = "filter";
/// Buttons per row, and their width.
pub const PICKER_COLUMNS: i32 = 3;
pub const PICKER_BUTTON: i32 = 120;
/// The most characters the search box takes; a dialog whose text is longer
/// than its box allows is refused by a vanilla client (and the connection
/// with it), so a longer filter is cut.
pub const FILTER_MAX: usize = 32;

/// What the picker shows.
#[derive(Clone, Debug, PartialEq)]
pub struct Picker<'a> {
    /// The dimension's biomes.
    pub entries: &'a [Entry],
    pub filter: Option<&'a str>,
    pub dimension: &'a str,
    /// The player's search, as `/biomecompass status` says it.
    pub status: Option<String>,
    /// The biome (or tag) the player's search is for.
    pub current: Option<&'a str>,
}

fn button(label: Nbt, tooltip: Option<&str>, action: Nbt) -> Nbt {
    let mut entries = vec![
        ("label".to_owned(), label),
        ("width".to_owned(), Nbt::Int(PICKER_BUTTON)),
        ("action".to_owned(), action),
    ];
    if let Some(t) = tooltip {
        entries.push(("tooltip".to_owned(), Nbt::string(t)));
    }
    Nbt::Compound(entries)
}

fn run(command: String) -> Nbt {
    Nbt::compound([
        ("type", Nbt::string("minecraft:run_command")),
        ("command", Nbt::string(command)),
    ])
}

/// The biome picker as a vanilla dialog (`minecraft:multi_action`, sent with
/// `ClientboundShowDialogPacket`): a search box, Search / All / Stop, then a
/// button per biome matching the filter. Its buttons run BiomeCompass's
/// commands as the player, as a click in chat does; they are in the command
/// tree a player receives, so a vanilla client sends them without asking.
pub fn picker_dialog(p: &Picker) -> Nbt {
    let shown = filter_entries(p.entries, p.filter);
    let place = dimension_name(p.dimension);
    let mut body = vec![];
    let summary = match p.filter {
        _ if p.entries.is_empty() => format!("No biome generates in {place}."),
        Some(f) if shown.is_empty() => format!("No biome of {place} matches \"{f}\"."),
        Some(f) => format!(
            "{} of the {} biomes of {place} match \"{f}\". Click one to search for it.",
            shown.len(),
            p.entries.len()
        ),
        None => format!(
            "The {} biomes of {place}. Click one to search for the nearest.",
            shown.len()
        ),
    };
    body.push(Nbt::compound([
        ("type", Nbt::string("minecraft:plain_message")),
        ("contents", Nbt::string(summary)),
        ("width", Nbt::Int(300)),
    ]));
    if let Some(status) = &p.status {
        body.push(Nbt::compound([
            ("type", Nbt::string("minecraft:plain_message")),
            ("contents", styled(status, "gray")),
            ("width", Nbt::Int(300)),
        ]));
    }
    let mut actions = vec![
        button(
            Nbt::string("Search"),
            Some("Show the biomes whose name has the text above"),
            Nbt::compound([
                ("type", Nbt::string("minecraft:dynamic/run_command")),
                (
                    "template",
                    Nbt::string(format!("{COMMAND} pick \"$({FILTER_KEY})\"")),
                ),
            ]),
        ),
        button(
            Nbt::string("All biomes"),
            None,
            run(format!("{COMMAND} pick")),
        ),
        button(
            styled("Stop", "red"),
            Some("Stop searching; the compass points at the world spawn again"),
            run(format!("{COMMAND} stop")),
        ),
    ];
    for e in &shown {
        let label = if p.current == Some(e.name.as_str()) {
            styled(&e.name, "yellow")
        } else {
            Nbt::string(e.name.as_str())
        };
        actions.push(button(
            label,
            Some(&e.id),
            run(format!("{COMMAND} find {}", e.id)),
        ));
    }
    Nbt::compound([
        ("type", Nbt::string("minecraft:multi_action")),
        ("title", Nbt::string(ITEM_NAME)),
        ("body", Nbt::List(body)),
        (
            "inputs",
            Nbt::List(vec![Nbt::compound([
                ("type", Nbt::string("minecraft:text")),
                ("key", Nbt::string(FILTER_KEY)),
                ("label", Nbt::string("Biome name")),
                ("width", Nbt::Int(PICKER_COLUMNS * (PICKER_BUTTON + 2))),
                (
                    "initial",
                    Nbt::string(
                        p.filter
                            .unwrap_or_default()
                            .chars()
                            .take(FILTER_MAX)
                            .collect::<String>(),
                    ),
                ),
                ("max_length", Nbt::Int(FILTER_MAX as i32)),
            ])]),
        ),
        ("can_close_with_escape", Nbt::Byte(1)),
        // Single player's server keeps running behind it.
        ("pause", Nbt::Byte(0)),
        ("after_action", Nbt::string("close")),
        ("actions", Nbt::List(actions)),
        ("columns", Nbt::Int(PICKER_COLUMNS)),
        (
            "exit_action",
            Nbt::compound([
                (
                    "label",
                    Nbt::compound([("translate", Nbt::string("gui.done"))]),
                ),
                ("width", Nbt::Int(200)),
            ]),
        ),
    ])
}

/// The line over the hotbar while a Biome Compass is held: the search, or
/// the biome and its distance (and colour).
pub fn action_bar(track: &Track, viewer: &Viewer) -> (String, u32) {
    let view = hud_view(track, viewer);
    let line = |i: usize| view.lines.get(i).map(|l| l.0.clone()).unwrap_or_default();
    match &track.phase {
        Phase::Searching { .. } => (format!("{}, {}", line(0), line(1)), WHITE),
        Phase::Missing { .. } => (format!("{} {}", line(0), line(1)), RED),
        Phase::Found { .. } => {
            let rgb = view.lines.get(1).map_or(WHITE, |l| l.1);
            (format!("{}: {}", line(0), line(1)), rgb)
        }
    }
}

/// `/biomecompass status` without its "BiomeCompass: " (the picker's line).
pub fn status_line(track: Option<&Track>, from: [f64; 2], dimension: &str) -> Option<String> {
    let line = status_message(Some(track?), from, dimension);
    Some(
        line.strip_prefix("BiomeCompass: ")
            .map(|rest| {
                let mut c = rest.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect())
                    .unwrap_or_default()
            })
            .unwrap_or(line),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashSet};

    fn spec(wanted: &[&str], radius: u32, spacing: u32, layers: Vec<i32>) -> Spec {
        Spec {
            wanted: wanted.iter().map(|s| s.to_string()).collect(),
            label: "Test".into(),
            dimension: "minecraft:overworld".into(),
            origin: [100, 64, -50],
            radius,
            spacing,
            layers,
        }
    }

    /// Runs `search` to the end over `world` (block x, y, z -> biome),
    /// `budget` samples a step; returns the steps taken.
    fn run(
        search: &mut Search,
        world: &dyn Fn(Position) -> Option<String>,
        budget: usize,
    ) -> usize {
        let mut steps = 0;
        while !search.finished() {
            let columns = search.next_columns(budget);
            let biomes: Vec<_> = search.probes(&columns).into_iter().map(world).collect();
            search.feed(&columns, &biomes);
            steps += 1;
            assert!(steps < 1_000_000, "the search never ends");
        }
        steps
    }

    #[test]
    fn rings_cover_each_column_once() {
        for ring in 0..12u32 {
            let len = if ring == 0 { 1 } else { 8 * ring };
            let seen: HashSet<[i32; 2]> = (0..len).map(|s| ring_offset(ring, s)).collect();
            assert_eq!(seen.len(), len as usize, "ring {ring}");
            for [x, z] in seen {
                assert_eq!(x.abs().max(z.abs()), ring as i32);
            }
        }
    }

    #[test]
    fn search_visits_the_circle_once() {
        let mut s = Search::new(spec(&["none"], 320, 32, vec![64]));
        let mut seen = HashSet::new();
        loop {
            let cols = s.next_columns(7);
            if cols.is_empty() {
                break;
            }
            for c in &cols {
                assert!(seen.insert(*c), "{c:?} twice");
                let (dx, dz) = (i64::from(c[0] - 100), i64::from(c[1] + 50));
                assert!(dx * dx + dz * dz <= 320 * 320);
                assert_eq!(dx % 32, 0);
            }
            let biomes = vec![Some("plains".to_owned()); cols.len()];
            s.feed(&cols, &biomes);
        }
        assert!(s.finished());
        // Every grid point of the circle: 11 x 11 grid's points within 10.
        let expected = (-10i64..=10)
            .flat_map(|x| (-10i64..=10).map(move |z| (x, z)))
            .filter(|(x, z)| x * x + z * z <= 100)
            .count();
        assert_eq!(seen.len(), expected);
        assert_eq!(s.samples, expected as u64);
        assert_eq!(s.best(), None);
        assert_eq!(s.searched(), 320);
    }

    #[test]
    fn finds_the_origin_column_at_once() {
        let mut s = Search::new(spec(&["minecraft:forest"], 6400, 32, vec![64]));
        let steps = run(&mut s, &|_| Some("minecraft:forest".into()), 1);
        assert_eq!(steps, 2);
        assert_eq!(
            s.best(),
            Some(&Hit {
                biome: "minecraft:forest".into(),
                pos: [100, 64, -50],
                distance2: 0
            })
        );
    }

    #[test]
    fn nearest_by_distance_not_by_ring() {
        // The first ring that holds the biome has it in a corner (ring 5,
        // 5*sqrt(2) = 7.07 columns away); the next rings have it straight
        // east, 6 columns away: the search goes on and takes that one.
        let world = |p: Position| {
            let (cx, cz) = ((p[0] - 100) / 16, (p[2] + 50) / 16);
            Some(if (cx, cz) == (5, 5) || (cx, cz) == (6, 0) {
                "minecraft:cherry_grove".to_owned()
            } else {
                "minecraft:plains".to_owned()
            })
        };
        let mut s = Search::new(spec(&["minecraft:cherry_grove"], 6400, 16, vec![70]));
        run(&mut s, &world, 64);
        let best = s.best().unwrap();
        assert_eq!(best.pos, [100 + 96, 70, -50]);
        assert_eq!(best.distance2, 96 * 96);
        // Ring 7 (7 columns) could still beat the corner's 7.07, so it is
        // searched; ring 8 can't beat 6: the search ends with rings 0 to 7.
        assert!(s.finished());
        assert_eq!(s.samples, 15 * 15);
    }

    #[test]
    fn ties_keep_the_first_found() {
        let world = |p: Position| {
            let (cx, cz) = ((p[0] - 100) / 32, (p[2] + 50) / 32);
            Some(if cx.abs() + cz.abs() == 2 && (cx == 0 || cz == 0) {
                "minecraft:desert".to_owned()
            } else {
                "minecraft:plains".to_owned()
            })
        };
        let mut s = Search::new(spec(&["minecraft:desert"], 6400, 32, vec![64]));
        run(&mut s, &world, 1000);
        // Ring 2 starts in the north-west corner: (0, -2) comes first.
        assert_eq!(s.best().unwrap().pos, [100, 64, -50 - 64]);
    }

    #[test]
    fn layers_are_tried_in_order_per_column() {
        let world = |p: Position| {
            Some(if p[1] < 0 {
                "minecraft:lush_caves".to_owned()
            } else {
                "minecraft:plains".to_owned()
            })
        };
        let mut s = Search::new(spec(
            &["minecraft:lush_caves"],
            1000,
            32,
            layers("minecraft:overworld", 64, -64, 319, Heights::All),
        ));
        run(&mut s, &world, 100);
        // Heights 64, 128, 0, 192, -64, 256: -64 is the first below 0.
        assert_eq!(s.best().unwrap().pos, [100, -64, -50]);
        // Missing with the player's height only.
        let mut s = Search::new(spec(&["minecraft:lush_caves"], 200, 32, vec![64]));
        run(&mut s, &world, 100);
        assert_eq!(s.best(), None);
    }

    #[test]
    fn layer_order() {
        assert_eq!(
            layers("minecraft:overworld", 70, -64, 319, Heights::All),
            vec![70, 134, 6, 198, -58, 262]
        );
        assert_eq!(
            layers("minecraft:overworld", 500, -64, 319, Heights::Player),
            vec![319]
        );
        assert_eq!(
            layers("minecraft:the_nether", 40, 0, 255, Heights::All),
            vec![40]
        );
        assert_eq!(
            layers("minecraft:overworld", -64, -64, 0, Heights::All),
            vec![-64, 0]
        );
    }

    #[test]
    fn a_missing_sample_is_skipped() {
        let mut s = Search::new(spec(&["minecraft:forest"], 64, 32, vec![64]));
        run(&mut s, &|_| None, 10);
        assert_eq!(s.best(), None);
        assert_eq!(s.samples, 0);
        assert!(s.finished());
    }

    #[test]
    fn progress_rises_to_one() {
        let mut s = Search::new(spec(&["x"], 640, 32, vec![64]));
        let mut last = 0.0;
        while !s.finished() {
            let cols = s.next_columns(40);
            let b = vec![Some("y".to_owned()); cols.len()];
            s.feed(&cols, &b);
            assert!(s.progress() >= last);
            last = s.progress();
        }
        assert_eq!(s.progress(), 1.0);
    }

    #[test]
    fn budget_counts_samples_not_columns() {
        let mut s = Search::new(spec(&["x"], 6400, 32, vec![0, 64, -64, 128]));
        assert_eq!(s.next_columns(1).len(), 1);
        assert_eq!(s.next_columns(40).len(), 10);
        assert_eq!(
            s.probes(&[[1, 2]]),
            vec![[1, 0, 2], [1, 64, 2], [1, -64, 2], [1, 128, 2]]
        );
    }

    #[test]
    fn far_origins_stay_in_the_world() {
        let mut sp = spec(&["x"], 100_000, 512, vec![64]);
        sp.origin = [29_999_990, 64, -29_999_990];
        let mut s = Search::new(sp);
        for _ in 0..50 {
            for [x, z] in s.next_columns(1000) {
                assert!(x.abs() <= 30_000_000 && z.abs() <= 30_000_000);
            }
        }
    }

    #[test]
    fn commands() {
        assert_eq!(parse_command("say hi"), None);
        assert_eq!(parse_command("biomecompassx"), None);
        assert_eq!(parse_command("biomecompass"), Some(Ok(Command::Status)));
        assert_eq!(parse_command("/bcompass status"), Some(Ok(Command::Status)));
        assert_eq!(
            parse_command("biomecompass list"),
            Some(Ok(Command::List(None)))
        );
        assert_eq!(
            parse_command("biomecompass list ocean"),
            Some(Ok(Command::List(Some("ocean".into()))))
        );
        assert_eq!(
            parse_command("biomecompass find Cherry  Grove"),
            Some(Ok(Command::Find("Cherry Grove".into())))
        );
        assert_eq!(
            parse_command("biomecompass FIND #is_forest"),
            Some(Ok(Command::Find("#is_forest".into())))
        );
        assert!(matches!(parse_command("biomecompass find"), Some(Err(_))));
        assert_eq!(
            parse_command("biomecompass tp"),
            Some(Ok(Command::Teleport))
        );
        assert_eq!(
            parse_command("biomecompass cancel"),
            Some(Ok(Command::Stop))
        );
        assert_eq!(
            parse_command("biomecompass reload"),
            Some(Ok(Command::Reload))
        );
        assert_eq!(parse_command("biomecompass help"), Some(Ok(Command::Help)));
        assert_eq!(
            parse_command("biomecompass tp now"),
            Some(Err(USAGE.into()))
        );
        assert_eq!(parse_command("biomecompass dance"), Some(Err(USAGE.into())));
        assert!(is_command("bcompass find desert"));
        assert!(!is_command("tp @s 0 0 0"));
    }

    fn entries() -> Vec<Entry> {
        [
            ("minecraft:cherry_grove", "Cherry Grove"),
            ("minecraft:ocean", "Ocean"),
            ("minecraft:deep_ocean", "Deep Ocean"),
            ("minecraft:warm_ocean", "Warm Ocean"),
            ("minecraft:forest", "Forest"),
            ("minecraft:birch_forest", "Birch Forest"),
            ("mymod:ruby_fields", "Ruby Fields"),
        ]
        .iter()
        .map(|(id, name)| Entry {
            id: id.to_string(),
            name: name.to_string(),
        })
        .collect()
    }

    #[test]
    fn resolving_names() {
        let e = entries();
        let biome = |s: &str| Resolved::Biome(s.to_owned());
        assert_eq!(resolve("cherry_grove", &e), biome("minecraft:cherry_grove"));
        assert_eq!(
            resolve("minecraft:cherry_grove", &e),
            biome("minecraft:cherry_grove")
        );
        assert_eq!(resolve("Cherry Grove", &e), biome("minecraft:cherry_grove"));
        assert_eq!(resolve("cherry", &e), biome("minecraft:cherry_grove"));
        assert_eq!(resolve("ruby", &e), biome("mymod:ruby_fields"));
        assert_eq!(resolve("mymod:ruby_fields", &e), biome("mymod:ruby_fields"));
        // An exact name wins over the parts of others.
        assert_eq!(resolve("ocean", &e), biome("minecraft:ocean"));
        assert_eq!(resolve("forest", &e), biome("minecraft:forest"));
        assert_eq!(
            resolve("oce", &e),
            Resolved::Several(vec![
                "minecraft:deep_ocean".into(),
                "minecraft:ocean".into(),
                "minecraft:warm_ocean".into()
            ])
        );
        assert_eq!(
            resolve("#is_forest", &e),
            Resolved::Tag("minecraft:is_forest".into())
        );
        assert_eq!(resolve("#c:is_hot", &e), Resolved::Tag("c:is_hot".into()));
        assert_eq!(resolve("#", &e), Resolved::Unknown);
        assert_eq!(resolve("jungle", &e), Resolved::Unknown);
        assert_eq!(resolve("  ", &e), Resolved::Unknown);
    }

    #[test]
    fn filtering_the_list() {
        let e = entries();
        let names = |f: Option<&str>| {
            filter_entries(&e, f)
                .into_iter()
                .map(|e| e.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(names(Some("ocean")), ["Deep Ocean", "Ocean", "Warm Ocean"]);
        assert_eq!(names(Some("mymod")), ["Ruby Fields"]);
        assert_eq!(names(None).len(), 7);
        assert_eq!(names(None)[0], "Birch Forest");
    }

    #[test]
    fn names_and_keys() {
        assert_eq!(pretty("minecraft:cherry_grove"), "Cherry Grove");
        assert_eq!(pretty("mymod:caves/glow_hollow"), "Caves Glow Hollow");
        assert_eq!(
            translation_key("minecraft:the_void"),
            "biome.minecraft.the_void"
        );
        assert_eq!(translation_key("plains"), "biome.minecraft.plains");
        assert_eq!(full_id("Desert"), "minecraft:desert");
        assert_eq!(
            dimension_tag("minecraft:the_nether"),
            Some("minecraft:is_nether")
        );
        assert_eq!(dimension_tag("mymod:moon"), None);
        assert_eq!(dimension_name("minecraft:the_end"), "the End");
    }

    #[test]
    fn bearings_follow_minecraft_yaw() {
        let o = [0.5, 0.5];
        assert_eq!(bearing(o, [0.5, 10.5]), 0.0); // south
        assert_eq!(bearing(o, [-9.5, 0.5]), 90.0); // west
        assert_eq!(bearing(o, [0.5, -9.5]), 180.0); // north
        assert_eq!(bearing(o, [10.5, 0.5]), 270.0); // east
        assert_eq!(compass_point(0.0), "S");
        assert_eq!(compass_point(90.0), "W");
        assert_eq!(compass_point(180.0), "N");
        assert_eq!(compass_point(225.0), "NE");
        assert_eq!(compass_point(-90.0), "E");
        assert_eq!(compass_point(359.0), "S");
        assert_eq!(relative(90.0, 0.0), 90.0);
        assert_eq!(relative(0.0, 90.0), -90.0);
        assert_eq!(relative(10.0, 350.0), 20.0);
        assert_eq!(relative(350.0, -370.0), 0.0);
        assert_eq!(relative(180.0, 0.0), 180.0);
    }

    fn viewer(x: f64, z: f64, yaw: f32) -> Viewer {
        Viewer {
            x,
            y: 64.0,
            z,
            yaw,
            dimension: "minecraft:overworld".into(),
        }
    }

    #[test]
    fn hud_points_the_needle() {
        let track = Track {
            label: "Desert".into(),
            dimension: "minecraft:overworld".into(),
            phase: Phase::Found {
                name: "Desert".into(),
                pos: [99, 70, 0],
            },
        };
        // Target east of the player, who faces south: turn left (east is
        // on the left when facing south).
        let v = hud_view(&track, &viewer(0.5, 0.5, 0.0));
        assert_eq!(v.needle, Needle::Point(-90.0));
        assert_eq!(
            v.lines,
            vec![("Desert".into(), WHITE), ("99 blocks E".into(), YELLOW)]
        );
        // Facing east: straight ahead.
        let v = hud_view(&track, &viewer(0.5, 0.5, 270.0));
        assert_eq!(v.needle, Needle::Point(0.0));
        // Deep below: the height is shown.
        let mut deep = track.clone();
        deep.phase = Phase::Found {
            name: "Lush Caves".into(),
            pos: [99, -20, 0],
        };
        assert_eq!(
            hud_view(&deep, &viewer(0.5, 0.5, 0.0)).lines[1].0,
            "99 blocks E, y -20"
        );
        // Standing in it.
        let v = hud_view(&track, &viewer(99.4, 0.6, 0.0));
        assert_eq!(v.needle, Needle::Hidden);
        assert_eq!(v.lines[1].0, "You are there");
        // Another dimension.
        let mut nether = viewer(0.0, 0.0, 0.0);
        nether.dimension = "minecraft:the_nether".into();
        let v = hud_view(&track, &nether);
        assert_eq!(v.needle, Needle::Hidden);
        assert_eq!(v.lines[1].0, "in the Overworld");
    }

    #[test]
    fn only_a_client_of_another_computers_server_predicts_no_compass() {
        // In-frame (no server to join): the game makes it.
        assert!(!foreign_client(false, false));
        // Single player's server thread, or the LAN world this window hosts.
        assert!(!foreign_client(true, true));
        // A server on another computer (a LAN host's guest included).
        assert!(foreign_client(true, false));
        // The flag is the thread's own.
        set_client_thread(true);
        assert!(client_thread());
        std::thread::spawn(|| assert!(!client_thread()))
            .join()
            .unwrap();
        set_client_thread(false);
        assert!(!client_thread());
    }

    #[test]
    fn f3_hides_a_top_box() {
        assert!(hud_hidden_by_f3(Corner::TopLeft, true));
        assert!(hud_hidden_by_f3(Corner::TopRight, true));
        assert!(!hud_hidden_by_f3(Corner::BottomLeft, true));
        assert!(!hud_hidden_by_f3(Corner::BottomRight, true));
        for c in Corner::ALL {
            assert!(!hud_hidden_by_f3(c, false));
        }
    }

    #[test]
    fn hud_while_searching_and_after() {
        let mut track = Track {
            label: "#minecraft:is_forest".into(),
            dimension: "minecraft:overworld".into(),
            phase: Phase::Searching {
                progress: 0.256,
                radius: 6400,
            },
        };
        let v = hud_view(&track, &viewer(0.0, 0.0, 0.0));
        assert_eq!(v.needle, Needle::Spin);
        assert_eq!(v.lines[0].0, "Searching: #minecraft:is_forest");
        assert_eq!(v.lines[1].0, "25% of 6400 blocks");
        track.phase = Phase::Missing { radius: 6400 };
        let v = hud_view(&track, &viewer(0.0, 0.0, 0.0));
        assert_eq!(v.needle, Needle::Hidden);
        assert_eq!(v.lines[0], ("No #minecraft:is_forest".into(), RED));
    }

    #[test]
    fn needle_shape() {
        let close =
            |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4;
        let (front, back) = needle(0.0, [10.0, 10.0], 8.0, 2.0);
        assert!(close(front[0], [10.0, 2.0])); // tip up
        assert!(close(back[0], [10.0, 18.0]));
        assert!(close(front[1], [12.0, 10.0]));
        assert!(close(front[3], [8.0, 10.0]));
        let (front, _) = needle(90.0, [10.0, 10.0], 8.0, 2.0);
        assert!(close(front[0], [18.0, 10.0])); // tip right
        let (front, _) = needle(-90.0, [0.0, 0.0], 5.0, 1.0);
        assert!(close(front[0], [-5.0, 0.0])); // tip left
    }

    #[test]
    fn chat_lines() {
        let (before, after) =
            found_message("Cherry Grove", "Cherry Grove", [300, 90, -400], [0.5, 0.5]);
        assert_eq!(before, "BiomeCompass: found Cherry Grove at ");
        assert_eq!(after, ", 500 blocks NE.");
        let (before, _) = found_message("Forest", "#minecraft:is_forest", [0, 0, 0], [0.0, 0.0]);
        assert_eq!(
            before,
            "BiomeCompass: found Forest (#minecraft:is_forest) at "
        );
        assert_eq!(
            started_message("Desert", 6400, "minecraft:overworld"),
            "BiomeCompass: searching for Desert within 6400 blocks of you in the Overworld..."
        );
        assert!(status_message(None, [0.0, 0.0], "minecraft:overworld").contains("no search"));
        let found = Track {
            label: "Desert".into(),
            dimension: "minecraft:overworld".into(),
            phase: Phase::Found {
                name: "Desert".into(),
                pos: [-100, 64, 0],
            },
        };
        assert_eq!(
            status_message(Some(&found), [0.5, 0.5], "minecraft:overworld"),
            "BiomeCompass: Desert at -100 64 0, 100 blocks W."
        );
        assert_eq!(
            status_message(Some(&found), [0.5, 0.5], "minecraft:the_nether"),
            "BiomeCompass: Desert at -100 64 0 in the Overworld."
        );
    }

    #[test]
    fn settings_file_round_trip() {
        let c = Config {
            radius: 10_000,
            spacing: 16,
            samples_per_tick: 4096,
            heights: Heights::Player,
            hud: Some(Corner::BottomLeft),
            hud_needs_compass: true,
            action_bar: false,
            teleport: Teleport::Off,
        };
        let (back, problems) = Config::parse(&c.to_file());
        assert_eq!(back, c);
        assert!(problems.is_empty(), "{problems:?}");
        let (d, problems) = Config::parse(&Config::default().to_file());
        assert_eq!(d, Config::default());
        assert!(problems.is_empty());
    }

    #[test]
    fn settings_file_problems_keep_defaults() {
        let (c, problems) = Config::parse(
            "radius = 3\nspacing = 64 # comment\nhud = off\nmystery = 1\nnonsense\nteleport = maybe\nsamples-per-tick = 2_048\n",
        );
        assert_eq!(c.radius, 6400);
        assert_eq!(c.spacing, 64);
        assert_eq!(c.hud, None);
        assert_eq!(c.teleport, Teleport::Operators);
        assert_eq!(c.samples_per_tick, 2048);
        assert_eq!(problems.len(), 4, "{problems:?}");
        assert!(problems[0].starts_with("line 1: bad value for `radius`"));
        assert!(problems[1].contains("unknown setting `mystery`"));
    }

    #[test]
    fn settings_page() {
        let c = Config::default();
        let labels = c.labels();
        assert_eq!(labels.len(), 10);
        assert_eq!(labels[0], "Search radius: 6400 blocks");
        assert_eq!(labels[4], "Compass HUD: top right");
        assert_eq!(labels[6], "Action bar while holding the Biome Compass: ON");
        assert_eq!(labels[9], "Read config.txt again");
        let saved = |b| match click(&c, b) {
            Some(Click::Save(c)) => c,
            other => panic!("{other:?}"),
        };
        assert_eq!(saved(0).radius, 10_000);
        assert_eq!(saved(1).spacing, 48);
        assert_eq!(saved(2).samples_per_tick, 2048);
        assert_eq!(saved(3).heights, Heights::Player);
        assert_eq!(saved(4).hud, Some(Corner::BottomLeft));
        assert!(saved(5).hud_needs_compass);
        assert!(!saved(6).action_bar);
        assert_eq!(saved(7).teleport, Teleport::Off);
        assert_eq!(click(&c, 9), Some(Click::Reload));
        assert_eq!(click(&c, 10), None);
        // Steps wrap, and a hand-set value goes to the next step up.
        let mut odd = c.clone();
        odd.radius = 50_000;
        odd.spacing = 20;
        let Some(Click::Save(next)) = click(&odd, 0) else {
            panic!()
        };
        assert_eq!(next.radius, 500);
        let Some(Click::Save(next)) = click(&odd, 1) else {
            panic!()
        };
        assert_eq!(next.spacing, 24);
        let Some(Click::Save(reset)) = click(&odd, 8) else {
            panic!()
        };
        assert_eq!(reset, Config::default());
        // The HUD corner cycles through off.
        let mut seen = BTreeMap::new();
        let mut h = c.clone();
        for i in 0..5 {
            let Some(Click::Save(n)) = click(&h, 4) else {
                panic!()
            };
            seen.insert(i, n.hud);
            h = n;
        }
        assert_eq!(h.hud, c.hud);
        assert!(seen.values().any(Option::is_none));
    }

    fn sapling(id: &str) -> bool {
        [
            "minecraft:oak_sapling",
            "minecraft:cherry_sapling",
            "minecraft:mangrove_propagule",
        ]
        .contains(&id)
    }

    #[test]
    fn the_recipe() {
        let s = Some("minecraft:oak_sapling");
        let c = Some(ITEM);
        let grid = [
            None,
            s,
            None,
            Some("minecraft:cherry_sapling"),
            c,
            s,
            None,
            Some("minecraft:mangrove_propagule"),
            None,
        ];
        assert!(recipe_matches(&grid, 3, &sapling));
        // A corner filled, a side missing, no compass, a log for a sapling.
        let mut corner = grid;
        corner[0] = s;
        assert!(!recipe_matches(&corner, 3, &sapling));
        let mut side = grid;
        side[7] = None;
        assert!(!recipe_matches(&side, 3, &sapling));
        let mut no_compass = grid;
        no_compass[4] = Some("minecraft:recovery_compass");
        assert!(!recipe_matches(&no_compass, 3, &sapling));
        let mut log = grid;
        log[1] = Some("minecraft:oak_log");
        assert!(!recipe_matches(&log, 3, &sapling));
        // The 2x2 grid can't hold it; a wrong-sized grid is refused.
        assert!(!recipe_matches(&grid[..4], 2, &sapling));
        assert!(!recipe_matches(&grid[..8], 3, &sapling));
        // Without a sapling tag, the ids tell.
        assert!(looks_like_sapling("minecraft:pale_oak_sapling"));
        assert!(looks_like_sapling("minecraft:flowering_azalea"));
        assert!(looks_like_sapling("minecraft:mangrove_propagule"));
        assert!(!looks_like_sapling("minecraft:oak_log"));
    }

    fn found(pos: Position) -> Track {
        Track {
            label: "Cherry Grove".into(),
            dimension: "minecraft:overworld".into(),
            phase: Phase::Found {
                name: "Cherry Grove".into(),
                pos,
            },
        }
    }

    #[test]
    fn the_tracker_component() {
        assert_eq!(pointing(None), Pointing::Idle);
        assert_eq!(tracker(&Pointing::Idle), None);
        let searching = Track {
            label: "Desert".into(),
            dimension: "minecraft:overworld".into(),
            phase: Phase::Searching {
                progress: 0.5,
                radius: 6400,
            },
        };
        // Searching: no target, not tracked: the needle spins.
        assert_eq!(
            tracker(&pointing(Some(&searching))),
            Some(Nbt::compound([("tracked", Nbt::Byte(0))]))
        );
        let t = tracker(&pointing(Some(&found([300, 90, -400])))).unwrap();
        let target = t.get("target").unwrap();
        assert_eq!(
            target.get("dimension").and_then(Nbt::as_str),
            Some("minecraft:overworld")
        );
        assert_eq!(target.get("pos"), Some(&Nbt::IntArray(vec![300, 90, -400])));
        assert_eq!(t.get("tracked"), Some(&Nbt::Byte(0)));
        // Nothing found: a plain compass again.
        let mut missing = searching.clone();
        missing.phase = Phase::Missing { radius: 6400 };
        assert_eq!(tracker(&pointing(Some(&missing))), None);
    }

    #[test]
    fn lore_and_mark() {
        let Nbt::List(idle) = lore(&Pointing::Idle) else {
            panic!()
        };
        assert_eq!(idle.len(), 1);
        assert_eq!(
            idle[0].get("text").and_then(Nbt::as_str),
            Some("Right-click to choose a biome")
        );
        assert_eq!(idle[0].get("italic"), Some(&Nbt::Byte(0)));
        let Nbt::List(lines) = lore(&pointing(Some(&found([0, 64, 0])))) else {
            panic!()
        };
        assert_eq!(
            lines[1].get("text").and_then(Nbt::as_str),
            Some("Points to: Cherry Grove")
        );
        assert_eq!(
            lines[2].get("text").and_then(Nbt::as_str),
            Some("in the Overworld")
        );
        let marked = |c: &str| (c == CUSTOM_DATA).then(custom_data);
        let named = |c: &str| (c == ITEM_NAME_COMPONENT).then(item_name);
        let plain = |_: &str| None;
        assert!(is_biome_compass(ITEM, &marked));
        assert!(is_biome_compass(ITEM, &named));
        assert!(!is_biome_compass(ITEM, &plain));
        assert!(!is_biome_compass("minecraft:recovery_compass", &marked));
        let other_data =
            |c: &str| (c == CUSTOM_DATA).then(|| Nbt::compound([("someone", Nbt::Byte(1))]));
        assert!(!is_biome_compass(ITEM, &other_data));
    }

    /// The buttons of a picker dialog: (label text, command).
    fn buttons(dialog: &Nbt) -> Vec<(String, String)> {
        let Some(Nbt::List(actions)) = dialog.get("actions") else {
            panic!("no actions")
        };
        actions
            .iter()
            .map(|b| {
                let label = b.get("label").unwrap();
                let text = label
                    .as_str()
                    .or_else(|| label.get("text").and_then(Nbt::as_str))
                    .unwrap_or_default()
                    .to_owned();
                let action = b.get("action").unwrap();
                let command = action
                    .get("command")
                    .or_else(|| action.get("template"))
                    .and_then(Nbt::as_str)
                    .unwrap_or_default()
                    .to_owned();
                (text, command)
            })
            .collect()
    }

    #[test]
    fn the_picker_filters() {
        let e = entries();
        let picker = |filter| Picker {
            entries: &e,
            filter,
            dimension: "minecraft:overworld",
            status: None,
            current: None,
        };
        let all = picker_dialog(&picker(None));
        assert_eq!(
            all.get("type").and_then(Nbt::as_str),
            Some("minecraft:multi_action")
        );
        assert_eq!(all.get("pause"), Some(&Nbt::Byte(0)));
        let b = buttons(&all);
        assert_eq!(b.len(), 3 + 7);
        assert_eq!(
            b[0],
            ("Search".into(), "biomecompass pick \"$(filter)\"".into())
        );
        assert_eq!(b[1].1, "biomecompass pick");
        assert_eq!(b[2], ("Stop".into(), "biomecompass stop".into()));
        // By name, each a search.
        assert_eq!(
            b[3],
            (
                "Birch Forest".into(),
                "biomecompass find minecraft:birch_forest".into()
            )
        );
        let oceans = buttons(&picker_dialog(&picker(Some("ocean"))));
        let names: Vec<&str> = oceans[3..].iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Deep Ocean", "Ocean", "Warm Ocean"]);
        let none = picker_dialog(&picker(Some("lava lake")));
        assert_eq!(buttons(&none).len(), 3);
        let Some(Nbt::List(body)) = none.get("body") else {
            panic!()
        };
        assert_eq!(
            body[0].get("contents").and_then(Nbt::as_str),
            Some("No biome of the Overworld matches \"lava lake\".")
        );
        // The text box keeps the filter.
        let filtered = picker_dialog(&picker(Some("ocean")));
        let Some(Nbt::List(inputs)) = filtered.get("inputs") else {
            panic!()
        };
        assert_eq!(inputs[0].get("key").and_then(Nbt::as_str), Some(FILTER_KEY));
        assert_eq!(
            inputs[0].get("initial").and_then(Nbt::as_str),
            Some("ocean")
        );
        // A filter longer than the box is cut to fit it.
        let long = "o".repeat(40);
        let cut = picker_dialog(&picker(Some(&long)));
        let Some(Nbt::List(inputs)) = cut.get("inputs") else {
            panic!()
        };
        assert_eq!(
            inputs[0].get("initial").and_then(Nbt::as_str).map(str::len),
            Some(FILTER_MAX)
        );
        // The biome searched for stands out; the status shows.
        let mut p = picker(None);
        p.current = Some("Ocean");
        p.status = Some("Searching for Ocean: 10% of 6400 blocks.".into());
        let d = picker_dialog(&p);
        let Some(Nbt::List(actions)) = d.get("actions") else {
            panic!()
        };
        let ocean = actions
            .iter()
            .find(|a| a.get("tooltip").and_then(Nbt::as_str) == Some("minecraft:ocean"))
            .unwrap();
        assert_eq!(
            ocean
                .get("label")
                .and_then(|l| l.get("color"))
                .and_then(Nbt::as_str),
            Some("yellow")
        );
        let Some(Nbt::List(body)) = d.get("body") else {
            panic!()
        };
        assert_eq!(body.len(), 2);
    }

    #[test]
    fn picker_commands() {
        assert_eq!(
            parse_command("biomecompass pick"),
            Some(Ok(Command::Pick(None)))
        );
        assert_eq!(
            parse_command("biomecompass pick \"\""),
            Some(Ok(Command::Pick(None)))
        );
        assert_eq!(
            parse_command("biomecompass pick \"cherry grove\""),
            Some(Ok(Command::Pick(Some("cherry grove".into()))))
        );
        assert_eq!(
            parse_command("bcompass menu ocean"),
            Some(Ok(Command::Pick(Some("ocean".into()))))
        );
        assert_eq!(unquote(r#""a\"b\\c""#), Some("a\"b\\c".into()));
        assert_eq!(unquote("  "), None);
        assert_eq!(parse_command("biomecompass give"), Some(Ok(Command::Give)));
        assert!(matches!(
            parse_command("biomecompass give 64"),
            Some(Err(_))
        ));
    }

    #[test]
    fn the_action_bar() {
        let t = found([99, 70, 0]);
        assert_eq!(
            action_bar(&t, &viewer(0.5, 0.5, 0.0)),
            ("Cherry Grove: 99 blocks E".into(), YELLOW)
        );
        assert_eq!(
            action_bar(&t, &viewer(99.5, 0.5, 0.0)),
            ("Cherry Grove: You are there".into(), GREEN)
        );
        let mut s = t.clone();
        s.phase = Phase::Searching {
            progress: 0.25,
            radius: 6400,
        };
        assert_eq!(
            action_bar(&s, &viewer(0.0, 0.0, 0.0)).0,
            "Searching: Cherry Grove, 25% of 6400 blocks"
        );
        s.phase = Phase::Missing { radius: 6400 };
        assert_eq!(
            action_bar(&s, &viewer(0.0, 0.0, 0.0)),
            ("No Cherry Grove within 6400 blocks".into(), RED)
        );
        assert_eq!(
            status_line(Some(&t), [0.5, 0.5], "minecraft:overworld"),
            Some("Cherry Grove at 99 70 0, 99 blocks E.".into())
        );
        assert_eq!(status_line(None, [0.5, 0.5], "minecraft:overworld"), None);
    }

    #[test]
    fn on_and_off() {
        assert!(enabled());
        set_enabled(false);
        assert!(!enabled());
        set_enabled(true);
        assert!(enabled());
        // Per thread: a window that is a client, beside a server thread.
        set_client_thread(true);
        assert!(client_thread());
        assert!(!std::thread::spawn(client_thread).join().unwrap());
        set_client_thread(false);
        assert!(!client_thread());
    }
}
