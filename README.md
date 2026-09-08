# 🏛️ MacroSim (Nombre Provisional)

Un simulador de políticas públicas, gestión macroeconómica y redes de energía desarrollado en **Rust** usando **Bevy Engine**.

El proyecto prescinde de gráficos 3D pesados para enfocarse por completo en un modelo de simulación profundo, datos en tiempo real, informes financieros e interacciones tipo *dashboard*.

---

## 🚀 Características Principales

* **Modelo Macroeconómico:** Bucles de retroalimentación en tiempo real entre impuestos, empleo, consumo, inflación y gasto público.
* **Red y Mix Energético:** Planificación y construcción de centrales (Nuclear, Gas/Carbón, Solar, Eólica) para satisfacer la demanda de la población y la industria.
* **Toma de Decisiones y Leyes:** Ajuste de pensiones, reformas laborales y presión fiscal con consecuencias dinámicas.
* **Alta Eficiencia (ECS):** Desarrollado sobre la arquitectura Entity Component System de Bevy para garantizar un rendimiento óptimo y bajo consumo de memoria.

---

## 🛠️ Requisitos e Instalación

### Requisitos Previos
* [Rust](https://www.rust-lang.org/) (Edición 2021 o superior)
* `cargo` (incluido con la instalación estándar de Rust)

### Ejecutar en Desarrollo
Clona el repositorio e inicia el proyecto con compilación rápida:

```bash
git clone [https://github.com/tu-usuario/tu-repositorio.git](https://github.com/tu-usuario/tu-repositorio.git)
cd tu-repositorio
cargo run