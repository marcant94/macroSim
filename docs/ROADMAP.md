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
**demanda de la población**, con indicadores de cobertura en las estadísticas:

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

1. Impuestos por tramos + indicadores de fuga/inversión privada.
2. UI del panel de servicios: construir unidades (respetando `LandState::can_build`), ver cobertura pública/privada y hectáreas libres.
3. Población dinámica ligada a servicios + empleo.
4. Delincuencia + seguridad (comisarías, policía).
5. Basuras/reciclaje y bomberos.
6. Carreteras: construir vs mantener vs privatizar.
7. Mix energético renovable vs fósil.
8. Todo lo demás (leyes, ejército, comercio) → más adelante.
