//! "Saltar a…" (Ctrl+K): elegir una conversación escribiendo parte de su nombre.

/// Una conversación que se puede elegir: ID, nombre visible y si tiene no leídos.
pub struct Candidato<'a> {
    pub id: &'a str,
    pub nombre: String,
    pub no_leido: bool,
}

/// IDs de los candidatos que coinciden, del mejor al peor: primero los que empiezan con
/// la consulta, luego los que tienen una palabra que empieza con ella, y al final los
/// que solo la contienen. En cada nivel van primero los no leídos y luego el orden
/// original. Sin consulta salen todos, los no leídos primero.
pub fn coincidencias<'a>(candidatos: &[Candidato<'a>], consulta: &str) -> Vec<&'a str> {
    let consulta = normalizar(consulta.trim().trim_start_matches(['#', '@']));
    let mut con_nivel: Vec<_> = candidatos
        .iter()
        .filter_map(|c| {
            let nombre = normalizar(&c.nombre);
            let nivel = if nombre.starts_with(&consulta) {
                0
            } else if nombre
                .split(|ch: char| !ch.is_alphanumeric())
                .any(|p| p.starts_with(&consulta))
            {
                1
            } else if nombre.contains(&consulta) {
                2
            } else {
                return None;
            };
            Some((nivel, !c.no_leido, c.id))
        })
        .collect();
    con_nivel.sort_by_key(|&(nivel, leido, _)| (nivel, leido));
    con_nivel.into_iter().map(|(_, _, id)| id).collect()
}

/// Minúsculas y sin acentos, para comparar como lo escribe la gente.
fn normalizar(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            otro => otro,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c<'a>(id: &'a str, nombre: &str, no_leido: bool) -> Candidato<'a> {
        Candidato {
            id,
            nombre: nombre.into(),
            no_leido,
        }
    }

    fn lista() -> Vec<Candidato<'static>> {
        vec![
            c("C1", "general", false),
            c("C2", "desarrollo", false),
            c("C3", "despliegues", true),
            c("C4", "diseño-web", false),
            c("D1", "Ana López", false),
            c("D2", "Beto Ramírez", true),
            c("C5", "ops-desarrollo", false),
        ]
    }

    #[test]
    fn vacia_pone_primero_los_no_leidos() {
        assert_eq!(
            coincidencias(&lista(), ""),
            ["C3", "D2", "C1", "C2", "C4", "D1", "C5"]
        );
    }

    #[test]
    fn prefijo_antes_que_palabra_antes_que_subcadena() {
        // "desarrollo" empieza igual; "ops-desarrollo" tiene la palabra; nada más la contiene.
        assert_eq!(coincidencias(&lista(), "desa"), ["C2", "C5"]);
        // Mismo nivel: gana el no leído.
        assert_eq!(coincidencias(&lista(), "des"), ["C3", "C2", "C5"]);
        assert_eq!(coincidencias(&lista(), "rrollo"), ["C2", "C5"]);
    }

    #[test]
    fn ignora_acentos_mayusculas_y_espacios() {
        assert_eq!(coincidencias(&lista(), " LOPEZ"), ["D1"]);
        assert_eq!(coincidencias(&lista(), "ramirez"), ["D2"]);
        assert_eq!(coincidencias(&lista(), "diseno"), ["C4"]);
        assert_eq!(coincidencias(&lista(), "#general"), ["C1"]);
        assert!(coincidencias(&lista(), "zzz").is_empty());
    }
}
