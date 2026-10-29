// Game::craft_recipe HEAD (pinned): the window's own crafting grid, in the
// game that simulates the world (in-frame), holding the Biome Compass's recipe
// makes a Biome Compass; any other grid goes on to the data pack's recipes.
fn biomecompass_craft(
    game: &crate::game::Game<'_>,
) -> core::ops::ControlFlow<Option<(String, ItemStack)>> {
    match game.biomecompass_craft() {
        Some(made) => core::ops::ControlFlow::Break(Some(made)),
        None => core::ops::ControlFlow::Continue(()),
    }
}
