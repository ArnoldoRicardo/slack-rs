//! Textos derivados que pinta la UI: quién reaccionó, quién se unió, enlaces
//! cortos y enlace permanente de un mensaje.

/// Tooltip de una reacción: "Ana, Beto y tú reaccionaron con :eyes:".
/// Nombra hasta tres personas además de ti; el resto va como "y N más".
pub(crate) fn quienes_reaccionaron(otros: &[&str], incluye_yo: bool, emoji: &str) -> String {
    let mut nombres: Vec<&str> = otros.iter().take(3).copied().collect();
    if incluye_yo {
        nombres.push("tú");
    }
    let resto = otros.len().saturating_sub(3);
    let quienes = if resto > 0 {
        format!("{} y {resto} más", nombres.join(", "))
    } else {
        lista(&nombres)
    };
    let verbo = match (otros.len(), incluye_yo) {
        (0, true) => return format!("Tú reaccionaste con :{emoji}:"),
        (1, false) => "reaccionó",
        _ => "reaccionaron",
    };
    format!("{quienes} {verbo} con :{emoji}:")
}

/// "a", "a y b", "a, b y c".
fn lista(nombres: &[&str]) -> String {
    match nombres {
        [] => String::new(),
        [uno] => uno.to_string(),
        [inicio @ .., ultimo] => format!("{} y {ultimo}", inicio.join(", ")),
    }
}

/// Aviso bajo el compositor: "Ana está escribiendo…".
pub(crate) fn escribiendo(nombres: &[&str]) -> String {
    match nombres {
        [] => String::new(),
        [uno] => format!("{uno} está escribiendo…"),
        [a, b] => format!("{a} y {b} están escribiendo…"),
        _ => "Varias personas están escribiendo…".into(),
    }
}

/// Mensajes de "se unió" seguidos, en una línea: "Ana y 3 más se unieron a #general."
pub(crate) fn agrupar_union(nombres: &[&str], canal: &str) -> String {
    match nombres {
        [uno] => format!("{uno} se unió a #{canal}."),
        [a, b] => format!("{a} y {b} se unieron a #{canal}."),
        [primero, resto @ ..] => format!("{primero} y {} más se unieron a #{canal}.", resto.len()),
        [] => String::new(),
    }
}

/// Enlace para mostrar, sin esquema y recortado al medio como Slack:
/// `drive.google.com/drive/folders/…?usp=sharing`. Conserva el principio y la
/// consulta, o el último tramo de la ruta si no hay consulta.
pub(crate) fn enlace_corto(url: &str, max: usize) -> String {
    let sin_esquema = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    if sin_esquema.chars().count() <= max {
        return sin_esquema.to_string();
    }
    let fin_dominio = sin_esquema.find('/').unwrap_or(sin_esquema.len());
    let cola = match sin_esquema.find('?') {
        Some(i) => &sin_esquema[i..],
        None => match sin_esquema.rfind('/') {
            Some(i) if i > fin_dominio => &sin_esquema[i..],
            _ => "",
        },
    };
    let espacio = max.saturating_sub(cola.chars().count() + 2);
    let prefijo: String = sin_esquema.chars().take(espacio).collect();
    let cabeza = match prefijo.rfind('/') {
        Some(i) if i >= fin_dominio => &prefijo[..i],
        _ => &sin_esquema[..fin_dominio],
    };
    format!("{cabeza}/…{cola}")
}

/// Enlace permanente de un mensaje ("Copy link"). `dominio` es `<sub>.slack.com`.
pub(crate) fn permalink(dominio: &str, canal: &str, ts: &str, hilo: Option<&str>) -> String {
    let base = format!(
        "https://{dominio}/archives/{canal}/p{}",
        ts.replace('.', "")
    );
    match hilo {
        Some(raiz) => format!("{base}?thread_ts={raiz}&cid={canal}"),
        None => base,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn quien_esta_escribiendo() {
        assert_eq!(escribiendo(&[]), "");
        assert_eq!(escribiendo(&["Ana"]), "Ana está escribiendo…");
        assert_eq!(
            escribiendo(&["Ana", "Beto"]),
            "Ana y Beto están escribiendo…"
        );
        assert_eq!(
            escribiendo(&["Ana", "Beto", "Caro"]),
            "Varias personas están escribiendo…"
        );
    }

    #[test]
    fn reacciones_de_una_persona() {
        assert_eq!(
            quienes_reaccionaron(&["Ana"], false, "eyes"),
            "Ana reaccionó con :eyes:"
        );
        assert_eq!(
            quienes_reaccionaron(&[], true, "eyes"),
            "Tú reaccionaste con :eyes:"
        );
    }

    #[test]
    fn reacciones_de_varias_con_y_sin_ti() {
        assert_eq!(
            quienes_reaccionaron(&["Ana", "Beto"], false, "+1"),
            "Ana y Beto reaccionaron con :+1:"
        );
        assert_eq!(
            quienes_reaccionaron(&["Ana", "Beto", "Caro"], true, "+1"),
            "Ana, Beto, Caro y tú reaccionaron con :+1:"
        );
    }

    #[test]
    fn reacciones_de_muchas_resume_el_resto() {
        assert_eq!(
            quienes_reaccionaron(&["Ana", "Beto", "Caro", "Dani", "Eli"], true, "tada"),
            "Ana, Beto, Caro, tú y 2 más reaccionaron con :tada:"
        );
    }

    #[test]
    fn union_de_uno_dos_y_muchos() {
        assert_eq!(
            agrupar_union(&["Ana"], "general"),
            "Ana se unió a #general."
        );
        assert_eq!(
            agrupar_union(&["Ana", "Beto"], "general"),
            "Ana y Beto se unieron a #general."
        );
        assert_eq!(
            agrupar_union(&["Ana", "Beto", "Caro", "Dani"], "general"),
            "Ana y 3 más se unieron a #general."
        );
    }

    #[test]
    fn enlace_corto_quita_el_esquema() {
        assert_eq!(enlace_corto("https://github.com/org", 40), "github.com/org");
        assert_eq!(enlace_corto("http://a.com/", 40), "a.com/");
    }

    #[test]
    fn enlace_largo_conserva_la_consulta() {
        let url = "https://drive.google.com/drive/folders/1AbCdEfGhIjKlMn?usp=sharing";
        assert_eq!(
            enlace_corto(url, 45),
            "drive.google.com/drive/folders/…?usp=sharing"
        );
    }

    #[test]
    fn enlace_largo_sin_consulta_conserva_el_ultimo_tramo() {
        let url = "https://github.com/acme/infra-ooh/blob/main/.github/deploy.yml";
        assert_eq!(
            enlace_corto(url, 40),
            "github.com/acme/infra-ooh/…/deploy.yml"
        );
    }

    #[test]
    fn enlace_largo_sin_ruta_que_quepa_deja_el_dominio() {
        let url = "https://ejemplo.com/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/archivo.pdf";
        assert_eq!(enlace_corto(url, 30), "ejemplo.com/…/archivo.pdf");
    }

    #[test]
    fn permalink_de_canal_y_de_hilo() {
        assert_eq!(
            permalink("equipo.slack.com", "C123", "1726086000.123456", None),
            "https://equipo.slack.com/archives/C123/p1726086000123456"
        );
        assert_eq!(
            permalink(
                "equipo.slack.com",
                "C123",
                "1726086100.000200",
                Some("1726086000.123456")
            ),
            "https://equipo.slack.com/archives/C123/p1726086100000200\
             ?thread_ts=1726086000.123456&cid=C123"
        );
    }
}
