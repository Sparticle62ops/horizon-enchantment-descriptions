# Horizon Enchantment Descriptions

An exact 1:1 Rust translation of
[Enchantment Descriptions](https://github.com/Darkhax-Minecraft/Enchantment-Descriptions)
(upstream Java, LGPL-2.1-only (c) Darkhax) for the Horizon client's mod API.
Carries the upstream license and credit. Ported for Horizon by sparticle.

A short description line right under each enchantment on the hovered item's
tooltip: every enchantment on the stack, the held ones and an enchanted
book's stored ones, finds the tooltip line carrying its name and the
description goes right after it, dark gray.

## What rides the mod API

- The whole flow is on the `item_tooltip` hook: the finished tooltip with
  mutable lines, the surface `EnchDescCommon.onItemTooltip` mutates upstream.
  The first Horizon port of this mod ran inside the game crate behind a HEAD
  mixin on `Enchantments::tooltip`; this port is a code mod on the current
  SDK instead.
- Four of the five config entries register as settings with upstream's
  defaults and labels taken from the original's field names (its config
  schema carries no lang labels): `enable_mod`, `only_display_on_books`,
  `require_keybind_press`, `indent_size`. The config file stays
  `config/enchdesc.json` in the original's camelCase and pretty printing,
  load-or-create and always save (`ConfigSchema`). Settings overlay the file
  at client setup.
- `require_keybind_press` needs `Screen.hasShiftDown()`; the port tracks the
  two shift keys through the mod API's `input_key` hook (InputEvent.Key),
  press sets the side, release clears it. Upstream reads the window's own
  modifier state.
- Descriptions ship as the original's en_us lang file
  (`assets/enchdesc/lang/en_us.json`) and resolve through the game's lang
  layer: `.desc`, or `.description` when `.desc` is missing from the lang
  and `.description` exists (`DescriptionManager`'s key choice, cached per
  enchantment description id). Upstream's `I18n.exists` reads the merged
  lang layers; the host's translate lookup does the same here.
- The enchantment maps read the stack's `minecraft:enchantments` and
  `minecraft:stored_enchantments` components as NBT, entries in the stack's
  own order. An enchanted book is the stored component's presence, which
  vanilla puts on enchanted books only; an empty enchanted book reaches the
  same empty-set return upstream does.

## Known seams (tracked)

- **Name-line matching.** Upstream matches the name line by component key
  (`TranslatableContents`, the enchantment's description id). Horizon's name
  lines are the game's own resolved strings: the lang lookup lives inside
  `Enchantments::tooltip` in the game crate, which the mod API does not
  expose. The port matches the same lines by their text and the colors the
  game paints them with (`hud::rgba(170, 170, 170, 255)` for names,
  `rgba(255, 85, 85, 255)` for curses): the translated name, with an
  optional level suffix. A name the lang does not know gets no insertion,
  never a wrong one.
- **onlyDisplayInEnchantingTable** gates on `Minecraft.getInstance().screen
  instanceof EnchantmentScreen`. The mod API has no current-screen query, so
  this one entry stays file-held and the gate stays unenforceable; it ships
  in the config file at upstream's default, false.

## Building and testing

```
cargo test
```

Tests cover the config file round trip (camelCase keys, broken-file
fallback), the lang key mapping, the description key choice, the name-line
match, the insertion plan, the description line's indent, the component NBT
reading and the shift state.

## License

LGPL-2.1-only, the upstream license text in [LICENSE](LICENSE). Upstream:
https://github.com/Darkhax-Minecraft/Enchantment-Descriptions
