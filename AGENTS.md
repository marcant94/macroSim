# AGENTS.md - Context & Rules for AI Assistants

## Project Overview
This project is a **Geopolitical & Economic Management Simulator** built with **Rust** and a lightweight desktop GUI using **eframe / egui**.
It features a data-driven simulation of tax policies, energy grids, labor markets, and macroeconomics. There is **no map, no sprites and no 3D visuals**: the game is a pure management dashboard (in the spirit of SimCity / Cities: Skylines reports), focused on UI/data visualization, statistics and deep simulation.

> **Important decision:** Bevy was rejected on purpose. A full game engine (2-4 GB of dependencies, 5-8 GB of `target/`) is overkill for a dashboard-only game. Do **not** reintroduce Bevy or any ECS unless explicitly asked.

---

## Tech Stack & Core Dependencies
- **Language:** Rust (Edition 2021)
- **GUI Framework:** `eframe` + `egui` (v0.32) — immediate-mode desktop UI
- **Charts:** Hand-drawn with egui painters (no extra plotting crate; if adding one later, keep versions in sync with `egui` to avoid duplicate-crate conflicts)
- **Data/State:** `serde` + `serde_json` for serialization/deserialization
- **Localization (planned):** `fluent` or JSON key-value mapping

---

## Coding Rules & Conventions

### 1. Rust & eframe/egui Best Practices
- **Separation of concerns:**
  - `src/core/` holds the simulation as plain data structs + logic methods (`Simulation`, `EconomyState`, `EnergyState`, `History`). It must NOT depend on `egui`.
  - `src/ui/` holds all drawing (`eframe::App` impl, panels, tabs). It must NOT mutate simulation rules; only expose controls (sliders/buttons) that call core methods.
- **Update loop:** Advance simulation in `update()` using `dt` with a fixed monthly tick (`Simulation::tick_month()`), never run logic inside draw closures.
- **Memory & Borrowing:** Avoid `Rc<RefCell<T>>` and `Arc<Mutex<T>>`; `eframe` owns the app state and egui is immediate-mode, so plain `&mut self` is enough.
- **Error Handling:** Avoid `.unwrap()` or `.expect()` in production code. Use `Result` or graceful fallbacks.
- **Idiomatic Rust:** Follow `clippy` suggestions. Avoid over-engineering; the UI is immediate-mode — prefer simple labels/grids over custom widgets unless needed.

### 2. Project Architecture Pattern
Organize code by module (simulation vs UI):
```text
src/
├── main.rs                  # Entry point: native window via eframe::run_native
├── core/
│   ├── mod.rs               # Simulation state, monthly tick, history buffers
│   ├── economy.rs           # Macroeconomics simulation logic & math (future split)
│   ├── energy.rs            # Power grid generation & consumption logic (future split)
│   └── time.rs              # Game clock / calendar ticker (future split)
├── ui/
│   ├── mod.rs               # MacroSimApp: top bar, side panel, tabbed reports
│   └── views/               # Taxes, Energy, and Laws sub-views (future split)
└── i18n/
    └── mod.rs               # Localization system manager (planned)
```

### 3. Build & Verify
- Dev run: `cargo run` (fast; ~1-2 GB `target/`, vs 5-8 GB with Bevy)
- Release: `cargo run --release` (`lto = true`, `strip = true`, binary ~10 MB)
- Always run `cargo check` (and `clippy`) before finishing changes.