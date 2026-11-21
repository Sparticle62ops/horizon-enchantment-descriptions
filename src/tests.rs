//! Tests for the port's logic: the config file round trip, the lang key
//! mapping, the name-line match, the insertion plan, the description line's
//! indent and the shift state.


use horizon_core::nbt::JavaString;
use mod_api::{Tag, Text};
use mod_api::text::TextContent;

use crate::description as d;
use crate::config::{self, Config};

/// java: ConfigSchema's defaults.
#[test]
fn config_defaults() {
    let config = Config::default();
    assert!(config.enable_mod);
    assert!(!config.only_display_on_books);
    assert!(!config.only_display_in_enchanting_table);
    assert!(!config.require_keybind_press);
    assert_eq!(config.indent_size, 0);
}

/// The config file round trips through the original's camelCase keys.
#[test]
fn config_round_trip() {
    let path = std::env::temp_dir().join("enchdesc-test-config.json");
    let mut config = Config::default();
    config.only_display_on_books = true;
    config.require_keybind_press = true;
    config.indent_size = 4;
    assert!(config::save(&path, &config));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"enableMod\""));
    assert!(text.contains("\"indentSize\""));
    let loaded = config::load(&path);
    assert_eq!(loaded.enable_mod, true);
    assert_eq!(loaded.only_display_on_books, true);
    assert_eq!(loaded.require_keybind_press, true);
    assert_eq!(loaded.indent_size, 4);
    std::fs::remove_file(&path).ok();
}

/// A broken config file falls back to the defaults, upstream's error path.
#[test]
fn config_broken_file_falls_back() {
    let path = std::env::temp_dir().join("enchdesc-test-broken.json");
    std::fs::write(&path, "{ not json").unwrap();
    let loaded = config::load(&path);
    assert_eq!(loaded, Config::default());
    std::fs::remove_file(&path).ok();
}

/// The enchantment's vanilla lang key mapping.
#[test]
fn lang_key_maps_the_description_id() {
    assert_eq!(
        d::lang_key("minecraft:sharpness").as_deref(),
        Some("enchantment.minecraft.sharpness")
    );
    assert_eq!(d::lang_key("sharpness"), None);
}

/// The description key choice: `.desc`, or `.description` when `.desc` is
/// missing from the lang and `.description` exists.
#[test]
fn description_key_choice() {
    assert_eq!(
        d::choose_description_key("enchantment.minecraft.sharpness", true, false),
        "enchantment.minecraft.sharpness.desc"
    );
    assert_eq!(
        d::choose_description_key("enchantment.minecraft.sharpness", true, true),
        "enchantment.minecraft.sharpness.desc"
    );
    assert_eq!(
        d::choose_description_key("enchantment.minecraft.sharpness", false, true),
        "enchantment.minecraft.sharpness.description"
    );
    assert_eq!(
        d::choose_description_key("enchantment.minecraft.sharpness", false, false),
        "enchantment.minecraft.sharpness.desc"
    );
}

/// The name-line match: the name, with or without a level suffix, in one of
/// the game's two name colors; anything else refuses.
#[test]
fn name_line_match() {
    assert!(d::is_name_line("Sharpness", "Sharpness", d::NAME_COLOR));
    assert!(d::is_name_line("Sharpness V", "Sharpness", d::NAME_COLOR));
    assert!(d::is_name_line("Curse of Vanishing", "Curse of Vanishing", d::CURSE_COLOR));
    // The item's own name line is white, never a name line.
    assert!(!d::is_name_line("Sharpness", "Sharpness", 0xFFFFFFFF));
    // A line merely starting with the name is not the name line.
    assert!(!d::is_name_line("Sharpnessless", "Sharpness", d::NAME_COLOR));
    assert!(!d::is_name_line("Increases damage", "Sharpness", d::NAME_COLOR));
}

/// The insertion plan: after each matched name line, back to front.
#[test]
fn insertion_plan_after_each_name_line() {
    let lines = vec![
        ("Diamond Sword".to_owned(), 0xFFFFFFFF),
        ("Sharpness V".to_owned(), d::NAME_COLOR),
        ("An iron sword with an edge.".to_owned(), 0xFF555555),
        ("Unbreaking III".to_owned(), d::NAME_COLOR),
        ("Increases durability.".to_owned(), 0xFF555555),
    ];
    let targets = vec![
        (
            "minecraft:sharpness".to_owned(),
            "Sharpness".to_owned(),
            "enchantment.minecraft.sharpness.desc".to_owned(),
        ),
        (
            "minecraft:unbreaking".to_owned(),
            "Unbreaking".to_owned(),
            "enchantment.minecraft.unbreaking.desc".to_owned(),
        ),
        (
            "minecraft:mending".to_owned(),
            "Mending".to_owned(),
            "enchantment.minecraft.mending.desc".to_owned(),
        ),
    ];
    let plan = d::insertion_plan(&lines, &targets);
    assert_eq!(
        plan,
        vec![
            // descending: applied back to front, positions in the
            // original tooltip's coordinates so no insert shifts another
            (4, "enchantment.minecraft.unbreaking.desc".to_owned()),
            (2, "enchantment.minecraft.sharpness.desc".to_owned()),
        ]
    );
}

/// The description line: the translatable, dark gray, with the indent as a
/// literal prefix when one is set.
#[test]
fn description_line_shape() {
    let (text, color) =
        d::description_line("enchantment.minecraft.sharpness.desc".to_owned(), 0);
    assert_eq!(
        text.content,
        Text::translate("enchantment.minecraft.sharpness.desc", Vec::new()).content
    );
    assert_eq!(color, d::DARK_GRAY);
    let (text, _) =
        d::description_line("enchantment.minecraft.sharpness.desc".to_owned(), 4);
    match &text.content {
        TextContent::Literal(prefix) => {
            assert_eq!(prefix, "    ");
            assert_eq!(text.children.len(), 1);
        }
        other => panic!("expected a literal prefix, got {other:?}"),
    }
}

/// The component tag's entries: id then level, in the stack's own order.
#[test]
fn enchantment_entries_from_components() {
    assert_eq!(
        d::entries_from_tag(&Tag::compound(vec![(
            JavaString::from("minecraft:sharpness"),
            Tag::Short(5)
        )])),
        vec![("minecraft:sharpness".to_owned(), 5)]
    );
    assert_eq!(
        d::entries_from_tag(&Tag::compound(vec![(
            JavaString::from("minecraft:mending"),
            Tag::Int(1)
        )])),
        vec![("minecraft:mending".to_owned(), 1)]
    );
    assert!(d::entries_from_tag(&Tag::End).is_empty());
}
