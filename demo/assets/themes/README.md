# Theme CSS packs

RML ships built-in `light` / `dark` palettes in `crates/core/src/theme.rs`. Files in this directory are optional overrides or custom theme packs.

## Override built-in theme

```css
/* dark.css */
:root {
    --primary: #ff6600;
    --button-secondary: #333333; /* optional: override a derived token */
}
```

## Add a new theme pack

1. Create `{name}.css` with a `:root { ... }` block.
2. Override base variables (`--primary`, `--background`, `--foreground`, …).
3. Load at runtime: `cx.use_theme_with_dir("ocean", "assets/themes")`.

Derived tokens (`--button-secondary`, `--description-list-label`, hover/active variants, …) are computed automatically from base colors. Set `--background` to a dark color and the pack is treated as dark (luminance detection); or use theme name `dark` / `light` for built-ins.

See `ocean.css` for a minimal custom dark pack example.
