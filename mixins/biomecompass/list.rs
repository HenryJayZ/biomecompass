// native_hooks::Game::native_mods (a hook point): Biome Compass's row in
// Options > Mods.
fn biomecompass_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::biomecompass as bc;
    mods.push(crate::game::native_hooks::NativeMod {
        id: bc::ID.into(),
        title: format!("Biome Compass {}", bc::VERSION),
        description: bc::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
