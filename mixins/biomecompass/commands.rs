// lan::hub::Hub::set_mod_commands HEAD (pinned): the server mods' commands,
// which the command tree every LAN or server guest receives lists after
// vanilla's, get `/biomecompass` and `/bcompass` too, so a guest's chat box
// knows them and a vanilla client runs the picker's buttons without a
// confirmation screen. The game sets them when a hub opens and whenever the
// server mods change; the rest is the original's body, on the extended root.
fn biomecompass_mod_commands(
    hub: &mut Hub,
    key: u64,
    commands: &serde_json::Value,
) -> core::ops::ControlFlow<()> {
    let mut extended = commands.clone();
    crate::biomecompass_item::add_commands(&mut extended);
    let resend = hub.mod_commands.0 != extended;
    hub.mod_commands = (extended, Some(key));
    if resend {
        for i in 0..hub.guests.len() {
            if hub.guests[i].placed {
                hub.send_commands(i);
            }
        }
    }
    core::ops::ControlFlow::Break(())
}
