//! Sugerencias mientras se escribe: el selector de emoji y el autocompletado del
//! compositor.

/// Un emoji que se puede elegir: su código y, si es estándar, el carácter.
#[derive(Debug, Clone, PartialEq)]
pub struct OpcionEmoji {
    pub nombre: String,
    /// `None` en los propios del workspace (se pintan con su imagen).
    pub unicode: Option<String>,
}

/// Qué se está completando en el compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disparador {
    Usuario,
    Canal,
    Emoji,
}

/// La palabra con `@`, `#` o `:` que está justo antes del cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    pub tipo: Disparador,
    /// Lo escrito después del disparador.
    pub prefijo: &'a str,
    /// Byte de la línea donde está el disparador; de ahí al cursor se reemplaza.
    pub inicio: usize,
}

/// Lo que se está completando en `linea` con el cursor en el byte `columna`. El
/// disparador tiene que ir al principio o después de un espacio o paréntesis, así
/// `a@b.com` y `10:30` no disparan nada; los emoji piden al menos dos letras.
pub fn disparador(linea: &str, columna: usize) -> Option<Token<'_>> {
    let antes = linea.get(..columna)?;
    let inicio_palabra = antes
        .char_indices()
        .rev()
        .take_while(|(_, c)| es_de_nombre(*c))
        .last()
        .map_or(antes.len(), |(i, _)| i);
    let prefijo = &antes[inicio_palabra..];
    let tipo = match antes[..inicio_palabra].chars().next_back()? {
        '@' => Disparador::Usuario,
        '#' => Disparador::Canal,
        ':' => Disparador::Emoji,
        _ => return None,
    };
    let inicio = inicio_palabra - 1;
    let separado = antes[..inicio]
        .chars()
        .next_back()
        .is_none_or(|p| p.is_whitespace() || "([{¿¡\"'".contains(p));
    if !separado || (tipo == Disparador::Emoji && prefijo.chars().count() < 2) {
        return None;
    }
    Some(Token {
        tipo,
        prefijo,
        inicio,
    })
}

fn es_de_nombre(c: char) -> bool {
    c.is_alphanumeric() || "_-.+'".contains(c)
}

/// Cambia lo que el autocompletado insertó a la vista (`@Ana López`, `#general`) por
/// lo que entiende Slack (`<@U1>`, `<#C1>`). Solo palabras completas: `#generales` no
/// es `#general`.
pub fn traducir_menciones(texto: &str, pares: &[(String, String)]) -> String {
    let mut pares: Vec<_> = pares.iter().collect();
    // Los más largos primero, para que `@Ana López` gane a `@Ana`.
    pares.sort_by_key(|(visible, _)| std::cmp::Reverse(visible.len()));
    let mut salida = texto.to_string();
    for (visible, marca) in pares {
        let mut desde = 0;
        while let Some(i) = salida[desde..].find(visible.as_str()).map(|i| i + desde) {
            let fin = i + visible.len();
            if salida[fin..].chars().next().is_some_and(es_de_nombre) {
                desde = fin;
                continue;
            }
            salida.replace_range(i..fin, marca);
            desde = i + marca.len();
        }
    }
    salida
}

/// Los que el selector muestra antes de escribir nada, como los frecuentes de Slack.
const FRECUENTES: [&str; 16] = [
    "+1",
    "white_check_mark",
    "eyes",
    "joy",
    "heart",
    "tada",
    "pray",
    "fire",
    "raised_hands",
    "100",
    "muscle",
    "rocket",
    "thinking_face",
    "clap",
    "wave",
    "ok_hand",
];

/// Emoji cuyo código coincide con la consulta, hasta `limite`: primero los que
/// empiezan con ella, luego los que la contienen; en cada grupo el más corto y luego
/// en orden alfabético. Busca en todos los códigos de cada emoji y en los propios.
pub fn buscar_emoji<'a>(
    consulta: &str,
    propios: impl IntoIterator<Item = &'a str>,
    limite: usize,
) -> Vec<OpcionEmoji> {
    let consulta = consulta.trim().trim_matches(':').to_lowercase();
    if consulta.is_empty() {
        return FRECUENTES
            .iter()
            .filter_map(|n| {
                let e = emojis::get_by_shortcode(n)?;
                Some(OpcionEmoji {
                    nombre: n.to_string(),
                    unicode: Some(e.as_str().into()),
                })
            })
            .take(limite)
            .collect();
    }
    let estandar = emojis::iter().flat_map(|e| {
        e.shortcodes()
            .map(move |c| (c.to_string(), Some(e.as_str().to_string())))
    });
    let propios = propios.into_iter().map(|n| (n.to_string(), None));
    let mut con_nivel: Vec<_> = estandar
        .chain(propios)
        .filter_map(|(nombre, unicode)| {
            let nivel = match nombre.find(&consulta)? {
                0 => 0,
                _ => 1,
            };
            Some((nivel, nombre, unicode))
        })
        .collect();
    con_nivel.sort_by(|a, b| (a.0, a.1.len(), &a.1).cmp(&(b.0, b.1.len(), &b.1)));
    con_nivel
        .into_iter()
        .take(limite)
        .map(|(_, nombre, unicode)| OpcionEmoji { nombre, unicode })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nombres(v: &[OpcionEmoji]) -> Vec<&str> {
        v.iter().map(|o| o.nombre.as_str()).collect()
    }

    #[test]
    fn sin_consulta_salen_los_frecuentes() {
        let v = buscar_emoji("", ["loro"], 6);
        assert_eq!(
            nombres(&v),
            ["+1", "white_check_mark", "eyes", "joy", "heart", "tada"]
        );
        assert_eq!(v[0].unicode.as_deref(), Some("👍"));
    }

    #[test]
    fn busca_en_todos_los_codigos_prefijo_primero() {
        let v = buscar_emoji("thumbs", [], 10);
        assert_eq!(nombres(&v)[..2], ["thumbsup", "thumbsdown"]);
        assert_eq!(v[0].unicode.as_deref(), Some("👍"));
        let v = buscar_emoji("ROCKE", [], 10);
        assert_eq!(
            v[0].nombre, "rocket",
            "sin mayúsculas, el más corto primero"
        );
    }

    #[test]
    fn incluye_los_propios() {
        let v = buscar_emoji("lor", ["loro", "otro"], 50);
        let loro = v.iter().find(|o| o.nombre == "loro").unwrap();
        assert_eq!(loro.unicode, None);
        assert!(!nombres(&v).contains(&"otro"));
    }

    fn token(linea: &str) -> Option<(Disparador, &str, usize)> {
        disparador(linea, linea.len()).map(|t| (t.tipo, t.prefijo, t.inicio))
    }

    #[test]
    fn detecta_lo_que_se_esta_escribiendo() {
        assert_eq!(token("hola @an"), Some((Disparador::Usuario, "an", 5)));
        assert_eq!(token("@"), Some((Disparador::Usuario, "", 0)));
        assert_eq!(token("ve a #gen"), Some((Disparador::Canal, "gen", 5)));
        assert_eq!(token("listo :tad"), Some((Disparador::Emoji, "tad", 6)));
        assert_eq!(
            token("(@ana.lopez"),
            Some((Disparador::Usuario, "ana.lopez", 1))
        );
        assert_eq!(
            token("¿y @josé"),
            Some((Disparador::Usuario, "josé", 4)),
            "en bytes: ¿ son dos"
        );
    }

    #[test]
    fn no_dispara_donde_no_va() {
        assert_eq!(token("correo a@b"), None, "email");
        assert_eq!(token("a las 10:30"), None, "hora");
        assert_eq!(token("ok :t"), None, "emoji pide dos letras");
        assert_eq!(token("hola @ana "), None, "ya terminó la palabra");
        assert_eq!(token("listo :tada:"), None, "emoji ya cerrado");
    }

    #[test]
    fn el_cursor_en_medio_de_la_linea() {
        let linea = "hola @an y más";
        let t = disparador(linea, "hola @an".len()).unwrap();
        assert_eq!((t.prefijo, t.inicio), ("an", 5));
    }

    #[test]
    fn traduce_las_menciones_al_enviar() {
        let pares = [
            ("@Ana López".to_string(), "<@U1>".to_string()),
            ("#general".to_string(), "<#C1>".to_string()),
        ];
        assert_eq!(
            traducir_menciones("hola @Ana López, ve #general y #generales", &pares),
            "hola <@U1>, ve <#C1> y #generales"
        );
        assert_eq!(traducir_menciones("nada", &pares), "nada");
    }

    #[test]
    fn respeta_el_limite() {
        assert_eq!(buscar_emoji("a", [], 7).len(), 7);
    }
}
