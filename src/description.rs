//! java: EnchDescCommon.onItemTooltip and DescriptionManager.getDescription,
//! riding the mod API's item_tooltip hook (the finished tooltip with mutable
//! lines — the hook the first port's HEAD mixin stood in for).

use std::cell::RefCell;
use std::collections::BTreeMap;

use mod_api::{ClientHost, InputKeyEvent, KeyAction, ItemStack, Tag, Text};
use mod_api::text::TextContent;

use crate::config::Config;

/// Upstream's description color, `ChatFormatting.DARK_GRAY`, as the tooltip
/// lines' colors are packed: `hud::rgba(85, 85, 85, 255)`.
pub const DARK_GRAY: u32 = 0xFF555555;
/// The color the game paints enchantment name lines with
/// (`hud::rgba(170, 170, 170, 255)`), non-curse names included.
pub const NAME_COLOR: u32 = 0xFFAAAAAA;
/// The color the game paints curse name lines with
/// (`hud::rgba(255, 85, 85, 255)`).
pub const CURSE_COLOR: u32 = 0xFF5555FF;

/// The enchantment map's components: the held enchantments, and an enchanted
/// book's stored ones.
const HELD_COMPONENT: &str = "minecraft:enchantments";
const STORED_COMPONENT: &str = "minecraft:stored_enchantments";

#[derive(Clone)]
struct State {
    config: Config,
    shift_left_down: bool,
    shift_right_down: bool,
}

impl State {
    /// java: Screen.hasShiftDown() — either shift key.
    fn shift_down(&self) -> bool {
        self.shift_left_down || self.shift_right_down
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        config: Config::default(),
        shift_left_down: false,
        shift_right_down: false,
    });
    /// java: DescriptionManager's ConcurrentHashMap, the chosen description
    /// key per enchantment description id.
    static DESCRIPTIONS: RefCell<BTreeMap<String, String>> =
        RefCell::new(BTreeMap::new());
}

/// java: the config the constructor loaded, now the settings' overlay.
pub(crate) fn set_state(config: Config) {
    STATE.with(|state| {
        *state.borrow_mut() = State {
            config,
            shift_left_down: false,
            shift_right_down: false,
        }
    });
}

/// java: EnchDescCommon.onItemTooltip, on the mod API's item_tooltip hook.
pub(crate) fn on_item_tooltip(host: &dyn ClientHost, event: &mut mod_api::ItemTooltipEvent<'_>) {
    let state = STATE.with(|state| state.borrow().clone());
    // java: if (config.enableMod && !stack.isEmpty() && stack.hasTag())
    if !state.config.enable_mod
        || event.stack.count() == 0
        || event.stack.components().is_none()
    {
        return;
    }
    // java: stack.isEnchanted() and the enchanted book check. The held map is
    // the enchantments component; a book's enchantments live in
    // stored_enchantments, which vanilla puts on enchanted books only, so the
    // book check rides it. An empty enchanted book reaches the same
    // empty-set return upstream does.
    let held = enchantment_entries(&event.stack, HELD_COMPONENT);
    let stored = enchantment_entries(&event.stack, STORED_COMPONENT);
    let is_book = !stored.is_empty();
    if !((!state.config.only_display_on_books && !held.is_empty()) || is_book) {
        return;
    }
    // java: onlyDisplayInEnchantingTable gates on
    // `Minecraft.getInstance().screen instanceof EnchantmentScreen`. The mod
    // API has no current-screen query, so this one entry stays file-held and
    // the gate stays unenforceable (README's seams); the entry itself ships
    // in the config file at upstream's default, false.
    // java: EnchantmentHelper.getEnchantments(stack).keySet() — an enchanted
    // book's stored enchantments, the held ones otherwise.
    let enchantments = if is_book { stored } else { held };
    if enchantments.is_empty() {
        return;
    }
    if !state.config.require_keybind_press || state.shift_down() {
        let targets: Vec<(String, String, String)> = enchantments
            .iter()
            .filter_map(|(id, _level)| {
                let description_id = lang_key(id)?;
                // java: translatable.getKey().equals(enchantment.getDescriptionId()):
                // Horizon's name lines are the game's own resolved strings,
                // so the match goes through the same lang key — the line's
                // text is the enchantment's translated name, with an optional
                // level suffix, in the name color the game paints those lines
                // with. A name the lang does not know gets no insertion,
                // never a wrong one.
                let name = host.translate(&description_id)?;
                let chosen = description_key(host, &description_id);
                Some((description_id, name, chosen))
            })
            .collect();
        let lines: Vec<(String, u32)> = event
            .lines
            .iter()
            .map(|(line, color)| (line_text(line).map(str::to_owned).unwrap_or_default(), *color))
            .collect();
        let plan = insertion_plan(&lines, &targets);
        for (position, key) in plan {
            event
                .lines
                .insert(position, description_line(key, state.config.indent_size));
        }
    } else {
        // java: tooltip.add(Component.translatable("enchdesc.activate.message")
        // .withStyle(ChatFormatting.DARK_GRAY));
        event
            .lines
            .push((Text::translate("enchdesc.activate.message", Vec::new()), DARK_GRAY));
    }
}

/// The description line: the chosen translatable, dark gray, with the indent
/// as a literal prefix component when one is set (java: if
/// `config.indentSize > 0`, a literal of repeated spaces appends the
/// description).
pub(crate) fn description_line(key: String, indent_size: i32) -> (Text, u32) {
    let mut text = Text::translate(key, Vec::new());
    if indent_size > 0 {
        text = Text::literal(" ".repeat(indent_size as usize)).append(text);
    }
    (text, DARK_GRAY)
}

/// The insertion plan: for each enchantment, the first name line's index + 1
/// and its description key, in back-to-front order. Upstream's loop scans the
/// current tooltip per enchantment (`indexOf` each time); inserting back to
/// front keeps the same result with one pass, so the earlier positions stay
/// put while the later ones land.
pub(crate) fn insertion_plan(
    lines: &[(String, u32)],
    targets: &[(String, String, String)],
) -> Vec<(usize, String)> {
    let mut plan: Vec<(usize, String)> = Vec::new();
    for (_, name, chosen) in targets {
        for (index, (plain, color)) in lines.iter().enumerate() {
            if is_name_line(plain, name, *color) {
                plan.push((index + 1, chosen.clone()));
                break;
            }
        }
    }
    plan.sort_by(|a, b| b.0.cmp(&a.0));
    plan
}

/// java: DescriptionManager.getDescription — the chosen description key,
/// cached per enchantment description id: `.desc`, or `.description` when
/// `.desc` is missing from the lang and `.description` exists. Upstream's
/// `I18n.exists` reads the merged lang layers; the host's translate lookup
/// does the same here, `None` when no layer defines the key.
pub(crate) fn description_key(host: &dyn ClientHost, description_id: &str) -> String {
    DESCRIPTIONS.with(|descriptions| {
        let mut descriptions = descriptions.borrow_mut();
        if let Some(key) = descriptions.get(description_id) {
            return key.clone();
        }
        let key = choose_description_key(
            description_id,
            host.translate(&format!("{description_id}.desc")).is_some(),
            host.translate(&format!("{description_id}.description")).is_some(),
        );
        descriptions.insert(description_id.to_owned(), key.clone());
        key
    })
}

/// java: DescriptionManager's key choice, the `I18n.exists` pair as booleans
/// so the tests drive it without a lang.
pub(crate) fn choose_description_key(
    description_id: &str,
    desc_exists: bool,
    long_exists: bool,
) -> String {
    let base = format!("{description_id}.desc");
    let long = format!("{description_id}.description");
    if !desc_exists && long_exists {
        long
    } else {
        base
    }
}

/// The enchantment's vanilla lang key: minecraft:sharpness turns into
/// enchantment.minecraft.sharpness, the description id upstream matches.
pub(crate) fn lang_key(id: &str) -> Option<String> {
    let (namespace, path) = id.split_once(':')?;
    Some(format!("enchantment.{namespace}.{path}"))
}

/// Whether a tooltip line's text is this enchantment's name, with an optional
/// level suffix such as " V", in one of the two colors the game paints name
/// lines with.
pub(crate) fn is_name_line(plain: &str, name: &str, color: u32) -> bool {
    if color != NAME_COLOR && color != CURSE_COLOR {
        return false;
    }
    plain == name
        || (plain.len() > name.len()
            && plain.starts_with(name)
            && plain.as_bytes()[name.len()] == b' ')
}

/// The line's literal text, `None` for anything not a plain string (the
/// game's name lines are literal strings; translatable components resolve at
/// draw time and are not name lines here).
pub(crate) fn line_text(line: &Text) -> Option<&str> {
    match &line.content {
        TextContent::Literal(text) => Some(text),
        _ => None,
    }
}

/// The stack's enchantment map from a data component: the component's tag,
/// then the entries, id then level, in the stack's own order. Vanilla encodes
/// the levels as NBT shorts; bytes and ints ride along.
pub(crate) fn enchantment_entries(stack: &ItemStack, component: &str) -> Vec<(String, u32)> {
    let Some(components) = stack.components() else {
        return Vec::new();
    };
    let nbt = components.as_nbt();
    let Some((_, tag)) = nbt
        .iter()
        .find(|(id, _)| id.to_string_lossy() == component)
    else {
        return Vec::new();
    };
    entries_from_tag(tag)
}

/// The component tag's entries: id then level, in the stack's own order.
pub(crate) fn entries_from_tag(tag: &Tag) -> Vec<(String, u32)> {
    let Tag::Compound(levels) = tag else {
        return Vec::new();
    };
    levels
        .iter()
        .filter_map(|(id, level)| {
            let level = match level {
                Tag::Short(level) => *level as u32,
                Tag::Int(level) => *level as u32,
                Tag::Byte(level) => *level as u32,
                _ => return None,
            };
            Some((id.to_string_lossy(), level))
        })
        .collect()
}

/// java: Screen.hasShiftDown()'s live state, fed by the mod API's input_key
/// hook (InputEvent.Key): a shift press sets the side, a release clears it.
/// Upstream reads the window's own modifier state.
pub(crate) fn on_input_key(event: &InputKeyEvent<'_>) {
    let Some(side) = shift_side(&event.key) else {
        return;
    };
    apply_key_action(side, &event.action);
}

/// The shift keys the key events report: `Some(true)` is the left one,
/// `Some(false)` the right, `None` not a shift key.
pub(crate) fn shift_side(key: &str) -> Option<bool> {
    match key {
        "key.keyboard.left_shift" => Some(true),
        "key.keyboard.right_shift" => Some(false),
        _ => None,
    }
}

/// A press or repeat sets the side, a release clears it.
pub(crate) fn apply_key_action(side: bool, action: &KeyAction) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        match action {
            KeyAction::Press | KeyAction::Repeat => {
                if side {
                    state.shift_left_down = true;
                } else {
                    state.shift_right_down = true;
                }
            }
            KeyAction::Release => {
                if side {
                    state.shift_left_down = false;
                } else {
                    state.shift_right_down = false;
                }
            }
        }
    });
}

#[cfg(test)]
pub(crate) fn shift_down() -> bool {
    STATE.with(|state| state.borrow().shift_down())
}
