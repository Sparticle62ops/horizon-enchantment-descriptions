//! Exact 1:1 Rust translation of Enchantment Descriptions by Darkhax
//! (upstream Java, LGPL-2.1-only,
//! https://github.com/Darkhax-Minecraft/Enchantment-Descriptions). Carries
//! the upstream license and credit. Ported for the Horizon client by
//! sparticle.
//!
//! A short description line right under each enchantment on the hovered
//! item's tooltip. The whole flow rides the mod API's item_tooltip hook (the
//! finished tooltip with mutable lines, where the first port's HEAD mixin
//! stood in): four of the five config entries register as settings, the
//! config file stays config/enchdesc.json in the original's camelCase, the
//! descriptions ship as the original's en_us lang file and resolve through
//! the game's lang layer, and the enchantment name lines are matched by
//! their resolved text and the game's own name colors (upstream matches the
//! name component's key; see the README's seams).

mod config;
mod description;
#[cfg(test)]
mod tests;

use mod_api::{
    ClientHost, ClientSetupEvent, InputKeyEvent, ItemTooltipEvent, Registrar,
    RegistrationContext, SettingSpec,
};

pub use crate::config::Config;

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {
    /// java: ConfigSchema's entries as settings, same defaults; the labels
    /// are the original's field names (its config schema carries no lang
    /// labels). onlyDisplayInEnchantingTable stays file-held: the mod API
    /// has no current-screen query to enforce it (README's seams).
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let _ = registrar.register_setting(SettingSpec::toggle(
            "enable_mod",
            "Enable Mod",
            true,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "only_display_on_books",
            "Only Display On Books",
            false,
        ));
        let _ = registrar.register_setting(SettingSpec::toggle(
            "require_keybind_press",
            "Require Keybind Press",
            false,
        ));
        let _ = registrar.register_setting(SettingSpec::range(
            "indent_size",
            "Indent Size",
            0.0,
            16.0,
            1.0,
            0.0,
        ));
    }

    /// java: EnchDescCommon's constructor — the config file loads, the
    /// settings overlay it, and the file saves (upstream's Gson schema
    /// writes every run).
    fn client_setup(&self, host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        let path = config::config_path();
        let mut config = config::load(&path);
        if let Some(value) = host.setting_bool("enable_mod") {
            config.enable_mod = value;
        }
        if let Some(value) = host.setting_bool("only_display_on_books") {
            config.only_display_on_books = value;
        }
        if let Some(value) = host.setting_bool("require_keybind_press") {
            config.require_keybind_press = value;
        }
        if let Some(value) = host.setting_f64("indent_size") {
            config.indent_size = value as i32;
        }
        config::save(&path, &config);
        description::set_state(config);
    }

    /// java: Screen.hasShiftDown()'s live state, fed by the key events.
    fn input_key(&self, _host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        description::on_input_key(event);
    }

    /// java: EnchDescCommon.onItemTooltip on the mod API's item_tooltip hook.
    fn item_tooltip(&self, host: &dyn ClientHost, event: &mut ItemTooltipEvent<'_>) {
        description::on_item_tooltip(host, event);
    }
}
