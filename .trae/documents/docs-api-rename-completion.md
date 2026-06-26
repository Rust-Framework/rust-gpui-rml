# Plan: Complete Docs API Rename (Batches 5–8)

## Summary

Finish the documentation API-rename migration started in batches 1–4. The remaining
work is mechanical: convert the last 58 `#[view]` references to `#[component]` across
14 files, plus one `RmlApplication` run-pattern update. No `IRmlView`, `rml_view`,
`mod view`, `ModernWindow`, or trait-hierarchy text remains in `docs/`, so Rules 2–5
are already satisfied for these files.

## Current State Analysis (verified via Grep, 2026-06-26)

- `#[view]` count: **58 occurrences across 14 files** (see breakdown below).
- `IRmlView`: **0 matches** anywhere in `docs/`.
- `#[view(...)]` (parameterized): **0 matches** — all are bare `#[view]`.
- `rml_view` / `mod view` / `ModernWindow`: only false positive is `mod views;` (plural
  module name) in `quick-start.md` — **no change needed**.
- Rule 5 trait-hierarchy prose: **0 matches**.
- Title/size field scan: every `title`/`size` field found belongs to an already-correct
  `#[component]` struct (Dialog, Button) or a data Model — **none** sit on a
  `#[view]`-marked struct. Therefore every remaining `#[view]` is a component, not a
  window → all become `#[component]` per Rule 1's default.
- Rule 7 run pattern: exactly **1** site — `responsibility.md:229`.

## Proposed Changes

For 13 of the 14 files the change is a single `Edit` with `replace_all=true`:
`#[view]` → `#[component]`. File 14 (`responsibility.md`) gets that replace_all **plus**
one targeted Rule 7 edit.

> Edit tool requires a prior `Read` of each file in this session. Read each file
> (parallel, in batch groups) immediately before its Edit.

### Batch 5 — `docs/05-events/` (3 files, 7 occurrences)

| File | Occurrences | Structs |
|------|-------------|---------|
| `docs/05-events/event-objects.md` | 1 (L322) | EditorView |
| `docs/05-events/debounce-throttle.md` | 4 (L46, 145, 270, 305) | SearchView, DragView, SearchView, SearchView |
| `docs/05-events/custom-events.md` | 2 (L119, 362) | SearchPanel, UserView |

Action: `replace_all` `#[view]` → `#[component]` per file.
(Note: `custom-events.md` L263 already has `#[component(template = ...)]` — untouched.)

### Batch 6 — `docs/06-components/` (4 files, 11 occurrences)

| File | Occurrences | Structs |
|------|-------------|---------|
| `docs/06-components/slots.md` | 1 (L162) | MyView |
| `docs/06-components/custom-components.md` | 2 (L84, 523) | MyView, UserListView |
| `docs/06-components/composition.md` | 5 (L91, 149, 192, 321, 576) | UserManagement, SearchPanel, App, UserListContainer, UserManagement |
| `docs/06-components/component-props.md` | 3 (L177, 273, 518) | ParentView, MyView, UploadView |

Action: `replace_all` `#[view]` → `#[component]` per file.
(The `App` struct at composition.md L192 holds services and has no title/size → component.)

### Batch 7 — `docs/07-styling/` + `docs/08-lifecycle/` (6 files, 31 occurrences)

| File | Occurrences | Structs |
|------|-------------|---------|
| `docs/07-styling/theming.md` | 1 (L239) | App |
| `docs/08-lifecycle/lifecycle-overview.md` | 3 (L97, 367, 400) | MyView, EditorView, ParentView |
| `docs/08-lifecycle/on-loaded.md` | 8 (L31, 85, 129, 167, 206, 359, 388, 440) | UserListView, ClockView, NotificationView, SearchView, ChartView, DataView, DetailView, UserProfileView |
| `docs/08-lifecycle/on-unloaded.md` | 6 (L30, 86, 115, 144, 172, 307) | DataView, NotificationView, EditorView, VideoPlayerView, ChatView, ChatView |
| `docs/08-lifecycle/async-tasks.md` | 5 (L25, 95, 332, 396, 516) | DataView, SearchView, ClockView, UploadView, DataLoader |
| `docs/08-lifecycle/resource-management.md` | 8 (L25, 98, 169, 220, 316, 353, 465, 703) | DataView, FileView, ApiView, ChatView, NotificationView, DashboardView, CachedDataView, ImageViewer |

Action: `replace_all` `#[view]` → `#[component]` per file.

### Batch 8 — `docs/09-architecture/responsibility.md` (1 file, 9 + 1 occurrences)

`#[view]` at L32, 73, 100, 142, 152, 158, 191, 208, 285 — mix of prose (backticked),
code comments, and code attributes. All refer to the general "mark a ViewModel /
Code-Behind" concept → `#[component]`.

Plus Rule 7 run pattern at L229:
- Old: `` `RmlApplication::new().run::<MyViewModel>()` 启动根视图 ``
- New: `` `RmlApplication::new().main_window::<MyWindow>().run()` 启动根视图 ``

Action:
1. `replace_all` `#[view]` → `#[component]` (handles all 9).
2. Targeted `Edit` for the L229 run pattern (old_string includes surrounding prose for
   uniqueness).

## Assumptions & Decisions

1. **All `#[view]` → `#[component]`, none → `#[window]`.** Justified: no `#[view]`-marked
   struct in the remaining files carries window-defining title/size fields; the title/size
   fields that do exist belong to Dialog/Button components (already `#[component]`) or
   data Models. Rule 1 says default to `#[component]` when in doubt.
2. **`mod views;` in quick-start.md is left alone** — it is a plural Rust module name
   (a `views/` directory), not the `mod view` pattern from Rule 3.
3. **`responsibility.md` run-pattern type param** renamed `MyViewModel` → `MyWindow` to
   match the new API semantics (entry point is now a window) and the Rule 7 example.
4. **No Rule 2/3/4/5 work needed** in remaining files — Grep confirmed zero matches.
5. `replace_all` is safe for every file: `#[view]` is a unique token, and no parameterized
   `#[view(...)]` forms exist to be accidentally touched.

## Execution Order

1. Batch 5 (3 files): Read all → Edit (replace_all) all.
2. Batch 6 (4 files): Read all → Edit (replace_all) all.
3. Batch 7 (6 files): Read all → Edit (replace_all) all.
4. Batch 8 (1 file): Read → Edit replace_all → Edit run pattern.
5. Verification: Grep `#[view]` and `IRmlView` across `docs/` → expect zero matches.

## Verification

After all edits, run two Grep calls over `e:\GitCode\RF\rust-gpui-rml\docs`:
- `#\[view\]` → must return **0 files**.
- `IRmlView` → must return **0 files** (already 0; re-confirm no regression).

Also spot-check `responsibility.md` L229 to confirm the run pattern reads
`main_window::<MyWindow>().run()`.
