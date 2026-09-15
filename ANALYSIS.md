# RustGeany — Analysis & Improvement Plan

**Repo:** `minigulfayzullina-stack/RustGeany_version`
**Single commit:** `f7986a6 — Geany-Rs v0.6.0 - COMPLETE IDE`
**State at review:** 6 files, `geany-rs/src/main.rs` = 1010 lines, everything in one file.
Reviewed with a real `cargo check` on rustc 1.98.1 (results in §3).

---

## 1. What's actually here

| Path | What it is |
|------|------------|
| `geany-rs/Cargo.toml` | eframe/egui 0.32 + serde/parking_lot/rfd/regex/whoami/dirs. No tests, no lints, no LICENSE file. |
| `geany-rs/src/main.rs` | The entire application in one file — types, parser, UI, build runner, terminal. |
| `geany-rs/src/example.rs` | Demo file loaded into doc #1 at startup via `include_str!`. |
| `geany-rs/README.md` | Marketing-style README listing features, most of which don't work (see §4). |
| `RUST_REBUILD_PLAN.md` | A *separate*, unrelated high-level "rebuild Geany from C into Rust" plan with a multi-crate workspace. It describes a target architecture that the actual `main.rs` does not follow at all. |
| `newfile` | Junk file, contents = `new`. Should be deleted. |

The README calls this "v0.6.0 — Final Polish Edition" with a long ✅ feature list. The code does not match that claim.

---

## 2. Architecture assessment

- **Monolith.** One 1010-line `main.rs`. `RUST_REBUILD_PLAN.md` already proposes a sensible 4-crate workspace (`geany-core`, `geany-ui`, `geany-plugin`, `geany` bin) — none of it exists.
- **No separation of concerns.** UI, state, parsing, file I/O, process spawning, and "plugins" are all interleaved in the `eframe::App::update` impl and inline modules.
- **Immediate-mode misuse.** Expensive work (regex symbol parse, fold recalculation) is done inside `update()` every frame instead of cached/memoized.
- **No persistence.** Theme, window size, recent files, open docs, project — all lost on quit.
- **No tests, no CI, no .gitignore, no LICENSE file** (README says GPL-2+).
- **Plan vs. reality divergence.** `RUST_REBUILD_PLAN.md` recommends `egui_code_editor`/`syntect` for syntax highlighting; the code uses a plain `TextEdit` and never colorizes anything.

---

## 3. Does it compile? — NO. (real `cargo check` on rustc 1.98.1)

**It does not build.** `cargo check` fails with **25 compile errors** (after working past a phantom "mismatched delimiter" that cascades from the first real error). The commit message "COMPLETE IDE" never passed a compiler. Confirmed categories:

| Count | Error | Where / why | Fix |
|---|---|---|---|
| 8 | `E0609: no field 'cmd' on Modifiers` | every keyboard shortcut uses `mods.cmd` (lines 773–780) | `mods.ctrl` is the cross-platform Ctrl; `mods.command` is macOS-cmd. Use `mods.ctrl` (or `mods.ctrl && !mods.alt` etc.) |
| 4 | `E0277: ? operator in a closure/method that doesn't return Option/Result` | `self.active?` / `app.active?` inside egui closures and `update()` (lines 780, 783, 441, 811…) | `self.active.unwrap_or(0)` or proper `if let Some(i) = self.active` — don't use `?` here |
| 1 | `E0061: horizontal takes 1 arg, 2 supplied` | `ui.horizontal(ui, |ui| { … })` (line 910) | `ui.horizontal(|ui| { … })` |
| 1 | `E0277: expected FnOnce(&mut Ui) closure, found &mut Ui` | same line 910 call (the stray `ui` arg) | same fix |
| 1 | `E0599: no method toggle_sized on &mut Ui` | sidebar tab icons (line 820) | `ui.selectable_value(&mut sel, icon, …)` or `ui.toggle_value` |
| 1 | `E0599: no variant Key::Shift` | `mods.cmd && Shift && R` macro-toggle (line 811) | check `mods.shift && key R` — `Key::Shift` isn't a `Key` variant the way it's used |
| 1 | `E0599: no method request_focus on TextEdit` | Go-to-Line input (line 1015) | `TextEdit::request_focus` doesn't exist; use `ctx.memory_mut(|m| m.request_focus(id))` |
| 1 | `E0599: no method cursor on &Context` | menu dropdown positioning (line 796) | `ctx.pointer_position()` / `ctx.input(|i| i.pointer.latest_pos)` |
| 1 | `E0599: no method bold on RichText` | (line 441) | `.strong()` |
| 1 | `E0599: always_auto_resize not on Window` | completion popup (line 954) | `.auto_sized()` |
| 1 | `E0599: no constant LEFT_UP on Align2` | completion popup anchor (line 954) | `Align2::LEFT_TOP` |
| 1 | `E0277: [Document] cannot be indexed by Option<usize>` | `self.docs[i]` with `active: Option` used directly (line 848) | `self.docs[i.unwrap_or(0)]` or `if let Some` |
| 1 | `E0382: borrow of moved value 'pr'` | project load (line 677) | load into a binding and clone before move |
| 1 | `E0277: &&Vec<MacroAction> is not an iterator` | `MacroManager::execute` (line 441) | `.iter()` / fix the `saved.last().map(\|m\| &m.actions).unwrap_or(&vec![])` borrow |
| 1 | `E0277: size of 'str' not known` | macro execute return path (line 441) | return `String`, not a dangling `&str`-ish value |
| 1 | `E0277` (closure type mismatch) | cascading from the 2-arg `horizontal` | fixed by the same line-910 fix |

**Bottom line:** ~25 errors, most from a handful of repeated mistakes — wrong modifier field (`cmd`), `?`-abuse, one stray extra arg, and several egui-0.32 API names that don't exist (`toggle_sized`, `request_focus`, `cursor`, `bold`, `always_auto_resize`, `Align2::LEFT_UP`, `Key::Shift`). None are subtle; all are mechanical to fix. The code was committed without ever compiling.

The earlier grep-level suspects (`ui.horizontal(ui, …)`, `toggle_sized`) are both confirmed real errors.

---

## 4. Feature-by-feature reality check

The README ✅ list vs. what the code actually does:

| Claimed feature | Reality |
|---|---|
| ✅ Syntax highlighting (Rust, C, C++, Python, JS, …) | **Not implemented.** `SyntaxHighlight` is defined (keywords/types/builtins) but never instantiated or applied. The editor is a plain monospace `TextEdit` — all text is one color. Six themes only swap the background, not token colors. |
| ✅ Code Folding (clickable UI) | **Broken.** `FoldState::parse` counts `{`/`}` per line with `net = opens - closes`; lines with equal opens/closes (`} else {`) are dropped, and matching braces to regions is naive. `get_visual_lines` is recomputed **inside** the per-line render loop (O(n²) per frame) and its hidden-line accounting (`hidden.saturating_sub(i)`) is wrong. The +/- markers render but don't reliably hide text. |
| ✅ Bracket Matching (visual) | **Not wired.** `BracketHighlight::find(&d.content, d.content.len().min(100))` runs against a *static* position (end of buffer or char 100), never the cursor, and the result `self.bracket` is never drawn. |
| ✅ Auto-completion (keywords + symbols) | **Partly broken.** Popup shows items, but `Enter` does `d.content.push_str(&t)` — inserts the snippet at the **end of the file**, not at the cursor. |
| ✅ Find and replace | **Half-working.** `search()` returns matching *line indices* and only logs "Found N matches" — no selection/scroll/highlight. Replace only does "Replace All"; no next/replace-one. |
| ✅ Project Management (.geany files) | Loads/saves a JSON `.geany` file, but `recent_files` is never reopened and "Open Folder" creates a stub project with no file tree. The sidebar "Files" tab lists *open documents*, not project files. |
| ✅ Plugin System | **Fake.** Four hardcoded `Plugin { enabled: bool }` rows. `enabled` toggles nothing — no loading, no behavior. |
| ✅ Macros (record/playback) | **Broken.** `record()` is only called on a buffer change and stores the *entire new buffer* as one `Insert` action; `execute()` appends the whole captured buffer back. Macros don't replay edits, they duplicate the document. |
| ✅ Terminal panel | Works-ish: shell passthrough + a few builtins. `cd ~` is broken — it `replace('~', whoami::username())`, producing a relative `<username>/...` path instead of `$HOME`. |
| ✅ Multiple themes | 6 themes, but only bg/fg differ; editor text stays monochrome (see highlighting). |
| ✅ Tab-based document management | Works. `close_doc` index handling is OK; the `active` recompute is fine. |
| ✅ Cross-platform | Mostly; uses `#[cfg(windows)]` shells. Unverified — won't even build here yet. |
| ✅ Dark/Light theme support | Works at the egui `Visuals` level. |
| Keyboard shortcuts | `Ctrl+Space` completion uses `mods.cmd`, which on Linux/Windows is Ctrl only via `mods.ctrl`; should use `mods.ctrl` cross-platform. Go-to-line sets `cursor_line` but doesn't move the real `TextEdit` cursor, and `cursor_line/col` are never read from the editor, so the Ln/Col status is always `1, 1`. |
| Save / Save As | `save_file()` always pops a file dialog even when `path` is already known; "Save" and "Save As" call the same function. |
| Undo / Redo | Menu items only feed the (broken) macro recorder; they do not trigger editor undo. |

---

## 5. Correctness bugs (prioritized)

**P0 — blocks use / claims false**
1. Likely does not compile (`ui.horizontal(ui, …)`, `toggle_sized`). **Verify with `cargo check`, fix APIs.**
2. Syntax highlighting is absent — the editor renders monochrome text despite the README. This is the headline IDE feature.
3. Auto-complete inserts at end-of-file instead of cursor.
4. Macros replay by appending the whole buffer; not real macros.

**P1 — feature is broken or misleading**
5. Bracket matching searches a static position and is never rendered.
6. Find/replace doesn't select, scroll, or step through matches; replace is all-or-nothing.
7. Code-folding logic is incorrect and O(n²) per frame.
8. Terminal `cd ~` doesn't expand to `$HOME`.
9. Save always shows a dialog; no real "Save" when path known.
10. Undo/Redo menu items don't do anything.
11. Cursor Ln/Col never tracked from the editor; Go-to-Line doesn't move the cursor.

**P2 — polish / architecture**
12. Symbol parse + fold parse run every frame (perf cliff on large files).
13. No persistence of settings/recents/theme/window.
14. "Open Folder" produces no file tree.
15. Plugins are inert.
16. Menu dropdown window is re-anchored to `ctx.cursor()` every frame → it jumps as the mouse moves.
17. `newfile` junk; no `.gitignore`, `LICENSE`, CI, tests.

---

## 6. Improvement plan

The existing `RUST_REBUILD_PLAN.md` is a fine *long-term* north star (multi-crate workspace). What's missing is a **grounded, sequenced plan for the current prototype**: make it real, then make it good, then grow it.

### Phase 0 — Make it actually build and run (1–2 days)
Goal: a green `cargo check`/`cargo run` you can trust.
- [ ] Run `cargo check`, fix every error. Known suspects: `ui.horizontal(ui, …)` → `ui.horizontal(|ui| …)`; `toggle_sized` → `ui.selectable_value` or `ui.toggle_value`.
- [ ] Run `cargo clippy` and `cargo fmt --all`; commit clean baseline.
- [ ] Add `.gitignore` (target/), `LICENSE` (GPL-2+), delete `newfile`.
- [ ] Add a minimal GitHub Actions workflow: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo build --release` on linux/macOS/windows.
- [ ] One smoke test: launch headless (eframe `App`) and assert the app constructs `GeanyApp::new`.

### Phase 1 — Make the headline features real (1–2 weeks)
Goal: an editor that actually edits code, not a monochrome text box.
- [ ] **Syntax highlighting.** Either adopt `egui_code_editor` (or `eui-code-editor`/`syntect`) and feed it `ThemeColors`, or write a small tokenizer that produces `egui::text::LayoutJob` with per-token colors and feed it to `TextEdit::multiline(..).layouter(..)`. Wire `SyntaxHighlight` keyword/type/builtin lists into the tokenizer. Apply all `ThemeColors` fields (keyword/string/comment/number/function/type).
- [ ] **Real cursor model.** Read cursor position from the `TextEdit` response (`response.cursor_range`), update `Document::cursor_line/col`. Make Go-to-Line scroll/select in the editor. Show live Ln/Col.
- [ ] **Completion at cursor.** Insert the chosen item at the cursor, replacing the partial word — not `push_str` at end. Trigger on `.` and identifier typing, not only `Ctrl+Space`.
- [ ] **Find/replace that works.** Highlight matches in the editor, jump to next/prev, replace-one + replace-all. Reuse the `LayoutJob` highlighter to color match ranges.
- [ ] **Bracket matching at cursor.** Call `BracketHighlight::find` with the real cursor char index; render matched pair via the highlighter (background or bold).
- [ ] **Working save.** If `path` is set, write without a dialog; "Save As" opens the dialog. Track `modified` accurately and reflect `●` on tabs.

### Phase 2 — Make the "power" features real (2–3 weeks)
- [ ] **Code folding, done right.** Parse with a real brace stack (handle `} else {`, strings, comments, nested levels). Compute the visual-line map **once** per content change (cache on `Document`), not per frame. Store `fold_state` per document.
- [ ] **Macros, done right.** Record keystrokes/edits (insert-at-pos, delete-range), not whole-buffer snapshots. Playback applies the recorded ops at the current cursor.
- [ ] **Symbol tree that's useful.** Cache parsed symbols per document, invalidate on edit (debounced). Clicking a symbol jumps to its line/col in the editor.
- [ ] **Project + file tree.** Build a recursive file tree from `base_path`, honor `file_patterns`, watch with `notify`, show in the sidebar Files tab. Reopen `recent_files` on project load.
- [ ] **Terminal fixes.** `cd ~` → expand via `dirs::home_dir()`. Don't `trim_start_matches("rm ")` (strips repeatedly); parse the arg. Keep an output stream, not a 200-line cap with silent drops.
- [ ] **Undo/redo** wired to the editor's undo stack (egui `TextEdit` has one; expose it or wrap the buffer in a history).

### Phase 3 — Architecture & quality (ongoing)
- [ ] **Split the monolith** along the lines `RUST_REBUILD_PLAN.md` already drew: `geany-core` (document, project, filetype, search, symbols, build, config) and `geany-ui` (panels, editor widget, themes). Keep the `geany` bin thin. Do this *after* features stabilize so the move is mechanical.
- [ ] **Performance.** Memoize per-frame: only recompute symbols/folds/highlight-layout on content change, keyed by a content hash. Avoid allocating a `Vec<VisualLine>` per line.
- [ ] **Persistence.** Save settings (theme, window, panel sizes, recents) to `dirs::config_dir()/geany-rs/settings.json`. Save/restore session.
- [ ] **Real plugin story.** Decide: in-process trait-object plugins (`dyn Plugin`) loaded from a plugins dir, or scripted. At minimum, make the existing "plugins" actually do something (e.g. AutoSave, TodoViewer) so the UI isn't a lie.
- [ ] **Tests.** Unit tests for the pure logic (`SymbolParser`, `FindReplace`, `FoldState`, `BuildSystem::expand`, `Filetype::from_ext`). Snapshot tests for the highlighter output. An integration test that drives the `eframe::App` headless.
- [ ] **i18n** later (`fluent-rs`), matching the original plan.
- [ ] **Release builds** with `lto=true`, `strip=true`, `codegen-units=1` (already partly set).

### Suggested immediate first PRs
1. `fix: make the app compile + clippy clean + add CI` (Phase 0).
2. `feat: real syntax highlighting via LayoutJob + ThemeColors` (Phase 1, the biggest credibility win).
3. `feat: cursor tracking + completion-at-cursor + go-to-line` (Phase 1).

---

## 7. TL;DR

The repository is a **single-file egui prototype that markets itself ("v0.6.0 — Final Polish Edition, COMPLETE IDE") but has never been compiled** — `cargo check` produces **25 errors** from a handful of repeated mistakes: `mods.cmd` (no such field; 8×), `?`-abuse inside egui closures (4×), a stray extra argument in `ui.horizontal(ui, …)` (1×), and several egui-0.32 API names that don't exist (`toggle_sized`, `request_focus`, `Context::cursor`, `RichText::bold`, `Window::always_auto_resize`, `Align2::LEFT_UP`, `Key::Shift`).

Beyond not building, the headline features are **absent or broken once it did parse**: syntax highlighting is *not implemented* (the editor is a monochrome `TextEdit`; `SyntaxHighlight` is dead code), auto-complete inserts at end-of-file instead of the cursor, macros replay by appending the whole buffer, bracket matching searches a static position and is never rendered, find/replace doesn't navigate, code-folding is incorrect and O(n²) per frame, and the "plugins" are inert checkboxes. The included `RUST_REBUILD_PLAN.md` is a reasonable long-term architecture but is disconnected from the actual code.

The fastest path to credibility is: **(0)** fix the ~25 mechanical compile errors and add CI (1–2 days), then **(1)** implement real syntax highlighting + a real cursor model + working completion — because right now the editor isn't actually an editor of code, it's a (non-compiling) text box.
---

# RustGeany — Анализ и план улучшений (перевод на русский)

**Репозиторий:** `minigulfayzullina-stack/RustGeany_version`
**Единственный коммит:** `f7986a6 — Geany-Rs v0.6.0 - COMPLETE IDE`
**Состояние на момент проверки:** 6 файлов, `geany-rs/src/main.rs` = 1010 строк, всё в одном файле.
Проверено реальным `cargo check` на rustc 1.98.1 (результаты в §3).

---

## 1. Что здесь на самом деле

| Путь | Что это |
|------|---------|
| `geany-rs/Cargo.toml` | eframe/egui 0.32 + serde/parking_lot/rfd/regex/whoami/dirs. Нет тестов, линтеров, файла LICENSE. |
| `geany-rs/src/main.rs` | Всё приложение в одном файле — типы, парсер, UI, запуск сборки, терминал. |
| `geany-rs/src/example.rs` | Демо-файл, загружаемый в документ №1 при старте через `include_str!`. |
| `geany-rs/README.md` | README в маркетинговом стиле со списком возможностей, большинство из которых не работает (см. §4). |
| `RUST_REBUILD_PLAN.md` | *Отдельный*, не связанный высокоуровневый план «переписать Geany с C на Rust» с multi-crate workspace. Описывает целевую архитектуру, которой реальный `main.rs` вообще не следует. |
| `newfile` | Мусорный файл, содержимое = `new`. Следует удалить. |

README называет это «v0.6.0 — Final Polish Edition» с длинным списком ✅ возможностей. Код не соответствует этому заявлению.

---

## 2. Оценка архитектуры

- **Монолит.** Один `main.rs` на 1010 строк. `RUST_REBUILD_PLAN.md` уже предлагает разумный 4-crate workspace (`geany-core`, `geany-ui`, `geany-plugin`, бинарь `geany`) — ничего из этого не существует.
- **Нет разделения ответственности.** UI, состояние, парсинг, файловый ввод-вывод, запуск процессов и «плагины» всё перемешано в реализации `eframe::App::update` и inline-модулях.
- **Неправильное использование immediate-mode.** Дорогие операции (регэксп-парсинг символов, пересчёт свёрток) выполняются внутри `update()` каждый кадр вместо кэширования/мемоизации.
- **Нет персистентности.** Тема, размер окна, недавние файлы, открытые документы, проект — всё теряется при выходе.
- **Нет тестов, нет CI, нет .gitignore, нет файла LICENSE** (README утверждает GPL-2+).
- **Расхождение плана и реальности.** `RUST_REBUILD_PLAN.md` рекомендует `egui_code_editor`/`syntect` для подсветки синтаксиса; код использует обычный `TextEdit` и ничего не раскрашивает.

---

## 3. Компилируется ли это? — НЕТ. (реальный `cargo check` на rustc 1.98.1)

**Не собирается.** `cargo check` падает с **25 ошибками компиляции** (после обхода фантомной «mismatched delimiter», которая каскадно возникает из первой реальной ошибки). Сообщение коммита «COMPLETE IDE» никогда не проходило через компилятор. Подтверждённые категории:

| Кол-во | Ошибка | Где / почему | Исправление |
|---|---|---|---|
| 8 | `E0609: no field 'cmd' on Modifiers` | каждый горячая клавиша использует `mods.cmd` (строки 773–780) | `mods.ctrl` — это кросс-платформенный Ctrl; `mods.command` — cmd на macOS. Используйте `mods.ctrl` (или `mods.ctrl && !mods.alt` и т.д.) |
| 4 | `E0277: оператор ? в замыкании/методе, не возвращающем Option/Result` | `self.active?` / `app.active?` внутри egui-замыканий и `update()` (строки 780, 783, 441, 811…) | `self.active.unwrap_or(0)` или корректный `if let Some(i) = self.active` — не используйте `?` здесь |
| 1 | `E0061: horizontal принимает 1 аргумент, передано 2` | `ui.horizontal(ui, |ui| { … })` (строка 910) | `ui.horizontal(|ui| { … })` |
| 1 | `E0277: ожидалось замыкание FnOnce(&mut Ui), найден &mut Ui` | тот же вызов на строке 910 (лишний аргумент `ui`) | то же исправление |
| 1 | `E0599: нет метода toggle_sized у &mut Ui` | иконки вкладок сайдбара (строка 820) | `ui.selectable_value(&mut sel, icon, …)` или `ui.toggle_value` |
| 1 | `E0599: нет варианта Key::Shift` | `mods.cmd && Shift && R` переключение макроса (строка 811) | проверяйте `mods.shift && key R` — `Key::Shift` не является вариантом `Key` в таком использовании |
| 1 | `E0599: нет метода request_focus у TextEdit` | поле ввода Go-to-Line (строка 1015) | `TextEdit::request_focus` не существует; используйте `ctx.memory_mut(|m| m.request_focus(id))` |
| 1 | `E0599: нет метода cursor у &Context` | позиционирование выпадающего меню (строка 796) | `ctx.pointer_position()` / `ctx.input(|i| i.pointer.latest_pos)` |
| 1 | `E0599: нет метода bold у RichText` | (строка 441) | `.strong()` |
| 1 | `E0599: always_auto_resize нет у Window` | всплывающее окно автодополнения (строка 954) | `.auto_sized()` |
| 1 | `E0599: нет константы LEFT_UP у Align2` | якорь всплывающего окна (строка 954) | `Align2::LEFT_TOP` |
| 1 | `E0277: [Document] нельзя индексировать Option<usize>` | `self.docs[i]` с `active: Option` напрямую (строка 848) | `self.docs[i.unwrap_or(0)]` или `if let Some` |
| 1 | `E0382: borrow of moved value 'pr'` | загрузка проекта (строка 677) | загрузить в переменную и клонировать перед перемещением |
| 1 | `E0277: &&Vec<MacroAction> не является итератором` | `MacroManager::execute` (строка 441) | `.iter()` / исправить borrow `saved.last().map(|m| &m.actions).unwrap_or(&vec![])` |
| 1 | `E0277: размер 'str' неизвестен` | путь возврата из macro execute (строка 441) | возвращать `String`, не висячий `&str` |
| 1 | `E0277` (несовпадение типа замыкания) | каскад из 2-аргументного `horizontal` | исправляется тем же фиксом строки 910 |

**Итог:** ~25 ошибок, большинство из горстки повторяющихся ошибок — неверное поле модификатора (`cmd`), злоупотребление `?`, один лишний аргумент и несколько несуществующих имён API egui 0.32 (`toggle_sized`, `request_focus`, `cursor`, `bold`, `always_auto_resize`, `Align2::LEFT_UP`, `Key::Shift`). Ни одна не является тонкой; все исправляются механически. Код закоммичен без единой компиляции.

Ранее заподозренные при grep-анализе (`ui.horizontal(ui, …)`, `toggle_sized`) — оба подтверждены как реальные ошибки.

---

## 4. Проверка возможностей функция за функцией

Список ✅ из README vs. что код делает на самом деле:

| Заявленная возможность | Реальность |
|---|---|
| ✅ Подсветка синтаксиса (Rust, C, C++, Python, JS, …) | **Не реализовано.** `SyntaxHighlight` определён (ключевые слова/типы/встроенные), но никогда не инстанцируется и не применяется. Редактор — обычный моноширинный `TextEdit`, весь текст одного цвета. Шесть тем меняют только фон, не цвета токенов. |
| ✅ Свёртка кода (кликабельный UI) | **Сломано.** `FoldState::parse` считает `{`/`}` по строке с `net = opens - closes`; строки с равным числом (`} else {`) отбрасываются, сопоставление скобок с регионами наивное. `get_visual_lines` пересчитывается **внутри** покадрового цикла рендера (O(n²) на кадр), а учёт скрытых строк (`hidden.saturating_sub(i)`) неверен. Маркеры +/- рендерятся, но текст скрывают ненадёжно. |
| ✅ Парные скобки (визуально) | **Не подключено.** `BracketHighlight::find(&d.content, d.content.len().min(100))` работает со *статической* позицией (конец буфера или символ 100), никогда с курсором, а результат `self.bracket` никогда не отрисовывается. |
| ✅ Автодополнение (ключевые слова + символы) | **Частично сломано.** Попап показывает элементы, но `Enter` делает `d.content.push_str(&t)` — вставляет сниппет в **конец файла**, а не на позицию курсора. |
| ✅ Поиск и замена | **Наполовину работает.** `search()` возвращает *индексы строк* и только логирует «Found N matches» — без выделения/прокрутки/подсветки. Замена только «Replace All»; нет next/replace-one. |
| ✅ Управление проектом (.geany файлы) | Загружает/сохраняет JSON `.geany` файл, но `recent_files` никогда не открываются повторно, а «Open Folder» создаёт заглушку проекта без дерева файлов. Вкладка «Files» в сайдбаре показывает *открытые документы*, а не файлы проекта. |
| ✅ Система плагинов | **Фикция.** Четыре захардкоженных `Plugin { enabled: bool }`. `enabled` ничего не переключает — ни загрузки, ни поведения. |
| ✅ Макросы (запись/воспроизведение) | **Сломано.** `record()` вызывается только при изменении буфера и сохраняет *весь новый буфер* как одно действие `Insert`; `execute()` дописывает весь захваченный буфер обратно. Макросы не воспроизводят правки, они дублируют документ. |
| ✅ Панель терминала | Работает приблизительно: проброс шелла + несколько встроенных команд. `cd ~` сломан — `replace('~', whoami::username())` даёт относительный путь `<username>/...` вместо `$HOME`. |
| ✅ Несколько тем | 6 тем, но меняются только bg/fg; текст редактора остаётся монохромным (см. подсветку). |
| ✅ Управление документами через вкладки | Работает. Индексация `close_doc` в порядке; пересчёт `active` корректен. |
| ✅ Кросс-платформенность | В основном; использует `#[cfg(windows)]` шеллы. Не проверено — пока даже не собирается. |
| ✅ Поддержка тёмной/светлой темы | Работает на уровне egui `Visuals`. |
| Горячие клавиши | `Ctrl+Space` для автодополнения использует `mods.cmd`, что на Linux/Windows — это Ctrl только через `mods.ctrl`; следует использовать `mods.ctrl` кросс-платформенно. Go-to-line устанавливает `cursor_line`, но не двигает реальный курсор `TextEdit`, а `cursor_line/col` никогда не считывается из редактора, так что статус Ln/Col всегда `1, 1`. |
| Сохранить / Сохранить как | `save_file()` всегда вызывает диалог даже если `path` уже известен; «Save» и «Save As» вызывают одну и ту же функцию. |
| Undo / Redo | Пункты меню только питают (сломанный) рекордер макросов; они не запускают undo редактора. |

---

## 5. Ошибки корректности (по приоритету)

**P0 — блокирует использование / заявления ложны**
1. Скорее всего не компилируется (`ui.horizontal(ui, …)`, `toggle_sized`). **Проверить через `cargo check`, исправить API.**
2. Подсветка синтаксиса отсутствует — редактор рендерит монохромный текст вопреки README. Это главная возможность IDE.
3. Автодополнение вставляет в конец файла вместо курсора.
4. Макросы воспроизводятся дописыванием всего буфера; это не настоящие макросы.

**P1 — возможность сломана или вводит в заблуждение**
5. Парные скобки ищутся по статической позиции и не отрисовываются.
6. Поиск/замена не выделяет, не прокручивает и не перебирает совпадения; замена — всё или ничего.
7. Логика свёртки кода неверна и O(n²) на кадр.
8. `cd ~` в терминале не раскрывается в `$HOME`.
9. Сохранение всегда показывает диалог; нет настоящего «Save» при известном пути.
10. Пункты меню Undo/Redo ничего не делают.
11. Ln/Col курсора никогда не отслеживается из редактора; Go-to-Line не двигает курсор.

**P2 — полировка / архитектура**
12. Парсинг символов и свёрток выполняется каждый кадр (провал производительности на больших файлах).
13. Нет сохранения настроек/недавних/темы/окна.
14. «Open Folder» не создаёт дерево файлов.
15. Плагины инертны.
16. Окно выпадающего меню каждый кадр перепривязывается к `ctx.cursor()` → прыгает при движении мыши.
17. Мусор `newfile`; нет `.gitignore`, `LICENSE`, CI, тестов.

---

## 6. План улучшений

Существующий `RUST_REBUILD_PLAN.md` — хороший *долгосрочный* ориентир (multi-crate workspace). Чего не хватает — **обоснованного, последовательного плана для текущего прототипа**: сделать его настоящим, затем сделать хорошим, затем развивать.

### Фаза 0 — Заставить собираться и запускаться (1–2 дня)
Цель: зелёный `cargo check`/`cargo run`, которому можно доверять.
- [ ] Запустить `cargo check`, исправить каждую ошибку. Известные подозреваемые: `ui.horizontal(ui, …)` → `ui.horizontal(|ui| …)`; `toggle_sized` → `ui.selectable_value` или `ui.toggle_value`.
- [ ] Запустить `cargo clippy` и `cargo fmt --all`; закоммитить чистую базу.
- [ ] Добавить `.gitignore` (target/), `LICENSE` (GPL-2+), удалить `newfile`.
- [ ] Добавить минимальный GitHub Actions workflow: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo build --release` на linux/macOS/windows.
- [ ] Один smoke-тест: запустить headless (eframe `App`) и проверить, что `GeanyApp::new` конструируется.

### Фаза 1 — Сделать главные возможности настоящими (1–2 недели)
Цель: редактор, который действительно редактирует код, а не монохромный текстбокс.
- [ ] **Подсветка синтаксиса.** Либо использовать `egui_code_editor` (или `eui-code-editor`/`syntect`) и подать в него `ThemeColors`, либо написать небольшой токенайзер, выдающий `egui::text::LayoutJob` с цветами по токенам, и подать в `TextEdit::multiline(..).layouter(..)`. Подключить списки ключевых слов/типов/встроенных из `SyntaxHighlight`. Применить все поля `ThemeColors` (keyword/string/comment/number/function/type).
- [ ] **Настоящая модель курсора.** Считывать позицию курсора из ответа `TextEdit` (`response.cursor_range`), обновлять `Document::cursor_line/col`. Заставить Go-to-Line прокручивать/выделять в редакторе. Показывать живые Ln/Col.
- [ ] **Автодополнение на курсоре.** Вставлять выбранный элемент на позиции курсора, заменяя частичное слово — не `push_str` в конец. Триггерить на `.` и наборе идентификатора, не только `Ctrl+Space`.
- [ ] **Рабочий поиск/замена.** Подсвечивать совпадения в редакторе, переход next/prev, replace-one + replace-all. Переиспользовать `LayoutJob`-хайлайтер для раскраски диапазонов совпадений.
- [ ] **Парные скобки на курсоре.** Вызывать `BracketHighlight::find` с реальным индексом символа курсора; отрисовывать пару через хайлайтер (фон или жирность).
- [ ] **Рабочее сохранение.** Если `path` задан — писать без диалога; «Save As» открывает диалог. Точно отслеживать `modified` и отражать `●` на вкладках.

### Фаза 2 — Сделать «продвинутые» возможности настоящими (2–3 недели)
- [ ] **Свёртка кода как надо.** Парсить через настоящий стек скобок (учитывать `} else {`, строки, комментарии, вложенные уровни). Вычислять карту визуальных строк **один раз** на изменение содержимого (кэш на `Document`), не каждый кадр. Хранить `fold_state` по документу.
- [ ] **Макросы как надо.** Записывать нажатия/правки (insert-at-pos, delete-range), а не снапшоты всего буфера. Воспроизведение применяет записанные операции на текущем курсоре.
- [ ] **Полезное дерево символов.** Кэшировать распарсенные символы по документу, инвалидировать при редактировании (с дебаунсом). Клик по символу переходит к его строке/колонке в редакторе.
- [ ] **Проект + дерево файлов.** Строить рекурсивное дерево файлов из `base_path`, учитывать `file_patterns`, наблюдать через `notify`, показывать во вкладке Files сайдбара. Открывать `recent_files` при загрузке проекта.
- [ ] **Исправления терминала.** `cd ~` → раскрывать через `dirs::home_dir()`. Не использовать `trim_start_matches("rm ")` (срезает повторно); парсить аргумент. Держать поток вывода, а не обрезку в 200 строк с тихими потерями.
- [ ] **Undo/redo** подключить к стеку undo редактора (у egui `TextEdit` он есть; exposed или обернуть буфер в history).

### Фаза 3 — Архитектура и качество (постоянно)
- [ ] **Разделить монолит** по линиям, которые уже намечены в `RUST_REBUILD_PLAN.md`: `geany-core` (document, project, filetype, search, symbols, build, config) и `geany-ui` (panels, editor widget, themes). Бинарь `geany` держать тонким. Делать *после* стабилизации возможностей, чтобы переезд был механическим.
- [ ] **Производительность.** Мемоизировать покадрово: пересчитывать symbols/folds/highlight-layout только при изменении содержимого, по хэшу. Избегать выделения `Vec<VisualLine>` на каждую строку.
- [ ] **Персистентность.** Сохранять настройки (тема, окно, размеры панелей, недавние) в `dirs::config_dir()/geany-rs/settings.json`. Сохранять/восстанавливать сессию.
- [ ] **Настоящая история плагинов.** Решить: in-process trait-object плагины (`dyn Plugin`) из директории plugins или скриптовые. Как минимум — заставить существующие «плагины» что-то делать (например AutoSave, TodoViewer), чтобы UI не врал.
- [ ] **Тесты.** Юнит-тесты для чистой логики (`SymbolParser`, `FindReplace`, `FoldState`, `BuildSystem::expand`, `Filetype::from_ext`). Снапшот-тесты вывода хайлайтера. Интеграционный тест, драйвящий `eframe::App` headless.
- [ ] **i18n** позже (`fluent-rs`), в соответствии с оригинальным планом.
- [ ] **Release-сборки** с `lto=true`, `strip=true`, `codegen-units=1` (частично уже задано).

### Предлагаемые первые PR
1. `fix: make the app compile + clippy clean + add CI` (Фаза 0).
2. `feat: real syntax highlighting via LayoutJob + ThemeColors` (Фаза 1, наибольший выигрыш в доверии).
3. `feat: cursor tracking + completion-at-cursor + go-to-line` (Фаза 1).

---

## 7. TL;DR

Репозиторий — **однофайловый egui-прототип, который подаёт себя («v0.6.0 — Final Polish Edition, COMPLETE IDE»), но никогда не компилировался** — `cargo check` выдаёт **25 ошибок** из горсти повторяющихся ошибок: `mods.cmd` (нет такого поля; 8×), злоупотребление `?` внутри egui-замыканий (4×), лишний аргумент в `ui.horizontal(ui, …)` (1×) и несколько несуществующих имён API egui 0.32 (`toggle_sized`, `request_focus`, `Context::cursor`, `RichText::bold`, `Window::always_auto_resize`, `Align2::LEFT_UP`, `Key::Shift`).

Помимо несобираемости, главные возможности **отсутствуют или сломаны, даже если бы код парсился**: подсветка синтаксиса *не реализована* (редактор — монохромный `TextEdit`; `SyntaxHighlight` — мёртвый код), автодополнение вставляет в конец файла вместо курсора, макросы воспроизводятся дописыванием всего буфера, парные скобки ищутся по статической позиции и не отрисовываются, поиск/замена не навигирует, свёртка кода неверна и O(n²) на кадр, а «плагины» — инертные чекбоксы. Прилагаемый `RUST_REBUILD_PLAN.md` — разумная долгосрочная архитектура, но оторвана от реального кода.

Самый быстрый путь к доверительности: **(0)** исправить ~25 механических ошибок компиляции и добавить CI (1–2 дня), затем **(1)** реализовать настоящую подсветку синтаксиса + настоящую модель курсора + рабочее автодополнение — потому что сейчас редактор не является редактором кода, это (несобирающийся) текстбокс.
