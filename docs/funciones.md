# Inventario de funciones: Slack oficial vs. slack-rs

Una fila por función que vimos en el cliente oficial
([`slack-ui.md`](slack-ui.md) y [`slack-servidor.md`](slack-servidor.md)), con su
estado en el clon según el código ([`clon-rust.md`](clon-rust.md)).

**Estado**: ✅ hecho · 🟡 a medias · ❌ no existe · ➖ no aplica o no conviene copiar.

**Prioridad** para el clon (sugerida): **P1** sin esto no reemplaza al oficial en el
día a día · **P2** se nota que falta · **P3** de nicho.

Revisado contra el código el 2026-10-22.

## Estructura y navegación

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Barra superior + rail + lateral + canal + panel derecho | §1 | ✅ | — | La misma distribución. |
| Historial atrás / adelante (Alt + ← →) y menú 🕘 | §2 | ✅ | — | Botones ← → ◷ en la barra y Alt + ← →. |
| Varios workspaces en el rail, con insignias | §3 | ✅ | — | Ctrl + 1…9 igual que el oficial. |
| Iniciar sesión desde la app y agregar workspaces (+) | §3 | ✅ | — | Chrome con perfil temporal; sesiones en `~/.config/slack-rs/credenciales.json` (`login.rs`). Cerrar sesión desde el menú de tu avatar. |
| Ctrl + Tab / Ctrl + Shift + Tab entre workspaces | §14 | ✅ | — | |
| Rail: Home / DMs / Activity / Files / To-dos / More | §3 | 🟡 | P3 | Inicio, MD, Actividad, Archivos, +, 🌙 y tu avatar. Sin To-dos ni More. |
| Rail configurable (Preferencias → Navigation) | §13 | ❌ | P3 | |
| Lateral redimensionable | §1 | ❌ | P3 | Ancho fijo. |
| Panel derecho que angosta el canal | §1 | ✅ | — | Para el hilo (400 px; el oficial usa 553). |
| Split view (Ctrl + Alt + clic) | §4.4 | ❌ | P3 | |

## Lateral

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Canales públicos `#`, privados 🔒, DMs con avatar, grupos con número | §4.3 | ✅ | — | |
| No leídos en negrita, insignia de menciones | §4.3 | ✅ | — | Con `client.counts` + RTM. |
| Secciones plegables | §4.3 | ✅ | — | |
| Secciones propias del usuario (Starred, las creadas, External, Apps) | §4.3 | 🟡 | P2 | Con `users.channelSections.list`; se saltan las de tipo External y Apps. |
| Filtro "Find a conversation…" | §4.1 | ✅ | — | Caja "Buscar conversación…" en la lateral, y Ctrl + K. |
| Filtro Active only (30 días) / Unreads / Mentions / All | §4.3 | ✅ | — | Menú ⚙; por defecto Activas, siempre con el canal abierto. No toca Destacados. |
| Orden A–Z / Most recent / Priority | §4.3 | 🟡 | P3 | A–Z y Más reciente desde el menú ⚙; falta Priority. |
| Presencia (punto verde) en DMs | §4.3 | ✅ | — | `presence_sub` al conectar; verde en línea, hueco ausente. |
| Silenciados atenuados u ocultos | §4.3 | ✅ | — | Atenuados; "Ocultar silenciadas" en el menú ⚙. |
| Accesos Threads / Huddles / Drafts & sent / Saved / Directories | §4.2 | ✅ | — | Hilos, Borradores y enviados, Guardados y Directorio; sin Huddles. |
| Menú contextual de canal o DM | §4.4 | ❌ | P3 | |
| Menú del workspace (▾) | §4.1 | ❌ | P3 | El ▾ es decorativo. |

## Canal

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Encabezado con nombre | §5 | ✅ | — | |
| Miembros, huddle, notificaciones, buscar en el canal, ⋮ | §5 | ✅ | — | ☆ destaca (`stars.add`), 🔔 elige Todos / Solo menciones / Silenciar (`users.prefs.setNotifications`), miembros, 🔍 y ⋮. Sin huddles. |
| Detalles del canal (modal) | §5.1 | ✅ | — | Acerca de (tema, descripción, creador, enlace) y Miembros. |
| Pestañas Messages / Files & links / Pins / canvas | §5 | 🟡 | P3 | Mensajes, Archivos (`files.list`), Fijados (`pins.list`) y los marcadores (`bookmarks.list`). Sin canvas ni agregar marcadores. |
| Avatar, nombre, hora, agrupación por autor | §6.1 | ✅ | — | Ventana de 5 min. |
| Separadores de día | §6.2 | ✅ | — | |
| Separador sticky + "Jump to…" | §6.2 | ❌ | P3 | iced no dice qué mensaje queda arriba, y saltar a una fecha vieja pediría cargar hacia abajo. |
| Scroll infinito hacia atrás | servidor §3 | ✅ | — | Pide la página anterior al acercarse al tope. |
| mrkdwn: negrita, cursiva, tachado, código, enlaces, menciones | §6.1 | ✅ | — | Sin anidar. |
| Rich text (`blocks`): listas, citas, bloque de código, subrayado | §6.1 | 🟡 | P2 | Todo menos subrayado y sangría de listas anidadas. |
| Enlaces truncados con 🔗 y tarjeta al pasar el mouse | §6.1 | 🟡 | P3 | 🔗 y recorte al medio (`dominio/…/final`); sin la tarjeta. |
| Resaltado de @here / @channel / tu mención | §6.1 | ✅ | — | Amarillo en negrita, y la fila del mensaje que te nombra. `@here` y `@channel` cuentan en la insignia. |
| Emoji estándar | §6.1 | ✅ | — | |
| Emoji propios del workspace | §8.1 | 🟡 | P3 | En reacciones, selector, autocompletado y mensajes de solo emoji. Dentro del texto salen como `:nombre:`: el texto con estilo de iced no admite imágenes en línea. |
| Imágenes en línea / miniaturas | §6.1 | ✅ | — | Varias imágenes van como "N archivos" con tres miniaturas y "+N". El clic abre el archivo en Slack. |
| Vista previa de enlaces (unfurl) | §6.1 | ❌ | P2 | |
| Plegar archivos, `/collapse`, `/expand` | §6.1 | ❌ | P3 | |
| Reacciones con conteo, la tuya resaltada, alternar | §6.1 | ✅ | — | Optimista. |
| Tooltip "quiénes reaccionaron" | §6.1 | ✅ | — | Y el botón ☺+ al final de las reacciones. |
| "N respuestas" con avatares | §6.1 | ✅ | — | |
| Mensajes de sistema agrupados (se unió…) | §6.1 | ✅ | — | "Ana y 3 más se unieron a #canal." |
| Mensajes de bots y adjuntos | §6.1 | 🟡 | P3 | Tarjetas con barra de color, título con enlace, texto, campos y pie. Faltan los bloques de bots (botones, secciones). |

## Acciones sobre un mensaje

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Barra al pasar el mouse con 3 reacciones rápidas | §6.3 | ✅ | — | 👍 ✅ 👀 fijas; Slack usa las frecuentes. |
| Responder en hilo | §6.3 | ✅ | — | |
| Selector de emoji para reaccionar | §8.1 | ✅ | — | |
| Editar el mensaje propio (E, ↑ en el compositor vacío) | §6.3 | ✅ | — | ✏️ y ↑; falta la tecla E. |
| Borrar el mensaje propio | §6.3 | ✅ | — | Con confirmación. |
| Copiar enlace / copiar texto | §6.3 | ✅ | — | Menú ⋮ de la barra de acciones. |
| Marcar como no leído desde aquí | §6.3 | ✅ | — | |
| Guardar / to-do / recordatorio | §6.3 | 🟡 | P3 | Guardar y quitar de guardados (`saved.add` / `saved.delete`); sin to-dos ni recordatorios. |
| Fijar en el canal / agregar a lista | §6.3 | 🟡 | P3 | Fijar y desfijar (`pins.add` / `pins.remove`); sin listas. |
| Reenviar | §6.3 | ❌ | P3 | |
| Atajos de apps ("Connect to apps") | §6.3 | ➖ | — | |
| Navegar mensajes con el teclado y atajos de letra | §14 | 🟡 | P3 | Alt + ↑ ↓ entre conversaciones, Alt + Shift + ↑ ↓ entre no leídas, Esc / Shift + Esc marcan leído, Ctrl + / lista los atajos. Falta moverse entre mensajes. |

## Hilos

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Panel de hilo con raíz, respuestas y compositor | §7 | ✅ | — | |
| "Also send to #canal" | §7 | ✅ | — | Casilla bajo el compositor del hilo (`reply_broadcast`); se apaga al enviar. |
| Vista Threads (todos tus hilos) | §10.5 | ✅ | — | Raíz y últimas respuestas; abrir el hilo desde ahí. |
| Seguir o dejar de seguir un hilo | §6.3 | ❌ | P3 | |
| Resumir el hilo (IA) | §7 | ➖ | — | |

## Compositor

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Intro envía, Shift + Intro nueva línea | §8 | ✅ | — | |
| Invertir Intro y Ctrl + Intro | §13 | ✅ | — | En Preferencias. |
| Botones B / I / S / código | §8 | ✅ | — | Insertan marcas mrkdwn. |
| Subrayado, enlace, listas, cita, bloque de código | §8 | 🟡 | P3 | Listas numeradas y con viñetas, cita y bloque de código. Sin subrayado (mrkdwn no lo tiene) ni botón de enlace (Slack enlaza solo las URLs). Aa oculta la barra. |
| Atajos Ctrl + B / I / U, Shift + Ctrl + X… | §14 | ✅ | — | Ctrl + B / I, Ctrl + Shift + X / C / 7 / 8 / 9, Ctrl + Alt + Shift + C. |
| Edición WYSIWYG (formato visible al escribir) | §8 | ➖ | — | El clon edita con marcas, que es la opción "Format messages with markup" de Slack. |
| Autocompletar `@` personas | §8.2 | ✅ | — | |
| Autocompletar `#` canales | §8.2 | ✅ | — | |
| Autocompletar `:` emoji | §8.2 | ✅ | — | |
| Comandos `/` | §8.2 | ❌ | P3 | `/remind` y las apps viven del lado del servidor (`chat.command`). |
| Subir archivos (+, Ctrl + O, arrastrar) | §8 | 🟡 | P3 | + abre el selector y se puede arrastrar a la ventana; el texto va como comentario. Sin Ctrl + O. |
| Snippet, canvas, lista, workflow | §8 | ❌ | P3 | |
| Clips de audio y video | §8 | ➖ | — | |
| Programar el envío | §8 | 🟡 | P3 | ▾ junto a enviar: mañana o el lunes a las 9 (`chat.scheduleMessage`). Sin hora a elección ni la lista de programados. |
| Borrador por conversación, sincronizado | servidor §7 | 🟡 | P3 | Por conversación y por hilo, con ✎ en la lateral; vive en memoria, sin sincronizar con `drafts.list`. |
| "Unsend" con Ctrl + Z | §14 | ❌ | P3 | |
| No perder el texto si el envío falla | — | ✅ | — | El texto vuelve al compositor si no escribiste otro. |
| Placeholder "Message #canal" | §8 | ✅ | — | |

## Búsqueda

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Ctrl + K para saltar a una conversación | §9 | ✅ | — | En local, no leídos primero. |
| Búsqueda de mensajes con resultados | §9 | ✅ | — | `search.messages`, una página. Los modificadores `in:` / `from:` pasan tal cual. |
| Filtros (From, In, tipo, fecha, Has a link…) | §9 | ❌ | P3 | |
| Historial de búsquedas | §9 | ❌ | P3 | |

## Tiempo real y estado

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Mensajes nuevos, editados, borrados, respuestas, reacciones en vivo | servidor §4 | ✅ | — | Con `rtm.connect`. |
| Reconexión | servidor §4 | ✅ | — | Recupera lo perdido; sin respuesta en 75 s se da por caída; reintentos a 5, 10, 20, 40 y 60 s. |
| "Escribiendo…" | servidor §7 | ✅ | — | En los dos sentidos: se ve quién escribe y se avisa cuando escribes tú (cada 3 s como mucho). |
| Presencia | servidor §7 | ✅ | — | |
| Leído en otro dispositivo (`channel_marked`) | servidor §7 | ✅ | — | `channel_marked`, `im_marked`, `group_marked`, `mpim_marked`. |
| Marcar como leído al ver | servidor §7 | ✅ | — | Solo si hay algo sin leer y el workspace está a la vista. |
| Notificaciones de escritorio | §13 | ✅ | — | DMs y menciones (`@here` incluido), por D-Bus; el clic abre el workspace y el canal. No avisa de lo que estás viendo, de lo silenciado ni con 🌙. |
| No molestar, focus mode, horario laboral | §13 | 🟡 | P3 | 🌙 pausa los avisos de este equipo; no cambia tu estado de No molestar en Slack. |
| Estado personalizado (emoji + texto) | §12 | ❌ | P3 | |

## Otras vistas

| Función | Slack | Clon | Prio |
|---|---|---|---|
| DMs: lista con vista previa del último mensaje | §10.1 | ✅ | — |
| Activity (menciones, reacciones, hilos) | §10.2 | 🟡 | P3 |
| Files / canvases / listas | §10.3 | 🟡 | P3 |
| To-dos / guardados / recordatorios | §10.4 | 🟡 | P3 |
| Huddles (audio y video) | §10.5 | ➖ | — |
| Directorio de personas y canales; explorar y unirse a canales | §10.5 | 🟡 | P3 |
| Perfil de una persona | §11 | ✅ | — |
| Ayuda, novedades, Slackbot AI | §2 | ➖ | — |

## Preferencias y apariencia

| Función | Slack | Clon | Prio | Notas |
|---|---|---|---|---|
| Tema Aubergine | §13 | ✅ | — | Si el workspace no tiene otro. |
| Otros temas, modo oscuro, tema propio | §13 | ✅ | — | El tema de cada workspace (`ia_theme`) o uno elegido en Preferencias; modo oscuro. Sin brillo ni colores hex propios. |
| Densidad Clean / Compact | §13 | ✅ | — | Cómoda y Compacta (sin avatares, hora a la izquierda). |
| Reloj de 24 horas | §13 | ✅ | — | Como en Slack (`time24`), o 12 h / 24 h fijos en Preferencias. |
| Tamaño de texto (Ctrl + + / -) | §14 | ✅ | — | Escala toda la interfaz, como la app de escritorio; Ctrl + 0 vuelve al normal. Se recuerda. |
| Idioma | §13 | ➖ | — | El clon está en español. |

## Lo que el oficial hace y el clon no debería copiar

- **Telemetría** (`slackb.com/traces`, `beacon/error`) y **experimentos A/B**
  (`experiments.getByUser`, 42 KB en cada arranque).
- Ofertas del plan, de IA y de temas (megaphone, `teams.trials.info`).
- 157 MB de caché del service worker por versión del cliente. Si hace falta caché,
  conviene uno chico y propio (ver [`slack-servidor.md`](slack-servidor.md) §5).
- Las ~70 llamadas de arranque para workflows, calendario, Salesforce, IA, etc.
