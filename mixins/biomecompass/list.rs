// native_hooks::Game::native_mods (a hook point): BiomeCompass's row in
// Options > Mods.
fn biomecompass_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use horizon_client::biomecompass as bc;
    mods.push(crate::game::native_hooks::NativeMod {
        id: bc::ID.into(),
        title: format!("BiomeCompass {}", bc::VERSION),
        description: bc::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
