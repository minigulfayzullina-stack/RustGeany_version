# Test Plan

Status: **plan only** — most tests need Phase 0 (compiling code) before they can run. Testable pure logic can be extracted and tested first.

## Levels

### 1. Unit tests (pure logic — write these FIRST)
Pure, UI-free functions; put them beside the code (`#[cfg(test)] mod tests`) once the logic lives in a module. These are the highest-value because the current code has no coverage at all.

| Target | Key cases |
|---|---|
| `Filetype::from_ext` | known exts map correctly; unknown → `PlainText`; case-insensitivity (`PY`, `Rs`). |
| `Filetype::name` | every variant round-trips to a display name. |
| `SyntaxHighlight::for_filetype` | each supported language returns non-empty keyword list; `is_keyword/is_type/is_builtin` match; word boundaries (e.g. `int` shouldn't match `integer`). |
| `FindReplace::search` / `replace` | case-sensitivity on/off; whole-word; special regex chars in needle (`(a)`, `a+b`) are escaped; empty needle returns empty; multiline content; replace respects case/whole-word flags. |
| `FoldState::parse` | balanced braces; `} else {` one-line case; nested levels; unbalanced `{` (closes at EOF); braces inside strings/comments (post-fix); `get_visual_lines` hides exactly the inner range of a collapsed fold. |
| `BracketHighlight::find` | open→close and close→open matching; nested pairs; unmatched → `None`; cursor past EOF → `None`; ignores bracket types other than the one under cursor. |
| `SymbolParser::parse` | Rust `fn`/`struct`/`enum`/`trait`/`mod` (incl. `pub`/`async` prefixes); Python `def`/`class`; line numbers correct; sorted by line; empty/unsupported filetype → empty. |
| `BuildSystem::expand` | `{file}` and `{name}` substitution; `None` path leaves placeholders; extensionless stems. |
| `GeanyProject::save`/`load` | round-trip JSON equality; load of missing/invalid file → `Err`, not panic. |
| `AutoCompleter::trigger` | prefix match is case-insensitive; empty current word → hidden; symbols merged after keywords; `next/prev` wrap-around; `insert` returns selected item. |
| `Document::from_file` | name/extension/filetype derived from path; `modified=false` on open. |
| `Terminal` builtins | `cd ~` expands to `$HOME` (after fix); `pwd`; unknown command falls through to shell (skip in CI, or assert it doesn't panic). |

### 2. Snapshot tests (highlighter output)
Once the real highlighter lands (Phase 1), keep `tests/snapshots/` with small source snippets per language → assert the produced `LayoutJob` sections (text + color) match a stored snapshot. This catches silent color/regression changes in `ThemeColors`.

### 3. Integration tests (app level)
- **Construction smoke test:** `GeanyApp::new` builds with the example doc, one open doc, fold regions parsed. (eframe `App` can be constructed headless without a window.)
- **State-machine tests without UI:** call `new_doc/open_file(via path)/save_file(via path)/close_doc` directly and assert `docs`, `active`, `modified` transitions. Needs file dialogs to be injectable/mocked — add a `FileDialogProvider` trait with a headless stub.
- **Save/load round trip:** write to a tempdir, reopen, assert content + `modified` flag.

### 4. UI-level (later, optional)
`egui_kittest` (egui 0.32 test harness) to drive panels: tab switching, sidebar tab changes, find panel open/close. Defer until Phase 1 UI is stable.

## Tooling & CI gates

- `cargo test` in CI on every PR (already in `.github/workflows/ci.yml`).
- `cargo clippy -D warnings` and `cargo fmt --check` as gates.
- Optional later: `cargo-tarpaulin`/`llvm-cov` for coverage reporting; target ≥70% on the pure-logic modules.
- Test fixtures under `tests/fixtures/` (small `.rs`/`.py`/`.c` samples); tempdirs via `tempfile`.

## What is explicitly NOT tested

- Pixel-perfect rendering (immediate mode — flaky, low value).
- Real shell execution of the terminal fallback in CI.
- Native file dialogs (mocked behind `FileDialogProvider`).
