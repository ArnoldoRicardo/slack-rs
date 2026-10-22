//! Colores del marco (barra, rail, lateral) según el tema del usuario, como los temas
//! de Slack (Preferencias → Appearance). Un tema elige dos de las 19 paletas base de
//! Slack: la primaria (marco) y la de resalte (selección, insignias). Cada parte de la
//! interfaz usa un tono fijo de esas paletas, distinto en modo claro y oscuro; los
//! tonos salen de los tokens `--dt_color-*` del cliente web, guardados en
//! `docs/datos/`. Colores como `[r, g, b, a]` en 0…1.

use std::collections::HashMap;
use std::sync::OnceLock;

pub(crate) type Rgba = [f32; 4];

/// Colores que dependen del tema.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Paleta {
    /// Fondo de la barra superior y del rail.
    pub(crate) barra: Rgba,
    pub(crate) barra_texto: Rgba,
    pub(crate) lateral: Rgba,
    pub(crate) lateral_hover: Rgba,
    pub(crate) lateral_texto: Rgba,
    /// Texto en negrita de la lateral: no leídos, nombre del workspace.
    pub(crate) lateral_fuerte: Rgba,
    pub(crate) lateral_linea: Rgba,
    pub(crate) seleccion: Rgba,
    pub(crate) texto_seleccion: Rgba,
    pub(crate) insignia: Rgba,
    pub(crate) texto_insignia: Rgba,
}

/// Tema elegido por el usuario (`ia_theme` en `users.prefs.get`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tema {
    pub(crate) primaria: String,
    pub(crate) resalte: String,
}

impl Tema {
    /// `{"primary":{"palette":"horchata"},"highlight1":{"palette":"jade"},…}`.
    pub(crate) fn de_json(texto: &str) -> Option<Tema> {
        let v: serde_json::Value = serde_json::from_str(texto).ok()?;
        let paleta = |clave: &str| v[clave]["palette"].as_str().map(str::to_string);
        Some(Tema {
            primaria: paleta("primary")?,
            resalte: paleta("highlight1")?,
        })
    }
}

/// Paleta del tema en el modo pedido; Aubergine si no hay tema o no se conoce.
/// El brillo del tema (`brightness`) no se aplica: los tonos son los del brillo 5,
/// el que trae Slack por defecto.
pub(crate) fn del_tema(tema: Option<&Tema>, oscuro: bool) -> Paleta {
    tema.and_then(|t| calcular(t, oscuro))
        .unwrap_or_else(aubergine)
}

fn calcular(tema: &Tema, oscuro: bool) -> Option<Paleta> {
    let tonos = tonos(oscuro);
    let tono = |paleta: &str, n: u8, alfa: f32| {
        let [r, g, b, _] = *tonos.get(&(paleta.to_string(), n))?;
        Some([r, g, b, alfa])
    };
    let p = |n: u8, alfa: f32| tono(&tema.primaria, n, alfa);
    let h = |n: u8| tono(&tema.resalte, n, 1.0);
    // Tono de cada parte, leído de los tokens `--dt_color-theme-*` del cliente web.
    Some(if oscuro {
        let barra = p(10, 1.0)?;
        Paleta {
            barra,
            barra_texto: p(100, 1.0)?,
            lateral: sobre(tono("gray", 0, 0.6)?, barra),
            lateral_hover: p(100, 0.08)?,
            lateral_texto: p(90, 0.9)?,
            lateral_fuerte: p(100, 1.0)?,
            lateral_linea: p(100, 0.25)?,
            seleccion: h(30)?,
            texto_seleccion: h(100)?,
            insignia: h(70)?,
            texto_insignia: h(0)?,
        }
    } else {
        let barra = p(70, 1.0)?;
        Paleta {
            barra,
            barra_texto: p(0, 1.0)?,
            lateral: sobre([1.0, 1.0, 1.0, 0.9], barra),
            lateral_hover: p(70, 0.06)?,
            lateral_texto: p(100, 0.8)?,
            lateral_fuerte: p(100, 1.0)?,
            lateral_linea: p(100, 0.13)?,
            seleccion: h(80)?,
            texto_seleccion: h(0)?,
            insignia: h(70)?,
            texto_insignia: h(0)?,
        }
    })
}

/// Tonos de las paletas base: (paleta, tono) → color. Slack los cambia en modo oscuro.
fn tonos(oscuro: bool) -> &'static HashMap<(String, u8), Rgba> {
    static CLARO: OnceLock<HashMap<(String, u8), Rgba>> = OnceLock::new();
    static OSCURO: OnceLock<HashMap<(String, u8), Rgba>> = OnceLock::new();
    if oscuro {
        OSCURO
            .get_or_init(|| leer_tonos(include_str!("../docs/datos/slack-css-tokens-oscuro.json")))
    } else {
        CLARO.get_or_init(|| leer_tonos(include_str!("../docs/datos/slack-css-tokens.json")))
    }
}

fn leer_tonos(json: &str) -> HashMap<(String, u8), Rgba> {
    let tokens: HashMap<String, String> = serde_json::from_str(json).unwrap_or_default();
    tokens
        .iter()
        .filter_map(|(nombre, valor)| {
            let (paleta, tono) = nombre.strip_prefix("--dt_color-plt-")?.rsplit_once('-')?;
            Some(((paleta.to_string(), tono.parse().ok()?), rgba(valor)?))
        })
        .collect()
}

/// Lee un color de token: `rgba(120,76,31, 1)`, `rgb(…)`, `#1d1c1d`,
/// `#4544470f` o la terna suelta `57,6,58` de la paleta base.
pub(crate) fn rgba(token: &str) -> Option<Rgba> {
    let t = token.trim();
    if let Some(hex) = t.strip_prefix('#') {
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        let alfa = match hex.len() {
            6 => 255,
            8 => byte(6)?,
            _ => return None,
        };
        return Some([byte(0)?, byte(2)?, byte(4)?, alfa].map(|c| c as f32 / 255.0));
    }
    let dentro = t
        .strip_prefix("rgba(")
        .or_else(|| t.strip_prefix("rgb("))
        .map(|r| r.strip_suffix(')'))
        .unwrap_or(Some(t))?;
    let partes: Vec<f32> = dentro
        .split(',')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    match partes[..] {
        [r, g, b] => Some([r / 255.0, g / 255.0, b / 255.0, 1.0]),
        [r, g, b, a] => Some([r / 255.0, g / 255.0, b / 255.0, a]),
        _ => None,
    }
}

/// `arriba` pintado sobre `abajo` (opaco), como lo compone el navegador.
pub(crate) fn sobre(arriba: Rgba, abajo: Rgba) -> Rgba {
    let a = arriba[3];
    let mezcla = |i: usize| arriba[i] * a + abajo[i] * (1.0 - a);
    [mezcla(0), mezcla(1), mezcla(2), 1.0]
}

#[cfg(test)]
/// Contraste WCAG entre dos colores opacos: 1 (igual) a 21 (negro sobre blanco).
pub(crate) fn contraste(a: Rgba, b: Rgba) -> f32 {
    let lineal = |c: f32| {
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let luz = |c: Rgba| 0.2126 * lineal(c[0]) + 0.7152 * lineal(c[1]) + 0.0722 * lineal(c[2]);
    let (x, y) = (luz(a), luz(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Aubergine clásico: el tema de siempre, igual en los dos modos.
pub(crate) fn aubergine() -> Paleta {
    Paleta {
        barra: de8(0x35, 0x0d, 0x36, 1.0),
        barra_texto: de8(0xff, 0xff, 0xff, 1.0),
        lateral: de8(0x3f, 0x0e, 0x40, 1.0),
        lateral_hover: de8(0x35, 0x0d, 0x36, 1.0),
        lateral_texto: de8(0xff, 0xff, 0xff, 0.7),
        lateral_fuerte: de8(0xff, 0xff, 0xff, 1.0),
        lateral_linea: de8(0xff, 0xff, 0xff, 0.1),
        seleccion: de8(0x11, 0x64, 0xa3, 1.0),
        texto_seleccion: de8(0xff, 0xff, 0xff, 1.0),
        insignia: de8(0xe0, 0x1e, 0x5a, 1.0),
        texto_insignia: de8(0xff, 0xff, 0xff, 1.0),
    }
}

const fn de8(r: u8, g: u8, b: u8, a: f32) -> Rgba {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn cerca(a: Rgba, b: Rgba) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.006)
    }

    fn opaco(r: u8, g: u8, b: u8) -> Rgba {
        de8(r, g, b, 1.0)
    }

    /// Las paletas base de Slack (`--dt_color-plt-<nombre>-<tono>`).
    const PALETAS: [&str; 20] = [
        "aquarium",
        "aubergine",
        "campfire",
        "cilantro",
        "flamingo",
        "grass",
        "gray",
        "honeycomb",
        "horchata",
        "indigo",
        "jade",
        "lagoon",
        "mojito",
        "ocean",
        "paprika",
        "rose",
        "sangria",
        "sunflower",
        "tangerine",
        "tomato",
    ];

    fn horchata() -> Tema {
        Tema {
            primaria: "horchata".into(),
            resalte: "jade".into(),
        }
    }

    #[test]
    fn lee_los_formatos_de_los_tokens() {
        assert!(cerca(
            rgba("rgba(120,76,31, 1)").unwrap(),
            opaco(120, 76, 31)
        ));
        assert!(cerca(rgba("rgb(14, 103, 77)").unwrap(), opaco(14, 103, 77)));
        assert!(cerca(rgba("#1d1c1d").unwrap(), opaco(0x1d, 0x1c, 0x1d)));
        assert!(cerca(rgba("57,6,58").unwrap(), opaco(57, 6, 58)));
        let con_alfa = rgba("#4544470f").unwrap();
        assert!((con_alfa[3] - 15.0 / 255.0).abs() < 1e-6);
        assert!((rgba("rgba(255,255,255, 0.9)").unwrap()[3] - 0.9).abs() < 1e-6);
    }

    #[test]
    fn rechaza_lo_que_no_es_color() {
        assert_eq!(rgba(""), None);
        assert_eq!(rgba("#12"), None);
        assert_eq!(rgba("rgba(1,2)"), None);
        assert_eq!(rgba("auto"), None);
    }

    #[test]
    fn contraste_conocido() {
        let negro = opaco(0, 0, 0);
        let blanco = opaco(255, 255, 255);
        assert!((contraste(negro, blanco) - 21.0).abs() < 0.01);
        assert!((contraste(blanco, blanco) - 1.0).abs() < 0.01);
    }

    #[test]
    fn tema_de_las_preferencias() {
        let ia = r#"{"primary":{"palette":"horchata"},"highlight1":{"palette":"jade"},"highlight2":{"palette":"gray"},"important":{"palette":"jade"},"brightness":5,"sidebarInverted":false,"useCustomHex":false}"#;
        assert_eq!(Tema::de_json(ia), Some(horchata()));
        assert_eq!(Tema::de_json(""), None);
        assert_eq!(Tema::de_json(r#"{"primary":{}}"#), None);
    }

    #[test]
    fn horchata_claro_da_lo_medido_en_slack() {
        // Medido en 00-inicio.png y en los tokens de slack-css-tokens.json.
        let p = del_tema(Some(&horchata()), false);
        assert!(cerca(p.barra, opaco(120, 76, 31)));
        assert!(cerca(p.lateral, opaco(242, 237, 233)));
        assert!(cerca(p.lateral_fuerte, opaco(66, 40, 13)));
        assert!(cerca(p.seleccion, opaco(14, 103, 77)));
        assert!(cerca(p.texto_seleccion, opaco(227, 255, 243)));
    }

    #[test]
    fn horchata_oscuro_da_lo_medido_en_slack() {
        // Medido en 99-vivo-comparacion.png (modo oscuro).
        let p = del_tema(Some(&horchata()), true);
        assert!(cerca(p.barra, opaco(72, 42, 15)));
        assert!(cerca(p.lateral, opaco(38, 28, 18)));
        assert!(cerca(p.seleccion, opaco(16, 111, 77)));
        assert!(cerca(p.lateral_fuerte, opaco(235, 227, 217)));
    }

    #[test]
    fn sin_tema_o_desconocido_es_aubergine() {
        assert_eq!(del_tema(None, false), aubergine());
        assert_eq!(del_tema(None, true), aubergine());
        let raro = Tema {
            primaria: "inventada".into(),
            resalte: "jade".into(),
        };
        assert_eq!(del_tema(Some(&raro), false), aubergine());
    }

    #[test]
    fn todas_las_paletas_se_leen() {
        for primaria in PALETAS {
            for oscuro in [false, true] {
                let tema = Tema {
                    primaria: primaria.to_string(),
                    resalte: "jade".into(),
                };
                let p = del_tema(Some(&tema), oscuro);
                assert_ne!(p, aubergine(), "{primaria} sin tonos");
                let fuerte = sobre(p.lateral_fuerte, p.lateral);
                assert!(
                    contraste(fuerte, p.lateral) >= 4.5,
                    "{primaria} oscuro={oscuro}: {}",
                    contraste(fuerte, p.lateral)
                );
                assert!(contraste(p.seleccion, p.texto_seleccion) >= 4.5);
            }
        }
    }
}
