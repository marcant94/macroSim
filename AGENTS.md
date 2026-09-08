# AGENTS.md - Context & Rules for AI Assistants

## Project Overview
This project is a high-performance **Geopolitical & Economic Management Simulator** built with **Rust** and **Bevy Engine**. 
It features a data-driven simulation of tax policies, energy grids, labor markets, and macroeconomics. There are no heavy 3D visuals; the core focus is UI/data visualization, system loops, and deep simulation.

---

## Tech Stack & Core Dependencies
- **Language:** Rust (Edition 2021)
- **Engine:** Bevy (v0.14+)
- **UI System:** Bevy UI / `bevy_egui` (for dense dashboard interfaces)
- **Localization:** `fluent` or JSON-based key-value mapping
- **Data/State:** `serde` for serialization/deserialization

---

## Coding Rules & Conventions

### 1. Rust & Bevy Best Practices
- **Strict ECS separation:** 
  - Keep **Components** purely data (`struct`).
  - Keep **Systems** strictly behavior/logic (`fn system(...)`).
  - Keep **Resources** for global state (e.g., `GlobalEconomy`, `EnergyGrid`, `Calendar`).
- **Memory & Borrowing:** Avoid heavy usage of `Rc<RefCell<T>>` or unnecessary `Arc<Mutex<T>>`. Trust Bevy's ECS scheduling to pass resources/queries mutably when needed.
- **Error Handling:** Avoid `.unwrap()` or `.expect()` in production systems. Use `Result` or handle fallback states gracefully within system queries.
- **Idiomatic Rust:** Follow `clippy` suggestions. Write clear, expressive code without over-engineering abstractions.

### 2. Project Architecture Pattern
Organize code using Bevy Plugins:
```text
src/
├── main.rs                  # Entry point & App Builder
├── core/
│   ├── mod.rs               # Core plugin declaration
│   ├── economy.rs           # Macroeconomics simulation logic & math
│   ├── energy.rs            # Power grid generation & consumption logic
│   └── time.rs              # Game clock / calendar ticker system
├── ui/
│   ├── mod.rs               # UI layout plugin
│   ├── dashboard.rs         # Main metrics dashboard
│   └── views/               # Taxes, Energy, and Laws sub-windows
└── i18n/
    └── mod.rs               # Localization system manager