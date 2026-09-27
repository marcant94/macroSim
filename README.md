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

## 🎯 Visión a Largo Plazo (Roadmap)

Gobiernas una **ciudad-estado** (como las antiguas polis griegas): eres presidente **y** alcalde a la vez, con los problemas y decisiones de ambos, al estilo de los modos de gestión de SimCity o Cities: Skylines (sin mapa: todo por sliders, gráficas e informes).

Ideas clave que guiarán el desarrollo:

* **Población como marcador:** si haces las cosas bien (impuestos razonables, servicios cubiertos, empleo sano) la población crece; si no, se va y quedas en crisis.
* **Servicios públicos:** sanidad (clínicas, hospitales), educación (colegios, institutos, universidades), seguridad (comisarías, policía), bomberos, cárceles, basuras/reciclaje y carreteras/transportes. Construirlos **y mantenerlos** (gasto mensual recurrente).
* **Público vs privado:** bajas impuestos/favoreces leyes para atraer inversión privada, o prestas el servicio públicamente; puedes privatizar a cambio de ingresos y pérdida de control.
* **Delincuencia emergente:** sube si fallan empleo, servicios y educación; la combatirás con policía y cárceles… o mejorando la sociedad.
* **Mix energético:** renovables (caras de instalar, intermitentes) vs fósiles (baratas de instalar pero caras de operar a la larga).

En versiones futuras y muy lejanas: leyes, ejército, comercio, importaciones/exportaciones (modo presidente). Ver `docs/ROADMAP.md` para el detalle y la prioridad de implementación.

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