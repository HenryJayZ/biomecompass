// lan_actions::Game::guest_command HEAD (pinned): queue a LAN or server
// guest's `/biomecompass` so it runs as that guest, at their own permission
// level, even where guests may not run commands; anything else goes on as
// usual.
fn biomecompass_guest_command(
    game: &mut crate::game::Game<'_>,
    host: &mut crate::game::lan::LanHost,
    conn: u32,
    command: &str,
) -> core::ops::ControlFlow<()> {
    if game.biomecompass_guest_command(host, conn, command) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
