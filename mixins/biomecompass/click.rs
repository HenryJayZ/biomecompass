// native_hooks::Game::native_settings_click (a hook point): a click on Biome
// Compass's settings page changes the setting, keeps it and puts it in effect.
fn biomecompass_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id == crate::game::biomecompass::ID {
        game.biomecompass_settings_click(button);
    }
}
