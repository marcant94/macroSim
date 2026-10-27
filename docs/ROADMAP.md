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

Implementado en `core/crime.rs`. La tasa (delitos por 1.000 hab y mes) tiende cada
mes hacia un objetivo compuesto por:

| Factor | Efecto |
|---|---|
| Paro por encima del estructural (6 %) | sube (hasta +15 puntos de tasa) |
| Exclusión social (clase baja insatisfecha) | sube (hasta +8) |
| Quiebra de la ciudad | sube (+6) |
| Cobertura de Seguridad | baja (hasta −35 % de la gravedad) |
| Cobertura de Educación | baja (hasta −15 %) |
| Cobertura de Cárceles | baja (hasta −10 %) |

Por debajo de la **tasa tolerable** (4) no hay presión social: una ciudad normal no
sufre por su criminalidad de fondo, así que la partida inicial parte de cero
consecuencias y solo empeorar cuesta. La presión (0..1) resta hasta 0,30 de
satisfacción a todas las clases, recorta hasta un 45 % la inversión privada y
expulsa hasta un 0,6 % de población al mes. Es un bucle cerrado: menos policía →
más crimen → menos satisfacción → se va la gente → la demanda por habitante baja y
la cobertura se recupera. El jugador decide dónde cortar ese círculo.

### 2.4 bis Deuda y quiebra (el sumidero de dinero)
La caja nunca queda en negativo: si el balance es deficitario, el déficit se
**financia emitiendo deuda**. El jugador puede además pedir préstamos y amortizar
manualmente desde el panel de deuda.

- **Límite de crédito**: dos años de recaudación (`CREDIT_YEARS`). Crece con la
  población y los ingresos, así que una ciudad rica endeuda más… y al perder
  población el límite baja.
- **Interés**: 0,35 %/mes de base más una prima de riesgo que sube con el cuadrado
  de la deuda sobre el límite (hasta +2 %/mes). Se paga cada mes como gasto fijo.
- **Quiebra** (`debt > límite`): sin crédito nuevo, la inversión privada se hunde,
  la delincuencia sube (+6 de tasa) y la población se va. No es un *game over*
  instantáneo: es una espiral de la que se sale subiendo impuestos y recortando
  gasto, o de la que no se sale.

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
La ciudad-estado arranca con **4.000 ha** y usa ~19 % (unos 760 ha) con los
servicios iniciales: el suelo es una restricción de medio plazo, no un muro del
primer año. Cuando el suelo se agota, tocará elegir: **reciclar** (demoler lo viejo,
  quizá con coste social), **densificar** (modernizar servicios para que rindan
  más en menos terreno) o **expandir** (ver Fase 3/4: conquista o compra de
  territorio).
- Este límite es el que da presión y significado a las decisiones: un hospital
  nuevo puede exigir tirar una comisaría o perder capacidad industrial.

---

## 3. Fase 2 — Mix energético (implementado)

`core/energy.rs`. Ocho fuentes, cada una con su balance declarado como datos en
`EnergySource::spec()`:

| Fuente | Instalación | Construir /MW | Mant. /MW·mes | Factor de carga | Suelo ha/MW | kg CO₂/MWh |
|---|---|---|---|---|---|---|
| ⛏ Carbón | 300 MW | 8.000 | 60 | 0,85 | 0,05 | 1.000 |
| ⛽ Petróleo | 150 MW | 5.500 | 100 | 0,90 | 0,04 | 750 |
| 🔥 Gas | 250 MW | 6.000 | 90 | 0,90 | 0,03 | 450 |
| ☢ Nuclear | 500 MW | 25.000 | 120 | 0,95 | 0,05 | 12 |
| 🌋 Geotérmica | 120 MW | 18.000 | 90 | 0,90 | 0,10 | 15 |
| 💨 Eólica | 100 MW | 12.000 | 40 | 0,35 | 0,60 | 0 |
| ☀ Solar | 80 MW | 8.500 | 35 | 0,25 | 0,90 | 0 |
| 💧 Hidráulica | 200 MW | 20.000 | 50 | 0,50 | 1,20 | 0 |

Se compra **por instalaciones completas**, no por MW sueltos: cada fuente tiene un
tamaño de planta propio (una nuclear no es del tamaño de un parque solar). La
pestaña se divide en dos tablas para no mezclar lo que tienes con lo que puedes
comprar:

1. **Instalaciones actuales**: cuántas plantas de cada fuente, *instalado /
   efectivo*, mantenimiento total, suelo total y emisiones totales, con un
   resumen al pie y un botón para desmantelar una planta de esa fuente.
2. **Construir instalaciones**: catálogo con el tamaño, la producción, el coste
   de construcción y el mantenimiento de **una** instalación, su suelo y sus
   emisiones. El detalle fino (capital por MW efectivo y MW por 1.000 ₿ de
   explotación) va en el tooltip del coste.

Claves del diseño:
- **Factor de carga = media anual.** Una eólica de 100 MW entrega 35 MW de media
  al año (35 %). Ese número es la referencia para comparar fuentes y calcular el
  capital por MW efectivo.
- **La producción varía cada mes.** `monthly_factor()` aplica estacionalidad
  (sol en verano, viento en invierno, hidráulica en primavera) más un ruido
  climático determinista de ±15 %; su media anual es 1.0, así que las
  comparaciones de la tabla siguen siendo válidas. Las **térmicas son
  gestionables** y no varían nunca: producen lo que se les pide.
  Consecuencia de juego: un mix 100 % solar cubre de sobra en julio y se queda
  corto en enero, y el jugador lo ve en el balance del mes.
- **Sin ciclo día/noche**: el tick es mensual, así que el 25 % de la solar ya
  incluye las noches. Modelar horas sueltas exigiría un tick sub-mensual.
- **Métrica de eficiencia**: *MW efectivos por cada 1.000 ₿ de explotación*
  (mantenimiento + coste de emisiones). Orden resultante: hidráulica 10,0 >
  geotérmica 9,84 > eólica 8,75 > nuclear 7,84 > solar 7,14 > gas 6,70 >
  carbón 5,55 > petróleo 5,18. El petróleo es el más barato de construir por MW efectivo y el más caro
  de explotar: la trampa clásica del combustible fósil barato. Va en el tooltip
  del coste de construcción, no en la tabla: es información de segundo nivel.
- **Compromiso**: los fósiles son baratísimos de construir (gas 6.667 ₿ por MW
  efectivo frente a 40.000 de la hidráulica) pero caros de explotar y sucios;
  las renovables al revés y además comen suelo.
- **Emisiones con precio**: 0,15 ₿ por tonelada como coste sanitario, que entra en
  el presupuesto mensual. Así el carbón no es gratis aunque su mantenimiento lo
  parezca.
- **La demanda sigue a la población** (`MW_PER_CAPITA`), no crece sola: si la
  ciudad crece hay que instalar plantas o el déficit castiga el empleo. Ya no hay
  slider de capacidad.
- La tabla distingue **cobertura** (demanda atendida) de **red usada**
  (utilización de la capacidad): antes la cobertura estaba invertida.

Pendiente de esta fase: precio de la energía como ingreso por exportación y
embalses/sequías (años secos que recorten la hidráulica más de lo estacional).

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
├── population.rs     # Migración mensual (servicios, empleo, impuestos, crimen)
├── crime.rs          # Delincuencia emergente y su presión social
├── economy.rs        # Presupuesto: impuestos, deuda, intereses y quiebra
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
servicios (privado crece + gasto) → demografía (clases derivan) → población
(migración) → delincuencia → economía (ingresos, intereses y deuda) → energía →
calendario → histórico.

## 6. Prioridad de implementación sugerida

1. ✅ **Hecho** — Impuestos por tramos + indicadores de fuga/inversión privada (y balance de ingresos/gastos recalibrado).
2. ✅ **Hecho** — UI del panel de servicios: construir/demoler unidades (respetando `LandState::can_build` y el presupuesto), ver cobertura pública/privada y hectáreas libres.
3. ✅ **Hecho** — Población dinámica ligada a servicios + empleo (migración mensual con desglose visible).
4. ✅ **Hecho** — Delincuencia + seguridad como indicador emergente (con efectos en satisfacción, inversión y población).
5. Basuras/reciclaje y bomberos.
6. Carreteras: construir vs mantener vs privatizar.
7. ✅ **Hecho** — Mix energético: seis fuentes con capital, mantenimiento, factor de carga, suelo y emisiones.
8. ✅ **Hecho** — Sumidero de dinero: deuda con intereses, préstamos y quiebra.
9. **Gasto por habitante automático**: el jugador no debería fijar el gasto por
   habitante con un slider. Debe **calcularse** a partir de lo que cuesta mantener
   el nivel de servicios que ha construido (calidad de vida objetivo), y el jugador
   decide vía servicios e impuestos, no vía una cifra abstracta.
10. **Dificultad de partida** (fácil / normal / difícil): se elige al empezar y
    modula el punto de partida (caja inicial, deuda permitida, presión fiscal
    neutra, tolerancia social ante la delincuencia) y el margen del gasto
    autocalculado del punto 9. Debe guardarse en `Settings`.
11. **Contaminación → salud**: hoy las emisiones del mix eléctrico solo cuestan
    dinero (coste sanitario por tonelada). Deben además **encarecer y tensionar
    la sanidad**: la demanda del servicio de Salud subiría con las emisiones, así
    que la misma cobertura daría peor satisfacción (sobre todo a la clase baja,
    que es la que vive al lado de las centrales) y subiría la delincuencia por
    exclusión. Diseño previsto: `ServicesState::demand(kind, population)` pasa a
    aceptar un multiplicador de tensión sanitaria que calcula `core/energy.rs`,
    igual que ya se hace con `crime_pressure` en la satisfacción.
12. **Estado de fin de partida**: hoy la quiebra es una espiral sin final explícito;
    falta un cierre (rescate, moción de censura o derrota) con resumen.
13. **Empleo endógeno**: hoy el paro solo depende del déficit eléctrico; debería
    depender de la inversión privada (que ya existe) y del sector servicios.
14. Todo lo demás (leyes, ejército, comercio) → más adelante.
