// menu::Menu::recompute HEAD (pinned): a crafting table run by the host's menu
// engine (a LAN or server guest's, single player's own window included)
// holding the Biome Compass's recipe makes a Biome Compass; any other grid goes
// on to the data pack's recipes. Taking it uses one of each item, as a shaped
// recipe's result does (none of them leaves a remainder).
fn biomecompass_menu_craft(menu: &mut Menu, env: &Env<'_>) -> core::ops::ControlFlow<()> {
    if menu.grid_size != 3 {
        return core::ops::ControlFlow::Continue(());
    }
    match crate::biomecompass_item::craft(&menu.containers[GRID], menu.grid_size, env.items) {
        Some(stack) => {
            menu.containers[RESULT][0] = Some(stack);
            core::ops::ControlFlow::Break(())
        }
        None => core::ops::ControlFlow::Continue(()),
    }
}
