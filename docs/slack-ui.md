# La UI de Slack (referencia para el clon)

Relevamiento del cliente web oficial (`app.slack.com`, Chrome 150) hecho el
2026-10-21 por CDP sobre una sesión real, en un workspace con plan Pro, interfaz en
inglés y ventana de **1276 × 955** px a escala 1. Las medidas salen del DOM
(`getBoundingClientRect` y `getComputedStyle`), no de capturas a ojo.

- Las capturas están en `docs/capturas/` (no van al repo porque tienen datos reales
  del workspace; ver `.gitignore`). Se citan como `capturas/NN-nombre.png`.
- Los tokens de color completos del tema están en
  [`datos/slack-css-tokens.json`](datos/slack-css-tokens.json).
- Lo que pasa por la red (arranque, API, websocket, caché) está en
  [`slack-servidor.md`](slack-servidor.md). El inventario de funciones, con lo que el
  clon ya tiene, está en [`funciones.md`](funciones.md).

Los nombres entre corchetes, como `[data-qa=…]`, son los atributos `data-qa` que Slack
pone en su DOM. Sirven para volver a encontrar cada pieza con CDP o Playwright.

## 1. Distribución general

```
 0        70                358                                            1272 1276
 ┌────────┬─────────────────────────────────────────────────────────────────────┐  0
 │        │  ← → 🕘   [ 🔍 Search <workspace>            ] (AI) ▾   [app ▾] (?)  │  barra superior, 40 px
 ├────────┼──────────────────┬──────────────────────────────────────────────────┤ 40
 │ [WS]   │ Workspace ▾ ⚙ ✎ │ ☆ # general               👤40 🎧▾ 🔔 🔍 ⋮       │ encabezado 49 px
 │        │ [Find a conv…]   │ Messages | Files & links | Pins | <canvas> | +   │ pestañas ~38 px
 │ Home   │ Threads          ├──────────────────────────────────────────────────┤ 127
 │ DMs    │ Huddles          │            ── Friday, September 11th ▾ ──        │
 │Activity│ Drafts & sent    │ (av) Name  6:18 PM                               │
 │ Files  │ Saved            │      texto del mensaje …                         │  lista de
 │ To-dos │ Directories      │      [👀 4] [☺+]                                 │  mensajes
 │ More   │ ──────────────── │                                                  │
 │        │ + Create section │                                                  │
 │        │ External conn.   │                                                  │
 │        │ ▾ Channels       │ ┌──────────────────────────────────────────────┐ │ 809
 │        │   🔒 privado     │ │ B I U S │ 🔗 1. • │ ❝ </> ⌗                   │ │  compositor
 │        │   # general  ◀sel│ │ Message #general                             │ │  118 px
 │  (+)   │ ☆ Starred        │ │ + Aa ☺ @ │ 🎥 🎤 │ /             ➤ │ ▾      │ │
 │  (☾)   │ ▾ Direct messages│ └──────────────────────────────────────────────┘ │
 │  (yo)  │ ▾ Apps           │                                                  │
 └────────┴──────────────────┴──────────────────────────────────────────────────┘ 955
```

| Zona | Caja (x, y, ancho × alto) | Notas |
|---|---|---|
| Barra superior `[top-nav]` | 0,0 · 1276 × 40 | Va sobre el fondo del tema, sin color propio. |
| Rail `[tab_rail_desktop]` | 0,40 · 70 × 915 | Padding superior de 8 px. |
| Marco del workspace `.p-client_workspace__layout` | 70,40 · 1202 × 911 | Esquina superior izquierda redondeada a **6 px**. Deja 4 px de margen a la derecha y abajo, por donde se ve el fondo del tema. |
| Lateral `.p-view_contents` | 70,40 · 288 × 911 | Fondo `rgba(255,255,255,0.9)` en este tema. El ancho se cambia arrastrando el borde, o con ← → si el borde tiene el foco. |
| Encabezado del canal `[view_header]` | 358,40 · 914 × 49 | Fondo blanco, padding `0 12px 0 18px`. |
| Pestañas del canal | 358,89 · 914 × ~38 | |
| Lista de mensajes `[message_pane]` | 358,119 · 914 × 698 | |
| Compositor `[message_input_container]` | 358,809 · 914 × 142 | Padding de 20 px a los lados. |
| Panel derecho (ayuda, IA, atajos, perfil) | 915,40 · 357 × 911 | Se abre a la derecha y angosta el canal. |
| Panel de hilo `[threads_flexpane]` | 553 px de ancho | Con dos paneles abiertos, el hilo o el perfil reemplazan al canal. |

### Fondo y tema

El tema del workspace relevado es claro y cálido: barra y rail café
(`rgb(120,76,31)`, token `--dt_color-theme-base-pry`), lateral casi blanca
translúcida y selección verde jade. El clon usa Aubergine. Todos los colores de la UI
salen de unos 60 tokens `--dt_color-theme-*` que cambian con el tema. Estos son los que
importan para pintar:

| Token | Valor en este tema | Uso |
|---|---|---|
| `theme-base-pry` | `rgb(120,76,31)` | fondo de barra superior y rail |
| `theme-surf-inv-sec` | `rgba(255,255,255,0.9)` | fondo de la lateral |
| `theme-content-pry` | `rgb(66,40,13)` | texto fuerte de la lateral (título del workspace) |
| `theme-content-sec` | `rgba(66,40,13,0.8)` | texto normal de la lateral |
| `theme-content-inv-ter` | `rgba(66,40,13,0.5)` | texto tenue |
| `theme-base-hgl-1` | `rgb(14,103,77)` | fondo del canal seleccionado |
| `theme-content-hgl-1` | `rgb(227,255,243)` | texto del canal seleccionado |
| `theme-base-imp` | `rgb(0,122,90)` | acentos (insignias, toggles) |
| `theme-surf-pry-hover` | `rgba(120,76,31,0.06)` | hover de filas de la lateral |
| `theme-otl-pry` | `rgba(66,40,13,0.13)` | separadores de la lateral |

Colores fijos del área de contenido, que no cambian con el tema:

| Token | Valor | Uso |
|---|---|---|
| `content-pry` | `#1d1c1d` | texto de mensajes |
| `content-sec` | `#454447` | texto secundario |
| `rgb(97,96,97)` | — | hora del mensaje |
| `otl-pry` | `#7c7a7f` | bordes |
| `rgba(29,28,29,0.06)` | — | fondo de píldora de reacción |
| `rgba(255,198,0,0.18)` | — | resaltado de `@here` / `@channel` / tu mención (texto en 700) |
| `plt-ocean-50` | `rgb(41,133,205)` | azul de enlaces y de tu reacción |
| Aubergine 100 / 90 / 80 | `rgb(57,6,58)` / `rgb(74,21,75)` / `rgb(97,31,105)` | paleta del tema Aubergine clásico |

**De dónde sale el tema.** `users.prefs.get` lo guarda en `ia_theme`, un JSON con la
paleta primaria y la de resalte, por ejemplo
`{"primary":{"palette":"horchata"},"highlight1":{"palette":"jade"},"brightness":5,…}`.
`sidebar_theme` y `sidebar_theme_custom_values` son el sistema anterior. Hay 20 paletas
base (`--dt_color-plt-<nombre>-<tono>`, tonos 0–100), y cada token `theme-*` es un tono
fijo de una de ellas:

| Parte | Claro | Oscuro |
|---|---|---|
| Barra y rail | primaria 70 (`theme-base-pry`) | primaria 10 (`theme-base-inv-sec`) |
| Lateral | blanco al 90 % sobre la barra | `plt-gray-0` al 60 % sobre la barra |
| Texto fuerte / normal de la lateral | primaria 100 / al 80 % | primaria 100 / 90 al 90 % |
| Selección y su texto | resalte 80 / resalte 0 | resalte 30 / resalte 100 |
| Insignias | resalte 70 | resalte 70 |

En modo oscuro las paletas mismas cambian (el tono 10 oscuro se parece al 90 claro);
están en [`datos/slack-css-tokens-oscuro.json`](datos/slack-css-tokens-oscuro.json). El
modo claro u oscuro **no** viaja en las preferencias: es del dispositivo (la página lleva
la clase `sk-client-theme--dark`). Medido el 2026-10-22, con brillo 5.

Con **focus mode** prendido toda la UI pasa a modo oscuro (`capturas/10c-moon-hover.png`).
Hay una preferencia para desactivarlo: "Always use dark color mode when in focus mode".

### Tipografía

- Familia: `Slack-Lato, Slack-Fractions, appleLogo, sans-serif`. Lato es la fuente
  por defecto; desde Preferencias → Appearance se puede cambiar.
- Base: **15 px / 22 px**, peso 400.

| Elemento | Tamaño / interlineado | Peso |
|---|---|---|
| Nombre del workspace (lateral) | 18 / 24 | 900 |
| Nombre del canal (encabezado) | 18 / 24 | 900 |
| Filas de la lateral | 15 / 28 | 400 (los no leídos van en 700) |
| Encabezado de sección de la lateral | 15 / 28 | 500 |
| Autor del mensaje | 15 / 22 | 900 |
| Hora del mensaje | 12 / 17.6 | 400 |
| Texto del mensaje | 15 / 22 | 400 |
| Etiqueta del separador de día | 13 / 27 | 700 |
| Etiqueta bajo los íconos del rail | 11 / 12 | 700 |
| Buscador de la lateral | 13 / 19 | 400 |
| Conteo de una reacción | 12 | 400 |

## 2. Barra superior `[top-nav]`

De izquierda a derecha:

1. **← →** `[history_back_button]` `[history_forward_button]`: historial de
   navegación (Alt + ← / →).
2. **🕘** `[history_menu_button]`: menú "History" con las últimas conversaciones y
   vistas visitadas.
3. **Buscador** `[top_nav_search]`: 521 × 28, radio de 4 px, texto
   "Search <workspace>". Ver §9.
4. **Slackbot AI** `[slackbot-ai-button]` y su ▾ `[ai-apps-menu-container]`: el botón
   abre el panel derecho de Slackbot. En este plan muestra una oferta ("I Want This" /
   "No thanks, take me to basic Slackbot"). El ▾ lista los agentes de IA instalados y
   tiene "Manage".
5. **Selector de app o agente** con ▾, a la derecha: muestra el ícono de la app de IA
   activa.
6. **Ayuda (?)** `[top-nav-help-button]`: panel derecho "Help" con un buscador
   "How can we help?", un carrusel "Discover more" (novedades, 1/3), temas (configurar
   notificaciones, estado, recordatorios, reacciones, videos), categorías ("Getting
   started", "Using Slack", "Your profile & preferences", "Connect tools & automate
   tasks", "Workspace administration", "Tutorials & videos"), "Help requests" y
   "Contact Us". En el encabezado tiene botones a **Keyboard shortcuts** y **What's
   new**.

## 3. Rail `[tab_rail_desktop]`

Una columna de 70 px. Cada botón mide 52 × 68: ícono de 20 px dentro de un cuadro de
36 px con radio de 12 px, y la etiqueta debajo en 11 px, peso 700. El botón activo
lleva el ícono relleno y un fondo translúcido.

| Botón | `data-qa` | Atajo | Qué abre |
|---|---|---|---|
| Ícono del workspace (36 × 36) | `account_switcher_team_icon` | Ctrl + Shift + S abre el selector | "Switch workspaces…". Con varios workspaces, cada uno es un ícono `[team_sidebar_item]` con insignia. |
| Home | `tab_rail_home_button` | Ctrl + Shift + 1 | La lateral con canales (vista por defecto). |
| DMs | `tab_rail_dms_button` | Ctrl + Shift + 2 | La lista de DMs (§10.1). |
| Activity | `tab_rail_activity_button` | Ctrl + Shift + 3 | La bandeja de actividad (§10.2). Lleva insignia con el número de pendientes. |
| Files | `tab_rail_files_button` | Ctrl + Shift + 4 | Archivos, canvases y listas (§10.3). |
| To-dos | `tab_rail_later_button` | Ctrl + Shift + 5 | Pendientes y guardados (§10.4). |
| More… | `tab_rail_browse_button` | Ctrl + Shift + 0 | Menú: "Agents & tools" y "Customize navigation bar". |

Al pie del rail:

- **(+)**: crear (mensaje, canal, canvas, etc.).
- **☾ Focus mode** `[silent-sidebar-trigger]`: **es un interruptor, no un menú**. Un clic
  pausa las notificaciones, pasa la UI a oscuro y abre el popover "You're in focus
  mode", que tiene un engrane de preferencias, "Duration" (por ejemplo "Don't turn
  off") y "Continue to get notifications from VIPs". Otro clic lo apaga ("Exit focus
  mode").
- **Avatar** `[user-button]` 36 × 36, con un punto de presencia: abre el menú de
  usuario (§12).

Desde Preferencias → Navigation se elige qué pestañas aparecen en el rail y en qué
orden (Home, DMs, Activity, Files, To-dos, Agents & tools), y si se muestran "Icons &
text" o "Icons only". Con solo íconos el rail se angosta y se apagan las insignias
numéricas.

## 4. Lateral (Home)

### 4.1 Encabezado

- **Nombre del workspace ▾** `[workspace_actions_button]`: abre el menú del workspace
  (320 px de ancho, `capturas/01-workspace-menu.png`):
  - Tarjeta con el nombre y el dominio (`<sub>.slack.com`).
  - Aviso del plan ("Your workspace is currently on Slack's Pro Plan. Learn more").
  - Oferta de IA ("Want access to advanced AI features?").
  - Invite people to <workspace>.
  - Preferences.
  - Tools & settings ▸: Customize workspace, Workflow Builder, Workspace analytics,
    Manage apps.
  - Sign in on mobile, Sign out, Open the Slack App.
- **⚙** `[sweeper_button]` ("Manage my sidebar"): menú de filtro y orden.
  - Filter and sort: muestra el filtro actual, por ejemplo "Active only".
  - Plantillas de sección: Daily reads, My team, Announcements, Social, Create your own….
  - Show muted conversations.
  - Clean up your channel list (recomienda qué dejar).
  - Bulk manage conversations.
  - Edit defaults.
- **✎** `[composer_button]`: nuevo mensaje (Shift + Ctrl + K).
- **Find a conversation…** `[sidebar-text-filter-input]`: 272 × 28, radio de 8 px,
  fondo `rgba(255,255,255,0.9)`, borde de 1 px del color del tema, texto de 13 px.
  Filtra la lateral mientras escribes.

### 4.2 Accesos fijos

Son filas de 272 × 28 con radio de 6 px, padding `0 8px` e ícono de 16 px:
**Threads**, **Huddles**, **Drafts & sent**, **Saved** y **Directories**. En
Preferencias → Home se elige cuáles aparecen siempre; ahí también está **Unreads**.
Debajo va **+ Create a section** (con una etiqueta "Tip" azul) y un separador.

### 4.3 Secciones

Cada sección tiene un encabezado `[channel_sidebar__section_heading_label__<id>]` en
15 px y peso 500, con un ícono que al pasar el mouse se vuelve el ▸/▾ de plegar
(Alt + clic pliega o despliega todas). Las secciones relevadas, en orden:

1. **External connections** `slack_connect`: canales con otras organizaciones.
2. **Channels** `channels`.
3. **Starred** `stars`.
4. **Direct messages** `direct_messages`.
5. **Apps** `recent_apps`: apps y agentes, por ejemplo Google Calendar.

Una fila de canal mide 272 × 28, con radio de 6 px y padding `0 8px 0 24px`
(indentada respecto del encabezado). Lleva un ícono de 16 px: `#` si es público, 🔒 si
es privado. En una DM el ícono es el avatar de la persona (16 px, radio de 4 px) con
un punto de presencia; en un grupo, un cuadro con el número de miembros (por ejemplo
"2") y los nombres separados por coma. Tu propia DM aparece como "<nombre> you", con
"you" tenue.

Estados de una fila:

| Estado | Cómo se ve |
|---|---|
| Normal | texto `theme-content-sec` y peso 400 |
| Hover | fondo `theme-surf-pry-hover` |
| No leído | texto en 700, más oscuro |
| Con menciones | insignia a la derecha con el número de menciones, en `theme-base-imp` |
| Seleccionado | fondo `rgb(14,103,77)` y texto `rgb(227,255,243)` |
| Silenciado | texto tenue; se oculta con "Hide muted channels" |

**Filtro "Active only"** (es el que trae por defecto): la lateral solo muestra
conversaciones con actividad en los últimos 30 días, **más la que tienes abierta**. Al
salir de un canal inactivo, ese canal desaparece de la lateral (lo verificamos con
#general). Otras opciones del filtro: Unreads, Mentions y All. El orden puede ser A–Z,
Most recent o Priority, y cada sección puede tener el suyo.

### 4.4 Menús contextuales (clic derecho)

**Encabezado de sección** ("Section options"): Create a channel, Browse channels,
Manage section, más la lista de canales ocultos de la sección (por ejemplo los
silenciados) e "Inactive channels".

**Canal**:

- Channel details.
- Copy ▸ (nombre, enlace).
- Star channel.
- Move to new section.
- Summarize channel (IA).
- Notificaciones, en radio: All new posts / Just mentions / Mute and hide.
- More options.
- Open in split view (Ctrl + Alt + clic).
- Leave channel.

**DM** ("Conversation options"):

- Conversation details.
- Copy.
- Star conversation.
- Move to new section.
- Summarize conversation.
- Add to VIP.
- Hide <persona>.
- Open in split view.
- Close conversation.

## 5. Encabezado del canal `[view_header]`

```
☆  # general                                 [👤 40] [🎧 ▾] [🔔] [🔍] [⋮]
Messages │ Files & links │ Pins │ <canvas o lista> │ +
```

- **☆** `[entity-header-star-button]`: "Move channel"; destaca el canal o lo mueve
  de sección.
- **# nombre** `[channel_name_button]`: 18 px, peso 900. Abre los detalles del canal
  (§5.1).
- **👤 40** `[avatar_stack]`: miembros (47 × 28, radio de 8 px). Abre los detalles en
  la pestaña Members. La etiqueta accesible dice "View all 40 members. Includes A, B
  and C."
- **🎧** `[huddle_channel_header_button__start_button]`: inicia un huddle (llamada de
  audio) en el canal. Su **▾** ofrece "Start Huddle" y "Copy huddle link".
- **🔔** "Edit notifications": radio entre **All new posts** ("Messages and threads
  you follow"), **Just mentions** ("@you, @channel, @here") y **Mute and hide** ("Only
  badge the channel when someone @mentions you"); más "More options".
- **🔍** `[search_in_channel_button]`: búsqueda limitada a este canal.
- **⋮** "More channel actions": Channel details, Copy, Star channel, Move channel,
  Summarize channel, Add a workflow, Edit settings, Open in split view, Leave channel.

En una DM el encabezado cambia: "Conversation details for <persona>", "Mute
conversation", "Search in channel" y "More conversation actions". Sus pestañas son
Messages, Add canvas y Files & links.

**Pestañas del canal** (`role=tab`): **Messages** `[channel]`, **Files & links**
`[files]`, **Pins** `[bookmarks]`, una pestaña por cada canvas o lista anclada
`[canvas]` y **+** para agregar otra (el punto azul encima es una novedad sin ver). Se
cambia de pestaña con Ctrl + PageDown / PageUp.

### 5.1 Detalles del canal `[channel_details_modal]`

Modal de 580 × 812. Arriba tiene el nombre y tres botones: Star channel, el estado de
notificaciones (por ejemplo "Just mentions") y Huddle. Sus pestañas son:

| Pestaña | Contenido |
|---|---|
| About | Topic (Edit), Description, Managed by, Created by (quién y cuándo), Send emails to this channel (dirección de correo que publica en el canal), Copy channel ID. |
| Members N | Buscador y lista de miembros (nombre, nombre completo, título), "Add people". |
| Agents & apps | Agentes y apps del canal; "Add Agent or App". |
| Automations | Workflows del canal; "Go to Your Workflows". |
| Tabs | Gestión de pestañas: ocultar o mostrar, subir, bajar, renombrar, borrar y "New Tab"; quién puede editarlas. |
| Settings | Posting permissions; Huddles (Start Huddle, Copy Huddle Link, "Always Start AI Notes"); quién puede reordenar pestañas; sugerencias de miembros al @mencionar; Copy member names; Copy member email addresses; Archive / Leave. |

## 6. Lista de mensajes

### 6.1 Mensaje

```
 20px  36px  8px
 │    ┌────┐   Nombre (15/900)  6:18 PM (12, gris)       [barra flotante al hover]
 │    │ av │   texto 15/22 …
 │    └────┘   • listas, enlaces 🔗 con ícono, @menciones resaltadas
 │             [archivo / imagen / unfurl]
 │             [🥳 8] [👏 5] … [☺+]
```

- Margen interior `.c-message_kit__gutter`: `8px 20px`. Avatar de 36 px con esquinas
  redondeadas. La columna de texto empieza en x = 422, es decir 64 px desde el borde del
  panel.
- **Agrupación**: los mensajes seguidos del mismo autor, en una ventana corta, se
  pintan sin avatar ni nombre. Al pasar el mouse aparece la hora corta en el hueco del
  avatar.
- **Hover**: la fila se tiñe con un gris muy leve y aparece la barra de acciones.
- Fondo del mensaje: blanco. Un mensaje fijado o guardado lleva fondo amarillo pálido
  y una etiqueta arriba ("Pinned by…", "Saved for later"). *(Comportamiento conocido,
  no lo vimos en esta sesión.)*
- **Enlaces**: los de rich text llevan el ícono 🔗 delante y se recortan con "…"
  (`[rich_text_truncated_link_element]`). Al pasar el mouse muestran una tarjeta con
  la URL completa (`[mrkdwn-link-url-hover-card]`).
- **Menciones**: `@usuario` va en azul con fondo celeste. `@here`, `@channel` y la
  mención a ti van con fondo amarillo `rgba(255,198,0,0.18)` y peso 700.
- **Archivos** `[message_kit_files]`: imagen con miniatura `[file_image_thumbnail_img]`;
  una fila de metadatos (nombre y tamaño) con botón para plegar ▾
  `[file-collapse-button]`. `/collapse` y `/expand` pliegan o despliegan todos los del
  canal.
- **Reacciones** `[reaction_bar]`: píldoras `[reactji]` de 24 px de alto, radio
  completo, padding `0 8px`, fondo `rgba(29,28,29,0.06)`, emoji de 16 px y conteo en
  12 px. **Tu reacción** va con borde y fondo azul claro. Al final va **☺+**
  `[add_reaction_button]`. Al pasar el mouse sobre una píldora aparece un tooltip:
  "A, B, C, and you reacted with :eyes:".
- **Hilo**: debajo, la fila "N replies" con hasta 4 avatares de quienes respondieron y
  "Last reply …"; un clic abre el panel de hilo.
- **Mensajes de sistema** (se unió, salió, cambió el tema): ícono de 16 px en el hueco
  del avatar y texto gris en una línea, por ejemplo "A and 3 others joined #general.
  11:52 AM". Los "se unió" seguidos se agrupan.

### 6.2 Separador de día `[day-divider-label]`

Una línea horizontal con una píldora centrada: 28 px de alto, radio de 24 px, fondo
blanco con borde, 13 px en peso 700, por ejemplo "Friday, September 11th ▾". Mientras
haces scroll **se queda pegada arriba** (sticky). Un clic abre "Jump to…": Most recent,
Last week, Last month, The very beginning y Jump to a specific date.

### 6.3 Barra de acciones al pasar el mouse `[message-actions]`

Mide 264 × 40, flota arriba a la derecha del mensaje, con sombra, radio de 8 px y
botones de 32 px:

| Botón | `data-qa` | Notas |
|---|---|---|
| 3 reacciones rápidas | el nombre del emoji | Salen de "My most frequently used emoji" o de la lista que elijas en Preferencias → Messages & media ("Custom"). |
| Add reaction… | `add_reaction` | Abre el selector de emoji (R). |
| Reply in thread | `start_thread` | T. |
| Forward message… | `share_message` | F. |
| Add to-do | `add_to_later_todos` | A. |
| More actions ⋮ | `more_message_actions` | El menú de abajo. |

**More actions** en un mensaje ajeno (menú de 300 px; la letra a la derecha es el
atajo con el mensaje enfocado):

- Mark unread (U).
- Add to saved (A).
- Add a reminder ▸: In 30 mins, In 1 hour, In 3 hours, Tomorrow at 9:00 AM, Monday at
  9:00 AM, Custom… (M).
- Get notified about new replies.
- Copy link (L).
- Copy message (Ctrl + C).
- Organize ▸: Pin to channel (P), Add to list… (V).
- Connect to apps ▸: los atajos de mensaje de las apps instaladas (por ejemplo "Create
  a task", "Start a poll") y "More message shortcuts…".

En un **mensaje propio** se agregan **Edit message** (E) al principio y **Delete
message…** (Supr) al final. "Get notified about new replies" se vuelve "Turn off
notifications for replies", porque tú lo empezaste.

## 7. Panel de hilo `[threads_flexpane]`

Mide 553 px de ancho y ocupa toda la altura bajo la barra superior.

- Encabezado: "Thread" con el nombre del canal debajo, más "Summarize thread", "More
  channel actions" y "Close".
- Mensaje raíz completo, con sus reacciones.
- Separador "N replies" y las respuestas. Tienen la misma barra de acciones que el
  canal, salvo "Reply in thread".
- Su propio compositor, igual al del canal, con una casilla extra: **"Also send to
  #canal"**. Marcada, la respuesta también aparece en el canal (`thread_broadcast`).
- Ctrl + . cierra o reabre el panel derecho.

## 8. Compositor `[message_input_container]`

Una caja `.c-wysiwyg_container` de 874 × 118 con **radio de 8 px**, borde de 1 px
`rgba(94,93,96,0.4)` y fondo blanco. Al enfocarla, el borde se oscurece.

```
┌────────────────────────────────────────────────────────────────────┐
│ B  I  U  S │ 🔗 │ 1. • │ ❝ │ </> ⌗                                  │ 38 px, fondo #f8f8f8
│ Message #general                                                   │ ≥ 38 px, crece
│ (+) Aa ☺ @ │ 🎥 🎤 │ /                                       ➤ │ ▾   │ 40 px
└────────────────────────────────────────────────────────────────────┘
```

**Barra de formato** (arriba, 38 px, fondo `rgb(248,248,248)`). Se ve atenuada hasta
que escribes:

| Botón | Atajo | Resultado |
|---|---|---|
| Bold | Ctrl + B | `*texto*` |
| Italic | Ctrl + I | `_texto_` |
| Underline | Ctrl + U | subrayado (solo en rich text, no tiene mrkdwn) |
| Strikethrough | Shift + Ctrl + X | `~texto~` |
| Link | — | diálogo de texto + URL |
| Ordered list | Shift + Ctrl + 7 | `1.` |
| Bulleted list | Shift + Ctrl + 8 | `•` |
| Blockquote | Shift + Ctrl + 9 | `>` |
| Code | Shift + Ctrl + C | `` `código` `` |
| Code block | Alt + Shift + Ctrl + C | ```` ``` ```` |

El editor es WYSIWYG (Quill: `.ql-container`, `.ql-editor`): el formato se ve
aplicado mientras escribes, no como marcas. Texto de ayuda: "Message #canal" o
"Message <persona>", en 15 px.

**Barra inferior**:

| Control | `data-qa` | Qué hace |
|---|---|---|
| **+** Attach | `shortcuts_menu_trigger__Channel` | Menú "Attach": File, canvas, or list; Text snippet (Ctrl + Shift + Enter); Workflow; Upload from your computer (Ctrl + O). |
| **Aa** | `texty_composer_button` | Muestra u oculta la barra de formato. Es una preferencia ("Format messages with markup"). |
| **☺** | `emoji_toolbar_button` | Selector de emoji (§8.1). |
| **@** | `texty_mention_button` | Inserta `@` y abre el autocompletado de personas. |
| **🎥** | `video_composer_button` | Graba un clip de video, con pantalla o cámara. |
| **🎤** | `audio_composer_button` | Graba un clip de audio. |
| **/** | `slash_commands_composer_button` | Lista de atajos y comandos `/`. |
| **➤** Send | `texty_send_button` | Desactivado mientras el texto está vacío. |
| **▾** Schedule for later | `texty_send_options_button` | Programar el envío. Desactivado si no hay texto. |

En una DM, la barra inferior colapsa 🎥, 🎤 y / en un **⋯**.

**Teclas**: Intro envía y Shift + Intro agrega una línea. Las dos se pueden invertir:
"Start a new line (use Ctrl + Enter to send)". Con ```` ``` ```` abierto, hay una
opción para que Intro no envíe. ↑ con el compositor vacío edita tu último mensaje y
Ctrl + Z con el compositor vacío lo deshace ("unsend"). El texto se guarda como
borrador por conversación y se sincroniza con el servidor (§ servidor).

### 8.1 Selector de emoji `[emoji-picker]`

Mide 361 × 454. Lo componen:

- Un buscador.
- Pestañas por categoría: Search, Smileys & People, Animals & Nature, Food & Drink,
  Travel & Places, Activities, Objects, Symbols, Flags y **Custom** (los emoji del
  workspace).
- La sección "Frequently Used" arriba.
- Al pie, una vista previa del emoji bajo el cursor, "Add Emoji" (subir uno propio) y
  el selector de tono de piel ("Choose your default skin tone").

### 8.2 Autocompletados

Aparecen como una lista flotante sobre el compositor, de unos 528–560 px de ancho. La
primera opción va resaltada con la pista "Enter".

| Disparador | Muestra |
|---|---|
| `@` | Personas: nombre, nombre completo y estado (por ejemplo "Vacaciones"). Las más frecuentes primero. También `@here`, `@channel` y grupos. |
| `#` | Canales que coinciden. Los archivados se marcan "(archived)" y los renombrados "(formerly …)". |
| `:` + 2 letras | Emoji: `:thumbsup:`, `:thumbsup_all:`, …, incluidos los propios del workspace. |
| `/` | Comandos de Slack y de apps: nombre, parámetros, origen ("Slack", "App · GitHub") y descripción. Arriba van los atajos de workflows. |

Algunos comandos propios de Slack que vimos: `/remind`, `/collapse`, `/expand`,
`/feed`, "Resume all your notifications", "Set yourself a reminder" y "View all your
mentions & reactions". Lo demás lo agregan las apps instaladas (Giphy, GitHub, Google
Calendar, HubSpot, etc.).

## 9. Búsqueda

**Abrir**: clic en el buscador, **Ctrl + K** o Ctrl + F (si la preferencia está
activa). Ctrl + K ya no abre un "quick switcher" aparte: es el mismo buscador.

**Desplegable** (640 px de ancho, sobre la barra superior, `capturas/40-search-open.png`):

- Campo "Search across people, channels, files, workflows, and more", con botones
  "Show filters" y "Show search history" (🕘).
- Tarjeta "Try AI search" (descartable).
- Mientras escribes, sugerencias en vivo: "Show results for: <texto>", conversaciones,
  personas, mensajes que coinciden (con conversación, fecha y fragmento) y artículos de
  ayuda.
- Pie: "↑ ↓ Select" y "Give feedback".

**Página de resultados** (`/client/<team>/search`, `capturas/42-search-results.png`):

- Título "Results for: **<texto>**". La barra superior pasa a mostrar "Search:
  <texto> ✕".
- Filtros en píldoras:
  - Tipo ▾: Messages / DMs / Files / People / Channels / Canvases, cada uno con su
    conteo, y la opción "Show as a left sidebar".
  - From ▾ (personas o apps).
  - In ▾ (conversaciones).
  - Includes me, Has a link, Threads only, Last 7 days.
  - More ▾.
- "Sorted by Most relevant (default) ▾": Most relevant, Newest, Oldest y "Edit
  default". También el número de resultados (por ejemplo "2,250 results").
- Cada resultado es una tarjeta con borde y radio de ~8 px: avatar, autor, 🔒 canal o
  "Direct Message with …" o "Thread in …", la fecha a la derecha y el texto con la
  coincidencia **resaltada en amarillo**. Lleva "… Show more", archivos, reacciones
  y "N replies".

## 10. Vistas del rail y de la lateral

### 10.1 DMs (`/client/<team>/dms`)

- Lateral ancha (unos 457 px): "Direct messages ▾", un interruptor **Unreads**,
  ✎ "New message" y "Find a DM…".
- Cada fila mide 73 px: avatar de 36 px con presencia, nombre en 700, la hora o fecha
  relativa a la derecha ("3:38 PM", "Monday", "October 9th") y una vista previa de una
  o dos líneas del último mensaje. Si lo escribiste tú, va con el prefijo "You:".
- Sin selección, el lado derecho muestra una ilustración vacía.

### 10.2 Activity (`/client/<team>/activity-inbox`)

`capturas/61-activity.png`.

- Título "Activity". Arriba a la derecha, la densidad: **Detailed / Dense** y ⋮
  Settings.
- Pestañas: **All**, **DMs**, **Mentions**, **Threads** y **+** para crear una pestaña
  propia.
- Barra: casilla de selección múltiple ▾, "Unreads", Filters ▾ y 🔍.
- Cada elemento lleva avatar, "Autor +N", la hora, una línea de acción ("Reacted 😲
  to: …", el texto de la mención) y el contexto ("Mention in 🔒canal", "Thread in
  …", "Direct message"), además de las reacciones. Los no leídos tienen el fondo
  gris claro y los leídos llevan un ✓ a la derecha.
- Al seleccionar uno, el detalle se abre al lado. Atajos: U marca como leído, C
  descarta, A guarda, M crea un recordatorio, S pospone, F reenvía y X selecciona para
  acciones en lote.

### 10.3 Files (`/client/<team>/unified-files`)

- Lateral: All files, Canvases y Lists; Deleted; Starred (con los elementos
  destacados).
- Arriba: "More options" y "Create new". A la derecha: "New".
- Centro: galería de **Templates** (canvases y listas de ejemplo) y la lista de
  archivos con filtros.

### 10.4 To-dos (`/client/<team>/to-dos`)

- Título "To-dos", "Add To-do" y "Clean up".
- "Filter: no filters selected" y "Sort".
- Agrupa por estado ("In progress, 8", …). También hay "Saved" y "Bring all archived
  items to To-dos".
- Reúne lo guardado ("Add to saved"), los recordatorios y los to-dos.

### 10.5 Desde la lateral

| Vista | Contenido |
|---|---|
| **Threads** | Todos los hilos en los que participas, cada uno con su mensaje raíz, las respuestas recientes y un compositor en línea. |
| **Huddles** | Historial de huddles: "New Huddle", filtros "Filter huddles", "With" e "In"; cada fila con persona o canal, fecha, duración ("1 hour 19 minutes"), respuestas, participantes y ⋮. |
| **Drafts & sent** | Pestañas **Drafts** / **Scheduled** / **Sent**. |
| **Saved** | Lista de guardados, "Clean up", "Clear All Saved" y "Move to To-dos". |
| **Directories** | Pestañas **People** / **Channels** / **User Groups** / **External** / **Invitations**. People tiene filtros (Title, Location, More), orden "Most recommended" e "Invite People". |
| **External connections** | El mismo directorio, del lado de Slack Connect. |

## 11. Tarjeta de perfil

Un clic en el nombre o el avatar del autor abre el panel **Profile** (con ← para
volver):

- Foto grande (unos 256 px, esquinas redondeadas).
- Nombre grande en 900.
- Estado y presencia, por ejemplo "Away, notifications snoozed".
- 🕘 hora local de la persona ("10:06 PM local time").
- Botones **Message**, **Huddle ▾**, **VIP** y **⋮** (ver archivos, copiar ID, etc.).
- "Contact information" con el correo.

## 12. Menú de usuario `[user-button]`

`capturas/06-user.png`.

- Avatar, nombre y estado.
- Aviso de temas nuevos (descartable).
- Update your status.
- Set yourself as away / active.
- Notifications ▸: muestra "Paused" si están pausadas; ahí se pausan por un tiempo o
  se programan.
- Profile.
- Preferences.
- Account settings (abre la web de la cuenta).
- Sign out of <workspace>.

## 13. Preferencias

Modal con navegación lateral de 16 secciones (`capturas/51-prefs-NN.png`):

| Sección | Qué se configura |
|---|---|
| **Availability** | Horario laboral (notificaciones solo entre, por ejemplo, 8:00 AM y 6:00 PM, todos los días). Estados automáticos: "In a huddle", "In focus mode", "Outside work hours". Focus mode cuando el calendario tiene un bloque de foco. |
| **Notifications** | Escritorio y móvil: todo, solo menciones y DMs, o nada. Extras: respuestas de hilo, VIPs aunque estén pausadas, huddles nuevos. Qué va a Activity. **Palabras clave** (actúan como menciones). Hora por defecto de los recordatorios (9:00 AM). Cuándo mandar al móvil si no estás activo. Correo. Vista previa del mensaje en la notificación. Silenciar sonidos de mensajes o huddles. Sonido ("Boop Plus"). Excepciones por canal (por ejemplo, uno silenciado). |
| **VIP** | Permitir siempre a los VIP. Crear la sección "VIP unreads" arriba en la lateral. Lista de VIP con sugerencias ("Frequent collaborator"). |
| **Navigation** | Qué pestañas van en el rail y su orden. Icons & text o Icons only. Mostrar agentes de IA en la barra superior y en DMs. |
| **Home** | Sugerencias en canales nuevos. Ordenar en secciones automáticamente. Punto de actividad en Home. Accesos fijos (Unreads, Huddles, Threads, Saved, Drafts & sent, Directories). Filtro (Active only = 30 días / Unreads / Mentions / All). Orden (A–Z / Most recent / Priority). Fotos en las DMs. Separar privados de públicos. Separar DMs y apps de canales. Subir los que tienen menciones. Agrupar los externos. Subir a los usuarios en línea. Ocultar silenciados. |
| **Appearance** | Fuente (Lato por defecto). Modo de color Light / Dark / sistema, y oscuro en focus mode. **Temas**: Miles Davis (Kind of Blue, Funky Fresh, Jazz Club, Electric Fusion, Brassy, Sunglasses Inside); Single color (**Aubergine**, Clementine, Banana, Jade, Lagoon, Barbra, Gray, Mood Indigo); Vision assistive (Tritanopia, Protanopia & Deuteranopia); Fun and new (Raspberry Beret, Big Business, POG, Mint Chip, PB&J, Chill Vibes, Forest Floor, Slackr, Sea Glass, Lemon Lime, Falling Leaves, Sunrise); Updated classics (Choco Mint, CMYK, Haberdashery, Hoth, Ochin, Sweet Treat). Tema propio ("Custom theme"). |
| **Messages & media** | Densidad **Clean / Compact**. Nombres "Full & display names" o "Just display names". Indicador de "escribiendo…". Reloj de 24 horas. Muestras de color junto a valores hex. Tono de piel. Emoji como texto. **JUMBOMOJI** (emoji grandes sin texto, hasta 23). Convertir emoticones (`:D`). Reacciones de un clic: las frecuentes o 3 propias. Imágenes y archivos subidos, de sitios enlazados (aunque pesen más de 20 MB) y vistas previas de enlaces. Dirección para reenviar correos a Slack. |
| **Language & region** | Idioma, zona horaria (automática o manual), distribución del teclado y corrector ortográfico. |
| **Accessibility** | Simplified layout mode. Recordatorio de alt text. Vistas previas al pasar sobre pestañas. Subrayar enlaces. Animaciones. Qué hace ↑ en el compositor vacío. Teclado en listas. Lector de pantalla: orden y partes del mensaje que se leen y anuncios automáticos. |
| **Mark as read** | Al abrir un canal: empezar donde lo dejé y marcarlo leído / empezar en el más nuevo y marcarlo leído / empezar en el más nuevo y dejar lo no visto como no leído. Confirmar al marcar todo como leído. |
| **Audio & video** | Cámara, micrófono (ganancia automática, supresión de ruido) y altavoz. Acoplar el video en una esquina. Al entrar a un huddle: micrófono silenciado, subtítulos, aviso si el canal tiene más de 150 miembros, fondo desenfocado y vista previa. Reacciones y efectos. Música cuando estás solo. Prueba de audio y video. |
| **Salesforce** | Organizaciones conectadas. |
| **Connected accounts** | Sign in with Slack. Cuentas para vistas previas de enlaces (por ejemplo Atlassian). Cuentas de workflows (Google Calendar). |
| **Privacy & visibility** | Quién te encuentra por correo. Quién comparte tu contacto. Invitaciones bloqueadas. Personas ocultas. |
| **Slack AI** | Resúmenes en streaming. |
| **Advanced** | Con ```` ``` ```` Intro no envía. Formato con marcas en vez de barra de formato. Acortar enlaces externos. Qué hace Intro. Ctrl + F busca en Slack y Ctrl + K abre el Quick Switcher (los dos reemplazan lo que haría el navegador). Canales excluidos de la búsqueda. Orden por defecto de la búsqueda. Confirmar el *unsend* con Ctrl + Z. Avisos de enlaces maliciosos y de compartir con externos. Encuestas y sugerencias. |

## 14. Atajos de teclado

Se abren con Ctrl + / o desde Ayuda → ⌨. Es un panel derecho con buscador. Esta es la
lista completa; donde Slack dibuja una flecha va ↑ ↓ ← →.

**Básicos**

| Acción | Atajo |
|---|---|
| Nuevo mensaje | Shift + Ctrl + K |
| Ir a… (buscar) | Ctrl + K |
| Editar el último mensaje (con el compositor vacío) | ↑ o Ctrl + ↑ |
| Deshacer el envío del último mensaje (con el compositor vacío) | Ctrl + Z |
| Atrás / adelante en el historial | Alt + ← / Alt + → |
| Cerrar diálogos | Esc |
| Poner estado | Shift + Ctrl + Y |
| Cambiar el ancho de la lateral (con el foco en el borde) | ← / → |

**Navegación**

| Acción | Atajo |
|---|---|
| Home / DMs / Activity / Files / To-dos | Ctrl + Shift + 1 … 5 |
| Menú More | Ctrl + Shift + 0 |
| All Unreads | Ctrl + Shift + A |
| Recorrer mensajes | ↑ / ↓ |
| Sección siguiente / anterior | Ctrl + F6 / Shift + Ctrl + F6 |
| Activar el elemento enfocado | Enter |
| Agrandar / achicar el texto | Ctrl + + / Ctrl + - |
| Subir / bajar | PageUp / PageDown |
| Día anterior / siguiente | Shift + PageUp / Shift + PageDown |
| Cerrar o reabrir el panel derecho | Ctrl + . |
| Explorar canales | Shift + Ctrl + L |
| Directorio | Shift + Ctrl + E |
| Pantalla completa | F11 |

**Mensajes**

| Acción | Atajo |
|---|---|
| Nueva línea | Shift + Enter |
| Reaccionar al último mensaje (o al enfocado) | Shift + Ctrl + \ |
| Autocompletar | `@` personas, `#` canales, `:` emoji |
| Negrita / cursiva / subrayado | Ctrl + B / I / U |
| Tachado | Shift + Ctrl + X |
| Código | Shift + Ctrl + C |
| Lista con viñetas / numerada / cita | Shift + Ctrl + 8 / 7 / 9 |
| Bloque de código | Alt + Shift + Ctrl + C |

**Leídos y no leídos**

| Acción | Atajo |
|---|---|
| Marcar el canal actual como leído | Esc |
| Marcar todo como leído | Shift + Esc |
| Marcar como no leído desde un mensaje | Alt + clic en el mensaje |

**Canales y DMs**

| Acción | Atajo |
|---|---|
| Anterior / siguiente | Alt + ↑ / Alt + ↓ |
| Anterior / siguiente no leído | Alt + Shift + ↑ / Alt + Shift + ↓ |

**Con un mensaje enfocado**

| Acción | Atajo |
|---|---|
| Editar (propio) | E |
| Borrar (propio) | Supr |
| Reaccionar | R |
| Abrir el hilo | T o → |
| Fijar / quitar | P |
| Reenviar | F |
| Guardar | A |
| Marcar como no leído desde aquí | U |
| Recordatorio | M |
| Copiar enlace | L |
| Copiar texto | Ctrl + C |

**Workspaces y pestañas**

| Acción | Atajo |
|---|---|
| Workspace siguiente / anterior | Ctrl + Tab / Ctrl + Shift + Tab |
| Workspace N | Ctrl + 1 … 9 |
| Selector de workspaces | Shift + Ctrl + S |
| Pestaña siguiente / anterior | Ctrl + PageDown / Ctrl + PageUp |

**Otros**

| Grupo | Atajos |
|---|---|
| Archivos | Ctrl + O subir; Shift + Ctrl + Enter snippet; Shift + Ctrl + J descargas |
| Secciones | Alt + clic en una sección pliega o despliega todas |
| Huddles | Ctrl + Alt + Shift + H iniciar o terminar; Shift + Ctrl + Espacio silenciar |
| Activity | ↑ ↓ enfocar; Alt + ↑ ↓ seleccionar; Shift + Alt + ↑ ↓ no leídos; Enter o → abrir; Alt + . o Esc cerrar; U, C, A, M, S, F, X; Alt + U y Alt + C en lote |
| All Unreads | → / ← expandir o plegar el canal; Esc marcarlo leído; Shift + PageUp / PageDown canal anterior / siguiente |
| Lector de pantalla | Alt + , y 0–9 lee los últimos mensajes; Ctrl + Alt + S resumen de notificaciones |
| Canvas | Ctrl + Alt + 0–3 párrafo y encabezados; Ctrl + Shift + 7/8/9 listas y checklist; Ctrl + Shift + > cita; Ctrl + Alt + Shift + C código; Ctrl + ` alterna estilos; Ctrl + F buscar; Ctrl + Shift + U enlace; Ctrl + Alt + T comentarios; Ctrl + Alt + E reacciones; Ctrl + Alt + R lector o editor; Ctrl + Alt + Q copiar ancla; Ctrl + Alt + ↑ ↓ mover elemento de lista; Ctrl + Enter menú contextual o tarjeta; Ctrl + O insertar archivo |

## 15. Detalles de UX que conviene copiar

- El **separador de día es sticky** y además sirve para saltar en el tiempo (§6.2).
- **"Active only" oculta lo inactivo pero nunca lo que está abierto** (§4.3).
- **El placeholder nombra el destino**: "Message #canal" o "Message <persona>", así
  sabes a dónde va lo que escribes.
- **Los botones de enviar y programar se desactivan** con el compositor vacío, y la
  barra de formato se atenúa.
- **Las reacciones con tooltip de quiénes**: "A, B, and you".
- **Los atajos aparecen en los menús**, a la derecha de cada opción, y en los tooltips
  de los botones (por ejemplo "Home · Ctrl+Shift+1").
- **Casi todos los menús miden 300 px de ancho** (el del workspace 320, Attach 360,
  recordatorios 240, huddle 200), con secciones separadas por líneas y submenús ▸ que
  se abren al pasar el mouse.
- **Paneles a la derecha en lugar de modales** para ayuda, atajos, perfil, IA e hilos.
  El canal se angosta, no se tapa. Con dos paneles, el segundo reemplaza al canal y
  muestra un ← para volver.
- **Focus mode** es de un clic, cambia a oscuro y pausa todo salvo los VIP.
- **El borrador se conserva** por conversación y en el servidor: si escribes en #a y
  te vas a #b, al volver sigue ahí (y también en otro dispositivo).
- **Escribir `#te` sugiere también canales archivados y renombrados**, marcados como
  tales.
