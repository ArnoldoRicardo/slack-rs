//! Conversión del mrkdwn de Slack a segmentos con estilo: menciones, canales,
//! enlaces, `*negrita*`, `_cursiva_`, `~tachado~`, `código`, emoji y entidades HTML.

use std::collections::HashMap;

use crate::slack::{ElementoRico, EnLinea, EstiloRico, Mensaje, Usuario};

/// Estilos de texto que se pueden combinar: `*_x_*` es negrita y cursiva.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Marcas {
    pub negrita: bool,
    pub cursiva: bool,
    pub tachado: bool,
    pub codigo: bool,
}

impl Marcas {
    /// Agrega la marca de mrkdwn `c`; `None` si no es marca, ya está puesta, o se
    /// está dentro de código (ahí no se interpreta nada).
    fn con(self, c: char) -> Option<Self> {
        let mut m = self;
        let campo = match c {
            '*' => &mut m.negrita,
            '_' => &mut m.cursiva,
            '~' => &mut m.tachado,
            '`' => &mut m.codigo,
            _ => return None,
        };
        if *campo || self.codigo {
            return None;
        }
        *campo = true;
        Some(m)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tipo {
    Normal,
    /// Canal o grupo de usuarios.
    Mencion,
    /// Una persona, por id: la tuya se pinta distinto.
    Persona(String),
    /// `@here`, `@channel`, `@everyone`.
    Aviso,
    Enlace(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segmento {
    pub texto: String,
    pub marcas: Marcas,
    pub tipo: Tipo,
}

impl Segmento {
    fn new(texto: impl Into<String>, tipo: Tipo) -> Self {
        Self {
            texto: texto.into(),
            marcas: Marcas::default(),
            tipo,
        }
    }
}

/// Las referencias `<…>` se cambian por un carácter de uso privado antes de leer las
/// marcas, para que `*<@U1>*` quede en negrita sin que las marcas crucen la referencia.
const MARCADOR: u32 = 0xe000;

pub fn segmentos(crudo: &str, usuarios: &HashMap<String, Usuario>) -> Vec<Segmento> {
    let mut refs = Vec::new();
    let mut texto = String::with_capacity(crudo.len());
    let mut resto = crudo;
    while let Some(inicio) = resto.find('<')
        && let Some(fin) = resto[inicio..].find('>')
        && let Some(marcador) = char::from_u32(MARCADOR + refs.len() as u32)
    {
        texto.push_str(&resto[..inicio]);
        texto.push(marcador);
        refs.push(referencia(&resto[inicio + 1..inicio + fin], usuarios));
        resto = &resto[inicio + fin + 1..];
    }
    texto.push_str(resto);
    let mut salida = Vec::new();
    marcas(&entidades(&texto), Marcas::default(), &refs, &mut salida);
    salida
}

/// Bloques de un mensaje, en el orden en que se pintan.
#[derive(Debug, Clone, PartialEq)]
pub enum Bloque {
    Parrafo(Vec<Segmento>),
    Cita(Vec<Segmento>),
    /// Bloque ```` ``` ````: texto tal cual, sin marcas.
    Codigo(String),
    Lista {
        ordenada: bool,
        items: Vec<Vec<Segmento>>,
    },
}

/// Divide el mrkdwn en bloques: código ```` ``` ```` (puede cruzar líneas), y por
/// líneas citas `&gt;`, listas `•` / `1.` y párrafos. Es lo que pone Slack en `text`
/// cuando el mensaje se escribió con el editor de texto enriquecido.
pub fn bloques(crudo: &str, usuarios: &HashMap<String, Usuario>) -> Vec<Bloque> {
    let mut salida = Vec::new();
    let mut resto = crudo;
    while let Some(inicio) = resto.find("```")
        && let Some(fin) = resto[inicio + 3..].find("```")
    {
        lineas(&resto[..inicio], usuarios, &mut salida);
        let codigo = resto[inicio + 3..inicio + 3 + fin].trim_matches('\n');
        salida.push(Bloque::Codigo(texto_plano(codigo, usuarios)));
        resto = &resto[inicio + 6 + fin..];
    }
    lineas(resto, usuarios, &mut salida);
    salida
}

/// Bloques de un mensaje: de su rich text si lo trae, si no de su mrkdwn.
/// `canal` da el nombre de un canal por su ID; rich text solo trae el ID.
pub fn del_mensaje(
    m: &Mensaje,
    usuarios: &HashMap<String, Usuario>,
    canal: impl Fn(&str) -> String,
) -> Vec<Bloque> {
    let Some(rico) = &m.rico else {
        return bloques(m.cuerpo(), usuarios);
    };
    let en_linea = |elementos: &[EnLinea]| -> Vec<Segmento> {
        elementos
            .iter()
            .filter_map(|e| segmento_rico(e, usuarios, &canal))
            .collect()
    };
    rico.iter()
        .filter_map(|e| match e {
            ElementoRico::RichTextSection { elements } => {
                let mut segs = en_linea(elements);
                // Slack termina con `\n` la sección que va antes de una lista o cita.
                if let Some(ultimo) = segs.last_mut() {
                    ultimo
                        .texto
                        .truncate(ultimo.texto.trim_end_matches('\n').len());
                }
                segs.retain(|s| !s.texto.is_empty());
                (!segs.is_empty()).then_some(Bloque::Parrafo(segs))
            }
            ElementoRico::RichTextList { style, elements } => Some(Bloque::Lista {
                ordenada: style == "ordered",
                items: elements
                    .iter()
                    .filter_map(|item| match item {
                        ElementoRico::RichTextSection { elements } => Some(en_linea(elements)),
                        _ => None,
                    })
                    .collect(),
            }),
            ElementoRico::RichTextQuote { elements } => Some(Bloque::Cita(en_linea(elements))),
            ElementoRico::RichTextPreformatted { elements } => Some(Bloque::Codigo(
                en_linea(elements).into_iter().map(|s| s.texto).collect(),
            )),
            ElementoRico::Otro => None,
        })
        .collect()
}

fn segmento_rico(
    e: &EnLinea,
    usuarios: &HashMap<String, Usuario>,
    canal: impl Fn(&str) -> String,
) -> Option<Segmento> {
    let con = |texto: String, estilo: &EstiloRico, tipo| Segmento {
        texto,
        marcas: Marcas {
            negrita: estilo.bold,
            cursiva: estilo.italic,
            tachado: estilo.strike,
            codigo: estilo.code,
        },
        tipo,
    };
    let llano = EstiloRico::default();
    Some(match e {
        EnLinea::Text { text, style } => con(text.clone(), style, Tipo::Normal),
        EnLinea::User { user_id, style } => {
            let nombre = usuarios
                .get(user_id)
                .map_or(user_id.as_str(), Usuario::nombre_visible);
            con(format!("@{nombre}"), style, Tipo::Persona(user_id.clone()))
        }
        EnLinea::Channel { channel_id, style } => {
            con(format!("#{}", canal(channel_id)), style, Tipo::Mencion)
        }
        EnLinea::Link { url, text, style } => con(
            text.clone().unwrap_or_else(|| url.clone()),
            style,
            Tipo::Enlace(url.clone()),
        ),
        EnLinea::Emoji { name, unicode } => {
            let desde_unicode = unicode.as_deref().and_then(|u| {
                u.split('-')
                    .map(|h| u32::from_str_radix(h, 16).ok().and_then(char::from_u32))
                    .collect::<Option<String>>()
            });
            con(
                desde_unicode.unwrap_or_else(|| emoji(name)),
                &llano,
                Tipo::Normal,
            )
        }
        EnLinea::Broadcast { range } => con(format!("@{range}"), &llano, Tipo::Aviso),
        EnLinea::Usergroup { usergroup_id } => {
            con(format!("@{usergroup_id}"), &llano, Tipo::Mencion)
        }
        EnLinea::Otro => return None,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Clase {
    Parrafo,
    Cita,
    Lista { ordenada: bool },
}

fn clasificar(linea: &str) -> (Clase, &str) {
    if let Some(r) = linea.strip_prefix("&gt;") {
        return (Clase::Cita, r.strip_prefix(' ').unwrap_or(r));
    }
    if let Some(r) = ["• ", "◦ ", "▪ "]
        .iter()
        .find_map(|v| linea.strip_prefix(v))
    {
        return (Clase::Lista { ordenada: false }, r);
    }
    let digitos = linea.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if digitos > 0
        && let Some(r) = linea[digitos..].strip_prefix(". ")
    {
        return (Clase::Lista { ordenada: true }, r);
    }
    (Clase::Parrafo, linea)
}

/// Junta líneas seguidas de la misma clase en un bloque.
fn lineas(texto: &str, usuarios: &HashMap<String, Usuario>, salida: &mut Vec<Bloque>) {
    let texto = texto.trim_matches('\n');
    if texto.is_empty() {
        return;
    }
    let mut grupos: Vec<(Clase, Vec<&str>)> = Vec::new();
    for linea in texto.split('\n') {
        let (clase, contenido) = clasificar(linea);
        match grupos.last_mut() {
            Some((c, v)) if *c == clase => v.push(contenido),
            _ => grupos.push((clase, vec![contenido])),
        }
    }
    salida.extend(grupos.into_iter().map(|(clase, v)| match clase {
        Clase::Parrafo => Bloque::Parrafo(segmentos(&v.join("\n"), usuarios)),
        Clase::Cita => Bloque::Cita(segmentos(&v.join("\n"), usuarios)),
        Clase::Lista { ordenada } => Bloque::Lista {
            ordenada,
            items: v.iter().map(|l| segmentos(l, usuarios)).collect(),
        },
    }));
}

/// Texto sin marcas con las referencias ya traducidas, para los bloques de código.
pub fn texto_plano(crudo: &str, usuarios: &HashMap<String, Usuario>) -> String {
    let mut salida = String::with_capacity(crudo.len());
    let mut resto = crudo;
    while let Some(inicio) = resto.find('<')
        && let Some(fin) = resto[inicio..].find('>')
    {
        salida.push_str(&entidades(&resto[..inicio]));
        salida.push_str(&referencia(&resto[inicio + 1..inicio + fin], usuarios).texto);
        resto = &resto[inicio + fin + 1..];
    }
    salida.push_str(&entidades(resto));
    salida
}

/// Si el texto avisa a todo el canal (`<!here>`, `<!channel>`, `<!everyone>`).
pub fn aviso_general(crudo: &str) -> bool {
    ["<!here", "<!channel", "<!everyone"]
        .iter()
        .any(|a| crudo.contains(a))
}

/// IDs de usuario mencionados con `<@U…>`, para resolverlos.
pub fn menciones(crudo: &str) -> impl Iterator<Item = &str> {
    crudo.split("<@").skip(1).filter_map(|s| {
        let fin = s.find(['>', '|'])?;
        Some(&s[..fin])
    })
}

/// Emoji de un nombre de reacción (`thumbsup`, `+1::skin-tone-3`); si no es estándar
/// se devuelve como `:nombre:`.
pub fn emoji(nombre: &str) -> String {
    let base = nombre.split("::").next().unwrap_or(nombre);
    match emojis::get_by_shortcode(base) {
        Some(e) => e.as_str().to_string(),
        None => format!(":{base}:"),
    }
}

/// Parte el texto en tramos de emoji y de texto normal (`true` = emoji), para
/// pintar los emoji con la fuente a color: si no, el sistema puede tomarlos de una
/// fuente de símbolos en blanco y negro.
pub fn partir_emoji(s: &str) -> Vec<(&str, bool)> {
    let mut tramos: Vec<(&str, bool)> = Vec::new();
    let mut inicio = 0;
    let mut actual: Option<bool> = None;
    for (i, c) in s.char_indices() {
        // ZWJ y el selector de variación pegan con lo que tengan antes.
        let pegado = matches!(c, '\u{200d}' | '\u{fe0f}');
        let es = if pegado {
            actual.unwrap_or(false)
        } else {
            es_emoji(c)
        };
        if actual.is_some_and(|a| a != es) {
            tramos.push((&s[inicio..i], !es));
            inicio = i;
        }
        actual = Some(es);
    }
    if let Some(es) = actual {
        tramos.push((&s[inicio..], es));
    }
    tramos
}

/// Códigos `:nombre:` de un texto (sin los tonos de piel), para saber qué emoji
/// propios hay que bajar.
pub fn codigos_emoji(s: &str) -> impl Iterator<Item = &str> {
    let mut resto = s;
    std::iter::from_fn(move || {
        while let Some(inicio) = resto.find(':') {
            let despues = &resto[inicio + 1..];
            let fin = despues
                .find(|c: char| !es_letra_de_codigo(c))
                .unwrap_or(despues.len());
            if fin > 0 && despues[fin..].starts_with(':') {
                let nombre = &despues[..fin];
                resto = &despues[fin + 1..];
                if !nombre.starts_with("skin-tone-") {
                    return Some(nombre);
                }
            } else {
                resto = despues;
            }
        }
        None
    })
}

fn es_letra_de_codigo(c: char) -> bool {
    c.is_ascii_alphanumeric() || "_+-".contains(c)
}

/// Pieza de un mensaje que es solo emoji: Slack los pinta grandes.
#[derive(Debug, Clone, PartialEq)]
pub enum Jumbo {
    /// Emoji propio del workspace, por nombre.
    Propio(String),
    Unicode(String),
}

/// Si el texto es solo emoji (hasta 23), sus piezas. `es_propio` dice si hay imagen
/// para un `:nombre:` que no es estándar.
pub fn jumbo(texto: &str, es_propio: impl Fn(&str) -> bool) -> Option<Vec<Jumbo>> {
    let mut piezas = Vec::new();
    let mut resto = texto.trim();
    while !resto.is_empty() {
        if let Some(r) = resto.strip_prefix(':') {
            let fin = r.find(':')?;
            let nombre = &r[..fin];
            piezas.push(match emojis::get_by_shortcode(nombre) {
                Some(e) => Jumbo::Unicode(e.as_str().into()),
                None if es_propio(nombre) => Jumbo::Propio(nombre.into()),
                None => return None,
            });
            resto = &r[fin + 1..];
            if let Some(tono) = resto.strip_prefix(":skin-tone-")
                && let Some(cierre) = tono.find(':')
            {
                resto = &tono[cierre + 1..];
            }
        } else {
            let fin = resto
                .find(|c: char| c.is_whitespace() || c == ':')
                .unwrap_or(resto.len());
            let tramo = &resto[..fin];
            if partir_emoji(tramo).iter().any(|(_, es)| !es) {
                return None;
            }
            piezas.extend(racimos(tramo).map(|e| Jumbo::Unicode(e.into())));
            resto = &resto[fin..];
        }
        resto = resto.trim_start();
    }
    (!piezas.is_empty() && piezas.len() <= 23).then_some(piezas)
}

/// Parte una tira de emoji en emoji sueltos: tonos de piel, selectores y lo unido
/// con ZWJ se quedan con el anterior. Es una aproximación, no la segmentación Unicode
/// completa (las banderas, por ejemplo, salen en dos).
fn racimos(s: &str) -> impl Iterator<Item = &str> {
    let mut inicio = 0;
    let mut anterior: Option<char> = None;
    let mut cortes = Vec::new();
    for (i, c) in s.char_indices() {
        let pegado =
            matches!(c as u32, 0x1f3fb..=0x1f3ff | 0x200d | 0xfe0f) || anterior == Some('\u{200d}');
        if i > 0 && !pegado {
            cortes.push(&s[inicio..i]);
            inicio = i;
        }
        anterior = Some(c);
    }
    if !s.is_empty() {
        cortes.push(&s[inicio..]);
    }
    cortes.into_iter()
}

fn es_emoji(c: char) -> bool {
    matches!(c as u32,
        0x1f000..=0x1faff | 0x2600..=0x27bf | 0x2b00..=0x2bff | 0x2300..=0x23ff)
}

fn referencia(dentro: &str, usuarios: &HashMap<String, Usuario>) -> Segmento {
    let (destino, etiqueta) = match dentro.split_once('|') {
        Some((d, e)) => (d, Some(e)),
        None => (dentro, None),
    };
    if let Some(id) = destino.strip_prefix('@') {
        let nombre = usuarios
            .get(id)
            .map(Usuario::nombre_visible)
            .or(etiqueta)
            .unwrap_or(id);
        return Segmento::new(format!("@{nombre}"), Tipo::Persona(id.to_string()));
    }
    if let Some(id) = destino.strip_prefix('#') {
        return Segmento::new(format!("#{}", etiqueta.unwrap_or(id)), Tipo::Mencion);
    }
    if let Some(especial) = destino.strip_prefix('!') {
        let texto = match etiqueta {
            Some(e) => e.to_string(),
            None => format!("@{especial}"),
        };
        let tipo = if ["here", "channel", "everyone"].contains(&especial) {
            Tipo::Aviso
        } else {
            Tipo::Mencion
        };
        return Segmento::new(texto, tipo);
    }
    let texto = entidades(etiqueta.unwrap_or(destino));
    Segmento::new(texto, Tipo::Enlace(entidades(destino)))
}

/// Parte texto en segmentos según las marcas de mrkdwn, anidando: dentro de cada
/// tramo marcado se vuelve a buscar marcas con las de afuera puestas.
fn marcas(s: &str, actuales: Marcas, refs: &[Segmento], salida: &mut Vec<Segmento>) {
    let mut normal = String::new();
    let mut i = 0;
    while i < s.len() {
        let resto = &s[i..];
        if !actuales.codigo
            && let Some(bloque) = resto.strip_prefix("```")
            && let Some(fin) = bloque.find("```")
        {
            hoja(&std::mem::take(&mut normal), actuales, refs, salida);
            let codigo = Marcas {
                codigo: true,
                ..actuales
            };
            hoja(bloque[..fin].trim_matches('\n'), codigo, refs, salida);
            i += fin + 6;
            continue;
        }
        let c = resto.chars().next().expect("no vacío");
        let ancho = c.len_utf8();
        let antes_ok = s[..i]
            .chars()
            .next_back()
            .is_none_or(|p| !p.is_alphanumeric());
        if let Some(nuevas) = actuales.con(c).filter(|_| antes_ok)
            && let Some(fin) = resto[ancho..].find(c)
        {
            let dentro = &resto[ancho..ancho + fin];
            let despues = resto[2 * ancho + fin..].chars().next();
            let valido = !dentro.is_empty()
                && !dentro.contains('\n')
                && !dentro.starts_with(' ')
                && !dentro.ends_with(' ')
                && despues.is_none_or(|d| !d.is_alphanumeric());
            if valido {
                hoja(&std::mem::take(&mut normal), actuales, refs, salida);
                if nuevas.codigo {
                    hoja(dentro, nuevas, refs, salida);
                } else {
                    marcas(dentro, nuevas, refs, salida);
                }
                i += 2 * ancho + fin;
                continue;
            }
        }
        normal.push(c);
        i += ancho;
    }
    hoja(&normal, actuales, refs, salida);
}

/// Emite texto ya sin marcas, devolviendo las referencias a su lugar.
fn hoja(texto: &str, marcas: Marcas, refs: &[Segmento], salida: &mut Vec<Segmento>) {
    let emitir = |t: &str, salida: &mut Vec<Segmento>| {
        if !t.is_empty() {
            let t = if marcas.codigo {
                t.to_string()
            } else {
                con_emoji(t)
            };
            salida.push(Segmento {
                texto: t,
                marcas,
                tipo: Tipo::Normal,
            });
        }
    };
    let mut inicio = 0;
    for (i, c) in texto.char_indices() {
        let Some(r) = (c as u32)
            .checked_sub(MARCADOR)
            .and_then(|n| refs.get(n as usize))
        else {
            continue;
        };
        emitir(&texto[inicio..i], salida);
        salida.push(Segmento {
            marcas,
            ..r.clone()
        });
        inicio = i + c.len_utf8();
    }
    emitir(&texto[inicio..], salida);
}

pub(crate) fn entidades(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Reemplaza `:nombre:` por el emoji Unicode cuando es un código estándar.
fn con_emoji(s: &str) -> String {
    let mut salida = String::with_capacity(s.len());
    let mut resto = s;
    while let Some(inicio) = resto.find(':') {
        let despues = &resto[inicio + 1..];
        let fin = despues
            .find(|c: char| !es_letra_de_codigo(c))
            .unwrap_or(despues.len());
        if fin > 0 && despues[fin..].starts_with(':') {
            let nombre = &despues[..fin];
            if let Some(e) = emojis::get_by_shortcode(nombre) {
                salida.push_str(&resto[..inicio]);
                salida.push_str(e.as_str());
                resto = &despues[fin + 1..];
                // Tonos de piel (`:wave::skin-tone-2:`): se descartan.
                if let Some(tono) = resto.strip_prefix(":skin-tone-")
                    && let Some(cierre) = tono.find(':')
                {
                    resto = &tono[cierre + 1..];
                }
                continue;
            }
        }
        salida.push_str(&resto[..=inicio]);
        resto = despues;
    }
    salida.push_str(resto);
    salida
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slack::Perfil;

    fn usuarios() -> HashMap<String, Usuario> {
        let u = Usuario {
            id: "U1".into(),
            name: "arturo".into(),
            tz: None,
            tz_offset: 0,
            profile: Perfil {
                display_name: Some("Arturo".into()),
                ..Perfil::default()
            },
        };
        HashMap::from([("U1".into(), u)])
    }

    /// Segmento con marcas abreviadas: `n` negrita, `c` cursiva, `t` tachado, `k` código.
    fn seg(texto: &str, tipo: Tipo, m: &str) -> Segmento {
        Segmento {
            texto: texto.into(),
            marcas: Marcas {
                negrita: m.contains('n'),
                cursiva: m.contains('c'),
                tachado: m.contains('t'),
                codigo: m.contains('k'),
            },
            tipo,
        }
    }

    fn normal(texto: &str, m: &str) -> Segmento {
        seg(texto, Tipo::Normal, m)
    }

    fn sin_usuarios(crudo: &str) -> Vec<Segmento> {
        segmentos(crudo, &HashMap::new())
    }

    #[test]
    fn traduce_referencias() {
        let crudo = "hola <@U1>, ve <#C9|general> y <https://x.mx|esto> &amp; <!here> <@U2>";
        assert_eq!(
            segmentos(crudo, &usuarios()),
            [
                normal("hola ", ""),
                seg("@Arturo", Tipo::Persona("U1".into()), ""),
                normal(", ve ", ""),
                seg("#general", Tipo::Mencion, ""),
                normal(" y ", ""),
                seg("esto", Tipo::Enlace("https://x.mx".into()), ""),
                normal(" & ", ""),
                seg("@here", Tipo::Aviso, ""),
                normal(" ", ""),
                seg("@U2", Tipo::Persona("U2".into()), ""),
            ]
        );
    }

    #[test]
    fn avisos_a_todos() {
        assert!(aviso_general("hola <!here> ya"));
        assert!(aviso_general("<!channel|@channel> junta"));
        assert!(aviso_general("<!everyone>"));
        assert!(!aviso_general("hola <@U1> y <#C1|general>"));
    }

    #[test]
    fn interpreta_marcas() {
        assert_eq!(
            sin_usuarios("a *b* _c_ ~d~ `e*f*` x*y*z 2*3 ```\nfn x()\n```"),
            [
                normal("a ", ""),
                normal("b", "n"),
                normal(" ", ""),
                normal("c", "c"),
                normal(" ", ""),
                normal("d", "t"),
                normal(" ", ""),
                normal("e*f*", "k"),
                normal(" x*y*z 2*3 ", ""),
                normal("fn x()", "k"),
            ]
        );
    }

    #[test]
    fn anida_marcas() {
        assert_eq!(sin_usuarios("*_x_*"), [normal("x", "nc")]);
        assert_eq!(
            sin_usuarios("~*a*~ b"),
            [normal("a", "nt"), normal(" b", "")]
        );
        assert_eq!(
            sin_usuarios("*a _b_ c*"),
            [normal("a ", "n"), normal("b", "nc"), normal(" c", "n")]
        );
        assert_eq!(sin_usuarios("_`*x*`_"), [normal("*x*", "ck")]);
    }

    #[test]
    fn marcas_alrededor_de_referencias() {
        assert_eq!(
            segmentos("*hola <@U1>*", &usuarios()),
            [
                normal("hola ", "n"),
                seg("@Arturo", Tipo::Persona("U1".into()), "n")
            ]
        );
        assert_eq!(
            sin_usuarios("_ve <https://x.mx|aquí>_ ya"),
            [
                normal("ve ", "c"),
                seg("aquí", Tipo::Enlace("https://x.mx".into()), "c"),
                normal(" ya", ""),
            ]
        );
        assert_eq!(sin_usuarios("a < b"), [normal("a < b", "")]);
    }

    #[test]
    fn convierte_emoji() {
        assert_eq!(
            con_emoji("listo :+1::skin-tone-3: a las 10:30 :nope:"),
            "listo 👍 a las 10:30 :nope:"
        );
        assert_eq!(emoji("white_check_mark"), "✅");
        assert_eq!(emoji("logo-empresa"), ":logo-empresa:");
    }

    #[test]
    fn parte_emoji() {
        assert_eq!(
            partir_emoji("hola 👍🏽 y 👨‍💻!"),
            [
                ("hola ", false),
                ("👍🏽", true),
                (" y ", false),
                ("👨‍💻", true),
                ("!", false)
            ]
        );
        assert_eq!(partir_emoji("✅"), [("✅", true)]);
        assert!(partir_emoji("").is_empty());
    }

    fn parrafo(crudo: &str) -> Bloque {
        Bloque::Parrafo(sin_usuarios(crudo))
    }

    #[test]
    fn bloques_parrafo_y_cita() {
        assert_eq!(bloques("hola *x*", &HashMap::new()), [parrafo("hola *x*")]);
        assert_eq!(
            bloques("&gt; uno\n&gt;dos *b*\nfuera", &HashMap::new()),
            [Bloque::Cita(sin_usuarios("uno\ndos *b*")), parrafo("fuera")]
        );
    }

    #[test]
    fn bloques_de_codigo() {
        assert_eq!(
            bloques(
                "mira:\n```\nfn x() {\n  1 &lt; *2*\n}\n```\nya <@U1>",
                &usuarios()
            ),
            [
                parrafo("mira:"),
                Bloque::Codigo("fn x() {\n  1 < *2*\n}".into()),
                Bloque::Parrafo(segmentos("ya <@U1>", &usuarios())),
            ]
        );
        assert_eq!(
            bloques("```<https://x.mx>``` y ```abc", &HashMap::new()),
            [Bloque::Codigo("https://x.mx".into()), parrafo(" y ```abc")]
        );
    }

    #[test]
    fn bloques_de_listas() {
        assert_eq!(
            bloques("• a\n• *b*\n1. c\n2. d\n- e", &HashMap::new()),
            [
                Bloque::Lista {
                    ordenada: false,
                    items: vec![sin_usuarios("a"), sin_usuarios("*b*")],
                },
                Bloque::Lista {
                    ordenada: true,
                    items: vec![sin_usuarios("c"), sin_usuarios("d")],
                },
                parrafo("- e"),
            ]
        );
    }

    #[test]
    fn bloques_de_rich_text() {
        let m: crate::slack::Mensaje =
            serde_json::from_str(include_str!("../tests/datos/rich_text.json")).unwrap();
        let canal = |id: &str| {
            if id == "C1" {
                "general".into()
            } else {
                id.into()
            }
        };
        let enlace = |t: &str, url: &str, m: &str| seg(t, Tipo::Enlace(url.into()), m);
        assert_eq!(
            del_mensaje(&m, &usuarios(), canal),
            [
                Bloque::Parrafo(vec![
                    normal("Hola ", ""),
                    seg("@Arturo", Tipo::Persona("U1".into()), ""),
                    normal(", mira ", "nc"),
                    enlace("esto", "https://x.mx", ""),
                    normal(" en ", ""),
                    seg("#general", Tipo::Mencion, ""),
                    normal(" ", ""),
                    normal("👍", ""),
                    normal(" ", ""),
                    seg("@here", Tipo::Aviso, ""),
                ]),
                Bloque::Lista {
                    ordenada: true,
                    items: vec![vec![normal("uno", "")], vec![normal("dos", "k")]],
                },
                Bloque::Cita(vec![normal("citado", "t")]),
                Bloque::Codigo("let x = 1;\nhttps://y.mx".into()),
                Bloque::Parrafo(vec![normal(":logo-empresa:", "")]),
            ]
        );
    }

    #[test]
    fn sin_rich_text_usa_mrkdwn() {
        let m = crate::slack::Mensaje {
            text: "&gt; cita".into(),
            ..Default::default()
        };
        assert_eq!(
            del_mensaje(&m, &HashMap::new(), str::to_string),
            [Bloque::Cita(sin_usuarios("cita"))]
        );
    }

    #[test]
    fn extrae_codigos_de_emoji() {
        let codigos: Vec<_> =
            codigos_emoji("hola :loro: y :+1::skin-tone-2: a las 10:30 :fin:").collect();
        assert_eq!(codigos, ["loro", "+1", "fin"]);
    }

    #[test]
    fn mensajes_de_puro_emoji() {
        let propio = |n: &str| n == "loro";
        let p = |n: &str| Jumbo::Propio(n.into());
        let u = |s: &str| Jumbo::Unicode(s.into());
        assert_eq!(jumbo(" :loro: 🎉 ", propio), Some(vec![p("loro"), u("🎉")]));
        assert_eq!(
            jumbo(":loro::loro:", propio),
            Some(vec![p("loro"), p("loro")])
        );
        assert_eq!(jumbo("👍🏽✅", propio), Some(vec![u("👍🏽"), u("✅")]));
        assert_eq!(jumbo("hola :loro:", propio), None);
        assert_eq!(
            jumbo(":otro:", propio),
            None,
            "si no hay imagen se queda como texto"
        );
        assert_eq!(jumbo("", propio), None);
        assert_eq!(
            jumbo(&"🎉".repeat(24), propio),
            None,
            "con muchos ya no es jumbo"
        );
    }

    #[test]
    fn extrae_menciones() {
        let ids: Vec<_> = menciones("a <@U1> b <@U2|x> c").collect();
        assert_eq!(ids, ["U1", "U2"]);
    }
}
