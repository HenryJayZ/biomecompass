# BiomeCompass

A **native** [Horizon](https://github.com/horizon-rust/horizon) mod (a horizon-mixins source bundle, `.hnmod`): craft
a **Biome Compass**, right-click it, pick a biome, and BiomeCompass searches the world around you for the nearest
one, a little every tick; then the compass's needle points the way, and the distance shows over your hotbar and in
a box on the HUD. Operators can teleport there. The `/biomecompass` commands do it all too.

Inspired by Nature's Compass (https://modrinth.com/mod/natures-compass); not affiliated, no code from it. Biome
Compass is written from scratch for Horizon, from that mod's public description only.

> **Native mods are not sandboxed.** A native mod is Rust source compiled into your own Horizon game, and runs with
> your rights. Read the source (`mixins/biomecompass/`) before you build it.

## What's new in 0.2.3

- **Horizon Nightly 6475f62 (2026-11-04).** Horizon moved the game's library behind the thin
  `pub use horizon_client_lib::*` of `horizon-client`'s lib.rs into `horizon_client_lib` (its root:
  `client_lib.rs`). The two support modules moved there; the search module is now
  `horizon_client_lib::biomecompass_support` (was `horizon_client::biomecompass`) and the item module
  `horizon_client_lib::biomecompass_item` (was `horizon_client::biomecompass_item`). The two hooks that
  name the library's `menu` and `lan` now name `horizon_client_lib::menu::Menu::recompute` and
  `horizon_client_lib::lan::hub::Hub::set_mod_commands`. No game behaviour changed; the pins did.

## What's new in 0.2.2

- **The crafting table shows the Biome Compass.** In single player as it starts by default (the window joined to
  its own server thread) and on a LAN world you host, the result slot now shows the Biome Compass the recipe makes;
  it used to look empty, though taking from it worked. A client of another computer's server still shows nothing
  there until that server says so (it may not run BiomeCompass).
- **F3 and LookAt.** With F3's debug screen open, a top-left or top-right HUD box now hides instead of moving to the
  top centre, where it overlapped LookAt's box (LookAt goes there with F3 open). The Biome Compass's needle and the
  action bar over the hotbar still show the search; a bottom box stays where it is.

## What was new in 0.2.1

- With F3 open, a top-left or top-right HUD box moved to the top centre, under the boss bars (replaced in 0.2.2:
  it hides).

## The Biome Compass

### Crafting

On a crafting table: a **compass in the middle** and a **sapling on each of its four sides** (any saplings, mixed:
the data pack's `#minecraft:saplings`, mangrove propagules and azaleas included); the corners stay empty.

```
 .  S  .
 S  C  S        C: compass   S: any sapling   .: empty
 .  S  .
```

It makes one Biome Compass. Operators (and creative players) can also type `/biomecompass give`.

A Biome Compass is a vanilla compass with BiomeCompass's mark (`minecraft:custom_data` `{biomecompass: 1b}`), the
name "Biome Compass" and a line of lore; nothing is added to the game's registries, so vanilla clients, a vanilla
server and saves all read it. Without a search it is an ordinary compass (it points at the world spawn). It crafts
into what a compass crafts into (a recovery compass), and the recipe takes any compass in the middle, a Biome
Compass included (the new one points nowhere yet).

### Using it

**Right-click** with the Biome Compass in either hand (after the block you look at has had its turn: a chest still
opens). The **biome picker** opens:

- a text box and **Search**: the biomes whose name or id has that text (`ocean`, `cherry`);
- **All biomes**: the whole list again; **Stop**: stops your search, and the compass points at the world spawn again;
- a button per biome of the dimension you are in, by name (its id on hover): click one to start the search. The
  biome you are searching for or found is in yellow, and the picker says how far your search got or where the biome
  is.

The picker is a vanilla dialog screen (the server sends it, as servers send their own screens since 1.21.6), so it
works on any client, vanilla's too. Its buttons run `/biomecompass` commands as you, and the command tree the server
sends lists those commands, so a vanilla client runs them without a confirmation screen. The search box filters when
you press Search (a dialog has no live filtering).

### The needle

- **While searching** the needle spins (a lodestone tracker with no target, `tracked: false`).
- **Once found** it points at the biome (vanilla's `minecraft:lodestone_tracker`: the biome's dimension and block,
  `tracked: false`, so no lodestone is needed there and the game never takes the target away). In another dimension
  it spins, as a lodestone compass does.
- **Stopped, or nothing found**: the tracker goes, and it is an ordinary compass again.

The server sets every Biome Compass in your inventory (hotbar, main inventory, off hand) to your search every 5
ticks, so any client draws the needle, and the item's lore says what it points to (`Points to: Cherry Grove`).
While you hold one, the **action bar** (over the hotbar) shows `Cherry Grove: 812 blocks NE` (or the search's
progress), for every player, on a server too.

## Commands

| Command | Does |
| --- | --- |
| `/biomecompass pick [filter]` | The biome picker, as the Biome Compass opens it, its list filtered. |
| `/biomecompass list [filter]` | This dimension's biomes, by name. Each name is a link: click it to search for that biome. `list ocean` shows only the oceans. |
| `/biomecompass find <biome>` | Starts a search around you. The biome is typed as an id (`cherry_grove`, `minecraft:cherry_grove`, `mymod:glade`), as its name (`cherry grove`), as part of either if only one biome has it (`cherry`), or as a biome tag (`#is_forest`, `#minecraft:is_ocean`, `#c:is_hot`): the nearest biome of the tag. A new search replaces your last one. |
| `/biomecompass status` (or just `/biomecompass`) | How far the search got, or where the biome is and how far from where you stand now. |
| `/biomecompass stop` | Stops your search and puts the needle away. |
| `/biomecompass give` | A Biome Compass (operators, or in creative mode). |
| `/biomecompass tp` | Teleports you to the biome found (operators, see below). The found coordinates in chat are a link that puts this command in the chat box. |
| `/biomecompass reload` | Reads the settings file again (operators). |
| `/biomecompass help` | The commands. |

`/bcompass` is a short form of `/biomecompass`. Anyone may search, in single player, on LAN and on a server running
the native build, cheats on or off. The server lists the commands in the command tree it sends its players, so the
chat box completes them (in-frame single player, where no server sends one, may colour them red; the game still
runs them).

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

## The HUD box

The HUD box (top right by default) shows:

- while searching: a turning needle, `Searching: Cherry Grove` and how much of the radius is done;
- once found: a red needle pointing at the biome from where you stand and look, its name, and `812 blocks NE` (with
  the height, `y -20`, when it is more than 16 blocks above or below you); `You are there` when you stand on it;
- in another dimension than the biome: no needle, and which dimension it is in;
- when nothing was found: `No Cherry Grove within 6400 blocks`, for 10 seconds.

With F3's debug screen open, a top-left or top-right box hides: F3 has a column in each top corner, and the top
centre between them is where LookAt's box goes with F3 open (a native mod can't tell whether another one is on and
where its box is, so the two would overlap there). The Biome Compass's needle and the action bar over the hotbar
still show the search, and the box is back the moment F3 closes. A bottom-left or bottom-right box stays where it is
with F3 open: choose one to keep the box on screen while debugging. The box hides under F1. With **Only while holding a compass**
on, it shows only while a compass (a Biome Compass is one) or a recovery compass is in your main hand or off hand.
**Compass HUD: off** hides it; the Biome Compass then does the pointing alone.

The box shows for the player at the computer, in single player (the window and its server thread are one process)
and in-frame single player. LAN guests and players on a server have no box (their search runs on the host's
computer), but their Biome Compass points, their action bar counts down, and they get the chat lines (start,
progress, result, `/biomecompass status`).

## Teleport

`/biomecompass tp` runs the game's own `/tp` to the biome found, so it needs what `/tp` needs: **permission level
2** (operators; in single player, cheats on). It lands you on the highest block of that column when its chunk is
loaded, else on the terrain the generator gives there (on the water of an ocean), before trees and buildings: you
may land beside a tree or a structure. It is refused in the Nether (its roof is the only height the generator
answers; walk there with the needle), on an End column with no island, and when the settings turn it off.

## Settings

**Options > Mods > BiomeCompass > Settings**. Each button steps through its values; a change is in effect for the
next search (the HUD's at once).

| Button | Setting (file) | Default | Values a click steps through |
| --- | --- | --- | --- |
| Search radius | `radius` | 6400 blocks | 500, 1000, 2000, 4000, 6400, 10000, 16000, 25000, 50000 (file: 64 to 100000) |
| Sample spacing | `spacing` | 32 blocks | 8, 16, 24, 32, 48, 64, 96, 128 (file: 4 to 512) |
| Samples per tick | `samples-per-tick` | 1024 | 128 to 16384, doubling (file: 16 to 65536) |
| Heights searched | `heights` | every 64 blocks (caves too) | every 64 blocks (`all`), yours only (`player`) |
| Compass HUD | `hud` | top right | top left, top right, bottom left, bottom right, off |
| Only while holding a compass | `hud-needs-compass` | OFF | ON, OFF |
| Action bar while holding the Biome Compass | `action-bar` | ON | ON, OFF |
| Teleport (/biomecompass tp) | `teleport` | operators | operators, off |
| Reset to defaults | | | every setting back to its default |
| Read config.txt again | | | the file as edited by hand |

A value set by hand in the file between two steps goes to the next step up. The page writes, and the mod reads,
`horizon-mods/.horizon-mod-data/biomecompass/config.txt` (beside the game's `options.txt`): `key = value` lines, `#`
comments, written with every setting explained the first time BiomeCompass runs. A line it can't read is skipped
(logged, and listed by `/biomecompass reload`) and that setting keeps its default. A dedicated server has no settings
page: edit the file in the server folder and run `/biomecompass reload`.

**Options > Mods** also turns BiomeCompass off: from then on it does nothing (its commands are unknown, no needle,
the recipe makes nothing, a Biome Compass is an ordinary compass) until the launcher's next build leaves it out. Single player's server thread follows the window's switch.

## Not in this version

- **Not in the recipe book.** The recipe book's recipes are the data pack's, loaded once into read-only tables that
  no hook reaches at this commit; the recipe is matched where a crafting grid's result is looked up instead, so it
  crafts like any other, but the book doesn't show it (`/biomecompass help` gives it).
- **No crafter.** The crafter block makes what the data pack's recipes make; the Biome Compass is crafted by hand.
- **The lore is not sent to LAN guests and server players**: Horizon's server sends its players a stack's name and
  lodestone tracker but not its lore. They see "Biome Compass", its needle and the action bar.
- **In-frame single player has no picker screen**: its dialogs' buttons reach no server there, so a right-click
  lists the biomes in chat (each a link that starts the search, then a link that stops it). Single player as it
  starts by default (with its own server thread), LAN and servers have the screen.
- **No item of its own** (texture, model, registry entry): a native mod can't add one to Horizon's registries at
  this commit without far more pinned code, and a new item would make vanilla clients and servers refuse the world.
  The Biome Compass is a marked vanilla compass instead.
- **No biome blacklist**: every biome of the dimension can be searched.
- **No teleport in the Nether** (see above).

## Limits

- **No top box with F3 open.** LookAt's box goes to the top centre with F3 open, and isn't placed through `Corners`
  (its height changes with what you look at), so BiomeCompass has no clean way to know it is there and stack under
  it; a top-left or top-right box hides while F3 is open instead. Choose a bottom corner (Compass HUD: bottom left
  or bottom right) to see it while debugging.

## Compatibility

Horizon's native mods can use **hook points** (`game/native_hooks.rs`, `NATIVE_HOOKS_API` 1): functions kept stable
from release to release. BiomeCompass uses them for everything they cover:

- `native_command`: `/biomecompass`;
- `native_tick`: the searches, a slice a tick; the pickers; the Biome Compasses and the action bar every 5 ticks; and
  the window passing Options > Mods' on/off to single player's server thread;
- `native_hud`: the needle box, drawn with `native_fill` and `native_text` plus `HudBatch::fill_quad` for the needle's
  two triangles, placed with `Corners::place`, and `native_corner_shown` (whether F3 is open: it reports the top
  left taken then; at this commit it doesn't report F3's right column, so a top-right box checks the top left and
  hides too);
- `native_mods`, `native_settings`, `native_settings_click`: the row in Options > Mods and the settings page, with
  `native_setting`/`native_set_setting` for the file and `native_on`/`native_window` for the on/off switch.

Five hooks have no hook point, so they pin game functions (cancellable HEAD injections):

| Pinned hook | Why |
| --- | --- |
| `lan_actions::Game::guest_command` | Lets a LAN or server guest run `/biomecompass` (and the picker's buttons) where guests may not run commands (a LAN world without Allow Commands hosted in-frame, a dedicated server's non-operators): searching is a survival tool. `native_command` only sees commands that are allowed to run. TreeFall pins the same function for `/treefall`; each takes only its own command. |
| `potions::Game::use_potion_item` | The right-click. It is the first item use a compass reaches in `use_hand`, once the block aimed at has had its turn, for a guest's `UseItem` as for the window's player in-frame (`use_hand` itself holds a macro, which HEAD refuses). It takes only a Biome Compass. |
| `Game::craft_recipe` | The recipe on the window's own crafting grid, whose result slot the window draws from it: in-frame, and in a window joined to its own server (single player's server thread, a LAN world it hosts: `has_singleplayer_server`), where it shows what that server makes. A window joined to another computer's server leaves it to that server. |
| `menu::Menu::recompute` | The recipe on a crafting table the host's menu engine runs: LAN and server guests', single player's own window's; and the window's own engine predicting its clicks on its own server. A client of another computer's server leaves it to the server. |
| `lan::hub::Hub::set_mod_commands` | `/biomecompass` and `/bcompass` join the server mods' commands, which the command tree every guest receives lists after vanilla's (`send_commands`, which builds the packet, holds a macro). The game sets them when a hub opens (with the `mods` feature, which the launcher's native build passes). |

The adapter (`native.rs`) also calls game internals that hook points don't offer: the world generator of the
level in place (`end.locator()`, `VanillaTerrain::noise_biomes`, `ChunkGenerator::lod_surface` for the landing
height), the data pack's biome tags (`tag_entries`), the game's `/tp` (`run_command`), the LAN hub for lines,
dialogs and action bars to guests, the players' inventories, and whether the window's server is its own
(`has_singleplayer_server`, for the crafting grid and the HUD box). So BiomeCompass is **pinned**: Horizon
`6475f62f3c834b7489e1816bf47f47ea5eb2b879`, horizon-mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`, game 26.3
(0.2.3 verified with `horizon-mixins apply` on that commit: status ok; the pinned predecessors were checked the
same way; earlier builds checked `cargo check -p horizon-client --features mods` of the applied source as well).

## Build and try

BiomeCompass is a source bundle; the launcher compiles it into a game of its own:

```sh
horizon-launcher native-build C:\Users\degrando\projects\biomecompass --version v0.2.97 --trust-native-code
```

`--version` is the installed version whose game files the build reuses; the build compiles the pinned commit
whatever it is. With TreeFall and Mason, put the three bundles in one folder, one subfolder each, and build that
folder:

```sh
mkdir C:\Users\degrando\projects\native-set\biomecompass
tar -xf C:\Users\degrando\projects\biomecompass\dist\biomecompass-0.2.3.hnmod -C C:\Users\degrando\projects\native-set\biomecompass
horizon-launcher native-build C:\Users\degrando\projects\native-set --version v0.2.97 --trust-native-code
```

Then open a world, type `/biomecompass give` (cheats on) or craft a Biome Compass, and right-click it.

## Development

`mixins/biomecompass/`:

- `horizon-mixin.toml`: three support modules and eleven HEAD hooks (six hook points, five pinned).
- `runtime.rs`: everything that decides (the ring search, its nearest-sample rule and budget, heights, biome name
  matching, commands, the settings file and page, what the HUD shows and the needle's shape, the chat lines, the
  Biome Compass's components and lodestone tracker, the recipe, the picker dialog, the action bar) as pure code with
  unit tests. It is both this repository's Cargo library and the injected `horizon_client_lib::biomecompass_support`
  (in the game's library, public, so the library's hooks reach it).
- `item.rs`: `horizon_client_lib::biomecompass_item`, the Biome Compass on the game's item stacks and NBT, the recipe on
  its crafting grids, the commands in its command tree.
- `native.rs`: the adapter to the game (`crate::game::biomecompass_native`), and the state the window and single
  player's server thread share.
- `command.rs`, `tick.rs`, `hud.rs`, `list.rs`, `settings.rs`, `click.rs`, `guest-command.rs`, `use.rs`, `craft.rs`,
  `menu-craft.rs`, `commands.rs`: the hook helpers.

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
rustfmt --edition 2024 --check mixins/biomecompass/*.rs   # the game-side files too
python tools/pack_hnmod.py                                 # dist/biomecompass-0.2.3.hnmod
```

Written with [Claude Code](https://claude.com/claude-code).

## License

[MIT](LICENSE)
