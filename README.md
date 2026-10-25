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

### Compilar para Web (WASM)
La misma aplicación se puede ejecutar en el navegador (WebGL). Requiere una
vez:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
```

Y después, para generar `web/pkg/` y servirlo:

```bash
./local-compilar-wasm.sh
python3 -m http.server -d web 8080   # abrir http://localhost:8080
```

El HTML de entrada es [`web/index.html`](web/index.html). El código de
simulación y la UI son los mismos que en escritorio; solo cambia el arranque en
[`src/main.rs`](src/main.rs) y las fuentes de sistema (en web se embeben dos
subsets mínimos en `assets/fonts/`).

### Publicación automática (GitHub Actions)

[`.github/workflows/release.yml`](.github/workflows/release.yml) se ejecuta
**solo al hacer push a la rama `prod`** (o a mano desde *Actions → Release → Run
workflow*). En cada ejecución:

1. Compila la versión web y la publica en **GitHub Pages**.
2. Compila los binarios de **Linux x86_64** y **Windows x86_64**.
3. Crea un *release* con tag `yyyyMMdd-HHmm` (UTC, p. ej. `20251025-1345`) con
   `macrosim-linux-x86_64-<tag>.tar.gz`, `macrosim-windows-x86_64-<tag>.zip`,
   `macrosim-web-<tag>.zip` y `SHA256SUMS.txt`.
4. Borra las releases antiguas y conserva solo las **10** últimas
   (`KEEP_RELEASES` en el propio workflow).

Configuración inicial (una sola vez):

1. Crear la rama de publicación: `git push origin main:prod`
   (el workflow solo se ejecuta donde exista el fichero).
2. **Settings → Pages → Build and deployment → Source: GitHub Actions**.
   Sin esto el job `GitHub Pages` falla (el release no: son jobs independientes).
   No se puede automatizar: la opción `enablement` de `configure-pages` exige un
   token con permisos de administración, no el `GITHUB_TOKEN` del workflow.
3. Solo si el despliegue falla con *"not allowed to deploy to github-pages due to
   environment protection rules"*: **Settings → Environments → github-pages →
   Deployment branches** → añadir `prod`. Pasa cuando el entorno ya existía con
   una regla de rama; si lo crea el propio workflow, nace sin restricciones.

Flujo recomendado: desarrollar y probar en `main`; cuando haya una versión
publicable, `git push origin main:prod`.

`prod` es una rama normal en el remoto y **no hace falta tenerla en local**:
`main:prod` significa «sube mi `main` a la rama `prod` de `origin`», y eso dispara
el workflow igual que un push normal. Dos avisos:

- **No uses `-u`** en ese comando: `-u` dejaría tu `main` local *siguiendo* a
  `origin/prod` (`git status` compararía contra `prod`), y según tu
  `push.default` un `git push` sin argumentos acabaría publicando en `prod` o
  fallando con un error confuso.
- Si `prod` ya apunta a ese mismo commit, Git responde *"Everything up-to-date"* y
  **no** se ejecuta nada. Para republicar el mismo commit, lánzalo a mano desde
  *Actions → Release → Run workflow*.

Un tag no serviría como disparador: el workflow escucha `push` a la **rama**
`prod` (`on.push.branches`), que además avanza sola con cada publicación; para
desplegar dos commits distintos necesitarías un tag nuevo cada vez.

Sobre el almacenamiento: en repositorios públicos Actions y Releases no se
cobran, y no hay límite de tamaño total (solo 2 GiB por fichero y 1000 assets por
release). Aun así se limita a 10 para no acumular releases eternas.

El binario de Linux se compila en Ubuntu 22.04 (glibc ≥ 2.35) y necesita drivers
OpenGL; X11/Wayland se cargan en tiempo de ejecución (`dlopen`), así que no hacen
falta paquetes `-dev` para usarlo.

---

## ©️ Licencia

**Todos los derechos reservados** (© 2026 Mario Cantelar) — ver el fichero
[`LICENSE`](LICENSE).

Este repositorio es público para mostrar el proyecto y compartir las ideas de
diseño, pero **no se concede licencia** para copiar, modificar, distribuir o
usar el código o sus contenidos. El juego está destinado a un lanzamiento
comercial futuro; el código aquí publicado no puede usarse como base de un
producto propio, ni total ni parcialmente.

Si quieres aprender del enfoque (simulación macroeconómica, servicios con
retroalimentación, etc.), las preguntas y discusiones por GitHub Issues son
bienvenidas.