// native_hooks::Game::native_command (a hook point): run `/biomecompass`
// (`/bcompass`), leave every other line to the game and to other mods.
fn biomecompass_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    if !*ran && game.biomecompass_command(command) {
        *ran = true;
    }
}
