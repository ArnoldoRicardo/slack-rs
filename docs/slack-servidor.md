# Slack por dentro: red, tiempo real, caché y UX

Qué hace el cliente web oficial contra el servidor, observado con CDP
(`Network.*` y los frames del websocket) el 2026-10-21 sobre una sesión real de
`app.slack.com` en Chrome 150. Se guardaron solo método, ruta, tamaño y tiempos, sin
tokens, cuerpos ni contenido de mensajes. Lo que dice **observado** salió de la
captura; lo que dice **inferido** es interpretación.

Junto con [`slack-ui.md`](slack-ui.md) (la interfaz) y
[`clon-rust.md`](clon-rust.md) (el clon), esta es la base para decidir qué copiar y qué
no.

## 1. Las piezas

| Host | Para qué | Protocolo |
|---|---|---|
| `app.slack.com` | El documento HTML de la app, servido por el **service worker** desde su caché. | h3 |
| `<sub>.slack.com/api/<método>` | La Web API: unos 45 métodos distintos al arrancar. | h3, `POST` |
| `edgeapi.slack.com/cache/<team>/…` | **Caché de borde** ("flannel") para usuarios, canales, permisos, emoji, apps y huddles. | h3, `POST` |
| `wss-primary.slack.com` | El websocket de tiempo real. | wss |
| `a.slack-edge.com` | JS, CSS y fuentes de la app (44 scripts al arrancar, 38 desde caché). | h3 |
| `ca.slack-edge.com`, `avatars.slack-edge.com` | Avatares. | h3 |
| `emoji.slack-edge.com` | Emoji propios del workspace. | h3 |
| `files.slack.com`, `slack-imgs.com` | Archivos e imágenes de vistas previas (proxy). | h3 |
| `slackb.com/traces/v1/list_of_spans/json` | **Telemetría** (trazas de rendimiento): 15 envíos en los primeros 10 s. | h3 |
| `slack.com/beacon/error` | Reporte de errores del cliente. | h3 |

Casi todo va por **HTTP/3** (184 de 209 peticiones). Todas las llamadas a la API y al
caché de borde son `POST` con formulario.

## 2. Arranque (recarga completa, con caché caliente)

**Observado**: 209 peticiones y 731 KB transferidos en unos 10 s. Desglose:

| Tipo | Peticiones | KB transferidos | Notas |
|---|---|---|---|
| Script | 44 | 447 | 38 salen del caché del service worker. Ya descomprimidos son **45.6 MB de JS**. |
| XHR (API + edge) | 69 | 109 | |
| Imágenes | 69 | 175 | Avatares, emoji, miniaturas. |
| Telemetría (Ping) | 15 | — | |
| Fuentes / CSS | 9 | — | Desde caché. |

Tiempos medidos con `performance` en la misma recarga: *first paint* a los 888 ms,
*first contentful paint* a los 2,388 ms y `load` a los 3,979 ms.

### 2.1 Secuencia de la API

Todas las llamadas salen en ráfagas paralelas, y casi todas tardan 100–250 ms. Los
tiempos son desde que se pide el documento.

**Ráfaga 1, a los 0.97 s** (estado mínimo para pintar):

| Método | KB | Para qué *(inferido por el nombre)* |
|---|---|---|
| `client.init` | 8.9 | Reemplaza al viejo `client.boot`: equipo, yo y configuración. |
| `client.counts` | 2.2 | No leídos y menciones por conversación (el clon también lo usa). |
| `client.channels` | 6.3 | Conversaciones de la lateral. |
| `conversations.view` | 10.3 | El canal abierto: info y **los últimos mensajes**, en una sola llamada. |
| `client.extras` | 4.5 | Datos secundarios del cliente. |
| `experiments.getByUser` | **42.0** | Banderas de experimentos A/B. Es la respuesta más grande del arranque. |
| `api.features`, `features.access.policies.list` | 1.5 | Qué funciones están habilitadas. |
| `client.shouldReload` | 0.2 | Si hay versión nueva del cliente. |

**Ráfaga 2, entre 2.0 y 2.6 s** (preferencias y el resto de la UI):
`users.prefs.get` (9.4 KB, ×2), `drafts.list` (**borradores en el servidor**),
`dnd.info` (no molestar), `users.priority.list` (VIP), `activity.views`,
`aiApps.list`, `files.collections.list`, `calendar.getInstalledCalendars`,
`calendar.user.info`, `apps.profile.get`, `megaphone.notifications.list` (×2,
anuncios), `teams.trials.info`, `workflows.triggers.listRecentlyRunForUser`,
`workflows.triggers.list`, `functions.workflows.list`, `help.issues.ticketStats`,
`sfdc.integration.listOrgs`, `team.targetingCriteria`, `ublockworkaround.history`.

**Ráfaga 3, entre 2.7 y 3.6 s** (lo que el canal abierto necesita):
`users.customStatus.list`, `sharedInvites.canGetLink`, `dnd.teamInfo` (no molestar de
los demás, ×3), `conversations.listPrefs`, `files.info`, `users.interactions.list`,
`conversations.history`, `bookmarks.list` (las pestañas del canal),
`conversations.bulkReacjiTriggers`, `files.channelSettingsAllowDownloads` (×3).

**En paralelo, el caché de borde** `edgeapi.slack.com/cache/<team>/…`, 25 llamadas:
`users/info` (×9, en goteo hasta los 8.6 s, conforme aparecen IDs), `users/list` (4
KB), `users/counts`, `channels/membership` (×2), `permissions/info` (×8),
`emojis/info` (×3), `apps/info`, `huddles/list` y `huddles/info`.

**Conclusión** *(inferida)*: el cliente oficial pinta pronto gracias a `client.init`,
`client.channels` y `conversations.view`, más lo que ya tiene en caché local (§5). Pero
después hace unas 60 llamadas más, la mayoría para funciones que el clon no tiene
(experimentos, workflows, IA, calendario, Salesforce, ofertas de plan).

## 3. Cambiar de canal y subir en el historial

**Observado** al abrir dos canales seguidos y luego subir con la rueda. Cada canal
abierto dispara unas 12–15 peticiones en menos de 0.5 s, que suman de 11 a 13 KB:

```
conversations.history         5–8 KB   (los mensajes)
edge users/list               1–2.5 KB (autores que faltan)
edge permissions/info  ×2
conversations.listPrefs
edge users/counts
files.info                             (archivos citados en el canal)
bookmarks.list                         (pestañas del canal)
conversations.bulkReacjiTriggers
files.channelSettingsAllowDownloads ×1–5
dnd.teamInfo
edge channels/membership
```

Pasados unos 2 s en el canal llega `visits.record` (registro de visita, *inferido*:
para ordenar por "Most recent" y "Priority"). Pasados unos 5 s, `saved.list`.

**Paginación hacia atrás**: al subir, `conversations.history` se repite en páginas de
unos 3.5 KB (dos pedidos en 2 s de scroll). Los acompañan `messages.list` (×2),
`conversations.genericInfo` y `edge channels/info`.

**Marcar como leído**: en estos canales, que ya estaban leídos, **no apareció**
`conversations.mark`. *Inferido*: el cliente solo lo manda si hay algo nuevo que
marcar. El clon lo manda siempre al cargar el historial (ver
[`clon-rust.md`](clon-rust.md) §1).

## 4. Tiempo real (websocket)

El cliente web **no usa `rtm.connect`**. Abre `wss://wss-primary.slack.com/` con
parámetros propios (no los registramos). El clon usa `rtm.connect`, la API vieja, que
da un websocket compatible en lo básico.

### 4.1 Protocolo observado

| Dirección | `type` (y `subtype`) | Cuándo / para qué |
|---|---|---|
| ← | `hello` | Al conectar. Trae `fast_reconnect`, `host_id`, `region` y `start`. |
| ← | `reconnect_url` | Justo después de `hello` (unos 950 bytes): la URL para reconectar rápido sin volver a negociar *(inferido)*. |
| → | `subscription_update` / `user_change_subscribe_request` (`ids`) | Se suscribe a los cambios de perfil **solo de los usuarios visibles**. ← `…_response` con `ok`. |
| → | `presence_sub` (`ids`) | Se suscribe a la **presencia solo de los usuarios en pantalla**. Se manda otra vez cuando cambia el conjunto. |
| ← | `presence_change` (`users`, `presence`) | Cambios de presencia de los suscritos, agrupados en lote. |
| → | `subscription_update` / `dnd_invalidated_subscribe_request` | Suscripción a los cambios de "no molestar". |
| ← | `user_interaction_changed` | Actualiza el ranking de interacción (las sugerencias de `@`) *(inferido)*. |
| → | `tickle` | Al cambiar de canal: señal de actividad, para que te cuente como "activo" *(inferido)*. |
| ← | `search_recents` | Tras una búsqueda: sincroniza el historial de búsquedas. |
| → / ← | `ping` `{id}` / `pong` `{reply_to}` | **Cada 10 s exactos**, con unos 80 ms de ida y vuelta. Es el único tráfico en reposo. |

En 60 s de reposo hubo **6 pings y 6 pongs**, nada más: ni una petición HTTP.

Con más actividad llegan los eventos que el clon ya entiende (`message` y sus subtipos,
`reaction_added` / `reaction_removed`) y otros que no: `user_typing`, `channel_marked`
/ `im_marked` (leído desde otro dispositivo), `pref_change`, `dnd_updated`,
`user_huddle_changed`, `emoji_changed`, `channel_joined`, `member_joined_channel`,
etc. En esta sesión no se generaron, porque nadie escribió en los canales abiertos.

### 4.2 Lo que el clon puede aprender de aquí

- **Ping cada 10 s y verificar el `pong`**. El clon manda un ping cada 30 s y no revisa
  la respuesta (ver [`clon-rust.md`](clon-rust.md) §2.4).
- **Suscribirse a la presencia solo de lo visible** (`presence_sub` con los IDs de
  pantalla), en lugar de recibirla de todos.
- **Al reconectar, pedir otra vez `client.counts` y el historial del canal abierto**.
  Slack lo resuelve con `fast_reconnect` y `reconnect_url`; el clon hoy pierde lo que
  llegó durante el corte.

## 5. Estado local: caché y persistencia

**Observado** con `navigator.storage.estimate()`, `caches.keys()` e
`indexedDB.databases()`:

| Dónde | Qué | Tamaño |
|---|---|---|
| **Cache Storage** (service worker `service-worker.js`) | `gantry-<versión>` (×2: la versión actual y la anterior, el código de la app), `config-cache`, `csPersisted`. | **157 MB** |
| **IndexedDB** | `reduxPersistence` (el estado de Redux guardado entre sesiones), `objectStore-<team>-<user>` y `canvas-<team>-<user>`. | 0.46 MB |
| **localStorage** | 203 claves, unos 700 KB: `persist-v1::<team>::<user>::drafts` (borradores locales), `…::filePermissions`, `…::accessibilityPersistent`, `localConfig_v2` (de ahí sale el `xoxc`), banderas `TOGGLE_STORAGE-*` (por ejemplo `virtual_list_gpu_scroll` y `messages_cache_evict_thread_replies`) y `server-options-*` (parámetros que manda el servidor). | 0.7 MB |

Así el cliente web pinta con datos viejos antes de que respondan la API y la red.
Gracias al service worker, una recarga casi no baja código. El precio: **158 MB en
disco por workspace** y un estado de Redux enorme en memoria (§6).

El clon no guarda nada en disco: cada arranque vuelve a pedir canales, usuarios y
avatares. Es más simple, pero el primer cuadro con datos tarda lo que tarden 4
llamadas. Si se quisiera caché, bastaría con algo mucho más chico: la lista de canales,
los usuarios y los avatares en disco, invalidados por `client.counts` y el tiempo real.

## 6. Costo del cliente oficial en esta máquina

**Observado** en la pestaña de Slack, a los pocos minutos de abrirla, en reposo
(Chrome 150, i7-1185G7):

| Métrica | Valor |
|---|---|
| Heap de JS usado / reservado | **166 MB / 214 MB** |
| Nodos del DOM | 5,543 |
| Listeners de eventos JS | 2,815 |
| Documentos / frames | 6 / 6 |
| JS descargado ya descomprimido | 45.6 MB en 38 scripts |
| Proceso renderer de Slack | **RSS 617 MB, PSS 488 MB** |
| Proceso GPU de Chrome | RSS 235 MB, PSS 161 MB (compartido con todo Chrome) |
| Proceso principal de Chrome | RSS 388 MB, PSS 261 MB (compartido) |
| Chrome entero (20 procesos, incluye extensiones) | PSS 1,319 MB |
| CPU en reposo, 60 s | renderer 1.89 s + GPU 0.64 s + principal 0.33 s ≈ **2.9 s de CPU por minuto** (unos 4.8 % de un núcleo) |

El renderer solo, sin contar el proceso principal ni el GPU, ya ocupa unas 6–7 veces
lo que ocupa el clon entero (ver la comparación en [`clon-rust.md`](clon-rust.md) §5).

## 7. Comportamientos de UX que dependen del servidor

| Comportamiento | Cómo funciona | ¿El clon lo tiene? |
|---|---|---|
| **No leídos y menciones** | `client.counts` al arrancar, y luego eventos del websocket. | Sí: `client.counts` y conteo local por RTM. |
| **Marcar como leído** | `conversations.mark` cuando hay algo nuevo y el canal está a la vista. Lo leído en otro dispositivo llega como `channel_marked`. Preferencias → Mark as read cambia dónde empiezas y si se marca. | Parcial: marca siempre al abrir y no escucha `channel_marked`. |
| **Filtro "Active only"** | Ventana de 30 días con la actividad que dan `client.channels` y `client.counts`; siempre incluye el canal abierto. | No: muestra todo. |
| **Orden "Most recent" y "Priority"** | `visits.record`, `users.interactions.list` y `user_interaction_changed`. | No: ordena por nombre. |
| **Borradores** | `drafts.list` al arrancar, más `persist-v1::…::drafts` local. Sobreviven a cambiar de canal, recargar o usar otro dispositivo. | No: hay un editor por workspace, no por canal. |
| **Programar el envío** | Botón ▾ junto a enviar; luego Drafts & sent → Scheduled. | No. |
| **Presencia** | `presence_sub` de los visibles y `presence_change` en lote. | No. |
| **"Escribiendo…"** | `user_typing` del websocket (y su envío al escribir). Se desactiva en Preferencias. | No. |
| **No molestar / focus mode / horario** | `dnd.info`, `dnd.teamInfo` y `dnd_invalidated_subscribe`; el horario laboral pausa las notificaciones, salvo VIP. | No. |
| **VIP** | `users.priority.list`. | No. |
| **Suscripción a hilos** | "Get notified about new replies" o "Turn off notifications for replies". | No. |
| **Recordatorios / to-dos / guardados** | `saved.list` y To-dos. | No. |
| **Pestañas del canal** | `bookmarks.list` por canal. | No. |
| **Búsqueda** | Sugerencias en vivo, página de resultados con filtros e historial sincronizado (`search_recents`). | No: el buscador solo filtra la lateral. |
| **Experimentos** | `experiments.getByUser`: la UI cambia por usuario. Por eso dos personas pueden ver menús distintos. | No aplica. |
| **Telemetría** | `slackb.com/traces` y `beacon/error`. | No, y es parte de por qué el clon es más ligero. |
| **Versión nueva** | `client.shouldReload` y `gantry-<versión>` en el service worker. | No aplica: es un binario. |

## 7b. Vistas secundarias y acciones (capturado el 2026-10-22)

Lo que llama el cliente web al abrir cada vista, con los parámetros que manda (sin los
`_x_*` de telemetría). Ninguno está en la documentación pública; las formas de las
respuestas se probaron contra el clon (`src/slack.rs`).

| Vista o acción | Método | Parámetros | Respuesta |
|---|---|---|---|
| Threads | `subscriptions.thread.getView` | `limit=8`, `fetch_threads_state=true`, `priority_mode=all` | `threads[]` con `root_msg` (trae `channel`) y `latest_replies[]` |
| Activity | `activity.feed` | `mode=chrono_v1` (obligatorio), `types=at_user,at_user_group,at_channel,at_everyone,keyword` o `thread_v2`, `limit=20`, `is_activity_inbox=true` | `items[]` con `is_unread` e `item.message {ts, channel, author_user_id}` |
| Saved | `saved.list` | `limit`, `filter=saved` o `saved_with_archived` | `saved_items[]` con `item_type`, `item_id` (canal) y `ts` |
| Texto de lo guardado o de Activity | `messages.list` | `message_ids=[{"channel":"C…","timestamps":["…"]}]` | `messages {canal: [mensajes]}` |
| Drafts & sent | `drafts.list` (ya en el arranque) | — | `drafts[]` con `blocks`, `destinations[].channel_id`, `is_sent`, `is_deleted`, `date_scheduled` |
| Directories → People | `search.modules.people` | `module=people`, `query`, `page`, `count` | `items[]` como usuarios (`username` en lugar de `name`) |
| Files | `search.modules.files` | `module=files`, `query`, `page` | `items[]` como archivos |
| Guardar / quitar | `saved.add` / `saved.delete` | `item_type=message`, `item_id`, `ts` | `ok`. Probado escribiendo en la DM con uno mismo y borrando el mensaje de prueba. |
| ☆ destacar | `stars.add` / `stars.remove` | `channel` | El botón ☆ abre "Move to… Starred"; elegirlo llama a esto. |
| 🔔 de un canal | `users.prefs.setNotifications` | `channel_ids`, `prefs=[{"name":"desktop","value":"everything"\|"mentions_dms","sync":true},…]`, `global=false`, `sync=true` | "All new posts" / "Just mentions". |
| Silenciar | `users.prefs.setNotifications` | `name=muted`, `value=true\|false`, `channel_id`, `global=false` | "Mute conversation" en DMs y "Mute and hide" en canales. |

## 8. Cómo se midió (para repetirlo)

1. `chrome-debug status`; luego conectar Playwright por CDP a
   `http://127.0.0.1:9222` y tomar la pestaña `app.slack.com`.
2. `page.context.new_cdp_session(page)`, `Network.enable` y escuchar
   `requestWillBeSent`, `responseReceived`, `loadingFinished`, `webSocketCreated` y
   `webSocketFrameSent` / `webSocketFrameReceived`. De cada frame se guardó solo
   `type`, `subtype`, el largo y los nombres de las claves.
3. Fases: recarga completa del canal (25 s), cambio a dos canales con scroll hacia
   arriba, y 60 s de reposo.
4. Memoria: `Performance.getMetrics`, y `smaps_rollup` (Rss / Pss) de cada proceso de
   Chrome. CPU: `utime + stime` de `/proc/<pid>/stat` antes y después de 60 s.

Los scripts quedaron fuera del repo, en el scratchpad de la sesión. Se pueden rehacer
con lo de arriba.
