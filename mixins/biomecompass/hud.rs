// native_hooks::Game::native_hud (a hook point): the needle, the biome and
// its distance, in a corner of the HUD.
fn biomecompass_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    game.biomecompass_hud(batch, corners);
}
