# RML — Rust Markup Language for GPUI

**English** | [简体中文](README.zh-CN.md)

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org)

> An HTML-friendly, declarative UI framework for [GPUI](https://github.com/zed-industries/zed)
> — the GPU-accelerated Rust UI framework powering the Zed editor.
>
> Markup in `.rml` files, business logic in `.rml.rs` code-behind files, and native GPUI
> rendering code generated at **compile time** (zero runtime overhead).
> Design philosophy: the spirit of WPF XAML plus the syntactical friendliness of HTML.

## Table of Contents

- [What is RML?](#what-is-rml)
- [Motivation](#motivation)
- [Core Features](#core-features)
- [Architecture](#architecture)
- [Repository Layout](#repository-layout)
- [Quick Start](#quick-start)
- [RML at a Glance](#rml-at-a-glance)
- [Docs](#docs)
- [Building](#building)
- [License](#license)

## What is RML?

RML (**R**ust **M**arkup **L**anguage) is a developer-facing, designer-friendly UI
framework built on top of GPUI. It brings the industrial-grade UI development model of
WPF/XAML and modern web frameworks (Vue / React) to native Rust desktop applications:

- **`window.rml`** — a standalone, HTML-like markup file describing the UI structure,
  layout, data bindings and event bindings.
- **`window.rml.rs`** — the *code-behind*, containing plain Rust state, event handlers,
  computed properties and lifecycle callbacks, in the MVVM `ViewModel` role.
- **`build.rs`** — compiles every `.rml` into native GPUI rendering code (`impl Render`),
  so there is nothing to interpret at runtime.

Because the markup is compiled into plain GPUI code, the framework imposes **zero runtime
overhead** and keeps full access to GPUI's GPU-accelerated, immediate/retained hybrid
rendering model.

> **Status note:** Hot reload and a VS Code extension are on the roadmap but are **not yet
> implemented**. See [Core Features](#core-features) for what is already available today.

## Motivation

Authoring UI directly in GPUI is verbose and deeply couples UI structure with business
logic and event handling:

```rust
// Native GPUI — imperative builder chaining, UI + logic interleaved
div()
    .flex()
    .flex_col()
    .gap(px(16.0))
    .p(px(24.0))
    .child(
        div()
            .text_xl()
            .font_weight(FontWeight::BOLD)
            .child(Label::new("Hello World")),
    )
    .child(
        Button::new("Click me").on_click(cx.listener(|this, _ev, cx| {
            this.count += 1;
            cx.notify();
        })),
    );
```

This has real costs: UI logic is tightly coupled to Rust, code is verbose and nested,
designers cannot participate, and there is no standard markup for UI.

RML addresses these problems:

| Goal | Benefit |
|------|---------|
| **Separation of concerns** | UI structure (`.rml`) and logic (`.rml.rs`) are fully independent |
| **HTML-syntax affinity** | Standard HTML tags, attributes and events — near-zero learning curve for web developers |
| **WPF-grade data binding** | One-way / two-way binding, value converters, command system |
| **Zero runtime overhead** | `.rml` compiles to native GPUI rendering code at build time |
| **Designer-friendly** | Pure markup, usable with any XML/HTML tooling |
| **Hot-reload ready** | Standalone files provide the natural basis for future live editing |

## Core Features

**Markup language for GPUI**
- Standard HTML tags (`div`, `p`, `span`, `button`, `input`, `textarea`, `ul`/`li`, `h1`–`h6`, `img`, `label`, …) mapped to native GPUI elements.
- PascalCase tags (`<Button>`, `<Input>`, `<Dialog>`, …) route to components from [`gpui-component`](https://github.com/longbridge/gpui-component) through the `rust-rml-ui` extension crate.
- Standard HTML attributes (`class`, `id`, `style`, `placeholder`, `type`, `disabled`, …).

**MVVM data binding** (a WPF-class capability matrix)
- One-way binding: `{field}` / `attr={expr}` — ViewModel → View automatic sync.
- Two-way binding: `model={field}` — full bidirectional data flow with loop protection.
- Value converters: `model={field | Converter}` — built-in `UpperCase` / `LowerCase` / `Trim` / `Currency` / `Percent` / `BoolToYesNo`, plus custom `IConverter`.
- Computed properties: `#[computed]` with dependency tracking and `ComputedCache` invalidation.
- Command system: `#[command]` + `onclick={method}` (strongly typed) and declarative `command={field}` (aligned with WPF `ICommand`, dispatched via `can_execute`/`execute`).
- Field validation: `#[validate(range/length/required/regex/custom/IValidate)]` with automatic error-message management.
- Debounce / throttle: `#[command(debounce = "300ms")]`.

**Directives and events** (no framework prefixes)
- `if` / `else` / `each` / `key` / `model` / `show` / `once` / `html` / `ref` / `slot`.
- Standard event model: `onclick`, `oninput`, `onchange`, `onkeydown`, `onkeyup`, `onmouseenter`, `onmouseleave`, and more.

**Component system**
- Custom components defined as `#[component]` structs with corresponding `.rml` templates.
- Named slots for composition, `ref`/`#[element]` element references, and `#[on_loaded]` / `#[on_unloaded]` lifecycle hooks.

**Styling, theming & i18n**
- CSS stylesheet support: `.rml`/`.css` parsed and mapped to GPUI styling; themed with `assets/themes/*.css` (dark / light / custom).
- Built-in theme runtime (`assets/themes`), and an i18n layer (`t("key")`) that extracts keys at build time.

**Tooling**
- A bundled **tree-sitter grammar** for RML syntax highlighting/injection (highlights + injections for helix/neovim).
- An **LSP server** (`rust-rml-lsp`) providing completion, hover, diagnostics, formatting, definition/references, rename and more — including cross-language navigation between `.rml` and `.rml.rs` (and Rust via rust-analyzer).
- Incremental `build.rs` compilation with three-tier caching (`.rml` hash + code-behind hash + engine source hash).
- An embedded terminal component (`rust-rml-ui-term`, backed by `alacritty_terminal`).

## Architecture

### Three-layer runtime

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Presentation Layer                             │
│   window.rml (UI markup / bindings / events)  + window.rml.rs          │
│   (Code-Behind: state · handlers · computed · lifecycle)              │
└──────────────────────────────────────────────────────────────────────┘
                              │  compiled by
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        RML Compiler (build.rs + proc-macros)          │
│   .rml → tokenize → AST → semantic validation → GPUI codegen          │
└──────────────────────────────────────────────────────────────────────┘
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                         Framework Layer                               │
│   GPUI (render engine) · gpui-component (components) · RML runtime     │
│   (binding system · two-way binding · converter · computed cache)      │
└──────────────────────────────────────────────────────────────────────┘
```

The whole pipeline follows **MVVM**: a pure-Rust `Model`, a `ViewModel` (the `.rml.rs`
Code-Behind, an entity with reactive state and commands) and the `View` (the `.rml`
markup consuming state through bindings).

### Compile-time picture

```
1. build.rs runs
   ├── scans src/**/*.rml
   ├── parses each .rml into an AST
   ├── validates syntax and binding paths
   └── writes *.generated.rs into OUT_DIR/
2. rustc compiles .rml.rs (your logic) and include!-s the generated Render impl
3. the final binary links everything — no runtime interpreter involved
```

Each crate enforces `#![forbid(unsafe_code)]` where possible.

## Repository Layout

This is a Cargo workspace. Core crates:

| Crate | Role |
|-------|------|
| `crates/core` | Contract layer: `IModel` / `IViewModel` / `IComponent` / `IWindow` / `ICommand` / `IConverter` / `ITwoWayBinding` / `ILifecycle` and event/marker types. |
| `crates/macros` | Procedural macros: `#[derive(IModel)]`, `#[component]`, `#[window]`, `#[command]`, `#[computed]`, `#[validate]`, `#[rml::main]`, … |
| `crates/engine` | The compiler: tokenizer → AST → validator → codegen, plus CSS mapping, asset processing and i18n extraction, and the `build.rs` API. |
| `crates/ui` | Extension component library wrapping `gpui-component` (Dialog / Form / List / …) and built-in window types. |
| `crates/ui-term` | Embedded terminal component (`TerminalView`). |
| `crates/app` | WPF-style launcher: `RmlApplication::new().main_window::<W>().run::<L>()`. |
| `crates/rml` | RML client: tree-sitter grammar + LSP client hooks + code-editor providers. |
| `crates/lsp` | The RML language server (`rml-lsp` binary) with a cross-language coordinator. |
| `crates/dap` | Excluded from the workspace build (heavy `lldb` git deps); build it in its own directory when required. |
| `demo` | Showcase application validating the `.rml` + `.rml.rs` + `build.rs` flow with 100+ component cases. |
| `studio/*` | **Arc Studio**, a sample IDE product layered on top of the framework (shell / editor / explorer / chat + core DI). |

## Quick Start

Prerequisites: a Rust toolchain and a network connection (GPUI and `gpui-component` are
pulled from git, pinned to specific revisions; see `Cargo.toml`).

**1. Clone and build**

```bash
git clone https://github.com/Rust-Framework/rust-gpui-rml.git
cd rust-gpui-rml
cargo build
```

> The workspace deliberately puts `crates/dap` in `exclude` because its heavy `lldb`
> git dependency blocks compilation on restricted networks. Build it separately if needed.

**2. Run the showcase demo**

```bash
cargo run -p rust-rml-demo
```

This launches the RML showcase — a tabbed window exercising 100+ `.rml` cases (buttons,
forms, tables, menus, dialogs, i18n, theming, terminal, and more).

**3. Run Arc Studio**

```bash
cargo run -p arc-studio
```

Arc Studio is a small IDE built entirely with `.rml` views (project explorer, editor,
chat panel, status bar) — a realistic end-to-end example of the framework.

## RML at a Glance

Markup (`counter.rml`):

```html
<div class="app">
    <h1>Count: {count}</h1>
    <button onclick={increment}>+1</button>
    <button onclick={decrement} if={count > 0}>-1</button>
</div>
```

Code-behind (`counter.rml.rs`):

```rust
use rml::prelude::*;

#[component]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        // The macro auto-injects bump_version("count") + cx.notify()
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.count > 0 {
            self.count -= 1;
        }
    }
}
```

Entry point (`main.rs`):

```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;

mod counter;

#[rml::main] // auto-injects rml::embed_assets!(); (resource registration)
fn main() {
    rml_app::RmlApplication::new()
        .main_window::<counter::Counter>()
        .run();
}
```

No handwritten GPUI builder chains — UI and logic are cleanly separated and state changes
drive redraws automatically.

## Docs

- **`CLAUDE.md`** — the repository's explicit behavioral guidelines for large-language-model
  coding agents (think-before-coding, simplicity first, surgical changes, goal-driven
  execution). If you work on this codebase with an AI assistant, point it here.
- **`demo/`** — 100+ runnable showcase cases under `demo/src/cases/`.
- **`crates/*/README.md`** — per-crate design docs (`core`, `macros`, `engine`, `ui`, `app`,
  …) with constraints, traits and design rules.
- **`.trae/documents/`** — detailed design and planning documents (architecture plans,
  MVVM completion, RML iteration, component specs, etc.).

## Building

```bash
cargo build                  # build the whole workspace (excluding crates/dap)
cargo build -p rust-rml-demo # build the demo only
cargo test -p rust-rml-engine# run the engine's codegen / CSS / e2e tests
```

Notes:

- GPUI and `gpui-component` are pinned to specific git revisions in `Cargo.toml` for
  reproducibility. Regenerate/lock as needed when upgrading.
- Resource mode (embedded vs. filesystem) is configured once in `build.rs` via
  `.assets(path, embed)`; the runtime API stays identical in both modes.
- The `crates/dap` crate is excluded so the default workspace build works without the
  `lldb` bindings.

## License

[MIT](LICENSE)