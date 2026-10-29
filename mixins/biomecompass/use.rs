// potions::Game::use_potion_item HEAD (pinned): the first item use a compass
// reaches in `use_hand` (after the block aimed at had its turn, a guest's
// UseItem or the window's player in-frame). A Biome Compass opens the biome
// picker and the click ends there; any other item goes on as usual.
fn biomecompass_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
) -> core::ops::ControlFlow<horizon_client::Result<bool>> {
    if game.biomecompass_use(stack) {
        core::ops::ControlFlow::Break(Ok(true))
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
