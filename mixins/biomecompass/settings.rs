// native_hooks::Game::native_settings (a hook point): BiomeCompass's
// settings page.
fn biomecompass_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == horizon_client::biomecompass::ID {
        buttons.extend(game.biomecompass_settings());
    }
}
