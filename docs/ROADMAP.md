# 🎯 Visión de Diseño — MacroSim

> Este documento define la visión de largo plazo del juego. Cualquier cambio de diseño
> nuevo debe reflejarse aquí para no perder el rumbo.

## 1. Concepto central

No gobiernas solo "un país de laboratorio": gobiernas una **ciudad-estado** al estilo de
las antiguas polis griegas (o una ciudad moderna, según la ronda). Esto combina los
problemas y decisiones de un **presidente** y de un **alcalde**, como en los modos de
gestión de SimCity o Cities: Skylines, pero sin mapa ni edificación: todo es
*dashboard* (sliders, gráficas, informes, presupuestos).

**La regla de oro del juego:**
Si haces las cosas bien (impuestos razonables, economía sana, servicios que cubren lo
que la población necesita) → **la población crece** y tienes más ingresos.
Si lo haces mal → la gente se va, el desempleo y la delincuencia suben, y acabas en
quiebra o en crisis.

La simulación debe ser un **sistema emergente**: los indicadores (población, delincuencia,
salud, educación…) se derivan de las decisiones del jugador, no de guiones fijos.

---

## 2. Fase 1 — "Modo Alcalde" (núcleo del juego)

Todo lo público que construyes/amplías **aumenta también el gasto mensual fijo**
(salarios, mantenimiento): cada decisión tiene coste recurrente, no solo coste puntual.

### 2.1 Impuestos
- **Impuestos generales** (IVA / consumo).
- **Impuesto de sociedades** (empresas).
- **IRPF por tramos separados**: clase baja, media y alta, cada uno con su slider.
  Subir el tramo alto da ingresos pero fuga capital; subir el bajo castiga el consumo.
- Presión fiscal total y tipo efectivo mostrados en el panel económico.

**Implementado** (`core/economy.rs`): dos bases imponibles declaradas como datos —
renta de los hogares (30/hab·mes, con peso 0,5 / 1,0 / 2,5 según la clase) y
beneficio empresarial (10/hab·mes). El **tipo efectivo** es la recaudación por
habitante sobre la renta bruta (23,8 % con los impuestos de partida) y se muestra
en el panel y en el informe económico junto a ingresos, gastos y balance.
El **tramo alto** por encima del 35 % genera *fuga de capital* (`capital_flight`,
0..1) que (a) recorta la recaudación efectiva de esa clase, (b) penaliza la
inversión privada y (c) reduce el peso de la clase alta mes a mes. Pagar por
encima del tipo neutro resta satisfacción a cada clase (la baja es la más
sensible), lo que se traduce en menos población. Por debajo del neutro no hay
"satisfacción extra": el incentivo es la migración, no la felicidad instantánea.

### 2.2 Inversión privada vs. pública (decisiones de "cómo")
Para cada servicio hay que decidir **quién lo presta**:
- **Público:** tú construyes y mantienes → control total, más gasto mensual.
- **Privado:** bajas impuestos / das facilidades legales y el sector privado invierte
  y crece la oferta → menos gasto público, pero menos control (calidad, precios) y
  se mide con estadísticas (p. ej. cobertura privada vs pública).
- **Privatización** de lo ya público (p. ej. vender carreteras) → ingreso puntual
  pero pérdida de control a largo plazo.

**Detalle de diseño importante (a diferencia de SimCity/Cities: Skylines):**
- Aquí **SÍ existe sanidad y educación privadas**. El sector privado absorbe parte
  de la demanda (sobre todo de clases media y alta), así que el jugador no tiene
  que hacerlo todo con dinero público: un buen clima de inversión privada cubre
  huecos sin gastar.
- La cobertura **privada** satisface especialmente a las **clases media y alta**
  (pagan por calidad), mientras que la **pública** es la que sostiene a la clase
  baja. El descontento de cada clase depende de la cobertura que recibe de su
  tipo de servicio: esto añade un matiz estratégico (puedes ser un estado
  "liberal" con poca sanidad pública y buena privada… siempre que la clase
  baja no acabe revuelta o fugándose).
- El crecimiento del sector privado no es mágico: depende de impuestos,
  economía, empleo cualificado y demanda insatisfecha del servicio.

### 2.3 Servicios públicos (construcción + mantenimiento)
Cada servicio se gestiona como: **capacidad instalada (unidades)** vs
**demanda de la población**, con indicadores de cobertura en las estadísticas.

Construir una unidad cuesta **capital al contado** (≈40 meses de mantenimiento) y
consume **suelo**; el **mantenimiento mensual** es recurrente. Demoler recupera el
50 % del capital y libera las hectáreas, así que reciclar es siempre más barato que
ampliar cuando el suelo aprieta.

| Servicio | Indicador de necesidad | Efecto si faltan |
|---|---|---|
| **Sanidad**: clínicas, hospitales | salas/demandantes, mortalidad, salud media | baja esperanza de vida, fuga de población |
| **Educación**: colegios, institutos, universidades | plazas/niños, nivel educativo medio | menos cualificación → menos inversión privada y empleo cualificado |
| **Seguridad**: comisarías, policía, vehículos patrulla | dotación por habitante | **sube la delincuencia** |
| **Bomberos**: parques, camiones | tiempo de respuesta | riesgo de incendios/catastrofes no cubiertas |
| **Cárceles** | plazas/preso | presos en la calle → criminalidad persistente |
| **Basuras / reciclaje** | recogidas/semana, % reciclado | insalubridad, enfermedad, descontento |
| **Carreteras** | km de red, estado medio | construir vs **mantener/reparar** (¡mantener es más barato que construir!) |
| **Transportes** | red de transporte público | congestión, contaminación, descontento |

### 2.4 Delincuencia (indicador emergente)
La criminalidad **no es un slider del jugador**: sube si fallan empleo, servicios,
educación y seguridad; baja si aumentas policía (con coste) y mejoras las condiciones
sociales. Sirve de termómetro de "si haces las cosas mal".

### 2.5 Población (el marcador del juego)
Crecimiento/descenso mensual en función de:
- empleo (desempleo) y economía,
- cobertura de sanidad, educación, seguridad y basuras,
- presión fiscal comparada (fuga/atracción de residentes),
- vivienda y coste de vida (futuro cercano).

Implementado en `core/population.rs`: cada mes se aplica una **tasa neta de
migración** compuesta por tres términos (servicios → satisfacción media de la
población, empleo → desempleo frente al neutro y fiscalidad → presión fiscal
frente a la de referencia), acotada a ±2 % / +1,5 % mensual para que ningún
factor genere espirales. Los puntos neutros están calibrados para que la partida
inicial quede en equilibrio: **lo que mueve la población es lo que hace el
jugador**, no el paso del tiempo.

### 2.6 Límite de suelo (hectáreas)
Como en los juegos de ciudad, **no se puede construir infinitamente**: la
-ciudad-estado tiene un área total limitada en **hectáreas**.
- Cada servicio, unidad o ampliación consume suelo (hospitales, parques de
  bomberos, vertederos de basuras, km de carreteras, industria, energía…).
- El suelo restante es un indicador visible en la barra superior/panel.
- Cuando el suelo se agota, tocará elegir: **reciclar** (demoler lo viejo,
  quizá con coste social), **densificar** (modernizar servicios para que rindan
  más en menos terreno) o **expandir** (ver Fase 3/4: conquista o compra de
  territorio).
- Este límite es el que da presión y significado a las decisiones: un hospital
  nuevo puede exigir tirar una comisaría o perder capacidad industrial.

---

## 3. Fase 2 — Energía avanzada (ya existe la base)

Ya hay capacidad, demanda y precio. El siguiente paso es el **mix energético**:
- **Renovables:** inversión inicial (capital) alta y producción **intermitente/no
  estable**, pero combustible gratis.
- **Fósiles:** más baratas de instalar y estables, pero con **coste de combustible
  recurrente** que a la larga las hace más caras, además de contaminación
  (salud, descontento, reputación).
- El jugador elige su mix y asume el compromiso capital vs coste operativo.

---

## 4. Fase 3+ — "Modo Presidente" (versiones futuras, muy a largo plazo)

Con el modo alcalde ya es más que suficiente para empezar. Para versiones posteriores:

- **Leyes** (cambiar/aprobar normas: laboral, fiscal, sociales).
- **Ejército / defensa** (presupuesto, reclutamiento).
- **Comercio exterior**: importaciones y exportaciones, aranceles.
- **Diplomacia y relaciones exteriores**.
- **Expansión territorial (Fase 3/4):** conquista (si tienes más/mejor ejército
  que el vecino) o compra/tratado de territorio → aumenta el límite de hectáreas
  y la población gobernable. El suelo escaso de la Fase 1 encuentra aquí su
  válvula de escape natural.

---

## 5. Arquitectura técnica del esqueleto (implementada)

El núcleo ya tiene los cimientos pensados para crecer sin refactorizar:

```text
src/core/
├── services.rs       # Registro genérico de servicios (el patrón clave)
├── land.rs           # Límite de hectáreas y consumo de suelo
├── demographics.rs   # Clases sociales, satisfacción y movilidad
├── population.rs     # Migración mensual (servicios, empleo e impuestos)
├── economy.rs        # Presupuesto (incluye gasto recurrente de servicios)
├── energy.rs         # Red eléctrica
├── settings.rs       # Idioma/moneda
└── time.rs           # Calendario
```

**Patrón de extensión (lo importante):** añadir un servicio nuevo son solo 2 pasos:
1. Una variante nueva en `ServiceKind`.
2. Su fila de balance en `ServiceKind::spec()` (demanda/1000 hab, coste mensual,
   hectáreas por unidad, propensión a inversión privada).

Cobertura, gasto recurrente, crecimiento del sector privado, consumo de suelo y
satisfacción por clase se calculan en **bucles genéricos** sobre `ServiceKind::ALL`:
ninguna otra parte del código necesita conocer los servicios uno a uno. Lo mismo
se replicará en el futuro con edificios/industria.

Orden del tick mensual (`Simulation::tick_month`):
servicios (privado crece + gasto) → demografía (clases derivan) → economía →
energía → calendario → histórico.

## 6. Prioridad de implementación sugerida

1. ✅ **Hecho** — Impuestos por tramos + indicadores de fuga/inversión privada (y balance de ingresos/gastos recalibrado).
2. ✅ **Hecho** — UI del panel de servicios: construir/demoler unidades (respetando `LandState::can_build` y el presupuesto), ver cobertura pública/privada y hectáreas libres.
3. ✅ **Hecho** — Población dinámica ligada a servicios + empleo (migración mensual con desglose visible).
4. Delincuencia + seguridad (comisarías, policía).
5. Basuras/reciclaje y bomberos.
6. Carreteras: construir vs mantener vs privatizar.
7. Mix energético renovable vs fósil.
8. Todo lo demás (leyes, ejército, comercio) → más adelante.
