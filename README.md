# 🏛️ MacroSim (Nombre Provisional)

Un simulador de políticas públicas, gestión macroeconómica y redes de energía desarrollado en **Rust**, con una interfaz de escritorio construida con **egui / eframe**.

El juego es un *dashboard* de gestión puro, al estilo de los informes y paneles fiscales de SimCity o Cities: Skylines, pero **sin mapa ni edificación**: todo se gestiona con sliders, gráficas e informes.

---

## 🚀 Características Principales

* **Modelo Macroeconómico:** Presupuesto, impuestos (sociedades e IRPF), gasto público por habitante y desempleo con retroalimentación mensual.
* **Red y Mix Energético:** Capacidad instalada, demanda y precio de la energía; el déficit energético afecta al empleo.
* **Toma de Decisiones:** Subir/bajar impuestos y gastos en tiempo real y ver las consecuencias en las estadísticas.
* **Informes y Estadísticas:** Pestañas de Resumen, Economía, Energía e Informes (tabla de los últimos 12 meses y gráficas de evolución).
* **Ligero y Rápido:** `eframe`/`egui` en lugar de un motor 3D completo; compila en minutos y el binario release pesa ~10 MB.

---

## 🧱 Estructura del Proyecto

```text
src/
├── main.rs      # Entrada: ventana nativa eframe
├── core/        # Simulación: economía, energía, calendario e historial
└── ui/          # Dashboard: barra superior, panel de políticas y pestañas de informes
```

---

## 🛠️ Requisitos e Instalación

### Requisitos Previos
* [Rust](https://www.rust-lang.org/) (Edición 2021 o superior)
* `cargo` (incluido con la instalación estándar de Rust)
* En Linux, solo las librerías de sistema de siempre para GTK/Wayland/X11 (`build-essential`, `libwayland-dev`, etc.). No se necesitan librerías de audio ni Vulkan.

### Ejecutar en Desarrollo
Compila y ejecuta el proyecto:

```bash
cargo run
```