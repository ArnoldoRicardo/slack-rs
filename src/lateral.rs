//! Filtro y orden de la lateral, como el menú ⚙ de Slack ("Filter and sort").
//! Por defecto Slack muestra solo lo activo en 30 días, más lo que tengas abierto.

/// Ventana de "Active only": 30 días, en segundos.
const VENTANA_ACTIVOS: f64 = 30.0 * 24.0 * 3600.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Filtro {
    #[default]
    Activos,
    NoLeidos,
    Menciones,
    Todos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Orden {
    #[default]
    Alfabetico,
    Reciente,
}

/// Lo mínimo de una conversación para decidir si se ve y dónde.
#[derive(Debug, Clone, Default)]
pub(crate) struct Item<'a> {
    pub(crate) id: &'a str,
    pub(crate) nombre: &'a str,
    /// Último mensaje, en segundos Unix (el `ts` de Slack).
    pub(crate) ultima: Option<f64>,
    pub(crate) no_leido: bool,
    pub(crate) menciones: u32,
}

/// Ids a mostrar, ya ordenados. `abierto` se muestra siempre, pase o no el filtro.
pub(crate) fn visibles<'a>(
    items: &[Item<'a>],
    filtro: Filtro,
    orden: Orden,
    abierto: Option<&str>,
    ahora: f64,
) -> Vec<&'a str> {
    let pasa = |i: &Item| match filtro {
        Filtro::Activos => i.no_leido || i.ultima.is_some_and(|t| ahora - t < VENTANA_ACTIVOS),
        Filtro::NoLeidos => i.no_leido || i.menciones > 0,
        Filtro::Menciones => i.menciones > 0,
        Filtro::Todos => true,
    };
    let mut lista: Vec<&Item<'a>> = items
        .iter()
        .filter(|i| pasa(i) || abierto == Some(i.id))
        .collect();
    let por_nombre = |a: &Item, b: &Item| a.nombre.to_lowercase().cmp(&b.nombre.to_lowercase());
    match orden {
        Orden::Alfabetico => lista.sort_by(|a, b| por_nombre(a, b)),
        Orden::Reciente => lista.sort_by(|a, b| {
            let fecha = |i: &Item| i.ultima.unwrap_or(f64::NEG_INFINITY);
            fecha(b).total_cmp(&fecha(a)).then_with(|| por_nombre(a, b))
        }),
    }
    lista.into_iter().map(|i| i.id).collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const AHORA: f64 = 1_800_000_000.0;
    const DIA: f64 = 24.0 * 3600.0;

    fn item<'a>(id: &'a str, hace_dias: Option<f64>) -> Item<'a> {
        Item {
            id,
            nombre: id,
            ultima: hace_dias.map(|d| AHORA - d * DIA),
            ..Default::default()
        }
    }

    fn ver(items: &[Item], filtro: Filtro, abierto: Option<&str>) -> Vec<String> {
        visibles(items, filtro, Orden::Alfabetico, abierto, AHORA)
            .into_iter()
            .map(String::from)
            .collect()
    }

    #[test]
    fn activos_son_los_de_30_dias() {
        let items = [
            item("viejo", Some(31.0)),
            item("nuevo", Some(2.0)),
            item("borde", Some(29.9)),
        ];
        assert_eq!(ver(&items, Filtro::Activos, None), ["borde", "nuevo"]);
    }

    #[test]
    fn el_abierto_se_ve_aunque_este_inactivo() {
        let items = [item("general", Some(40.0)), item("tech", Some(1.0))];
        assert_eq!(
            ver(&items, Filtro::Activos, Some("general")),
            ["general", "tech"]
        );
    }

    #[test]
    fn sin_fecha_es_inactivo_salvo_que_tenga_no_leidos() {
        let mut pendiente = item("pendiente", None);
        pendiente.no_leido = true;
        let items = [item("mudo", None), pendiente];
        assert_eq!(ver(&items, Filtro::Activos, None), ["pendiente"]);
    }

    #[test]
    fn no_leidos_y_menciones() {
        let mut a = item("a", Some(1.0));
        a.no_leido = true;
        let mut b = item("b", Some(1.0));
        b.menciones = 2;
        let items = [a, b, item("c", Some(1.0))];
        assert_eq!(ver(&items, Filtro::NoLeidos, None), ["a", "b"]);
        assert_eq!(ver(&items, Filtro::Menciones, None), ["b"]);
        assert_eq!(ver(&items, Filtro::Todos, None), ["a", "b", "c"]);
    }

    #[test]
    fn alfabetico_ignora_mayusculas() {
        let items = [
            item("beta", Some(1.0)),
            item("Alfa", Some(1.0)),
            item("gama", Some(1.0)),
        ];
        assert_eq!(ver(&items, Filtro::Todos, None), ["Alfa", "beta", "gama"]);
    }

    #[test]
    fn reciente_primero_y_desempata_por_nombre() {
        let items = [
            item("b", Some(5.0)),
            item("a", Some(5.0)),
            item("sin", None),
            item("hoy", Some(0.1)),
        ];
        let orden = visibles(&items, Filtro::Todos, Orden::Reciente, None, AHORA);
        assert_eq!(orden, ["hoy", "a", "b", "sin"]);
    }
}
