//! Colores, fuentes y estilos de widgets. El marco (barra, rail, lateral) sigue el tema
//! del workspace activo (ver `paleta.rs`); el contenido, el modo claro u oscuro.

use std::sync::{LazyLock, RwLock};

use iced::widget::{button, container, scrollable, text_editor, text_input};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector, border, font};

use crate::paleta::{self, Paleta, Rgba};

/// Blanco puro, para texto o íconos sobre un color fijo (avatar, botón rojo).
pub const BLANCO: Color = Color::WHITE;

/// Colores de los avatares sin foto, como los genéricos de Slack.
pub const AVATARES: [Color; 6] = [
    Color::from_rgb8(0x2b, 0xac, 0x76),
    Color::from_rgb8(0xe0, 0x1e, 0x5a),
    Color::from_rgb8(0x36, 0xc5, 0xf0),
    Color::from_rgb8(0xec, 0xb2, 0x2e),
    Color::from_rgb8(0x4a, 0x15, 0x4b),
    Color::from_rgb8(0x12, 0x64, 0xa3),
];

/// Todos los colores de la interfaz con el tema y el modo actuales.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colores {
    pub barra: Color,
    pub barra_texto: Color,
    pub lateral: Color,
    pub lateral_hover: Color,
    pub lateral_texto: Color,
    pub lateral_fuerte: Color,
    pub lateral_linea: Color,
    pub seleccion: Color,
    pub texto_seleccion: Color,
    pub insignia: Color,
    pub texto_insignia: Color,
    /// Fondo del canal, el hilo y las tarjetas.
    pub fondo: Color,
    pub texto: Color,
    pub tenue: Color,
    pub borde: Color,
    pub borde_fuerte: Color,
    /// Fondo de un mensaje bajo el mouse y de los paneles secundarios.
    pub sobre: Color,
    pub enlace: Color,
    pub mencion_fondo: Color,
    /// Tu mención y `@here` / `@channel`: amarillo, y la fila de un mensaje que te
    /// menciona.
    pub mencion_tuya_fondo: Color,
    pub mencion_tuya_texto: Color,
    pub fila_mencion: Color,
    pub codigo_texto: Color,
    pub codigo_fondo: Color,
    pub codigo_borde: Color,
    pub reaccion_fondo: Color,
    pub aviso_fondo: Color,
    pub peligro: Color,
    pub exito: Color,
    /// Fondo y texto de los tooltips.
    pub globo: Color,
    pub texto_globo: Color,
    pub seleccion_texto: Color,
}

/// Colores para un tema y un modo.
pub fn colores(p: &Paleta, oscuro: bool) -> Colores {
    let c = |v: Rgba| Color::from_rgba(v[0], v[1], v[2], v[3]);
    let rgb = Color::from_rgb8;
    let (fondo, texto, tenue, borde, borde_fuerte, sobre) = if oscuro {
        (
            rgb(0x1a, 0x1d, 0x21),
            rgb(0xf8, 0xf8, 0xf8),
            rgb(0xab, 0xab, 0xad),
            rgb(0x35, 0x37, 0x3b),
            rgb(0x56, 0x58, 0x5c),
            rgb(0x22, 0x25, 0x29),
        )
    } else {
        (
            BLANCO,
            rgb(0x1d, 0x1c, 0x1d),
            rgb(0x61, 0x60, 0x61),
            rgb(0xdd, 0xdd, 0xdd),
            rgb(0xbb, 0xba, 0xbb),
            rgb(0xf8, 0xf8, 0xf8),
        )
    };
    let modo = |claro: Color, oscura: Color| if oscuro { oscura } else { claro };
    Colores {
        barra: c(p.barra),
        barra_texto: c(p.barra_texto),
        lateral: c(p.lateral),
        lateral_hover: c(p.lateral_hover),
        lateral_texto: c(p.lateral_texto),
        lateral_fuerte: c(p.lateral_fuerte),
        lateral_linea: c(p.lateral_linea),
        seleccion: c(p.seleccion),
        texto_seleccion: c(p.texto_seleccion),
        insignia: c(p.insignia),
        texto_insignia: c(p.texto_insignia),
        fondo,
        texto,
        tenue,
        borde,
        borde_fuerte,
        sobre,
        enlace: modo(rgb(0x12, 0x64, 0xa3), rgb(0x1d, 0x9b, 0xd1)),
        mencion_fondo: modo(rgb(0xe8, 0xf5, 0xfa), rgb(0x12, 0x33, 0x45)),
        mencion_tuya_fondo: modo(
            Color::from_rgba8(0xff, 0xc6, 0x00, 0.18),
            Color::from_rgba8(0xa0, 0x73, 0x00, 0.18),
        ),
        mencion_tuya_texto: modo(rgb(0x1d, 0x1c, 0x1d), rgb(0xde, 0xa7, 0x00)),
        fila_mencion: modo(rgb(0xff, 0xfa, 0xe0), rgb(0x2f, 0x1e, 0x00)),
        codigo_texto: modo(rgb(0xe0, 0x1e, 0x5a), rgb(0xe8, 0x91, 0x2d)),
        codigo_fondo: modo(rgb(0xf6, 0xf6, 0xf6), rgb(0x22, 0x25, 0x29)),
        codigo_borde: modo(rgb(0xe2, 0xe2, 0xe2), rgb(0x3c, 0x3f, 0x43)),
        reaccion_fondo: modo(rgb(0xef, 0xef, 0xef), rgb(0x2c, 0x2f, 0x33)),
        aviso_fondo: modo(rgb(0xfc, 0xe9, 0xee), rgb(0x30, 0x00, 0x05)),
        peligro: rgb(0xe0, 0x1e, 0x5a),
        exito: modo(rgb(0x00, 0x7a, 0x5a), rgb(0x3d, 0xaa, 0x7c)),
        globo: modo(rgb(0x1d, 0x1c, 0x1d), rgb(0xe8, 0xe8, 0xe8)),
        texto_globo: modo(BLANCO, rgb(0x1d, 0x1c, 0x1d)),
        seleccion_texto: modo(rgb(0xb4, 0xd5, 0xfe), rgb(0x1f, 0x4e, 0x79)),
    }
}

static ACTUALES: LazyLock<RwLock<Colores>> =
    LazyLock::new(|| RwLock::new(colores(&paleta::aubergine(), false)));

/// Los colores con que se pinta ahora.
pub fn col() -> Colores {
    *ACTUALES.read().unwrap_or_else(|e| e.into_inner())
}

/// Cambia los colores con que se pinta (al cambiar de workspace, tema o modo).
pub fn fijar(nuevos: Colores) {
    *ACTUALES.write().unwrap_or_else(|e| e.into_inner()) = nuevos;
}

pub const LATO: Font = Font::with_name("Lato");
pub const EMOJI: Font = Font::with_name("Noto Color Emoji");
pub const NEGRITA: Font = Font {
    weight: font::Weight::Bold,
    ..LATO
};
pub const NEGRA: Font = Font {
    weight: font::Weight::Black,
    ..LATO
};
pub const CURSIVA: Font = Font {
    style: font::Style::Italic,
    ..LATO
};

pub fn tema() -> Theme {
    let c = col();
    Theme::custom(
        "Slack".to_string(),
        iced::theme::Palette {
            background: c.fondo,
            text: c.texto,
            primary: c.enlace,
            success: c.exito,
            warning: Color::from_rgb8(0xec, 0xb2, 0x2e),
            danger: c.peligro,
        },
    )
}

pub fn fondo(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style::default().background(color)
}

pub fn redondeado(color: Color, radio: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(color.into()),
        border: border::rounded(radio),
        ..container::Style::default()
    }
}

/// Caja con el fondo del contenido y borde, como el compositor o la barra de acciones.
pub fn tarjeta(borde: Color, radio: f32, sombra: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(col().fondo.into()),
        border: Border {
            color: borde,
            width: 1.0,
            radius: radio.into(),
        },
        shadow: if sombra {
            Shadow {
                color: Color::from_rgba8(0, 0, 0, 0.08),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 3.0,
            }
        } else {
            Shadow::default()
        },
        ..container::Style::default()
    }
}

pub fn item_lateral(activo: bool, estado: button::Status) -> button::Style {
    let c = col();
    let fondo = match (activo, estado) {
        (true, _) => Some(c.seleccion),
        (false, button::Status::Hovered | button::Status::Pressed) => Some(c.lateral_hover),
        _ => None,
    };
    button::Style {
        background: fondo.map(Background::from),
        text_color: if activo {
            c.texto_seleccion
        } else {
            c.lateral_texto
        },
        border: border::rounded(6),
        ..button::Style::default()
    }
}

/// Opción de una lista sobre fondo blanco (Ctrl+K): azul la elegida, gris al pasar.
pub fn opcion(elegida: bool, estado: button::Status) -> button::Style {
    let c = col();
    let fondo = match (elegida, estado) {
        (true, _) => Some(c.seleccion),
        (false, button::Status::Hovered | button::Status::Pressed) => Some(c.sobre),
        _ => None,
    };
    button::Style {
        background: fondo.map(Background::from),
        text_color: if elegida { c.texto_seleccion } else { c.texto },
        border: border::rounded(6),
        ..button::Style::default()
    }
}

/// Botón rojo de acciones que no se deshacen (borrar).
pub fn peligro(_: &Theme, estado: button::Status) -> button::Style {
    let alfa = match estado {
        button::Status::Hovered | button::Status::Pressed => 0.85,
        _ => 1.0,
    };
    button::Style {
        background: Some(
            Color {
                a: alfa,
                ..col().peligro
            }
            .into(),
        ),
        text_color: BLANCO,
        border: border::rounded(6),
        ..button::Style::default()
    }
}

pub fn boton_rail(activo: bool, estado: button::Status) -> button::Style {
    let alfa = match (activo, estado) {
        (true, _) => 0.25,
        (false, button::Status::Hovered | button::Status::Pressed) => 0.15,
        _ => 0.0,
    };
    let texto = col().barra_texto;
    button::Style {
        background: Some(Color { a: alfa, ..texto }.into()),
        text_color: texto,
        border: border::rounded(8),
        ..button::Style::default()
    }
}

/// Botón que no se ve como botón (nombres, avatares, casillas): solo el texto, con el
/// color del contenido. El estilo por defecto de iced lo pondría en negro.
pub fn sin_fondo(_: &Theme, _: button::Status) -> button::Style {
    button::Style {
        text_color: col().texto,
        ..button::Style::default()
    }
}

/// Botón sin fondo que se sombrea al pasar el mouse (área blanca).
pub fn boton_plano(_: &Theme, estado: button::Status) -> button::Style {
    let fondo = match estado {
        button::Status::Hovered | button::Status::Pressed => Some(col().reaccion_fondo.into()),
        _ => None,
    };
    button::Style {
        background: fondo,
        text_color: col().tenue,
        border: border::rounded(4),
        ..button::Style::default()
    }
}

pub fn reaccion(mia: bool, estado: button::Status) -> button::Style {
    let c = col();
    let sobre = matches!(estado, button::Status::Hovered | button::Status::Pressed);
    let (fondo, borde) = match (mia, sobre) {
        (true, _) => (c.mencion_fondo, Color::from_rgb8(0x1d, 0x9b, 0xd1)),
        (false, true) => (c.fondo, c.tenue),
        (false, false) => (c.reaccion_fondo, c.reaccion_fondo),
    };
    button::Style {
        background: Some(fondo.into()),
        text_color: if mia { c.enlace } else { c.texto },
        border: Border {
            color: borde,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..button::Style::default()
    }
}

pub fn enviar(activo: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: activo.then_some(col().exito.into()),
        text_color: if activo { BLANCO } else { col().borde_fuerte },
        border: border::rounded(4),
        ..button::Style::default()
    }
}

/// Buscador de la barra superior: translúcido sobre la barra y con el fondo del
/// contenido al enfocarlo.
pub fn buscador(_: &Theme, estado: text_input::Status) -> text_input::Style {
    let c = col();
    let enfocado = matches!(estado, text_input::Status::Focused { .. });
    let sobre_barra = Color {
        a: 0.7,
        ..c.barra_texto
    };
    text_input::Style {
        background: if enfocado {
            c.fondo.into()
        } else {
            Color {
                a: 0.2,
                ..c.barra_texto
            }
            .into()
        },
        border: border::rounded(6),
        icon: sobre_barra,
        placeholder: if enfocado { c.tenue } else { sobre_barra },
        value: if enfocado { c.texto } else { c.barra_texto },
        selection: c.seleccion_texto,
    }
}

/// "Buscar conversación…": caja con el fondo del contenido sobre la lateral.
pub fn buscador_lateral(_: &Theme, estado: text_input::Status) -> text_input::Style {
    let c = col();
    let enfocado = matches!(estado, text_input::Status::Focused { .. });
    text_input::Style {
        background: c.fondo.into(),
        border: Border {
            color: if enfocado {
                c.seleccion
            } else {
                c.lateral_linea
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: c.tenue,
        placeholder: c.tenue,
        value: c.texto,
        selection: c.seleccion_texto,
    }
}

pub fn editor(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: Color::TRANSPARENT.into(),
        border: Border::default(),
        placeholder: col().tenue,
        value: col().texto,
        selection: col().seleccion_texto,
    }
}

/// Barra de desplazamiento delgada y translúcida para la lateral.
pub fn scroll_lateral(tema: &Theme, estado: scrollable::Status) -> scrollable::Style {
    let mut estilo = scrollable::default(tema, estado);
    let barra = scrollable::Rail {
        background: None,
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Color {
                a: 0.3,
                ..col().lateral_texto
            }
            .into(),
            border: border::rounded(4),
        },
    };
    estilo.vertical_rail = barra;
    estilo
}

/// Marco del workspace activo en el rail cuando hay varios.
pub fn anillo(activo: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        border: Border {
            color: if activo {
                col().barra_texto
            } else {
                Color::TRANSPARENT
            },
            width: 2.0,
            radius: 11.0.into(),
        },
        ..container::Style::default()
    }
}
