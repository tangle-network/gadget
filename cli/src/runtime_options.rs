use blueprint_manager::config::SourceType;
use cargo_tangle::command::tangle::{PreferredSourceArg, SpawnMethod};
use cargo_tangle::settings::RuntimePreferences;
use color_eyre::eyre::{Result, eyre};

pub(super) fn resolve_runtime_options(
    spawn_method: SpawnMethod,
    stored: RuntimePreferences,
    preferred_override: Option<PreferredSourceArg>,
    vm_flag: bool,
    no_vm_flag: bool,
) -> Result<(SourceType, bool)> {
    let mut preferred_source = stored
        .preferred_source
        .unwrap_or(spawn_method.preferred_source());
    if let Some(arg) = preferred_override {
        preferred_source = SourceType::from(arg);
    }

    let mut use_vm = stored.use_vm.unwrap_or(spawn_method.use_vm());
    match (vm_flag, no_vm_flag) {
        (true, true) => {
            return Err(eyre!(
                "Use either --vm or --no-vm when overriding manager runtime"
            ));
        }
        (true, false) => use_vm = true,
        (false, true) => use_vm = false,
        _ => {}
    }

    Ok((preferred_source, use_vm))
}
