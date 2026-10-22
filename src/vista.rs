//! Vistas: barra superior, rail de workspaces, lateral, canal, hilo y compositor,
//! con la distribución del cliente oficial de Slack.

use chrono::{DateTime, Datelike, Local, NaiveDate};
use iced::widget::text::Span;
use iced::widget::{
    Column, Row, Space, button, column, container, image, mouse_area, opaque, rich_text, row,
    scrollable, space, span, stack, text, text_editor, text_input, tooltip,
};
use iced::{
    Alignment, Border, Color, ContentFit, Element, Fill, FillPortion, Font, Padding, border,
    keyboard,
};

use crate::espacio::{
    Autocompletado, Busqueda, Destino, DestinoEmoji, Detalles, Edicion, Espacio, Hilo, ID_BUSCADOR,
    ID_EDICION, ID_EMOJI, ID_SALTAR, Msg, Panel, Pestana, REACCIONES_RAPIDAS, Saltar,
    SelectorEmoji, Vista,
};
use crate::estilo::{self, *};
use crate::formato::{self, Bloque, Jumbo, Marcas, Segmento, Tipo};
use crate::slack::{Adjunto, Archivo, Canal, Mensaje};
use crate::sugerencias::Disparador;
use crate::{Global, Inicio, Slack, textos};

const ANCHO_RAIL: f32 = 70.0;
const ANCHO_LATERAL: f32 = 260.0;
const ANCHO_HILO: f32 = 400.0;
const ALTO_ENCABEZADO: f32 = 56.0;

const DIAS: [&str; 7] = [
    "lunes",
    "martes",
    "miércoles",
    "jueves",
    "viernes",
    "sábado",
    "domingo",
];
const MESES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

impl Slack {
    pub(super) fn view(&self) -> Element<'_, Global> {
        if self.espacios.is_empty() {
            return self.bienvenida();
        }
        let espacio = self.actual();
        let i = espacio.clave;
        let mut base = Column::new();
        if let Some(e) = &self.aviso {
            base = base.push(aviso_error(e, Global::CerrarAviso));
        }
        if espacio.sesion_vencida {
            let fila = row![
                text(format!(
                    "Slack cerró tu sesión en {}. Vuelve a entrar para seguir viéndolo.",
                    espacio.nombre_equipo()
                ))
                .size(13),
                space::horizontal(),
                button(text("Volver a iniciar sesión").size(13).font(NEGRITA))
                    .padding([4, 12])
                    .style(estilo::enviar(self.inicio == Inicio::Libre))
                    .on_press_maybe(
                        (self.inicio == Inicio::Libre).then_some(Global::IniciarSesion)
                    ),
            ]
            .align_y(Alignment::Center);
            base = base.push(
                container(fila)
                    .padding([6, 20])
                    .width(Fill)
                    .style(fondo(col().aviso_fondo)),
            );
        }
        let ayuda = tooltip(
            button(text("?").size(15).font(NEGRITA).color(col().barra_texto))
                .padding([2, 9])
                .style(|_, s| estilo::boton_rail(false, s))
                .on_press(Global::VerAtajos),
            etiqueta("Atajos de teclado  ·  Ctrl + /"),
            tooltip::Position::Left,
        );
        let barra = container(
            row![
                espacio.barra_superior().map(move |m| Global::Espacio(i, m)),
                ayuda
            ]
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
        .width(Fill)
        .style(fondo(col().barra));
        let base = base.push(barra).push(
            row![
                self.rail(),
                espacio.cuerpo().map(move |m| Global::Espacio(i, m))
            ]
            .height(Fill),
        );
        let capa = match (
            &espacio.saltar,
            &espacio.selector_emoji,
            &espacio.por_borrar,
        ) {
            (Some(s), _, _) => Some(espacio.ventana_saltar(s)),
            (None, Some(s), _) => Some(espacio.ventana_emoji(s)),
            (None, None, Some(_)) => Some(confirmar_borrar()),
            (None, None, None) => None,
        };
        let mut capas = stack![base];
        if let Some(c) = capa {
            capas = capas.push(c.map(move |m| Global::Espacio(i, m)));
        }
        if espacio.menu_avisos {
            capas = capas.push(espacio.menu_avisos().map(move |m| Global::Espacio(i, m)));
        }
        if espacio.menu_canal {
            capas = capas.push(espacio.menu_canal().map(move |m| Global::Espacio(i, m)));
        }
        if let Some(d) = espacio.detalles {
            capas = capas.push(
                espacio
                    .ventana_detalles(d)
                    .map(move |m| Global::Espacio(i, m)),
            );
        }
        if espacio.menu_recientes {
            capas = capas.push(espacio.menu_recientes().map(move |m| Global::Espacio(i, m)));
        }
        if self.menu_cuenta {
            capas = capas.push(self.menu_cuenta());
        }
        if self.ver_atajos {
            capas = capas.push(ventana_atajos());
        }
        if self.ver_preferencias {
            capas = capas.push(self.ventana_preferencias());
        }
        capas.into()
    }

    /// 🌙 del rail: pausa las notificaciones de escritorio de todos los workspaces.
    fn boton_pausa(&self) -> Element<'_, Global> {
        let pausado = self.ajustes.pausado;
        let ayuda = if pausado {
            "Notificaciones pausadas: clic para reanudar"
        } else {
            "Pausar notificaciones"
        };
        tooltip(
            button(container(text("🌙").size(18).font(EMOJI)).center(36))
                .padding(0)
                .style(move |_, s| estilo::boton_rail(pausado, s))
                .on_press(Global::Pausar),
            etiqueta(ayuda),
            tooltip::Position::Right,
        )
        .gap(6)
        .into()
    }

    /// Ctrl + ,: apariencia, mensajes y notificaciones. Cada cambio se aplica y se
    /// guarda al momento.
    fn ventana_preferencias(&self) -> Element<'_, Global> {
        use crate::Cambio;
        use crate::ajustes::Reloj;
        let a = &self.ajustes;
        let titulo = |t: &'static str| text(t).size(15).font(NEGRA);
        let opcion = |etiqueta: &'static str, elegida: bool, msg: Global| {
            button(text(etiqueta).size(13))
                .padding([5, 12])
                .style(move |_, s| estilo::opcion(elegida, s))
                .on_press(msg)
        };
        let si_no = |etiqueta: &'static str, valor: bool, msg: fn(bool) -> Cambio| {
            button(
                row![
                    text(if valor { "☑" } else { "☐" }).size(16),
                    text(etiqueta).size(14)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([4, 4])
            .style(estilo::sin_fondo)
            .on_press(Global::Ajustar(msg(!valor)))
        };
        let mut temas = Row::new().spacing(8);
        for (nombre, clave) in [
            ("Del workspace", None),
            ("Aubergine", Some("clasico")),
            ("Horchata", Some("horchata")),
            ("Ocean", Some("ocean")),
            ("Jade", Some("jade")),
            ("Indigo", Some("indigo")),
            ("Tomato", Some("tomato")),
        ] {
            let elegida = a.tema.as_deref() == clave;
            let muestra = match clave {
                None => col().barra,
                Some("clasico") => estilo::colores(&crate::paleta::aubergine(), false).barra,
                Some(p) => {
                    let t = crate::paleta::Tema {
                        primaria: p.into(),
                        resalte: "jade".into(),
                    };
                    estilo::colores(&crate::paleta::del_tema(Some(&t), a.oscuro), a.oscuro).barra
                }
            };
            temas = temas.push(
                button(
                    column![
                        container(Space::new())
                            .width(56)
                            .height(28)
                            .style(redondeado(muestra, 6.0)),
                        text(nombre).size(12),
                    ]
                    .spacing(4)
                    .align_x(Alignment::Center),
                )
                .padding(4)
                .style(move |_, s| estilo::opcion(elegida, s))
                .on_press(Global::Ajustar(Cambio::Tema(clave.map(String::from)))),
            );
        }
        let ventana = column![
            row![
                text("Preferencias").size(20).font(NEGRA),
                space::horizontal(),
                button(text("✕").size(15))
                    .padding([2, 8])
                    .style(estilo::boton_plano)
                    .on_press(Global::VerPreferencias),
            ]
            .align_y(Alignment::Center),
            linea_horizontal(col().borde),
            titulo("Apariencia"),
            row![
                opcion("Claro", !a.oscuro, Global::Ajustar(Cambio::Oscuro(false))),
                opcion("Oscuro", a.oscuro, Global::Ajustar(Cambio::Oscuro(true))),
            ]
            .spacing(6),
            temas.wrap().vertical_spacing(8),
            row![
                opcion(
                    "Cómoda",
                    !a.compacta,
                    Global::Ajustar(Cambio::Compacta(false))
                ),
                opcion(
                    "Compacta",
                    a.compacta,
                    Global::Ajustar(Cambio::Compacta(true))
                ),
                text(format!("Tamaño {:.0} %  ·  Ctrl + + / −", a.escala * 100.0))
                    .size(13)
                    .color(col().tenue),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            linea_horizontal(col().borde),
            titulo("Mensajes"),
            row![
                text("Hora").size(14).width(80),
                opcion(
                    "Como en Slack",
                    a.reloj == Reloj::Slack,
                    Global::Ajustar(Cambio::Reloj(Reloj::Slack))
                ),
                opcion(
                    "12 h",
                    a.reloj == Reloj::Doce,
                    Global::Ajustar(Cambio::Reloj(Reloj::Doce))
                ),
                opcion(
                    "24 h",
                    a.reloj == Reloj::Veinticuatro,
                    Global::Ajustar(Cambio::Reloj(Reloj::Veinticuatro))
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            si_no(
                "Intro hace salto de línea y Ctrl + Intro envía",
                a.ctrl_intro,
                Cambio::CtrlIntro
            ),
            linea_horizontal(col().borde),
            titulo("Notificaciones"),
            button(
                row![
                    text(if a.pausado { "☑" } else { "☐" }).size(16),
                    text("Pausar las notificaciones (🌙)").size(14)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([4, 4])
            .style(estilo::sin_fondo)
            .on_press(Global::Pausar),
            si_no(
                "Mostrar el texto del mensaje en la notificación",
                a.vista_previa,
                Cambio::VistaPrevia
            ),
        ]
        .spacing(12)
        .padding(24);
        container(opaque(
            container(scrollable(ventana))
                .width(620)
                .max_height(680)
                .style(estilo::tarjeta(col().borde, 12.0, true)),
        ))
        .center(Fill)
        .style(fondo(Color::from_rgba8(0, 0, 0, 0.4)))
        .into()
    }

    /// Tu avatar al pie del rail.
    fn boton_cuenta(&self) -> Element<'_, Global> {
        let e = self.actual();
        let clave = e.clave;
        let yo = e.sesion.as_ref().map(|s| s.user_id.as_str());
        button(e.avatar(yo, 36.0).map(move |m| Global::Espacio(clave, m)))
            .padding(0)
            .style(estilo::sin_fondo)
            .on_press(Global::MenuCuenta)
            .into()
    }

    /// Menú de tu avatar: quién eres, modo oscuro y cerrar sesión en este workspace.
    fn menu_cuenta(&self) -> Element<'_, Global> {
        let e = self.actual();
        let nombre = e
            .sesion
            .as_ref()
            .map(|s| e.nombre_usuario(&s.user_id).to_string())
            .unwrap_or_default();
        let opcion = |etiqueta: String, msg: Global| {
            button(text(etiqueta).size(14))
                .padding([6, 12])
                .width(Fill)
                .style(|_, s| estilo::opcion(false, s))
                .on_press(msg)
        };
        let oscuro = if self.ajustes.oscuro {
            "✓  Modo oscuro"
        } else {
            "Modo oscuro"
        };
        let menu = column![
            container(
                column![
                    text(nombre).size(15).font(NEGRA),
                    text(e.nombre_equipo().to_string())
                        .size(13)
                        .color(col().tenue),
                ]
                .spacing(2)
            )
            .padding([8, 12]),
            linea_horizontal(col().borde),
            opcion(format!("{oscuro}    Ctrl + Shift + Y"), Global::ModoOscuro),
            opcion("Preferencias    Ctrl + ,".into(), Global::VerPreferencias),
            opcion("Atajos de teclado    Ctrl + /".into(), Global::VerAtajos),
            linea_horizontal(col().borde),
            opcion(
                format!("Cerrar sesión en {}", e.nombre_equipo()),
                Global::CerrarSesion(e.clave)
            ),
        ]
        .padding(4);
        container(opaque(container(menu).width(280).style(estilo::tarjeta(
            col().borde,
            8.0,
            true,
        ))))
        .padding(Padding::default().left(ANCHO_RAIL + 4.0).bottom(8.0))
        .align_bottom(Fill)
        .into()
    }

    fn rail(&self) -> Element<'_, Global> {
        let varios = self.espacios.len() > 1;
        let mut espacios = Column::new().spacing(8).align_x(Alignment::Center);
        for (i, e) in self.espacios.iter().enumerate() {
            espacios = espacios.push(self.boton_espacio(i, e, varios));
        }
        let i = self.actual().clave;
        let columna = column![
            espacios,
            self.boton_agregar(),
            self.actual()
                .navegacion()
                .map(move |m| Global::Espacio(i, m)),
            space::vertical(),
            self.boton_pausa(),
            self.boton_cuenta(),
        ]
        .spacing(16)
        .align_x(Alignment::Center)
        .padding([10, 0]);
        container(columna)
            .width(ANCHO_RAIL)
            .height(Fill)
            .align_x(Alignment::Center)
            .style(fondo(col().barra))
            .into()
    }

    /// Pantalla sin sesiones: un botón que abre Chrome para entrar a Slack.
    fn bienvenida(&self) -> Element<'_, Global> {
        let boton = match self.inicio {
            Inicio::Libre => button(text("Iniciar sesión en Slack").size(15).font(NEGRITA))
                .padding([10, 20])
                .style(estilo::enviar(true))
                .on_press(Global::IniciarSesion),
            Inicio::Esperando => button(text("Esperando a que entres en Chrome…").size(15))
                .padding([10, 20])
                .style(estilo::enviar(false)),
        };
        let mut tarjeta = column![
            text("slack-rs").size(28).font(NEGRA).color(col().texto),
            text(
                "Se abre una ventana de Chrome: entra a tu workspace como siempre y se \
                 cierra sola. La sesión queda en ~/.config/slack-rs/ y luego puedes \
                 sumar otro workspace con el + del rail."
            )
            .size(14)
            .color(col().tenue)
            .align_x(Alignment::Center),
            boton,
        ]
        .spacing(18)
        .max_width(420)
        .align_x(Alignment::Center);
        if let Some(e) = &self.aviso {
            tarjeta = tarjeta.push(text(e.as_str()).size(13).color(col().peligro));
        }
        container(
            container(tarjeta)
                .padding(32)
                .style(estilo::tarjeta(col().borde, 12.0, true)),
        )
        .center(Fill)
        .style(fondo(col().barra))
        .into()
    }

    /// El + del rail: inicia sesión en otro workspace sin cerrar los abiertos.
    fn boton_agregar(&self) -> Element<'_, Global> {
        let esperando = self.inicio == Inicio::Esperando;
        let boton = button(
            container(
                text(if esperando { "…" } else { "+" })
                    .size(20)
                    .color(col().barra_texto),
            )
            .center(36),
        )
        .padding(0)
        .style(move |_, s| estilo::boton_rail(esperando, s))
        .on_press_maybe((!esperando).then_some(Global::IniciarSesion));
        let ayuda = if esperando {
            "Entra a Slack en la ventana de Chrome"
        } else {
            "Agregar un workspace"
        };
        tooltip(boton, etiqueta(ayuda), tooltip::Position::Right)
            .gap(6)
            .into()
    }

    /// Ícono del workspace en el rail. Con varios, el activo lleva un anillo blanco y
    /// los demás una insignia de menciones o un punto si tienen algo sin leer.
    fn boton_espacio<'a>(&'a self, i: usize, e: &'a Espacio, varios: bool) -> Element<'a, Global> {
        if !varios {
            return e.icono_equipo(36.0);
        }
        let activo = i == self.activo;
        let anillo = container(e.icono_equipo(36.0))
            .padding(3)
            .style(estilo::anillo(activo));
        let (menciones, no_leidos) = e.pendientes();
        let marca: Element<'a, Global> = if activo {
            Space::new().into()
        } else if menciones > 0 {
            insignia(menciones)
        } else if no_leidos {
            container(Space::new())
                .width(10)
                .height(10)
                .style(redondeado(col().barra_texto, 5.0))
                .into()
        } else {
            Space::new().into()
        };
        let caja = stack![
            container(anillo).center(48),
            container(marca).align_right(48).height(48),
        ];
        let boton = button(caja)
            .padding(0)
            .style(estilo::sin_fondo)
            .on_press(Global::Activar(i));
        let nombre = container(
            text(format!("{}  ·  Ctrl+{}", e.nombre_equipo(), i + 1))
                .size(13)
                .font(NEGRITA)
                .color(col().texto_globo),
        )
        .padding([4, 8])
        .style(redondeado(col().globo, 6.0));
        tooltip(boton, nombre, tooltip::Position::Right)
            .gap(6)
            .into()
    }
}

impl Espacio {
    pub(super) fn cuerpo(&self) -> Element<'_, Msg> {
        let mut cuerpo = row![self.lateral(), self.principal()];
        if let Some(u) = &self.perfil {
            cuerpo = cuerpo
                .push(linea_vertical(col().borde))
                .push(self.panel_perfil(u));
        } else if let Some(h) = &self.hilo {
            cuerpo = cuerpo
                .push(linea_vertical(col().borde))
                .push(self.panel_hilo(h));
        }
        cuerpo.into()
    }

    fn nombre_equipo(&self) -> &str {
        self.equipo
            .as_ref()
            .map(|e| e.name.as_str())
            .or(self.sesion.as_ref().map(|s| s.team.as_str()))
            .unwrap_or("Slack")
    }

    /// Barra superior: ← → 🕘 junto al buscador, centrado como en Slack, y el estado
    /// del tiempo real. El fondo y el botón de ayuda los pone `Slack::view`.
    fn barra_superior(&self) -> Element<'_, Msg> {
        let texto = col().barra_texto;
        let tenue = Color { a: 0.7, ..texto };
        let boton = |simbolo: &'static str, ayuda: &'static str, msg: Msg| {
            tooltip(
                button(text(simbolo).size(16).color(texto))
                    .padding([2, 8])
                    .style(|_, s| estilo::boton_rail(false, s))
                    .on_press(msg),
                etiqueta(ayuda),
                tooltip::Position::Bottom,
            )
        };
        let navegar = row![
            boton("←", "Atrás  ·  Alt + ←", Msg::Atras),
            boton("→", "Adelante  ·  Alt + →", Msg::Adelante),
            boton("◷", "Recientes", Msg::MenuRecientes),
        ]
        .spacing(2);
        let buscador = text_input(&format!("Buscar en {}", self.nombre_equipo()), &self.filtro)
            .id(ID_BUSCADOR)
            .on_input(Msg::Filtro)
            .on_submit(Msg::Buscar)
            .padding([5, 12])
            .size(13)
            .style(estilo::buscador);
        let (punto, estado) = if self.tiempo_real {
            (AVATARES[0], "En vivo")
        } else {
            (tenue, "Sin conexión en vivo")
        };
        row![
            container(navegar)
                .padding(Padding::default().right(10.0))
                .align_right(FillPortion(1)),
            container(buscador).width(FillPortion(2)).max_width(720),
            row![
                text("●").size(10).color(punto),
                text(estado).size(12).color(tenue)
            ]
            .spacing(6)
            .padding(Padding::default().left(12.0))
            .width(FillPortion(1))
            .align_y(Alignment::Center),
        ]
        .align_y(Alignment::Center)
        .into()
    }

    /// Menú 🕘: las conversaciones visitadas, la más reciente primero. Va alineado con
    /// el botón usando las mismas proporciones que la barra.
    fn menu_recientes(&self) -> Element<'_, Msg> {
        let mut lista = Column::new().push(
            container(text("Recientes").size(12).font(NEGRITA).color(col().tenue)).padding([6, 10]),
        );
        let recientes = self.recientes();
        if recientes.is_empty() {
            lista = lista
                .push(container(text("Nada todavía").size(14).color(col().tenue)).padding([5, 10]));
        }
        for id in recientes.into_iter().take(12) {
            lista = lista.push(
                button(text(self.etiqueta_canal(id)).size(14))
                    .padding([5, 10])
                    .width(Fill)
                    .style(|_, s| estilo::opcion(false, s))
                    .on_press(Msg::Seleccionar(id.to_string())),
            );
        }
        let menu = container(lista.padding(4))
            .width(260)
            .style(estilo::tarjeta(col().borde, 8.0, true));
        row![
            container(opaque(menu))
                .padding(Padding::default().top(40.0).right(10.0))
                .align_right(FillPortion(1)),
            container(Space::new()).width(FillPortion(2)).max_width(720),
            Space::new().width(FillPortion(1)),
        ]
        .into()
    }

    fn icono_equipo<'a, M: 'a>(&'a self, tam: f32) -> Element<'a, M> {
        match &self.icono_equipo {
            Some(h) => image(h.clone())
                .width(tam)
                .height(tam)
                .border_radius(8)
                .content_fit(ContentFit::Cover)
                .into(),
            None => container(
                text(inicial(self.nombre_equipo()))
                    .font(NEGRA)
                    .size(tam * 0.5)
                    .color(col().barra),
            )
            .center(tam)
            .style(redondeado(col().barra_texto, 8.0))
            .into(),
        }
    }

    /// Inicio y MD, debajo de los workspaces en el rail.
    fn navegacion(&self) -> Element<'_, Msg> {
        let nav = |simbolo: &'static str, etiqueta: &'static str, vista: Vista| {
            let activo = self.vista == vista;
            column![
                button(container(text(simbolo).size(18)).center(36))
                    .padding(0)
                    .style(move |_, s| estilo::boton_rail(activo, s))
                    .on_press(Msg::Vista(vista)),
                text(etiqueta)
                    .size(11)
                    .color(col().barra_texto)
                    .font(NEGRITA),
            ]
            .spacing(3)
            .align_x(Alignment::Center)
        };
        let panel = |simbolo: &'static str, etiqueta: &'static str, p: Panel| {
            let activo = self.panel == Some(p);
            column![
                button(container(text(simbolo).size(18)).center(36))
                    .padding(0)
                    .style(move |_, s| estilo::boton_rail(activo, s))
                    .on_press(Msg::AbrirPanel(p)),
                text(etiqueta)
                    .size(11)
                    .color(col().barra_texto)
                    .font(NEGRITA),
            ]
            .spacing(3)
            .align_x(Alignment::Center)
        };
        column![
            nav("⌂", "Inicio", Vista::Inicio),
            nav("✉", "MD", Vista::Directos),
            panel("🔔", "Actividad", Panel::Actividad),
            panel("🗂", "Archivos", Panel::Archivos),
        ]
        .spacing(16)
        .align_x(Alignment::Center)
        .into()
    }

    fn lateral(&self) -> Element<'_, Msg> {
        if self.vista == Vista::Directos {
            return self.lateral_directos();
        }
        let icono = |simbolo: &'static str, msg: Msg, activo: bool| {
            button(container(text(simbolo).size(16).color(col().lateral_fuerte)).center(28))
                .padding(0)
                .style(move |_, s| estilo::boton_rail(activo, s))
                .on_press(msg)
        };
        let encabezado = container(
            row![
                text(self.nombre_equipo())
                    .font(NEGRA)
                    .size(18)
                    .color(col().lateral_fuerte)
                    .wrapping(text::Wrapping::None),
                text("▾").size(13).color(col().lateral_fuerte),
                space::horizontal(),
                tooltip(
                    icono("⚙", Msg::MenuLateral, self.menu_lateral),
                    etiqueta("Filtrar y ordenar"),
                    tooltip::Position::Bottom,
                ),
                tooltip(
                    icono("✎", Msg::AbrirSaltar, false),
                    etiqueta("Mensaje nuevo"),
                    tooltip::Position::Bottom,
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([10, 12])
        .width(Fill);
        let buscar = container(
            text_input("Buscar conversación…", &self.buscar_conversacion)
                .on_input(Msg::BuscarConversacion)
                .padding([4, 10])
                .size(13)
                .style(estilo::buscador_lateral),
        )
        .padding([6, 12]);

        let mut lista = Column::new().spacing(1).padding([8, 8]);
        for (icono, etiqueta, p) in [
            ("🧵", "Hilos", Panel::Hilos),
            ("📝", "Borradores y enviados", Panel::Borradores),
            ("🔖", "Guardados", Panel::Guardados),
            ("📇", "Directorio", Panel::Directorio),
        ] {
            let activo = self.panel == Some(p);
            lista = lista.push(
                button(
                    row![
                        text(icono).size(14).font(EMOJI).width(20),
                        text(etiqueta).size(15)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding([4, 12])
                .width(Fill)
                .style(move |_, s| estilo::item_lateral(activo, s))
                .on_press(Msg::AbrirPanel(p)),
            );
        }
        lista = lista.push(Space::new().height(12));
        for g in self.secciones_lateral() {
            let plegada = self.plegadas.contains(&g.id);
            let flecha = if plegada { "▸" } else { "▾" };
            lista = lista.push(
                button(
                    row![text(flecha).size(12).width(20)]
                        .push(g.emoji.map(|e| text(e).size(14).font(EMOJI)))
                        .push(text(g.titulo).size(15))
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .padding([5, 12])
                .width(Fill)
                .style(|_, s| estilo::item_lateral(false, s))
                .on_press(Msg::Plegar(g.id)),
            );
            if plegada {
                continue;
            }
            for c in g.canales {
                lista = lista.push(self.item_canal(c, self.nombre_canal(&c.id)));
            }
            lista = lista.push(Space::new().height(12));
        }

        let contenido = column![
            encabezado,
            buscar,
            linea_horizontal(col().lateral_linea),
            scrollable(lista).style(estilo::scroll_lateral).height(Fill),
        ];
        let contenido: Element<'_, Msg> = if self.menu_lateral {
            let menu = container(opaque(self.menu_lateral()))
                .padding(Padding::default().top(44.0).right(8.0))
                .align_right(Fill);
            stack![contenido, menu].into()
        } else {
            contenido.into()
        };
        container(contenido)
            .width(ANCHO_LATERAL)
            .height(Fill)
            .clip(true)
            .style(fondo(col().lateral))
            .into()
    }

    /// Menú ⚙ de la lateral: qué conversaciones mostrar y en qué orden.
    fn menu_lateral(&self) -> Element<'_, Msg> {
        use crate::lateral::{Filtro, Orden};
        let opcion = |etiqueta: &'static str, elegida: bool, msg: Msg| {
            button(
                row![
                    text(if elegida { "✓" } else { "" })
                        .size(13)
                        .width(16)
                        .color(col().exito),
                    text(etiqueta).size(14),
                ]
                .spacing(6),
            )
            .padding([5, 10])
            .width(Fill)
            .style(|_, s| estilo::opcion(false, s))
            .on_press(msg)
        };
        let titulo = |t: &'static str| {
            container(text(t).size(12).font(NEGRITA).color(col().tenue)).padding([6, 10])
        };
        let mut menu = Column::new().push(titulo("Mostrar"));
        for (filtro, etiqueta) in [
            (Filtro::Activos, "Activas (30 días)"),
            (Filtro::NoLeidos, "No leídas"),
            (Filtro::Menciones, "Menciones"),
            (Filtro::Todos, "Todas"),
        ] {
            let elegida = self.filtro_lateral == filtro;
            menu = menu.push(opcion(etiqueta, elegida, Msg::FiltrarLateral(filtro)));
        }
        menu = menu
            .push(linea_horizontal(col().borde))
            .push(titulo("Ordenar"));
        for (orden, etiqueta) in [
            (Orden::Alfabetico, "A–Z"),
            (Orden::Reciente, "Más reciente"),
        ] {
            let elegida = self.orden_lateral == orden;
            menu = menu.push(opcion(etiqueta, elegida, Msg::OrdenarLateral(orden)));
        }
        menu = menu.push(linea_horizontal(col().borde)).push(opcion(
            "Ocultar silenciadas",
            self.ocultar_silenciados,
            Msg::AlternarSilenciados,
        ));
        container(menu.padding(4))
            .width(236)
            .style(estilo::tarjeta(col().borde, 8.0, true))
            .into()
    }

    /// Pestaña *MD*: DMs y grupos por actividad, con hora y vista previa, como el
    /// panel "Mensajes directos" del cliente oficial.
    fn lateral_directos(&self) -> Element<'_, Msg> {
        let encabezado = container(text("Mensajes directos").font(NEGRA).size(18))
            .padding([14, 16])
            .width(Fill);
        let ahora = Local::now();
        let filtro = self.filtro.to_lowercase();
        let mut lista = Column::new();
        for c in self.directos_recientes() {
            let nombre = self.nombre_canal(&c.id);
            if !nombre.to_lowercase().contains(&filtro) {
                continue;
            }
            let activo = self.seleccionado.as_deref() == Some(c.id.as_str());
            let pendiente = self.contadores.get(&c.id).is_some_and(|n| n.no_leidos);
            let hora = self
                .actividad
                .get(&c.id)
                .and_then(|s| DateTime::from_timestamp(*s, 0).filter(|_| *s > 0))
                .map(|t| hora_corta(t.with_timezone(&Local), ahora))
                .unwrap_or_default();
            let icono = match c.user.as_deref() {
                Some(u) => self.con_presencia(Some(u), 36.0),
                None => container(text("👥").size(18).font(EMOJI)).center(36).into(),
            };
            let previa = self.vista_previa(&c.id).unwrap_or_default();
            let tenue = if activo {
                col().texto_seleccion
            } else {
                col().tenue
            };
            let fila = row![
                icono,
                column![
                    row![
                        text(nombre)
                            .size(15)
                            .font(if pendiente { NEGRA } else { NEGRITA })
                            .wrapping(text::Wrapping::None),
                        space::horizontal(),
                        text(hora).size(12).color(tenue),
                    ]
                    .align_y(Alignment::Center),
                    text(previa)
                        .size(14)
                        .color(tenue)
                        .wrapping(text::Wrapping::None),
                ]
                .spacing(2)
                .clip(true),
            ]
            .spacing(10);
            lista = lista
                .push(
                    button(fila)
                        .padding([10, 16])
                        .width(Fill)
                        .style(move |_, e| estilo::opcion(activo, e))
                        .on_press(Msg::Seleccionar(c.id.clone())),
                )
                .push(linea_horizontal(col().borde));
        }
        container(column![
            encabezado,
            linea_horizontal(col().borde),
            scrollable(lista).height(Fill),
        ])
        .width(ANCHO_LATERAL + 60.0)
        .height(Fill)
        .clip(true)
        .style(fondo(col().sobre))
        .into()
    }

    fn item_canal(&self, c: &Canal, nombre: String) -> Element<'_, Msg> {
        let activo = self.panel.is_none() && self.seleccionado.as_deref() == Some(c.id.as_str());
        let contador = self.contadores.get(&c.id).copied().unwrap_or_default();
        let silenciado = self.silenciado(&c.id);
        let pendiente = !activo && ((contador.no_leidos && !silenciado) || contador.menciones > 0);
        let color = if activo {
            col().texto_seleccion
        } else if pendiente {
            col().lateral_fuerte
        } else if silenciado {
            Color {
                a: 0.4,
                ..col().lateral_fuerte
            }
        } else {
            col().lateral_texto
        };
        let fuente = if pendiente { NEGRA } else { LATO };

        let icono: Element<'_, Msg> = if c.is_im {
            self.con_presencia(c.user.as_deref(), 20.0)
        } else if c.is_mpim {
            let miembros = c.name.as_deref().map_or(0, |n| n.matches("--").count() + 1);
            container(
                text(miembros.to_string())
                    .size(11)
                    .font(NEGRITA)
                    .color(col().lateral_fuerte),
            )
            .center(20)
            .style(redondeado(
                Color {
                    a: 0.2,
                    ..col().lateral_fuerte
                },
                4.0,
            ))
            .into()
        } else {
            let (simbolo, fuente) = if c.is_private {
                ("🔒", EMOJI)
            } else {
                ("#", LATO)
            };
            container(text(simbolo).size(15).font(fuente).color(color))
                .center_x(20)
                .into()
        };

        let mut fila = row![
            icono,
            text(nombre)
                .size(15)
                .font(fuente)
                .color(color)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let borrador = self.tiene_borrador(&c.id);
        if borrador || (contador.menciones > 0 && !activo) {
            fila = fila.push(space::horizontal());
        }
        if borrador {
            fila = fila.push(text("✎").size(14).color(color));
        }
        if contador.menciones > 0 && !activo {
            fila = fila.push(insignia(contador.menciones));
        }
        button(fila)
            .padding([4, 12])
            .width(Fill)
            .style(move |_, s| estilo::item_lateral(activo, s))
            .on_press(Msg::Seleccionar(c.id.clone()))
            .into()
    }

    /// Avatar con el punto de presencia: verde en línea, hueco ausente.
    fn con_presencia(&self, usuario: Option<&str>, tam: f32) -> Element<'_, Msg> {
        let Some(u) = usuario else {
            return self.avatar(None, tam);
        };
        let lado = (tam * 0.4).max(8.0);
        let presente = self.presente(u);
        let punto = container(Space::new())
            .width(lado)
            .height(lado)
            .style(move |_| container::Style {
                background: presente.then(|| AVATARES[0].into()),
                border: Border {
                    color: if presente {
                        col().lateral
                    } else {
                        col().lateral_texto
                    },
                    width: if presente { 2.0 } else { 1.5 },
                    radius: (lado / 2.0).into(),
                },
                ..container::Style::default()
            });
        stack![
            self.avatar(Some(u), tam),
            container(punto)
                .width(tam + lado / 3.0)
                .height(tam + lado / 3.0)
                .align_right(tam + lado / 3.0)
                .align_bottom(tam + lado / 3.0),
        ]
        .into()
    }

    fn avatar(&self, usuario: Option<&str>, tam: f32) -> Element<'_, Msg> {
        let radio = (tam * 0.22).round();
        if let Some(h) = usuario.and_then(|u| self.avatares.get(u)) {
            return image(h.clone())
                .width(tam)
                .height(tam)
                .border_radius(radio)
                .content_fit(ContentFit::Cover)
                .into();
        }
        let nombre = usuario.map(|u| self.nombre_usuario(u)).unwrap_or("?");
        let color = AVATARES
            [usuario.map_or(0, |u| u.bytes().map(usize::from).sum::<usize>()) % AVATARES.len()];
        container(
            text(inicial(nombre))
                .size(tam * 0.5)
                .font(NEGRITA)
                .color(BLANCO),
        )
        .center(tam)
        .style(redondeado(color, radio))
        .into()
    }

    fn principal(&self) -> Element<'_, Msg> {
        let contenido: Element<'_, Msg> = match (&self.busqueda, &self.seleccionado) {
            (Some(b), _) => self.resultados_busqueda(b),
            _ if self.panel.is_some() => self.vista_panel(self.panel.unwrap_or(Panel::Hilos)),
            (None, None) => {
                let aviso = match (&self.error, &self.cliente) {
                    (Some(e), None) => format!(
                        "{e}\n\nDefine SLACK_TOKEN (xoxc-…) y SLACK_COOKIE (xoxd-…) \
                         o inicia sesión con el + del rail"
                    ),
                    _ => "Elige un canal o un mensaje directo".into(),
                };
                container(
                    text(aviso)
                        .size(15)
                        .color(col().tenue)
                        .align_x(Alignment::Center),
                )
                .center(Fill)
                .into()
            }
            (None, Some(canal)) => {
                let cuerpo: Element<'_, Msg> = match self.pestana {
                    Pestana::Mensajes => column![
                        self.lista_mensajes(canal),
                        self.compositor(
                            &self.editor,
                            Destino::Canal,
                            format!("Enviar mensaje a {}", self.etiqueta_canal(canal)),
                        ),
                    ]
                    .into(),
                    Pestana::Archivos => self.lista_archivos(canal),
                    Pestana::Fijados => self.lista_fijados(canal),
                };
                column![
                    self.encabezado_canal(canal),
                    linea_horizontal(col().borde),
                    cuerpo
                ]
                .into()
            }
        };

        let mut columna = Column::new();
        if let Some(e) = self.error.as_ref().filter(|_| self.cliente.is_some()) {
            columna = columna.push(aviso_error(e, Msg::CerrarError));
        }
        if let Some(n) = &self.nota {
            columna = columna.push(aviso(n, Msg::CerrarNota, col().mencion_fondo));
        }
        container(columna.push(contenido))
            .width(Fill)
            .height(Fill)
            .style(fondo(col().fondo))
            .into()
    }

    /// Selector de emoji: buscador y cuadrícula de 8 columnas.
    fn ventana_emoji<'a>(&'a self, s: &'a SelectorEmoji) -> Element<'a, Msg> {
        let opciones = self.opciones_emoji();
        let primero = opciones.first().map(|o| o.nombre.clone());
        let campo = text_input("Buscar emoji", &s.consulta)
            .id(ID_EMOJI)
            .on_input(Msg::EmojiConsulta)
            .on_submit_maybe(primero.map(Msg::ElegirEmoji))
            .padding([8, 12])
            .size(15);
        let mut cuadricula = Column::new().spacing(2);
        for fila in opciones.chunks(8) {
            let botones = fila.iter().map(|o| {
                let glifo: Element<'a, Msg> = match &o.unicode {
                    Some(u) => text(u.clone()).size(22).font(EMOJI).into(),
                    None => self.emoji(&o.nombre, 18.0),
                };
                tooltip(
                    button(container(glifo).center(32))
                        .padding(2)
                        .style(estilo::boton_plano)
                        .on_press(Msg::ElegirEmoji(o.nombre.clone())),
                    container(
                        text(format!(":{}:", o.nombre))
                            .size(12)
                            .color(col().texto_globo),
                    )
                    .padding([2, 6])
                    .style(redondeado(col().globo, 4.0)),
                    tooltip::Position::Bottom,
                )
                .into()
            });
            cuadricula = cuadricula.push(Row::with_children(botones).spacing(2));
        }
        let titulo = if opciones.is_empty() {
            "Sin coincidencias"
        } else if s.consulta.is_empty() {
            "Frecuentes"
        } else {
            "Resultados"
        };
        let caja = container(
            column![
                campo,
                text(titulo).size(13).font(NEGRITA).color(col().tenue),
                scrollable(cuadricula).height(260),
            ]
            .spacing(8),
        )
        .padding(12)
        .width(320)
        .style(tarjeta(col().borde, 8.0, true));
        capa_modal(caja.into())
    }

    /// Ctrl+K: velo oscuro sobre toda la ventana y una caja arriba al centro.
    fn ventana_saltar<'a>(&'a self, s: &'a Saltar) -> Element<'a, Msg> {
        let campo = text_input("Saltar a…", &s.consulta)
            .id(ID_SALTAR)
            .on_input(Msg::SaltarConsulta)
            .on_submit(Msg::SaltarElegir)
            .padding([10, 14])
            .size(16);
        let mut lista = Column::new().spacing(2);
        for (n, id) in self.opciones_saltar().into_iter().take(10).enumerate() {
            let canal = self.canales.iter().find(|c| c.id == id);
            let icono: Element<'a, Msg> = match canal {
                Some(c) if c.is_im => self.avatar(c.user.as_deref(), 20.0),
                Some(c) if c.is_private => text("🔒").size(13).font(EMOJI).into(),
                Some(c) if c.is_mpim => text("👥").size(13).font(EMOJI).into(),
                _ => text("#").size(16).into(),
            };
            let no_leido = self.contadores.get(id).is_some_and(|c| c.no_leidos);
            let nombre =
                text(self.nombre_canal(id))
                    .size(15)
                    .font(if no_leido { NEGRITA } else { LATO });
            let elegido = n == s.elegido;
            lista = lista.push(
                button(
                    row![container(icono).width(22), nombre]
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .width(Fill)
                .padding([6, 12])
                .style(move |_, estado| estilo::opcion(elegido, estado))
                .on_press(Msg::Seleccionar(id.to_string())),
            );
        }
        let caja = container(column![campo, lista].spacing(8))
            .padding(12)
            .width(560)
            .style(tarjeta(col().borde, 8.0, true));
        capa_modal(caja.into())
    }

    fn resultados_busqueda<'a>(&'a self, b: &'a Busqueda) -> Element<'a, Msg> {
        let total = match &b.resultados {
            Some(r) => format!("{} resultados", r.len()),
            None => String::new(),
        };
        let encabezado = row![
            text(format!("Resultados de «{}»", b.consulta))
                .size(18)
                .font(NEGRA),
            text(total).size(13).color(col().tenue),
            space::horizontal(),
            button(text("✕").size(16))
                .padding([4, 8])
                .style(estilo::boton_plano)
                .on_press(Msg::CerrarBusqueda),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([0, 20])
        .height(ALTO_ENCABEZADO);

        let cuerpo: Element<'a, Msg> = match &b.resultados {
            None => cargando(),
            Some(r) if r.is_empty() => container(
                text("No hay mensajes que coincidan")
                    .size(15)
                    .color(col().tenue),
            )
            .center(Fill)
            .into(),
            Some(r) => {
                let mut columna = Column::new().padding([8, 0]).spacing(4);
                for c in r {
                    let canal = &c.channel.id;
                    let dia = dia(&c.mensaje).map(etiqueta_dia).unwrap_or_default();
                    let origen = button(
                        text(format!("{} · {dia}", self.etiqueta_canal(canal)))
                            .size(13)
                            .font(NEGRITA)
                            .color(col().tenue),
                    )
                    .padding([2, 20])
                    .style(estilo::boton_plano)
                    .on_press(Msg::Seleccionar(canal.clone()));
                    columna = columna
                        .push(origen)
                        .push(self.mensaje(canal, &c.mensaje, false, true))
                        .push(linea_horizontal(col().borde));
                }
                scrollable(columna).height(Fill).width(Fill).into()
            }
        };
        column![encabezado, linea_horizontal(col().borde), cuerpo].into()
    }

    /// `#general` para canales, el nombre a secas para DMs.
    fn etiqueta_canal(&self, id: &str) -> String {
        let nombre = self.nombre_canal(id);
        match self.canales.iter().find(|c| c.id == id) {
            Some(c) if c.es_directo() => nombre,
            _ => format!("#{nombre}"),
        }
    }

    /// Encabezado del canal: ★, nombre ▾ (detalles), miembros, 🔍 y ⋮; debajo las
    /// pestañas y los marcadores.
    fn encabezado_canal(&self, id: &str) -> Element<'_, Msg> {
        let canal = self.canales.iter().find(|c| c.id == id);
        let icono: Element<'_, Msg> = match canal {
            Some(c) if c.is_im => self.avatar(c.user.as_deref(), 24.0),
            Some(c) if c.is_mpim => Space::new().into(),
            Some(c) if c.is_private => text("🔒").size(16).font(EMOJI).into(),
            _ => text("#").size(18).font(NEGRA).into(),
        };
        let destacado = self.destacado(id);
        let nombre = button(
            row![
                icono,
                text(self.nombre_canal(id)).size(18).font(NEGRA),
                text("▾").size(12).color(col().tenue),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([4, 6])
        .style(estilo::boton_plano)
        .on_press(Msg::AbrirDetalles(Detalles::Acerca));
        let accion = |simbolo: &'static str, ayuda: &'static str, msg: Msg| {
            tooltip(
                button(text(simbolo).size(16).color(col().tenue))
                    .padding([4, 8])
                    .style(estilo::boton_plano)
                    .on_press(msg),
                etiqueta(ayuda),
                tooltip::Position::Bottom,
            )
        };
        let estrella = button(text(if destacado { "★" } else { "☆" }).size(16).color(
            if destacado {
                Color::from_rgb8(0xe8, 0xa4, 0x00)
            } else {
                col().tenue
            },
        ))
        .padding([2, 4])
        .style(estilo::boton_plano)
        .on_press(Msg::Destacar(id.to_string()));
        let ayuda_estrella = if destacado {
            "Quitar de Destacados"
        } else {
            "Destacar"
        };
        let mut fila = Row::new()
            .spacing(4)
            .align_y(Alignment::Center)
            .push(tooltip(
                estrella,
                etiqueta(ayuda_estrella),
                tooltip::Position::Bottom,
            ))
            .push(nombre)
            .push(space::horizontal());
        let directo = canal.is_some_and(|c| c.is_im);
        if let Some(n) = self
            .info
            .get(id)
            .and_then(|i| i.miembros)
            .filter(|_| !directo)
        {
            let mut caras = Row::new().spacing(2);
            for u in self.miembros.get(id).into_iter().flatten().take(3) {
                caras = caras.push(self.avatar(Some(u), 20.0));
            }
            fila = fila.push(
                button(row![caras, text(n.to_string()).size(14).font(NEGRITA)].spacing(8))
                    .padding([4, 8])
                    .style(estilo::boton_plano)
                    .on_press(Msg::AbrirDetalles(Detalles::Miembros)),
            );
        }
        let campana = match self.nivel_aviso(id) {
            crate::slack::NivelAviso::Silencio => "🔕",
            _ => "🔔",
        };
        fila = fila
            .push(accion(
                campana,
                "Avisos de la conversación",
                Msg::MenuAvisos,
            ))
            .push(accion(
                "🔍",
                "Buscar en la conversación",
                Msg::BuscarEnCanal,
            ))
            .push(accion("⋮", "Más", Msg::MenuCanal));

        let pestana = |etiqueta: &'static str, p: Pestana| {
            let activa = self.pestana == p;
            let raya = container(Space::new())
                .width(Fill)
                .height(2)
                .style(fondo(if activa {
                    col().exito
                } else {
                    Color::TRANSPARENT
                }));
            button(
                column![
                    text(etiqueta)
                        .size(13)
                        .font(if activa { NEGRITA } else { LATO })
                        .color(if activa { col().texto } else { col().tenue }),
                    raya
                ]
                .spacing(6)
                .width(iced::Shrink),
            )
            .padding(Padding::default().top(4.0).left(8.0).right(8.0))
            .style(estilo::sin_fondo)
            .on_press(Msg::Pestana(p))
        };
        let mut pestanas = row![
            pestana("Mensajes", Pestana::Mensajes),
            pestana("Archivos", Pestana::Archivos),
            pestana("Fijados", Pestana::Fijados),
        ]
        .spacing(4)
        .align_y(Alignment::End);
        for m in self.marcadores.get(id).into_iter().flatten() {
            let emoji = (!m.emoji.is_empty()).then(|| formato::emoji(&m.emoji));
            let etiqueta = match emoji {
                Some(e) => format!("{e} {}", m.titulo),
                None => format!("🔗 {}", m.titulo),
            };
            pestanas = pestanas.push(
                button(
                    text(etiqueta)
                        .size(13)
                        .color(col().tenue)
                        .wrapping(text::Wrapping::None),
                )
                .padding([4, 8])
                .style(estilo::boton_plano)
                .on_press(Msg::AbrirEnlace(m.enlace.clone())),
            );
        }
        column![
            container(fila).padding([8, 14]),
            container(pestanas)
                .padding(Padding::default().left(12.0))
                .clip(true),
        ]
        .into()
    }

    /// Pestaña Archivos: lo compartido en el canal, el más nuevo primero.
    fn lista_archivos(&self, canal: &str) -> Element<'_, Msg> {
        match self.archivos_canal.get(canal) {
            None => cargando(),
            Some(l) if l.is_empty() => vacio("Nadie ha compartido archivos aquí"),
            Some(l) => self.filas_archivos(l),
        }
    }

    /// Archivos con autor, fecha y tipo; un clic los abre en Slack.
    fn filas_archivos<'a>(&'a self, lista: &'a [Archivo]) -> Element<'a, Msg> {
        let mut columna = Column::new().padding([8, 12]).spacing(2);
        for f in lista {
            let nombre = f.title.clone().or(f.name.clone()).unwrap_or_default();
            let tipo = f.pretty_type.clone().unwrap_or_default();
            let quien = f
                .user
                .as_deref()
                .map(|u| self.nombre_usuario(u))
                .unwrap_or("");
            let cuando = DateTime::from_timestamp(f.created, 0)
                .map(|t| etiqueta_dia(t.with_timezone(&Local).date_naive()))
                .unwrap_or_default();
            let fila = row![
                container(text("▤").size(18).color(BLANCO))
                    .center(36)
                    .style(redondeado(col().enlace, 6.0)),
                column![
                    text(nombre).size(15).font(NEGRITA),
                    text(format!("{quien} · {cuando} · {tipo}"))
                        .size(13)
                        .color(col().tenue),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center);
            let mut boton = button(fila)
                .padding([8, 8])
                .width(Fill)
                .style(|_, s| estilo::opcion(false, s));
            if let Some(url) = &f.permalink {
                boton = boton.on_press(Msg::AbrirEnlace(url.clone()));
            }
            columna = columna.push(boton);
        }
        scrollable(columna).height(Fill).width(Fill).into()
    }

    /// El centro con un panel de la lateral o del rail.
    fn vista_panel(&self, panel: Panel) -> Element<'_, Msg> {
        let (titulo, cuerpo): (&str, Element<'_, Msg>) = match panel {
            Panel::Hilos => ("Hilos", self.panel_hilos()),
            Panel::Actividad => ("Actividad", self.panel_actividad()),
            Panel::Borradores => ("Borradores y programados", self.panel_borradores()),
            Panel::Guardados => ("Guardados", self.panel_guardados()),
            Panel::Archivos => (
                "Archivos",
                match &self.archivos_equipo {
                    None => cargando(),
                    Some(l) if l.is_empty() => vacio("No hay archivos"),
                    Some(l) => self.filas_archivos(l),
                },
            ),
            Panel::Directorio => ("Directorio", self.panel_directorio()),
        };
        let encabezado = container(text(titulo).size(18).font(NEGRA))
            .padding([0, 20])
            .center_y(ALTO_ENCABEZADO);
        column![encabezado, linea_horizontal(col().borde), cuerpo].into()
    }

    /// "#canal · fecha" sobre un mensaje suelto; un clic abre el canal.
    fn origen<'a>(&self, canal: &str, extra: String) -> Element<'a, Msg> {
        button(
            text(format!("{}{extra}", self.etiqueta_canal(canal)))
                .size(13)
                .font(NEGRITA)
                .color(col().tenue),
        )
        .padding([2, 20])
        .style(estilo::boton_plano)
        .on_press(Msg::Seleccionar(canal.to_string()))
        .into()
    }

    fn panel_hilos(&self) -> Element<'_, Msg> {
        let Some(hilos) = &self.hilos_vista else {
            return cargando();
        };
        if hilos.is_empty() {
            return vacio("No sigues ningún hilo todavía");
        }
        let mut columna = Column::new().padding([8, 0]).spacing(4);
        for h in hilos {
            columna = columna
                .push(self.origen(&h.canal, String::new()))
                .push(self.mensaje(&h.canal, &h.raiz, false, false));
            for r in &h.respuestas {
                columna = columna.push(self.mensaje(&h.canal, r, false, true));
            }
            columna = columna.push(linea_horizontal(col().borde));
        }
        scrollable(columna).height(Fill).width(Fill).into()
    }

    fn panel_actividad(&self) -> Element<'_, Msg> {
        let Some(items) = &self.bandeja else {
            return cargando();
        };
        if items.is_empty() {
            return vacio("Nada nuevo por aquí");
        }
        let mut columna = Column::new().padding([8, 0]).spacing(4);
        for i in items {
            let que = match i.tipo.as_str() {
                "thread_v2" => "Hilo",
                "at_channel" | "at_everyone" => "@canal",
                "at_user_group" => "Mención a tu grupo",
                "keyword" => "Palabra clave",
                _ => "Mención",
            };
            let marca = if i.no_leido { "● " } else { "" };
            columna = columna.push(self.origen(&i.canal, format!(" · {marca}{que}")));
            if let Some(m) = self.suelto(&i.canal, &i.ts) {
                columna = columna.push(self.mensaje(&i.canal, m, false, false));
            }
            columna = columna.push(linea_horizontal(col().borde));
        }
        scrollable(columna).height(Fill).width(Fill).into()
    }

    fn panel_guardados(&self) -> Element<'_, Msg> {
        let Some(ids) = &self.guardados else {
            return cargando();
        };
        if ids.is_empty() {
            return vacio("No has guardado nada");
        }
        let mut columna = Column::new().padding([8, 0]).spacing(4);
        for (canal, ts) in ids {
            columna = columna.push(self.origen(canal, String::new()));
            if let Some(m) = self.suelto(canal, ts) {
                columna = columna.push(self.mensaje(canal, m, false, true));
            }
            columna = columna.push(linea_horizontal(col().borde));
        }
        scrollable(columna).height(Fill).width(Fill).into()
    }

    fn panel_borradores(&self) -> Element<'_, Msg> {
        let Some(lista) = &self.borradores_slack else {
            return cargando();
        };
        if lista.is_empty() {
            return vacio("No tienes borradores ni mensajes programados");
        }
        let mut columna = Column::new().padding([8, 0]).spacing(4);
        for b in lista {
            let cuando = DateTime::from_timestamp(b.programado, 0)
                .filter(|_| b.programado > 0)
                .map(|t| {
                    let t = t.with_timezone(&Local);
                    format!(
                        " · Programado: {} a las {}",
                        etiqueta_dia(t.date_naive()),
                        t.format(self.formato_hora())
                    )
                })
                .unwrap_or_else(|| " · Borrador".into());
            columna = columna.push(self.origen(&b.canal, cuando));
            for bloque in
                formato::del_mensaje(&b.mensaje, &self.usuarios, |id| self.nombre_canal(id))
            {
                columna = columna.push(container(self.bloque(bloque)).padding([2, 20]));
            }
            columna = columna.push(linea_horizontal(col().borde));
        }
        scrollable(columna).height(Fill).width(Fill).into()
    }

    fn panel_directorio(&self) -> Element<'_, Msg> {
        let buscar = container(
            text_input(
                "Buscar personas por nombre o cargo",
                &self.consulta_personas,
            )
            .on_input(Msg::BuscarPersonas)
            .padding([6, 12])
            .size(14)
            .style(estilo::buscador_lateral),
        )
        .padding([12, 20]);
        let lista: Element<'_, Msg> = match &self.personas {
            None => cargando(),
            Some(l) if l.is_empty() => vacio("Nadie coincide"),
            Some(l) => {
                let mut columna = Column::new().padding([0, 12]).spacing(2);
                for u in l {
                    let titulo = u.profile.title.clone().unwrap_or_default();
                    let fila = row![
                        self.con_presencia(Some(&u.id), 40.0),
                        button(
                            column![
                                text(u.nombre_visible()).size(15).font(NEGRITA),
                                text(titulo).size(13).color(col().tenue),
                            ]
                            .spacing(2),
                        )
                        .padding(0)
                        .width(Fill)
                        .style(estilo::sin_fondo)
                        .on_press(Msg::VerPerfil(u.id.clone())),
                        button(text("Mensaje").size(13))
                            .padding([4, 12])
                            .style(estilo::boton_plano)
                            .on_press(Msg::AbrirDm(u.id.clone())),
                    ]
                    .spacing(12)
                    .padding([6, 8])
                    .align_y(Alignment::Center);
                    columna = columna.push(fila);
                }
                scrollable(columna).height(Fill).width(Fill).into()
            }
        };
        column![buscar, lista].into()
    }

    /// Perfil de una persona en el panel derecho: foto, nombre, cargo, estado y su
    /// hora local.
    fn panel_perfil<'a>(&'a self, id: &'a str) -> Element<'a, Msg> {
        let u = self.usuarios.get(id);
        let nombre = self.nombre_usuario(id).to_string();
        let mut datos = column![
            self.avatar(Some(id), 160.0),
            text(nombre).size(22).font(NEGRA)
        ]
        .spacing(10);
        if let Some(t) = u
            .and_then(|u| u.profile.title.clone())
            .filter(|t| !t.is_empty())
        {
            datos = datos.push(text(t).size(15).color(col().tenue));
        }
        let estado = u
            .map(|u| {
                let emoji = u
                    .profile
                    .status_emoji
                    .as_deref()
                    .map(|e| formato::emoji(e.trim_matches(':')))
                    .unwrap_or_default();
                format!(
                    "{emoji} {}",
                    u.profile.status_text.clone().unwrap_or_default()
                )
            })
            .filter(|e| !e.trim().is_empty());
        if let Some(e) = estado {
            datos = datos.push(text(e).size(14));
        }
        let presencia = if self.presente(id) {
            "● En línea"
        } else {
            "○ Ausente"
        };
        datos = datos.push(text(presencia).size(13).color(col().tenue));
        if let Some(u) = u.filter(|u| u.tz.is_some()) {
            let local = chrono::Utc::now() + chrono::Duration::seconds(u.tz_offset);
            datos = datos.push(
                text(format!("{} hora local", local.format(self.formato_hora())))
                    .size(13)
                    .color(col().tenue),
            );
        }
        datos = datos.push(
            button(text("Mensaje").size(14).font(NEGRITA))
                .padding([6, 16])
                .style(estilo::enviar(true))
                .on_press(Msg::AbrirDm(id.to_string())),
        );
        let encabezado = row![
            text("Perfil").size(18).font(NEGRA),
            space::horizontal(),
            button(text("✕").size(16))
                .padding([4, 8])
                .style(estilo::boton_plano)
                .on_press(Msg::CerrarPerfil),
        ]
        .align_y(Alignment::Center)
        .padding([0, 16])
        .height(ALTO_ENCABEZADO);
        container(column![
            encabezado,
            linea_horizontal(col().borde),
            scrollable(datos.padding(20)).height(Fill),
        ])
        .width(ANCHO_HILO)
        .height(Fill)
        .style(fondo(col().fondo))
        .into()
    }

    /// Pestaña Fijados: los mensajes fijados del canal.
    fn lista_fijados<'a>(&'a self, canal: &'a str) -> Element<'a, Msg> {
        let Some(lista) = self.fijados.get(canal) else {
            return cargando();
        };
        if lista.is_empty() {
            return vacio("No hay mensajes fijados");
        }
        let mut columna = Column::new().padding([8, 0]).spacing(4);
        for m in lista {
            columna = columna
                .push(self.mensaje(canal, m, false, true))
                .push(linea_horizontal(col().borde));
        }
        scrollable(columna).height(Fill).width(Fill).into()
    }

    /// Menú 🔔 del encabezado: cuánto avisa esta conversación.
    pub(super) fn menu_avisos(&self) -> Element<'_, Msg> {
        use crate::slack::NivelAviso::*;
        let Some(canal) = self.seleccionado.clone() else {
            return Space::new().into();
        };
        let actual = self.nivel_aviso(&canal);
        let mut menu = column![
            container(
                text("Avisarte de…")
                    .size(12)
                    .font(NEGRITA)
                    .color(col().tenue)
            )
            .padding([6, 12])
        ];
        for (nivel, titulo, detalle) in [
            (Todo, "Todos los mensajes", "Cada mensaje nuevo"),
            (Menciones, "Solo menciones", "@tú, @channel, @here y DMs"),
            (
                Silencio,
                "Silenciar y ocultar",
                "Solo la insignia si te mencionan",
            ),
        ] {
            let marca = if actual == nivel { "✓" } else { "" };
            menu = menu.push(
                button(
                    row![
                        text(marca).size(13).width(16).color(col().exito),
                        column![
                            text(titulo).size(14),
                            text(detalle).size(12).color(col().tenue)
                        ]
                    ]
                    .spacing(6),
                )
                .padding([6, 10])
                .width(Fill)
                .style(|_, s| estilo::opcion(false, s))
                .on_press(Msg::AvisosCanal(canal.clone(), nivel)),
            );
        }
        let derecha = if self.hilo.is_some() || self.perfil.is_some() {
            ANCHO_HILO + 13.0
        } else {
            12.0
        };
        container(opaque(
            container(menu.padding(4))
                .width(280)
                .style(estilo::tarjeta(col().borde, 8.0, true)),
        ))
        .padding(Padding::default().top(88.0).right(derecha + 60.0))
        .align_right(Fill)
        .into()
    }

    /// Menú ⋮ del encabezado.
    pub(super) fn menu_canal(&self) -> Element<'_, Msg> {
        let opcion = |etiqueta: &'static str, msg: Msg| {
            button(text(etiqueta).size(14))
                .padding([6, 12])
                .width(Fill)
                .style(|_, s| estilo::opcion(false, s))
                .on_press(msg)
        };
        let mut menu = column![
            opcion(
                "Detalles de la conversación",
                Msg::AbrirDetalles(Detalles::Acerca)
            ),
            opcion("Buscar en la conversación", Msg::BuscarEnCanal),
        ];
        if self
            .seleccionado
            .as_deref()
            .and_then(|c| self.enlace_canal(c))
            .is_some()
        {
            menu = menu.push(opcion("Copiar enlace", Msg::CopiarEnlaceCanal));
        }
        let derecha = if self.hilo.is_some() {
            ANCHO_HILO + 13.0
        } else {
            12.0
        };
        container(opaque(
            container(menu.padding(4))
                .width(260)
                .style(estilo::tarjeta(col().borde, 8.0, true)),
        ))
        .padding(Padding::default().top(88.0).right(derecha))
        .align_right(Fill)
        .into()
    }

    /// Ventana de detalles: Acerca y Miembros.
    pub(super) fn ventana_detalles(&self, cual: Detalles) -> Element<'_, Msg> {
        let Some(id) = self.seleccionado.as_deref() else {
            return Space::new().into();
        };
        let info = self.info.get(id).cloned().unwrap_or_default();
        let miembros = self.miembros.get(id);
        let pestana = |etiqueta: String, d: Detalles| {
            let activa = cual == d;
            button(
                text(etiqueta)
                    .size(14)
                    .font(if activa { NEGRITA } else { LATO })
                    .color(if activa { col().texto } else { col().tenue }),
            )
            .padding([4, 10])
            .style(estilo::boton_plano)
            .on_press(Msg::AbrirDetalles(d))
        };
        let total = info
            .miembros
            .map(|n| n as usize)
            .or(miembros.map(Vec::len))
            .map(|n| format!("Miembros {n}"))
            .unwrap_or_else(|| "Miembros".into());
        let cuerpo: Element<'_, Msg> = match cual {
            Detalles::Acerca => {
                let campo = |titulo: &'static str, valor: String| {
                    column![
                        text(titulo).size(13).font(NEGRITA),
                        text(if valor.is_empty() {
                            "—".to_string()
                        } else {
                            valor
                        })
                        .size(14),
                    ]
                    .spacing(4)
                };
                let creado = DateTime::from_timestamp(info.creado, 0)
                    .filter(|_| info.creado > 0)
                    .map(|t| etiqueta_dia(t.with_timezone(&Local).date_naive()))
                    .unwrap_or_default();
                let por = info
                    .creador
                    .as_deref()
                    .map(|u| format!("{} el {creado}", self.nombre_usuario(u)))
                    .unwrap_or(creado);
                let avisos = if self.silenciado(id) {
                    "Silenciada"
                } else {
                    "Las de siempre"
                };
                let mut c = column![
                    campo("Tema", info.tema.clone()),
                    campo("Descripción", info.proposito.clone()),
                    campo("Creado por", por),
                    campo("Notificaciones", avisos.into()),
                ]
                .spacing(16);
                if let Some(enlace) = self.enlace_canal(id) {
                    c = c.push(
                        row![
                            campo("Enlace", enlace),
                            space::horizontal(),
                            button(text("Copiar").size(13))
                                .padding([4, 10])
                                .style(estilo::boton_plano)
                                .on_press(Msg::CopiarEnlaceCanal),
                        ]
                        .align_y(Alignment::Center),
                    );
                }
                scrollable(c.padding([4, 4])).into()
            }
            Detalles::Miembros => match miembros {
                None => cargando(),
                Some(ids) => {
                    let mut lista = Column::new().spacing(2);
                    for u in ids {
                        lista = lista.push(
                            row![
                                self.avatar(Some(u), 32.0),
                                text(self.nombre_usuario(u)).size(15)
                            ]
                            .spacing(10)
                            .align_y(Alignment::Center)
                            .padding([4, 4]),
                        );
                    }
                    scrollable(lista).into()
                }
            },
        };
        let ventana = column![
            row![
                text(self.etiqueta_canal(id)).size(20).font(NEGRA),
                space::horizontal(),
                button(text("✕").size(15))
                    .padding([2, 8])
                    .style(estilo::boton_plano)
                    .on_press(Msg::CerrarDetalles),
            ]
            .align_y(Alignment::Center),
            row![
                pestana("Acerca de".into(), Detalles::Acerca),
                pestana(total, Detalles::Miembros)
            ]
            .spacing(4),
            linea_horizontal(col().borde),
            cuerpo,
        ]
        .spacing(12)
        .padding(24);
        container(opaque(
            container(ventana)
                .width(560)
                .height(560)
                .style(estilo::tarjeta(col().borde, 12.0, true)),
        ))
        .center(Fill)
        .style(fondo(Color::from_rgba8(0, 0, 0, 0.4)))
        .into()
    }

    fn lista_mensajes<'a>(&'a self, canal: &'a str) -> Element<'a, Msg> {
        let Some(lista) = self.mensajes.get(canal) else {
            return cargando();
        };
        let mut columna = Column::new().padding([8, 0]);
        if self.cargando_mas.contains(canal) {
            columna = columna.push(
                container(
                    text("Cargando mensajes anteriores…")
                        .size(13)
                        .color(col().tenue),
                )
                .center_x(Fill)
                .padding(8),
            );
        } else if self.hay_mas.get(canal) == Some(&false) {
            columna = columna.push(
                container(
                    text(format!(
                        "Este es el principio de {}",
                        self.etiqueta_canal(canal)
                    ))
                    .size(15)
                    .font(NEGRITA),
                )
                .padding([12, 20]),
            );
        }
        let visibles: Vec<&Mensaje> = lista.iter().filter(|m| !m.es_respuesta_de_hilo()).collect();
        let union = |m: &Mensaje| m.subtype.as_deref() == Some("channel_join");
        let mut anterior: Option<&Mensaje> = None;
        let mut i = 0;
        while i < visibles.len() {
            let m = visibles[i];
            if anterior.is_none_or(|a| dia(a) != dia(m)) {
                columna = columna.push(separador_dia(dia(m)));
                anterior = None;
            }
            // Varios "se unió" seguidos del mismo día van en una línea.
            if union(m) {
                let tanda: Vec<&Mensaje> = visibles[i..]
                    .iter()
                    .take_while(|x| union(x) && dia(x) == dia(m))
                    .copied()
                    .collect();
                i += tanda.len();
                columna = columna.push(self.uniones(canal, &tanda));
                anterior = None;
                continue;
            }
            columna = columna.push(self.mensaje(canal, m, agrupa(anterior, m), false));
            anterior = Some(m);
            i += 1;
        }
        // Anclado abajo: el desplazamiento se mide desde abajo, así que agregar
        // mensajes viejos arriba no mueve lo que se está viendo.
        scrollable(columna)
            .anchor_bottom()
            .on_scroll(move |v| Msg::Desplazado(canal.to_string(), v.absolute_offset_reversed().y))
            .height(Fill)
            .width(Fill)
            .into()
    }

    /// "Ana y 3 más se unieron a #general." con el avatar del primero.
    fn uniones<'a>(&'a self, canal: &str, tanda: &[&'a Mensaje]) -> Element<'a, Msg> {
        let nombres: Vec<&str> = tanda
            .iter()
            .filter_map(|m| m.user.as_deref())
            .map(|u| self.nombre_usuario(u))
            .collect();
        let hora = tanda
            .last()
            .and_then(|m| fecha(m))
            .map(|t| t.format(self.formato_hora()).to_string())
            .unwrap_or_default();
        row![
            self.avatar(tanda[0].user.as_deref(), 36.0),
            text(textos::agrupar_union(&nombres, &self.nombre_canal(canal)))
                .size(15)
                .color(col().tenue),
            text(hora).size(12).color(col().tenue),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([6, 20])
        .into()
    }

    fn panel_hilo<'a>(&'a self, h: &'a Hilo) -> Element<'a, Msg> {
        let encabezado = row![
            column![
                text("Hilo").size(18).font(NEGRA),
                text(self.etiqueta_canal(&h.canal))
                    .size(13)
                    .color(col().tenue),
            ]
            .spacing(2),
            space::horizontal(),
            button(text("✕").size(16))
                .padding([4, 8])
                .style(estilo::boton_plano)
                .on_press(Msg::CerrarHilo),
        ]
        .align_y(Alignment::Center)
        .padding([0, 16])
        .height(ALTO_ENCABEZADO);

        let cuerpo: Element<'a, Msg> = match &h.mensajes {
            None => cargando(),
            Some(lista) => {
                let mut columna = Column::new().padding([8, 0]);
                let mut anterior: Option<&Mensaje> = None;
                for (i, m) in lista.iter().enumerate() {
                    if i == 1 {
                        columna = columna.push(separador_respuestas(lista.len() - 1));
                        anterior = None;
                    }
                    columna = columna.push(self.mensaje(&h.canal, m, agrupa(anterior, m), true));
                    anterior = Some(m);
                }
                scrollable(columna).height(Fill).width(Fill).into()
            }
        };

        let contenido = column![
            encabezado,
            linea_horizontal(col().borde),
            cuerpo,
            self.compositor(&h.editor, Destino::Hilo, "Responder…".into()),
        ];
        container(contenido)
            .width(ANCHO_HILO)
            .height(Fill)
            .style(fondo(col().fondo))
            .into()
    }

    fn mensaje<'a>(
        &'a self,
        canal: &'a str,
        m: &'a Mensaje,
        agrupado: bool,
        en_hilo: bool,
    ) -> Element<'a, Msg> {
        let sobre = self.sobre.as_deref() == Some(m.ts.as_str());
        let hora = fecha(m)
            .map(|t| t.format(self.formato_hora()).to_string())
            .unwrap_or_default();

        // Compacta: sin avatares, la hora a la izquierda y el nombre en cada mensaje.
        let compacta = self.ajustes.compacta;
        let agrupado = agrupado && !compacta;
        // En mensajes agrupados la hora aparece en el hueco del avatar al pasar el mouse.
        let lado: Element<'a, Msg> = if compacta {
            container(text(hora.clone()).size(11).color(col().tenue))
                .width(52)
                .padding(Padding::default().top(3.0))
                .into()
        } else if agrupado {
            let hora = if sobre { hora.clone() } else { String::new() };
            container(text(hora).size(11).color(col().tenue))
                .width(36)
                .padding(Padding {
                    top: 3.0,
                    ..Padding::ZERO
                })
                .align_x(Alignment::End)
                .into()
        } else {
            let avatar = self.avatar(m.user.as_deref(), 36.0);
            match m.user.clone() {
                Some(u) => button(avatar)
                    .padding(0)
                    .style(estilo::sin_fondo)
                    .on_press(Msg::VerPerfil(u))
                    .into(),
                None => avatar,
            }
        };

        let mut cuerpo = Column::new().spacing(4).width(Fill);
        if !agrupado {
            let autor = m
                .user
                .as_deref()
                .map(|u| self.nombre_usuario(u))
                .or(m.username.as_deref())
                .unwrap_or("bot");
            let autor: Element<'a, Msg> = match m.user.clone() {
                Some(u) => button(text(autor).size(15).font(NEGRA))
                    .padding(0)
                    .style(estilo::sin_fondo)
                    .on_press(Msg::VerPerfil(u))
                    .into(),
                None => text(autor).size(15).font(NEGRA).into(),
            };
            let mut cabeza = row![autor].spacing(8).align_y(Alignment::Center);
            if !compacta {
                cabeza = cabeza.push(text(hora).size(12).color(col().tenue));
            }
            cuerpo = cuerpo.push(cabeza);
        }
        let editando = self
            .edicion
            .as_ref()
            .filter(|ed| ed.canal == canal && ed.ts == m.ts);
        let bloques = match editando {
            Some(ed) => {
                cuerpo = cuerpo.push(editor_en_linea(ed));
                Vec::new()
            }
            // Un bot que solo manda adjuntos se ve en sus tarjetas, no en el `fallback`.
            None if m.text.is_empty() && m.attachments.iter().any(|a| a.tiene_contenido()) => {
                Vec::new()
            }
            None => formato::del_mensaje(m, &self.usuarios, |id| self.nombre_canal(id)),
        };
        match self.jumbo(&bloques) {
            Some(grande) => cuerpo = cuerpo.push(grande),
            None => {
                for b in bloques {
                    cuerpo = cuerpo.push(self.bloque(b));
                }
            }
        }
        for a in m.attachments.iter().filter(|a| a.tiene_contenido()) {
            cuerpo = cuerpo.push(self.adjunto(a));
        }
        let todas_imagenes = m.files.iter().all(|f| f.miniatura().is_some());
        if m.files.len() > 1 && todas_imagenes {
            cuerpo = cuerpo.push(self.galeria(&m.files));
        } else {
            for f in &m.files {
                cuerpo = cuerpo.push(self.archivo(f));
            }
        }
        if !m.reactions.is_empty() {
            cuerpo = cuerpo.push(self.reacciones(canal, m));
        }
        if !en_hilo && m.reply_count.is_some_and(|n| n > 0) {
            cuerpo = cuerpo.push(self.enlace_hilo(canal, m));
        }

        let relleno = Padding {
            top: if agrupado || compacta { 2.0 } else { 8.0 },
            bottom: 2.0,
            left: 20.0,
            right: 20.0,
        };
        let color = if self.me_menciona(m) {
            col().fila_mencion
        } else if sobre {
            col().sobre
        } else {
            col().fondo
        };
        let caja = container(row![lado, cuerpo].spacing(8))
            .padding(relleno)
            .width(Fill)
            .style(fondo(color));
        let contenido: Element<'a, Msg> = if sobre {
            let acciones = container(self.acciones(canal, m, en_hilo))
                .align_right(Fill)
                .padding(Padding {
                    right: 16.0,
                    ..Padding::ZERO
                });
            stack![caja, acciones].into()
        } else {
            caja.into()
        };
        mouse_area(contenido)
            .on_enter(Msg::Sobre(m.ts.clone()))
            .on_exit(Msg::Salir(m.ts.clone()))
            .into()
    }

    /// Imagen con su miniatura si ya bajó (clic la abre en Slack); lo demás, tarjeta.
    fn archivo<'a>(&self, f: &'a Archivo) -> Element<'a, Msg> {
        let (Some((_, ancho, alto)), Some(h)) = (f.miniatura(), self.miniaturas.get(&f.id)) else {
            return tarjeta_archivo(f);
        };
        // Slack no siempre manda el tamaño; entonces se usa una caja de 360 × 240.
        let (ancho, alto) = match (ancho, alto) {
            (0, _) | (_, 0) => (360.0, 240.0),
            (w, h) => {
                let escala = (360.0 / w as f32).min(1.0);
                (w as f32 * escala, h as f32 * escala)
            }
        };
        let imagen = image(h.clone())
            .width(ancho)
            .height(alto)
            .border_radius(8)
            .content_fit(ContentFit::Contain);
        let mut boton = button(imagen).padding(0).style(estilo::boton_plano);
        if let Some(url) = &f.permalink {
            boton = boton.on_press(Msg::AbrirEnlace(url.clone()));
        }
        boton.into()
    }

    /// Varias imágenes: "N archivos" y hasta tres miniaturas; la tercera lleva "+N"
    /// si hay más, como Slack.
    fn galeria<'a>(&'a self, archivos: &'a [Archivo]) -> Element<'a, Msg> {
        let mut fila = Row::new().spacing(6);
        let resto = archivos.len().saturating_sub(3);
        for (i, f) in archivos.iter().take(3).enumerate() {
            let cuadro: Element<'a, Msg> = match self.miniaturas.get(&f.id) {
                Some(h) => image(h.clone())
                    .width(150)
                    .height(112)
                    .border_radius(8)
                    .content_fit(ContentFit::Cover)
                    .into(),
                None => container(Space::new())
                    .width(150)
                    .height(112)
                    .style(redondeado(col().reaccion_fondo, 8.0))
                    .into(),
            };
            let cuadro: Element<'a, Msg> = if i == 2 && resto > 0 {
                stack![
                    cuadro,
                    container(text(format!("+{resto}")).size(24).font(NEGRA).color(BLANCO))
                        .center(Fill)
                        .style(redondeado(Color::from_rgba8(0, 0, 0, 0.45), 8.0)),
                ]
                .width(150)
                .height(112)
                .into()
            } else {
                cuadro
            };
            let mut boton = button(cuadro).padding(0).style(estilo::boton_plano);
            if let Some(url) = &f.permalink {
                boton = boton.on_press(Msg::AbrirEnlace(url.clone()));
            }
            fila = fila.push(boton);
        }
        column![
            text(format!("{} archivos", archivos.len()))
                .size(13)
                .color(col().tenue),
            fila
        ]
        .spacing(6)
        .into()
    }

    /// Tarjeta de un adjunto de bot: barra de color, título, texto y campos.
    fn adjunto<'a>(&'a self, a: &'a Adjunto) -> Element<'a, Msg> {
        let color = match a.color.as_deref() {
            Some("good") => Color::from_rgb8(0x2e, 0xb6, 0x7d),
            Some("warning") => Color::from_rgb8(0xec, 0xb2, 0x2e),
            Some("danger") => Color::from_rgb8(0xe0, 0x1e, 0x5a),
            Some(hex) => crate::paleta::rgba(hex)
                .map(|[r, g, b, _]| Color::from_rgb(r, g, b))
                .unwrap_or(col().borde),
            None => col().borde,
        };
        let mut cuerpo = Column::new().spacing(4);
        let parrafos = |texto: &str| {
            let mut c = Column::new().spacing(2);
            for b in formato::bloques(texto, &self.usuarios) {
                c = c.push(self.bloque(b));
            }
            c
        };
        if let Some(t) = a.pretext.as_deref().filter(|t| !t.is_empty()) {
            cuerpo = cuerpo.push(parrafos(t));
        }
        if let Some(t) = a.title.as_deref().filter(|t| !t.is_empty()) {
            let titulo = text(t).size(15).font(NEGRA);
            cuerpo = cuerpo.push(match &a.title_link {
                Some(url) => Element::from(
                    button(titulo.color(col().enlace))
                        .padding(0)
                        .style(estilo::sin_fondo)
                        .on_press(Msg::AbrirEnlace(url.clone())),
                ),
                None => titulo.into(),
            });
        }
        if let Some(t) = a.text.as_deref().filter(|t| !t.is_empty()) {
            cuerpo = cuerpo.push(parrafos(t));
        }
        let mut campos = Row::new().spacing(16);
        for c in &a.campos {
            let campo = column![
                text(c.titulo.as_str()).size(14).font(NEGRITA),
                parrafos(&c.valor)
            ]
            .spacing(2)
            .width(if c.corto {
                iced::Length::Fixed(200.0)
            } else {
                iced::Length::Fill
            });
            campos = campos.push(campo);
        }
        if !a.campos.is_empty() {
            cuerpo = cuerpo.push(campos.wrap().vertical_spacing(8));
        }
        if let Some(t) = a.footer.as_deref().filter(|t| !t.is_empty()) {
            cuerpo = cuerpo.push(text(t).size(12).color(col().tenue));
        }
        row![
            container(Space::new())
                .width(4)
                .height(Fill)
                .style(redondeado(color, 2.0)),
            cuerpo.width(Fill)
        ]
        .spacing(10)
        .height(iced::Shrink)
        .into()
    }

    /// Lista de sugerencias del autocompletado, sobre el compositor.
    fn lista_sugerencias<'a>(&'a self, a: &'a Autocompletado) -> Element<'a, Msg> {
        let titulo = match a.tipo {
            Disparador::Usuario => "Personas",
            Disparador::Canal => "Canales",
            Disparador::Emoji => "Emoji",
        };
        let mut lista = column![
            container(text(titulo).size(12).font(NEGRITA).color(col().tenue)).padding([4, 12])
        ];
        for (i, s) in a.opciones.iter().enumerate() {
            let icono: Element<'a, Msg> = match (a.tipo, &s.usuario) {
                (_, Some(u)) => self.avatar(Some(u), 20.0),
                (Disparador::Emoji, _) => self.emoji(&s.etiqueta, 16.0),
                _ => text("#").size(15).into(),
            };
            let etiqueta = match a.tipo {
                Disparador::Emoji => format!(":{}:", s.etiqueta),
                _ => s.etiqueta.clone(),
            };
            let elegida = i == a.elegido;
            lista = lista.push(
                button(
                    row![container(icono).width(22), text(etiqueta).size(14)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .width(Fill)
                .padding([4, 12])
                .style(move |_, e| estilo::opcion(elegida, e))
                .on_press(Msg::SugerenciaEn(i)),
            );
        }
        container(lista.padding([4, 4]))
            .width(Fill)
            .style(tarjeta(col().borde, 8.0, true))
            .into()
    }

    /// Un emoji por nombre: estándar con la fuente a color, propio con su imagen si
    /// ya bajó, y si no como `:nombre:`.
    fn emoji(&self, nombre: &str, tam: f32) -> Element<'_, Msg> {
        let unicode = formato::emoji(nombre);
        if !unicode.starts_with(':') {
            return text(unicode).size(tam).font(EMOJI).into();
        }
        let base = nombre.split("::").next().unwrap_or(nombre);
        match self.emoji_imagenes.get(base) {
            Some(h) => image(h.clone())
                .width(tam * 1.3)
                .height(tam * 1.3)
                .content_fit(ContentFit::Contain)
                .into(),
            None => text(unicode).size(tam * 0.85).color(col().tenue).into(),
        }
    }

    /// Mensaje que es solo emoji, sin formato: se pinta en grande como en Slack.
    fn jumbo(&self, bloques: &[Bloque]) -> Option<Element<'_, Msg>> {
        let [Bloque::Parrafo(segs)] = bloques else {
            return None;
        };
        let llano = segs
            .iter()
            .all(|s| s.tipo == Tipo::Normal && s.marcas == Marcas::default());
        if !llano {
            return None;
        }
        let texto: String = segs.iter().map(|s| s.texto.as_str()).collect();
        let piezas = formato::jumbo(&texto, |n| self.emoji_imagenes.contains_key(n))?;
        let fila = piezas.into_iter().map(|p| match p {
            Jumbo::Propio(n) => self.emoji(&n, 28.0),
            Jumbo::Unicode(u) => text(u).size(32).font(EMOJI).into(),
        });
        Some(
            Row::with_children(fila)
                .spacing(4)
                .align_y(Alignment::Center)
                .into(),
        )
    }

    fn bloque<'a>(&self, b: Bloque) -> Element<'a, Msg> {
        match b {
            Bloque::Parrafo(segs) => self.texto_rico(segs),
            // La primera capa del stack fija el alto; así la barra mide lo que el texto.
            Bloque::Cita(segs) => stack![
                container(self.texto_rico(segs)).padding(Padding {
                    left: 16.0,
                    ..Padding::ZERO
                }),
                container(Space::new().width(4).height(Fill)).style(redondeado(col().borde, 2.0)),
            ]
            .into(),
            Bloque::Codigo(codigo) => container(
                text(codigo)
                    .font(Font::MONOSPACE)
                    .size(12)
                    .color(col().texto),
            )
            .padding(8)
            .width(Fill)
            .style(tarjeta(col().codigo_borde, 4.0, false))
            .into(),
            Bloque::Lista { ordenada, items } => {
                let filas = items.into_iter().enumerate().map(|(i, segs)| {
                    let vineta = if ordenada {
                        format!("{}.", i + 1)
                    } else {
                        "•".into()
                    };
                    row![
                        container(text(vineta).size(15))
                            .width(22)
                            .align_x(Alignment::End),
                        self.texto_rico(segs)
                    ]
                    .spacing(6)
                    .into()
                });
                Column::with_children(filas).spacing(2).into()
            }
        }
    }

    /// Un párrafo de segmentos con estilo como `rich_text`.
    fn texto_rico<'a>(&self, segs: Vec<Segmento>) -> Element<'a, Msg> {
        let yo = self.mi_id();
        let mut spans: Vec<Span<'static, String>> = Vec::new();
        for mut s in segs {
            // Un enlace sin nombre se muestra corto, como Slack: 🔗 dominio/…/final.
            if let Tipo::Enlace(url) = &s.tipo
                && s.texto == *url
            {
                s.texto = format!("🔗 {}", textos::enlace_corto(url, 60));
            }
            let m = s.marcas;
            if m.codigo {
                spans.push(
                    span(s.texto)
                        .font(Font::MONOSPACE)
                        .size(13)
                        .color(col().codigo_texto)
                        .background(col().codigo_fondo)
                        .border(Border {
                            color: col().codigo_borde,
                            width: 1.0,
                            radius: 3.0.into(),
                        })
                        .strikethrough(m.tachado),
                );
                continue;
            }
            let fuente = Font {
                weight: if m.negrita {
                    NEGRITA.weight
                } else {
                    LATO.weight
                },
                style: if m.cursiva { CURSIVA.style } else { LATO.style },
                ..LATO
            };
            for (tramo, es_emoji) in formato::partir_emoji(&s.texto) {
                let base = span(tramo.to_string())
                    .size(15)
                    .font(if es_emoji { EMOJI } else { fuente })
                    .strikethrough(m.tachado);
                let tuya = match &s.tipo {
                    Tipo::Aviso => true,
                    Tipo::Persona(id) => Some(id.as_str()) == yo,
                    _ => false,
                };
                spans.push(match &s.tipo {
                    Tipo::Normal => base,
                    Tipo::Mencion | Tipo::Persona(_) | Tipo::Aviso if tuya => base
                        .color(col().mencion_tuya_texto)
                        .background(col().mencion_tuya_fondo)
                        .font(NEGRITA)
                        .border(border::rounded(3)),
                    Tipo::Mencion | Tipo::Persona(_) | Tipo::Aviso => base
                        .color(col().enlace)
                        .background(col().mencion_fondo)
                        .border(border::rounded(3)),
                    Tipo::Enlace(url) => base.color(col().enlace).link(url.clone()),
                });
            }
        }
        rich_text(spans)
            .on_link_click(Msg::AbrirEnlace)
            .line_height(1.4)
            .width(Fill)
            .into()
    }

    fn reacciones<'a>(&'a self, canal: &'a str, m: &'a Mensaje) -> Element<'a, Msg> {
        let yo = self.mi_id();
        let mut fila = Row::new().spacing(4);
        for r in &m.reactions {
            let mia = yo.is_some_and(|y| r.users.iter().any(|u| u == y));
            let contenido = row![
                self.emoji(&r.name, 14.0),
                text(r.count.to_string()).size(12).font(NEGRITA),
            ]
            .spacing(4)
            .align_y(Alignment::Center);
            let otros: Vec<&str> = r
                .users
                .iter()
                .filter(|u| Some(u.as_str()) != yo)
                .map(|u| self.nombre_usuario(u))
                .collect();
            let quienes = textos::quienes_reaccionaron(&otros, mia, &r.name);
            fila = fila.push(tooltip(
                button(contenido)
                    .padding([2, 8])
                    .style(move |_, s| estilo::reaccion(mia, s))
                    .on_press(Msg::Reaccionar {
                        canal: canal.to_string(),
                        ts: m.ts.clone(),
                        nombre: r.name.clone(),
                    }),
                etiqueta(quienes),
                tooltip::Position::Top,
            ));
        }
        fila = fila.push(tooltip(
            button(text("☺+").size(13).color(col().tenue))
                .padding([3, 8])
                .style(move |_, s| estilo::reaccion(false, s))
                .on_press(Msg::AbrirEmoji(DestinoEmoji::Reaccion {
                    canal: canal.to_string(),
                    ts: m.ts.clone(),
                })),
            etiqueta("Agregar reacción"),
            tooltip::Position::Top,
        ));
        fila.wrap().vertical_spacing(4).into()
    }

    fn enlace_hilo<'a>(&'a self, canal: &'a str, m: &'a Mensaje) -> Element<'a, Msg> {
        let n = m.reply_count.unwrap_or(0);
        let mut fila = Row::new().spacing(4).align_y(Alignment::Center);
        for u in m.reply_users.iter().take(4) {
            fila = fila.push(self.avatar(Some(u), 24.0));
        }
        let etiqueta = if n == 1 {
            "1 respuesta".to_string()
        } else {
            format!("{n} respuestas")
        };
        fila = fila.push(
            container(text(etiqueta).size(13).font(NEGRA).color(col().enlace)).padding(Padding {
                left: 4.0,
                ..Padding::ZERO
            }),
        );
        button(fila)
            .padding(4)
            .style(estilo::boton_plano)
            .on_press(Msg::AbrirHilo(canal.to_string(), m.ts.clone()))
            .into()
    }

    /// Barra flotante al pasar sobre un mensaje: reacciones rápidas y responder en hilo.
    fn acciones<'a>(&'a self, canal: &'a str, m: &'a Mensaje, en_hilo: bool) -> Element<'a, Msg> {
        let mut fila = Row::new().spacing(2).padding(2);
        if self.menu_mensaje.as_deref() == Some(m.ts.as_str()) {
            let opcion = |etiqueta: &'static str, msg: Msg| {
                button(text(etiqueta).size(13))
                    .padding([4, 8])
                    .style(estilo::boton_plano)
                    .on_press(msg)
            };
            let par = || (canal.to_string(), m.ts.clone());
            if self.enlace_mensaje(canal, m).is_some() {
                let (c, t) = par();
                fila = fila.push(opcion("Copiar enlace", Msg::CopiarEnlaceMensaje(c, t)));
            }
            let (c, t) = par();
            fila = fila.push(opcion("Copiar texto", Msg::CopiarTexto(c, t)));
            let (c, t) = par();
            fila = fila.push(opcion("Marcar como no leído", Msg::MarcarNoLeido(c, t)));
            let guardado = self.esta_guardado(canal, &m.ts);
            let (c, t) = par();
            fila = fila.push(opcion(
                if guardado {
                    "Quitar de guardados"
                } else {
                    "Guardar"
                },
                Msg::Guardar(c, t),
            ));
            let fijado = m.pinned_to.iter().any(|c| c == canal);
            let (c, t) = par();
            fila = fila.push(opcion(
                if fijado {
                    "Desfijar"
                } else {
                    "Fijar en el canal"
                },
                Msg::Fijar(c, t),
            ));
            fila = fila.push(opcion("✕", Msg::MenuMensaje(m.ts.clone())));
            return container(fila)
                .style(tarjeta(col().borde, 6.0, true))
                .into();
        }
        for nombre in REACCIONES_RAPIDAS {
            fila = fila.push(
                button(text(formato::emoji(nombre)).size(16).font(EMOJI))
                    .padding([2, 6])
                    .style(estilo::boton_plano)
                    .on_press(Msg::Reaccionar {
                        canal: canal.to_string(),
                        ts: m.ts.clone(),
                        nombre: nombre.to_string(),
                    }),
            );
        }
        fila = fila.push(
            button(text("😀").size(15).font(EMOJI))
                .padding([2, 6])
                .style(estilo::boton_plano)
                .on_press(Msg::AbrirEmoji(DestinoEmoji::Reaccion {
                    canal: canal.to_string(),
                    ts: m.ts.clone(),
                })),
        );
        let mio = m.user.is_some() && m.user.as_deref() == self.mi_id();
        if mio {
            let accion = |icono: &'static str, msg: Msg| {
                button(text(icono).size(14).font(EMOJI))
                    .padding([2, 6])
                    .style(estilo::boton_plano)
                    .on_press(msg)
            };
            fila = fila
                .push(accion(
                    "✏️",
                    Msg::EmpezarEdicion(canal.to_string(), m.ts.clone()),
                ))
                .push(accion(
                    "🗑️",
                    Msg::PedirBorrar(canal.to_string(), m.ts.clone()),
                ));
        }
        if !en_hilo {
            let raiz = m.thread_ts.clone().unwrap_or_else(|| m.ts.clone());
            fila = fila.push(
                button(text("↩").size(16))
                    .padding([2, 6])
                    .style(estilo::boton_plano)
                    .on_press(Msg::AbrirHilo(canal.to_string(), raiz)),
            );
        }
        fila = fila.push(
            button(text("⋮").size(16))
                .padding([2, 8])
                .style(estilo::boton_plano)
                .on_press(Msg::MenuMensaje(m.ts.clone())),
        );
        container(fila)
            .style(tarjeta(col().borde, 6.0, true))
            .into()
    }

    fn compositor<'a>(
        &'a self,
        contenido: &'a text_editor::Content,
        destino: Destino,
        placeholder: String,
    ) -> Element<'a, Msg> {
        let vacio = contenido.text().trim().is_empty();
        let sugerencias = self
            .autocompletado
            .as_ref()
            .filter(|a| a.destino == destino);
        let sugiriendo = sugerencias.is_some();
        let ctrl_intro = self.ajustes.ctrl_intro;
        use crate::atajos::Formato;
        let formato = |etiqueta: &'static str, fuente: Font, f: Formato| {
            button(text(etiqueta).size(14).font(fuente).color(col().tenue))
                .padding([2, 8])
                .style(estilo::boton_plano)
                .on_press(Msg::Formatear(destino, f))
        };
        let barra = row![
            formato("B", NEGRA, Formato::Negrita),
            formato("I", CURSIVA, Formato::Cursiva),
            formato("S", LATO, Formato::Tachado),
            formato("</>", Font::MONOSPACE, Formato::Codigo),
            text("|").size(14).color(col().borde),
            formato("1.", LATO, Formato::Numerada),
            formato("•", LATO, Formato::Vinetas),
            formato("❝", LATO, Formato::Cita),
            formato("```", Font::MONOSPACE, Formato::BloqueCodigo),
            space::horizontal(),
            button(text("😀").size(14).font(EMOJI))
                .padding([2, 8])
                .style(estilo::boton_plano)
                .on_press(Msg::AbrirEmoji(DestinoEmoji::Compositor(destino))),
        ]
        .spacing(2)
        .padding([4, 6]);

        let editor = text_editor(contenido)
            .placeholder(placeholder)
            .on_action(move |a| Msg::Editar(destino, a))
            .key_binding(move |tecla| enter_envia(destino, sugiriendo, vacio, ctrl_intro, tecla))
            .padding([4, 12])
            .size(15)
            .max_height(240)
            .style(estilo::editor);

        let adjuntos = self.adjuntos_de(destino);
        let listo = !vacio || !adjuntos.is_empty();
        let enviar = row![
            button(text("➤").size(14))
                .padding([4, 10])
                .style(estilo::enviar(listo))
                .on_press_maybe(listo.then_some(Msg::Enviar(destino))),
            tooltip(
                button(text("▾").size(12))
                    .padding([4, 6])
                    .style(estilo::enviar(!vacio))
                    .on_press_maybe((!vacio).then_some(Msg::MenuEnviar(destino))),
                etiqueta("Programar para más tarde"),
                tooltip::Position::Top,
            ),
        ]
        .spacing(1);
        let chico = |etiqueta: &'static str, msg: Msg| {
            button(text(etiqueta).size(14).color(col().tenue))
                .padding([2, 8])
                .style(estilo::boton_plano)
                .on_press(msg)
        };
        let arroba = Msg::Editar(
            destino,
            text_editor::Action::Edit(text_editor::Edit::Insert('@')),
        );
        let pie = row![
            tooltip(
                chico("+", Msg::ElegirArchivos(destino)),
                etiqueta("Adjuntar archivos (o arrástralos a la ventana)"),
                tooltip::Position::Top
            ),
            tooltip(
                chico("Aa", Msg::AlternarBarra),
                etiqueta("Mostrar u ocultar el formato"),
                tooltip::Position::Top
            ),
            tooltip(
                chico("@", arroba),
                etiqueta("Mencionar a alguien"),
                tooltip::Position::Top
            ),
            text(if ctrl_intro {
                "Ctrl + Intro: enviar"
            } else {
                "Mayús + Intro: nueva línea"
            })
            .size(11)
            .color(col().tenue),
            space::horizontal(),
            enviar,
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([4, 8]);

        let mut partes = Column::new();
        if self.barra_formato() {
            partes = partes.push(barra);
        }
        if !adjuntos.is_empty() {
            let mut fichas = Row::new().spacing(6);
            for r in adjuntos {
                let nombre = r
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                fichas = fichas.push(
                    container(
                        row![
                            text(format!("📎 {nombre}")).size(13),
                            button(text("✕").size(12))
                                .padding([0, 4])
                                .style(estilo::boton_plano)
                                .on_press(Msg::QuitarAdjunto(destino, r)),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .padding([3, 8])
                    .style(redondeado(col().reaccion_fondo, 6.0)),
                );
            }
            partes = partes.push(container(fichas.wrap().vertical_spacing(6)).padding([6, 12]));
        }
        partes = partes.push(editor);
        if let (Destino::Hilo, Some(h)) = (destino, &self.hilo) {
            let marca = if h.tambien { "☑" } else { "☐" };
            partes = partes.push(
                button(
                    row![
                        text(marca).size(15).color(col().tenue),
                        text(format!(
                            "Enviar también a {}",
                            self.etiqueta_canal(&h.canal)
                        ))
                        .size(13)
                        .color(col().tenue),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([2, 12])
                .style(estilo::sin_fondo)
                .on_press(Msg::AlternarTambien),
            );
        }
        let caja = container(partes.push(pie)).style(tarjeta(col().borde_fuerte, 8.0, false));
        let mut columna = Column::new().spacing(4);
        if let Some(a) = sugerencias {
            columna = columna.push(self.lista_sugerencias(a));
        }
        if self.menu_enviar == Some(destino) && !vacio {
            columna = columna.push(menu_programar(destino));
        }
        let quienes: Vec<&str> = match (destino, &self.seleccionado) {
            (Destino::Canal, Some(c)) => self
                .escribiendo_en(c)
                .into_iter()
                .map(|u| self.nombre_usuario(u))
                .collect(),
            _ => Vec::new(),
        };
        // La línea siempre ocupa su lugar para que el compositor no salte.
        let escribiendo = container(
            text(textos::escribiendo(&quienes))
                .size(12)
                .color(col().tenue),
        )
        .height(18)
        .padding(Padding::default().left(4.0));
        container(columna.push(caja).push(escribiendo))
            .padding(Padding {
                top: 0.0,
                bottom: 2.0,
                left: 20.0,
                right: 20.0,
            })
            .into()
    }
}

/// Intro envía; Mayús + Intro y el resto de teclas se comportan como siempre.
fn enter_envia(
    destino: Destino,
    sugiriendo: bool,
    vacio: bool,
    ctrl_intro: bool,
    tecla: text_editor::KeyPress,
) -> Option<text_editor::Binding<Msg>> {
    use keyboard::key::Named;
    let intro = matches!(tecla.key, keyboard::Key::Named(Named::Enter));
    let enfocado = matches!(tecla.status, text_editor::Status::Focused { .. });
    // ↑ con el compositor vacío edita el último mensaje propio, como en Slack.
    let arriba = matches!(tecla.key, keyboard::Key::Named(Named::ArrowUp));
    if arriba && vacio && enfocado && !sugiriendo {
        return Some(text_editor::Binding::Custom(Msg::EditarUltimo(destino)));
    }
    // Con sugerencias abiertas, Intro y Tab eligen; ↑ ↓ y Esc los toma el atajo
    // global (`Msg::Flecha`, `Msg::Esc`), así que el editor los deja pasar.
    if sugiriendo && enfocado {
        match tecla.key {
            keyboard::Key::Named(Named::Enter | Named::Tab) if !tecla.modifiers.shift() => {
                return Some(text_editor::Binding::Custom(Msg::ElegirSugerencia));
            }
            keyboard::Key::Named(Named::ArrowUp | Named::ArrowDown | Named::Escape) => {
                return None;
            }
            _ => {}
        }
    }
    let envia = crate::atajos::intro_envia(
        tecla.modifiers.command(),
        tecla.modifiers.shift(),
        ctrl_intro,
    );
    if intro && enfocado && envia {
        return Some(text_editor::Binding::Custom(Msg::Enviar(destino)));
    }
    // Con "Ctrl + Intro envía", Intro solo es un salto de línea.
    if intro && enfocado && ctrl_intro {
        return Some(text_editor::Binding::Enter);
    }
    // Ctrl + B / I, Ctrl + Shift + X / C / 7 / 8 / 9…, como en Slack.
    let mods = crate::atajos::Mods {
        ctrl: tecla.modifiers.command(),
        alt: tecla.modifiers.alt(),
        shift: tecla.modifiers.shift(),
    };
    let formato = crate::tecla_de_atajo(&tecla.key, tecla.physical_key)
        .and_then(|t| crate::atajos::formato(t, mods));
    if let (Some(f), true) = (formato, enfocado) {
        return Some(text_editor::Binding::Custom(Msg::Formatear(destino, f)));
    }
    // Ctrl + dígito cambia de workspace aunque el compositor tenga el foco.
    let digito = tecla
        .key
        .to_latin(tecla.physical_key)
        .is_some_and(|c| c.is_ascii_digit());
    if digito && tecla.modifiers.command() {
        return None;
    }
    text_editor::Binding::from_key_press(tecla)
}

/// Editor de un mensaje propio en su lugar: Intro guarda, Esc (global) cancela.
fn editor_en_linea(ed: &Edicion) -> Element<'_, Msg> {
    use keyboard::key::Named;
    let editor = text_editor(&ed.editor)
        .id(ID_EDICION)
        .on_action(Msg::EditarEdicion)
        .key_binding(|tecla| match tecla.key {
            keyboard::Key::Named(Named::Enter) if !tecla.modifiers.shift() => {
                Some(text_editor::Binding::Custom(Msg::GuardarEdicion))
            }
            keyboard::Key::Named(Named::Escape) => None,
            _ => text_editor::Binding::from_key_press(tecla),
        })
        .padding([6, 10])
        .size(15)
        .style(estilo::editor);
    let botones = row![
        space::horizontal(),
        button(text("Cancelar").size(13))
            .padding([4, 12])
            .style(estilo::boton_plano)
            .on_press(Msg::Esc),
        button(text("Guardar").size(13).font(NEGRITA))
            .padding([4, 12])
            .style(estilo::enviar(true))
            .on_press(Msg::GuardarEdicion),
    ]
    .spacing(6);
    container(column![editor, botones].spacing(6).padding(6))
        .style(tarjeta(col().borde_fuerte, 8.0, false))
        .into()
}

fn confirmar_borrar<'a>() -> Element<'a, Msg> {
    let caja = container(
        column![
            text("Borrar mensaje").size(20).font(NEGRA),
            text("¿Seguro que quieres borrar este mensaje? No se puede deshacer.")
                .size(15)
                .color(col().texto),
            row![
                space::horizontal(),
                button(text("Cancelar").size(14))
                    .padding([6, 14])
                    .style(estilo::boton_plano)
                    .on_press(Msg::Esc),
                button(text("Borrar").size(14).font(NEGRITA).color(BLANCO))
                    .padding([6, 14])
                    .style(estilo::peligro)
                    .on_press(Msg::ConfirmarBorrar),
            ]
            .spacing(8),
        ]
        .spacing(14),
    )
    .padding(20)
    .width(440)
    .style(tarjeta(col().borde, 8.0, true));
    capa_modal(caja.into())
}

/// Velo oscuro sobre toda la ventana con la caja arriba al centro; clic afuera la
/// cierra (`opaque` evita que los clics dentro lleguen al velo).
fn capa_modal(caja: Element<'_, Msg>) -> Element<'_, Msg> {
    mouse_area(
        container(opaque(caja))
            .center_x(Fill)
            .height(Fill)
            .padding(Padding {
                top: 90.0,
                ..Padding::ZERO
            })
            .style(fondo(Color::from_rgba8(0, 0, 0, 0.4))),
    )
    .on_press(Msg::Esc)
    .into()
}

fn agrupa(anterior: Option<&Mensaje>, m: &Mensaje) -> bool {
    anterior
        .is_some_and(|a| a.user.is_some() && a.user == m.user && m.segundos() - a.segundos() < 300)
}

fn fecha(m: &Mensaje) -> Option<DateTime<Local>> {
    DateTime::from_timestamp(m.segundos(), 0).map(|t| t.with_timezone(&Local))
}

fn dia(m: &Mensaje) -> Option<NaiveDate> {
    fecha(m).map(|t| t.date_naive())
}

fn etiqueta_dia(fecha: NaiveDate) -> String {
    let hoy = Local::now().date_naive();
    if fecha == hoy {
        return "Hoy".into();
    }
    if hoy.pred_opt() == Some(fecha) {
        return "Ayer".into();
    }
    let dia = DIAS[fecha.weekday().num_days_from_monday() as usize];
    let mut s = format!(
        "{dia}, {} de {}",
        fecha.day(),
        MESES[fecha.month0() as usize]
    );
    if fecha.year() != hoy.year() {
        s.push_str(&format!(" de {}", fecha.year()));
    }
    s
}

/// Hora de la lista de directos: la hora si es de hoy, "Ayer", el día de la semana si
/// fue en los últimos seis días, y si no la fecha.
fn hora_corta(t: DateTime<Local>, ahora: DateTime<Local>) -> String {
    let (dia, hoy) = (t.date_naive(), ahora.date_naive());
    let dias = (hoy - dia).num_days();
    match dias {
        0 => t.format("%H:%M").to_string(),
        1 => "Ayer".into(),
        2..=6 => DIAS[dia.weekday().num_days_from_monday() as usize].into(),
        _ if dia.year() == hoy.year() => {
            format!("{} de {}", dia.day(), MESES[dia.month0() as usize])
        }
        _ => format!(
            "{} de {} de {}",
            dia.day(),
            MESES[dia.month0() as usize],
            dia.year()
        ),
    }
}

fn separador_dia<'a>(fecha: Option<NaiveDate>) -> Element<'a, Msg> {
    let etiqueta = fecha.map(etiqueta_dia).unwrap_or_default();
    row![
        linea_horizontal(col().borde),
        container(text(etiqueta).size(13).font(NEGRA))
            .padding([4, 16])
            .style(tarjeta(col().borde, 24.0, false)),
        linea_horizontal(col().borde),
    ]
    .align_y(Alignment::Center)
    .padding([8, 0])
    .into()
}

fn separador_respuestas<'a>(n: usize) -> Element<'a, Msg> {
    let etiqueta = if n == 1 {
        "1 respuesta".to_string()
    } else {
        format!("{n} respuestas")
    };
    row![
        text(etiqueta).size(13).color(col().tenue),
        linea_horizontal(col().borde)
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([8, 20])
    .into()
}

fn tarjeta_archivo<'a>(f: &'a Archivo) -> Element<'a, Msg> {
    let nombre = f
        .title
        .as_deref()
        .or(f.name.as_deref())
        .unwrap_or("archivo");
    let tipo = f.pretty_type.as_deref().unwrap_or("Archivo");
    let fila = row![
        container(text("▤").size(20).color(BLANCO))
            .center(40)
            .style(redondeado(col().enlace, 6.0)),
        column![
            text(nombre).size(15).font(NEGRA),
            text(tipo).size(13).color(col().tenue)
        ]
        .spacing(2),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    container(fila)
        .padding(10)
        .max_width(420)
        .style(tarjeta(col().borde, 8.0, false))
        .into()
}

/// Ctrl + /: los atajos que entiende el clon.
fn ventana_atajos<'a>() -> Element<'a, Global> {
    const ATAJOS: [(&str, &str); 16] = [
        ("Saltar a una conversación", "Ctrl + K"),
        ("Atrás / adelante en el historial", "Alt + ← / →"),
        ("Conversación anterior / siguiente", "Alt + ↑ / ↓"),
        ("No leída anterior / siguiente", "Alt + Shift + ↑ / ↓"),
        ("Marcar como leída", "Esc"),
        ("Marcar todo como leído", "Shift + Esc"),
        ("Editar tu último mensaje", "↑ con el compositor vacío"),
        ("Cerrar el hilo", "Ctrl + ."),
        ("Workspace 1…9", "Ctrl + 1…9"),
        (
            "Workspace siguiente / anterior",
            "Ctrl + Tab / Ctrl + Shift + Tab",
        ),
        (
            "Inicio / MD / Actividad / Archivos / Guardados",
            "Ctrl + Shift + 1…5",
        ),
        ("Agrandar / achicar / normal", "Ctrl + + / − / 0"),
        ("Modo oscuro", "Ctrl + Shift + Y"),
        ("Nueva línea", "Shift + Intro"),
        (
            "Negrita / cursiva / tachado / código",
            "botones del compositor",
        ),
        ("Esta lista", "Ctrl + /"),
    ];
    let mut lista = Column::new().spacing(6);
    for (que, tecla) in ATAJOS {
        lista = lista.push(
            row![
                text(que).size(14).width(Fill),
                container(text(tecla).size(13).font(NEGRITA))
                    .padding([2, 8])
                    .style(redondeado(col().reaccion_fondo, 4.0)),
            ]
            .align_y(Alignment::Center),
        );
    }
    let ventana = column![
        row![
            text("Atajos de teclado").size(20).font(NEGRA),
            space::horizontal(),
            button(text("✕").size(15))
                .padding([2, 8])
                .style(estilo::boton_plano)
                .on_press(Global::VerAtajos),
        ]
        .align_y(Alignment::Center),
        linea_horizontal(col().borde),
        scrollable(lista),
    ]
    .spacing(12)
    .padding(24);
    container(opaque(
        container(ventana)
            .width(560)
            .max_height(620)
            .style(estilo::tarjeta(col().borde, 12.0, true)),
    ))
    .center(Fill)
    .style(fondo(Color::from_rgba8(0, 0, 0, 0.4)))
    .into()
}

/// Globo oscuro de los tooltips, como los del cliente oficial.
fn etiqueta<'a, M: 'a>(t: impl text::IntoFragment<'a>) -> Element<'a, M> {
    container(text(t).size(13).font(NEGRITA).color(col().texto_globo))
        .padding([4, 8])
        .style(redondeado(col().globo, 6.0))
        .into()
}

fn insignia<'a, M: 'a>(n: u32) -> Element<'a, M> {
    container(
        text(n.to_string())
            .size(12)
            .font(NEGRA)
            .color(col().texto_insignia),
    )
    .padding([1, 7])
    .style(redondeado(col().insignia, 9.0))
    .into()
}

fn aviso_error<M: Clone + 'static>(e: &str, cerrar: M) -> Element<'_, M> {
    aviso(e, cerrar, col().aviso_fondo)
}

/// ▾ de enviar: programar para mañana o el lunes a las 9.
fn menu_programar<'a>(destino: Destino) -> Element<'a, Msg> {
    use chrono::TimeZone;
    let mut lista = Column::new().push(
        container(
            text("Programar mensaje")
                .size(12)
                .font(NEGRITA)
                .color(col().tenue),
        )
        .padding([6, 12]),
    );
    for (etiqueta, cuando) in crate::espacio::opciones_programar(Local::now().naive_local()) {
        let Some(post_at) = Local.from_local_datetime(&cuando).single() else {
            continue;
        };
        lista = lista.push(
            button(text(etiqueta.clone()).size(14))
                .padding([6, 12])
                .width(Fill)
                .style(|_, s| estilo::opcion(false, s))
                .on_press(Msg::Programar(destino, post_at.timestamp(), etiqueta)),
        );
    }
    container(
        container(lista.padding(4))
            .width(260)
            .style(tarjeta(col().borde, 8.0, true)),
    )
    .align_right(Fill)
    .into()
}

/// Franja con un aviso y ✕, arriba del contenido.
fn aviso<M: Clone + 'static>(e: &str, cerrar: M, color: Color) -> Element<'_, M> {
    let fila = row![
        text(e).size(13),
        space::horizontal(),
        button(text("✕").size(13))
            .padding([2, 6])
            .style(estilo::boton_plano)
            .on_press(cerrar),
    ]
    .align_y(Alignment::Center);
    container(fila)
        .padding([6, 20])
        .width(Fill)
        .style(fondo(color))
        .into()
}

fn vacio<'a>(aviso: &'a str) -> Element<'a, Msg> {
    container(text(aviso).size(15).color(col().tenue))
        .center(Fill)
        .into()
}

fn cargando<'a>() -> Element<'a, Msg> {
    container(text("Cargando…").color(col().tenue))
        .center(Fill)
        .into()
}

fn linea_horizontal<'a, M: 'a>(color: Color) -> Element<'a, M> {
    container(Space::new())
        .width(Fill)
        .height(1)
        .style(fondo(color))
        .into()
}

fn linea_vertical<'a, M: 'a>(color: Color) -> Element<'a, M> {
    container(Space::new())
        .width(1)
        .height(Fill)
        .style(fondo(color))
        .into()
}

fn inicial(nombre: &str) -> String {
    nombre
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDateTime, TimeZone};

    fn t(s: &str) -> DateTime<Local> {
        let n = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        Local.from_local_datetime(&n).unwrap()
    }

    #[test]
    fn hora_corta_de_la_lista_de_directos() {
        let ahora = t("2026-10-21 18:00");
        assert_eq!(hora_corta(t("2026-10-21 09:05"), ahora), "09:05");
        assert_eq!(hora_corta(t("2026-10-20 23:59"), ahora), "Ayer");
        assert_eq!(hora_corta(t("2026-10-16 10:00"), ahora), "viernes");
        assert_eq!(hora_corta(t("2026-10-14 10:00"), ahora), "14 de octubre");
        assert_eq!(
            hora_corta(t("2025-12-31 10:00"), ahora),
            "31 de diciembre de 2025"
        );
    }
}
