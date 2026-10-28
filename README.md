# Biome Compass

A **native** [Horizon](https://github.com/horizon-rust/horizon) mod (a horizon-mixins source bundle, `.hnmod`): ask
for a biome, and Biome Compass searches the world around you for the nearest one, a little every tick, then a
needle on your HUD points the way and counts down the distance. Operators can teleport there.

Inspired by Nature's Compass (https://modrinth.com/mod/natures-compass); not affiliated, no code from it. Biome
Compass is written from scratch for Horizon, from that mod's public description only.

> **Native mods are not sandboxed.** A native mod is Rust source compiled into your own Horizon game, and runs with
> your rights. Read the source (`mixins/biomecompass/`) before you build it.

## Commands

| Command | Does |
| --- | --- |
| `/biomecompass list [filter]` | This dimension's biomes, by name. Each name is a link: click it to search for that biome. `list ocean` shows only the oceans. |
| `/biomecompass find <biome>` | Starts a search around you. The biome is typed as an id (`cherry_grove`, `minecraft:cherry_grove`, `mymod:glade`), as its name (`cherry grove`), as part of either if only one biome has it (`cherry`), or as a biome tag (`#is_forest`, `#minecraft:is_ocean`, `#c:is_hot`): the nearest biome of the tag. A new search replaces your last one. |
| `/biomecompass status` (or just `/biomecompass`) | How far the search got, or where the biome is and how far from where you stand now. |
| `/biomecompass stop` | Stops your search and puts the needle away. |
| `/biomecompass tp` | Teleports you to the biome found (operators, see below). The found coordinates in chat are a link that puts this command in the chat box. |
| `/biomecompass reload` | Reads the settings file again (operators). |
| `/biomecompass help` | The commands. |

`/bcompass` is a short form of `/biomecompass`. Anyone may search, in single player, on LAN and on a server running
the native build, cheats on or off. The commands are not in the chat box's suggestions, so the chat box may colour
them red; the game still runs them.

A biome that can't generate in your dimension is refused at once (`Cherry Grove doesn't generate in the Nether`),
and so is a tag none of whose biomes do.

## The search

- **Spread over ticks.** A search samples `samples-per-tick` biomes a tick (1024 by default), shared by every
  player's search, and stops a tick's share after 10 ms whatever that setting says, so a long search never freezes
  the game. Every 10 seconds a long search tells you how far it got; the HUD shows its progress all along.
- **From the world generator, not the loaded world.** Biomes are asked of the world generator of your dimension
  (Horizon's `noise_biomes`, the game's `BiomeSource.getNoiseBiome`, which `/locate biome` and the biome preview of
  explorer maps also use), so no chunk is generated or loaded by a search, however far it reaches. Like
  `/locate biome`, it sees the biomes the world generates, not what `/fillbiome` changed afterwards.
- **Outward in rings.** Columns `spacing` blocks apart (32 by default) are sampled ring by ring outward from where
  you stood, within a circle of `radius` blocks (6400 by default). Once a ring holds the biome, the search finishes
  the rings that could still hold a nearer sample, so the answer is the nearest sample by straight-line distance,
  not just the first ring's.
- **Heights.** In the Overworld each column is sampled at your height and every 64 blocks above and below it,
  nearest first, so cave biomes (lush caves, dripstone caves, the deep dark) are found too; the setting "yours
  only" samples your height alone (four to six times fewer samples). The Nether's and the End's biomes don't change
  with height: one sample a column there.
- **What it reports**: the column and height of the sample (`[x, y, z]`), and its distance and direction from
  where you started. Biomes narrower than the spacing can be missed; a smaller spacing finds them more surely and
  takes longer (halving it takes four times as long).
- A search belongs to the dimension it started in: change dimension and it carries on; the HUD then says where the
  biome is (`in the Overworld`).
- Searches and found biomes are kept in memory while the world is open, not saved.

## The needle

The HUD box (top right by default) shows:

- while searching: a turning needle, `Searching: Cherry Grove` and how much of the radius is done;
- once found: a red needle pointing at the biome from where you stand and look, its name, and `812 blocks NE` (with
  the height, `y -20`, when it is more than 16 blocks above or below you); `You are there` when you stand on it;
- in another dimension than the biome: no needle, and which dimension it is in;
- when nothing was found: `No Cherry Grove within 6400 blocks`, for 10 seconds.

The box gives way to F3's debug screen in the top left, and hides under F1. With **Only while holding a compass**
on, it shows only while a compass or a recovery compass is in your main hand or off hand.

The needle shows for the player at the computer, in single player (the window and its server thread are one
process) and in-frame single player. LAN guests and players on a server get the chat lines (start, progress,
result, `/biomecompass status`) but no needle: their search runs on the host's computer.

## Teleport

`/biomecompass tp` runs the game's own `/tp` to the biome found, so it needs what `/tp` needs: **permission level
2** (operators; in single player, cheats on). It lands you on the highest block of that column when its chunk is
loaded, else on the terrain the generator gives there (on the water of an ocean), before trees and buildings: you
may land beside a tree or a structure. It is refused in the Nether (its roof is the only height the generator
answers; walk there with the needle), on an End column with no island, and when the settings turn it off.

## Settings

**Options > Mods > Biome Compass > Settings**. Each button steps through its values; a change is in effect for the
next search (the HUD's at once).

| Button | Setting (file) | Default | Values a click steps through |
| --- | --- | --- | --- |
| Search radius | `radius` | 6400 blocks | 500, 1000, 2000, 4000, 6400, 10000, 16000, 25000, 50000 (file: 64 to 100000) |
| Sample spacing | `spacing` | 32 blocks | 8, 16, 24, 32, 48, 64, 96, 128 (file: 4 to 512) |
| Samples per tick | `samples-per-tick` | 1024 | 128 to 16384, doubling (file: 16 to 65536) |
| Heights searched | `heights` | every 64 blocks (caves too) | every 64 blocks (`all`), yours only (`player`) |
| Compass HUD | `hud` | top right | top left, top right, bottom left, bottom right, off |
| Only while holding a compass | `hud-needs-compass` | OFF | ON, OFF |
| Teleport (/biomecompass tp) | `teleport` | operators | operators, off |
| Reset to defaults | | | every setting back to its default |
| Read config.txt again | | | the file as edited by hand |

A value set by hand in the file between two steps goes to the next step up. The page writes, and the mod reads,
`horizon-mods/.horizon-mod-data/biomecompass/config.txt` (beside the game's `options.txt`): `key = value` lines, `#`
comments, written with every setting explained the first time Biome Compass runs. A line it can't read is skipped
(logged, and listed by `/biomecompass reload`) and that setting keeps its default. A dedicated server has no settings
page: edit the file in the server folder and run `/biomecompass reload`.

**Options > Mods** also turns Biome Compass off: from then on it does nothing (its commands are unknown, no needle)
until the launcher's next build leaves it out. Single player's server thread follows the window's switch.

## Not in this version

- **No compass item or biome selection screen.** Nature's Compass is an item with its own screens; a native mod
  can't add an item to Horizon's registries without far more pinned code, so Biome Compass is a command with a
  clickable list, and the needle is drawn on the HUD (optionally tied to holding the vanilla compass).
- **No needle toward the world spawn** when nothing is searched: the vanilla compass already does that.
- **No biome blacklist**: every biome of the dimension can be searched.
- **No needle for LAN guests and server players** (see above); they get the chat lines.
- **No teleport in the Nether** (see above).

## Compatibility

Horizon's native mods can use **hook points** (`game/native_hooks.rs`, `NATIVE_HOOKS_API` 1): functions kept stable
from release to release. Biome Compass uses them for everything they cover:

- `native_command`: `/biomecompass`;
- `native_tick`: the searches, a slice a tick, and the window passing Options > Mods' on/off to single player's
  server thread;
- `native_hud`: the needle box, drawn with `native_fill` and `native_text` plus `HudBatch::fill_quad` for the needle's
  two triangles, placed with `Corners::place`, skipping a corner `native_corner_shown` says is taken;
- `native_mods`, `native_settings`, `native_settings_click`: the row in Options > Mods and the settings page, with
  `native_setting`/`native_set_setting` for the file and `native_on`/`native_window` for the on/off switch.

One hook has no hook point, so it pins a game function (a cancellable HEAD injection):

| Pinned hook | Why there is no hook point for it |
| --- | --- |
| `lan_actions::Game::guest_command` | Lets a LAN or server guest run `/biomecompass` where guests may not run commands (a LAN world without Allow Commands hosted in-frame, a dedicated server's non-operators): searching is a survival tool, like the compass it stands in for. `native_command` only sees commands that are allowed to run. TreeFall pins the same function for `/treefall`; each takes only its own command. |

The adapter (`native.rs`) also calls game internals that hook points don't offer: the world generator of the
level in place (`end.locator()`, `VanillaTerrain::noise_biomes`, `ChunkGenerator::lod_surface` for the landing
height), the data pack's biome tags (`tag_entries`), the game's `/tp` (`run_command`) and the LAN hub for lines to
guests. So Biome Compass is **pinned**: Horizon `142388e1c136e622652ec7505af15d8ad195e041`, horizon-mixins
`0d6b103c70633d8fd9443d8d0acd3aadaacedb06`, game 26.3, the same pins as TreeFall 0.3.0 and Mason 0.2.0, so the three
build into one game (checked: `horizon-mixins plan` and `apply` of all three together, 23 injections, 14 modules).

## Build and try

Biome Compass is a source bundle; the launcher compiles it into a game of its own:

```sh
horizon-launcher native-build C:\Users\degrando\projects\biomecompass --version v0.2.97 --trust-native-code
```

`--version` is the installed version whose game files the build reuses; the build compiles the pinned commit
whatever it is. With TreeFall and Mason, put the three bundles in one folder, one subfolder each, and build that
folder:

```sh
mkdir C:\Users\degrando\projects\native-set\biomecompass
tar -xf C:\Users\degrando\projects\biomecompass\dist\biomecompass-0.1.0.hnmod -C C:\Users\degrando\projects\native-set\biomecompass
horizon-launcher native-build C:\Users\degrando\projects\native-set --version v0.2.97 --trust-native-code
```

Then open a world and type `/biomecompass list`.

## Development

`mixins/biomecompass/`:

- `horizon-mixin.toml`: two support modules and seven HEAD hooks (six hook points, one pinned).
- `runtime.rs`: everything that decides (the ring search, its nearest-sample rule and budget, heights, biome name
  matching, commands, the settings file and page, what the HUD shows and the needle's shape, the chat lines) as pure
  code with unit tests. It is both this repository's Cargo library and the injected `crate::game::biomecompass`.
- `native.rs`: the adapter to the game (`crate::game::biomecompass_native`), and the state the window and single
  player's server thread share.
- `command.rs`, `tick.rs`, `hud.rs`, `list.rs`, `settings.rs`, `click.rs`, `guest-command.rs`: the hook helpers.

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
rustfmt --edition 2024 --check mixins/biomecompass/*.rs   # the game-side files too
python tools/pack_hnmod.py                                 # dist/biomecompass-0.1.0.hnmod
```

Written with [Claude Code](https://claude.com/claude-code).

## License

[MIT](LICENSE)
