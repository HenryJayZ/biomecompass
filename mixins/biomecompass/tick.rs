// native_hooks::Game::native_tick (a hook point): a slice of every search in
// progress; on the window, Options > Mods' on/off for single player's server
// thread.
fn biomecompass_tick(game: &mut crate::game::Game<'_>) {
    game.biomecompass_tick();
}
