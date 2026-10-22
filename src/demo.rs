//! Datos de ejemplo para ver la interfaz sin credenciales (`SLACK_DEMO=1`).

use std::collections::HashMap;

use serde_json::json;

use crate::espacio::{Espacio, Hilo};
use crate::slack::{
    BorradorSlack, Canal, Contador, Equipo, Icono, InfoCanal, ItemActividad, Marcador, Mensaje,
    ResumenHilo, Seccion, Sesion, TipoSeccion, Usuario,
};

pub fn espacios() -> Vec<Espacio> {
    vec![principal(), segundo()]
}

fn principal() -> Espacio {
    let usuario = |id: &str, name: &str, real: &str| -> Usuario {
        serde_json::from_value(json!({"id": id, "name": name, "profile": {"real_name": real}}))
            .expect("usuario demo")
    };
    let usuarios: HashMap<_, _> = [
        usuario("U0", "arturo", "Arturo Hernández"),
        usuario("U1", "ana", "Ana López"),
        usuario("U2", "beto", "Beto Ramírez"),
        usuario("U3", "carla", "Carla Méndez"),
    ]
    .into_iter()
    .map(|u| (u.id.clone(), u))
    .collect();

    let canal = |id: &str, nombre: &str, privado: bool| -> Canal {
        serde_json::from_value(json!({"id": id, "name": nombre, "is_private": privado}))
            .expect("canal demo")
    };
    let mut canales = vec![
        canal("C1", "general", false),
        canal("C2", "desarrollo", false),
        canal("C3", "despliegues", false),
        canal("C4", "directiva", true),
        canal("C5", "random", false),
        // Sin mensajes en más de 30 días: el filtro "Activos" los oculta.
        canal("C6", "archivo-2024", false),
        canal("C7", "proyecto-viejo", true),
        canal("C8", "ventas", false),
    ];
    canales.push(serde_json::from_value(json!({"id": "D1", "is_im": true, "user": "U1"})).unwrap());
    canales.push(serde_json::from_value(json!({"id": "D2", "is_im": true, "user": "U2"})).unwrap());
    canales.push(
        serde_json::from_value(
            json!({"id": "G1", "is_mpim": true, "name": "mpdm-ana--beto--carla-1"}),
        )
        .unwrap(),
    );

    let ahora = chrono::Local::now().timestamp();
    let ts = |hace_min: i64| format!("{}.000100", ahora - hace_min * 60);
    let mensaje = |hace_min: i64, user: &str, text: &str, extra: serde_json::Value| -> Mensaje {
        let mut v = json!({"ts": ts(hace_min), "user": user, "text": text});
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(v).expect("mensaje demo")
    };
    let raiz = ts(62);
    let desarrollo = vec![
        mensaje(
            1600,
            "U2",
            "¿Alguien revisó el PR del *cliente de Slack*?",
            json!({}),
        ),
        mensaje(
            1595,
            "U1",
            "Sí, lo dejé aprobado :white_check_mark:",
            json!({}),
        ),
        mensaje(
            1590,
            "U2",
            "<@U2> se unió al canal",
            json!({"subtype": "channel_join"}),
        ),
        mensaje(
            1589,
            "U3",
            "<@U3> se unió al canal",
            json!({"subtype": "channel_join"}),
        ),
        mensaje(
            62,
            "U3",
            "Ya quedó el tiempo real por websocket. Lo probé con `rtm.connect` y reconecta solo cada 5 s",
            json!({"thread_ts": raiz, "reply_count": 3, "reply_users": ["U1", "U3", "U0"],
                   "reactions": [{"name": "tada", "count": 3, "users": ["U1", "U2", "U0"]},
                                 {"name": "eyes", "count": 1, "users": ["U2"]}]}),
        ),
        mensaje(
            61,
            "U3",
            "Detalles en <https://github.com/iced-rs/iced|el repo de iced>",
            json!({}),
        ),
        mensaje(
            30,
            "U1",
            "<@U0> ¿puedes revisar el _diseño_ del compositor? Falta ~el botón viejo~ y el atajo de Intro",
            json!({"files": [{"id": "F1", "name": "compositor.png", "title": "compositor-v2.png",
                              "pretty_type": "PNG", "mimetype": "image/png",
                              "thumb_360": "https://files.slack.com/demo.png",
                              "thumb_360_w": 360, "thumb_360_h": 160}]}),
        ),
        mensaje(
            20,
            "U2",
            "Pasos para el *_release_*:\n• correr `cargo test`\n• revisar <#C3|despliegues>\n&gt; nunca en viernes\n```\ncargo build --release\nstrip target/release/slack-rs\n```",
            json!({}),
        ),
        mensaje(
            12,
            "U1",
            "<!here> <@U0> ya están las capturas del tablero: <https://drive.google.com/drive/folders/1AbCdEfGhIjKlMnOpQrStUvWxYz?usp=sharing>",
            json!({"files": (1..=5).map(|i| json!({
                "id": format!("F{}", i + 1), "name": format!("tablero-{i}.png"),
                "title": format!("tablero-{i}.png"), "pretty_type": "PNG",
                "mimetype": "image/png", "thumb_360": "https://files.slack.com/demo.png",
                "thumb_360_w": 360, "thumb_360_h": 160})).collect::<Vec<_>>()}),
        ),
        mensaje(
            9,
            "",
            "",
            json!({"user": null, "username": "ec2-bot", "attachments": [{
                "fallback": "EC2 stopped", "color": "danger",
                "title": "EC2 stopped", "title_link": "https://example.com/ec2",
                "text": "La instancia `i-0abc123` se detuvo",
                "fields": [{"title": "Región", "value": "us-west-2", "short": true},
                           {"title": "Cuenta", "value": "1234-5678-9012", "short": true}],
                "footer": "AWS"
            }]}),
        ),
        mensaje(
            4,
            "U0",
            "Va, lo veo en un rato :+1:",
            json!({"reactions": [{"name": "+1", "count": 1, "users": ["U1"]},
                                 {"name": "loro", "count": 2, "users": ["U2", "U3"]}]}),
        ),
        mensaje(2, "U3", ":loro: :tada:", json!({})),
    ];
    let hilo = vec![
        desarrollo
            .iter()
            .find(|m| m.ts == raiz)
            .cloned()
            .expect("raíz del hilo"),
        mensaje(
            58,
            "U1",
            "¡Excelente! ¿Y qué pasa si se cae el Wi-Fi?",
            json!({"thread_ts": raiz}),
        ),
        mensaje(
            55,
            "U3",
            "Muestra *Sin conexión en vivo* arriba y reintenta",
            json!({"thread_ts": raiz}),
        ),
        mensaje(
            40,
            "U0",
            "Perfecto, gracias <@U3>",
            json!({"thread_ts": raiz}),
        ),
    ];

    // Para Actividad y Guardados: la mención a Arturo y los pasos del release.
    let menciones: Vec<Mensaje> = desarrollo
        .iter()
        .filter(|m| m.text.contains("<@U0>") || m.text.contains("release"))
        .cloned()
        .collect();

    let mut personas: Vec<Usuario> = usuarios.values().cloned().collect();
    personas.sort_by(|a, b| a.nombre_visible().cmp(b.nombre_visible()));

    let contadores = HashMap::from([
        (
            "C3".to_string(),
            Contador {
                no_leidos: true,
                menciones: 0,
                ultimo: 0,
            },
        ),
        (
            "D2".to_string(),
            Contador {
                no_leidos: true,
                menciones: 2,
                ultimo: 0,
            },
        ),
    ]);

    Espacio {
        sesion: Some(Sesion {
            user_id: "U0".into(),
            team: "Demo".into(),
        }),
        equipo: Some(Equipo {
            name: "Acme MX".into(),
            domain: "acme-mx".into(),
            icon: Icono::default(),
        }),
        canales,
        contadores,
        usuarios,
        actividad: HashMap::from([
            ("C1".to_string(), ahora - 5 * 86400),
            ("C2".to_string(), ahora - 2 * 60),
            ("C3".to_string(), ahora - 3600),
            ("C4".to_string(), ahora - 9 * 86400),
            ("C5".to_string(), ahora - 20 * 86400),
            ("C6".to_string(), ahora - 100 * 86400),
            ("C7".to_string(), ahora - 45 * 86400),
            ("C8".to_string(), ahora - 2 * 86400),
            ("D1".to_string(), ahora - 3 * 60),
            ("D2".to_string(), ahora - 26 * 3600),
            ("G1".to_string(), ahora - 4 * 86400),
        ]),
        mensajes: HashMap::from([
            ("C2".to_string(), desarrollo),
            (
                "D1".to_string(),
                vec![
                    mensaje(240, "U1", "¿Me pasas el enlace del tablero?", json!({})),
                    mensaje(3, "U0", "Va, ahorita te lo mando", json!({})),
                ],
            ),
            (
                "D2".to_string(),
                vec![mensaje(
                    1560,
                    "U2",
                    "Oye, ¿viste lo del *deploy*? Avísame",
                    json!({}),
                )],
            ),
            (
                "G1".to_string(),
                vec![mensaje(
                    5760,
                    "U3",
                    "Nos vemos el lunes en la junta :wave:",
                    json!({}),
                )],
            ),
        ]),
        emoji_propios: HashMap::from([("loro".to_string(), String::new())]),
        emoji_imagenes: HashMap::from([("loro".to_string(), loro())]),
        miniaturas: (1..=6).map(|i| (format!("F{i}"), boceto())).collect(),
        secciones: Some(vec![
            seccion("L1", TipoSeccion::Destacados, "", &["C2", "D1"]),
            Seccion {
                emoji: "rocket".into(),
                ..seccion("L2", TipoSeccion::Propia, "Operación", &["C3", "C4"])
            },
            seccion("L3", TipoSeccion::Canales, "", &[]),
            seccion("L4", TipoSeccion::Directos, "", &[]),
        ]),
        silenciados: ["C5".to_string()].into(),
        hilos_vista: Some(vec![ResumenHilo {
            canal: "C2".into(),
            raiz: hilo[0].clone(),
            respuestas: hilo[2..].to_vec(),
        }]),
        bandeja: Some(vec![ItemActividad {
            tipo: "at_user".into(),
            canal: "C2".into(),
            ts: menciones[0].ts.clone(),
            autor: Some("U1".into()),
            no_leido: true,
        }]),
        guardados: Some(vec![("C2".into(), menciones[1].ts.clone())]),
        sueltos: menciones
            .iter()
            .map(|m| (("C2".to_string(), m.ts.clone()), m.clone()))
            .collect(),
        borradores_slack: Some(vec![BorradorSlack {
            canal: "C3".into(),
            mensaje: mensaje(
                0,
                "U0",
                "Despliegue del lunes: revisar migraciones",
                json!({}),
            ),
            programado: ahora + 86400,
        }]),
        personas: Some(personas),
        presentes: ["U1".to_string(), "U3".to_string()].into(),
        info: HashMap::from([(
            "C2".to_string(),
            InfoCanal {
                miembros: Some(13),
                tema: "Cliente de Slack en Rust".into(),
                proposito: "Todo lo del desarrollo: PRs, revisiones y despliegues.".into(),
                creado: ahora - 400 * 86400,
                creador: Some("U1".into()),
            },
        )]),
        miembros: HashMap::from([(
            "C2".to_string(),
            ["U1", "U2", "U3", "U0"].map(String::from).to_vec(),
        )]),
        marcadores: HashMap::from([(
            "C2".to_string(),
            vec![
                Marcador {
                    titulo: "Tablero".into(),
                    enlace: "https://example.com/tablero".into(),
                    emoji: "bar_chart".into(),
                },
                Marcador {
                    titulo: "Repo".into(),
                    enlace: "https://github.com/iced-rs/iced".into(),
                    emoji: String::new(),
                },
            ],
        )]),
        // El tema de Slack por defecto hoy: horchata con jade.
        tema: Some(crate::paleta::Tema {
            primaria: "horchata".into(),
            resalte: "jade".into(),
        }),
        seleccionado: Some("C2".into()),
        hilo: Some(Hilo {
            canal: "C2".into(),
            ts: raiz,
            mensajes: Some(hilo),
            editor: Default::default(),
            tambien: false,
        }),
        ..Espacio::default()
    }
}

/// Emoji propio de ejemplo: un degradado redondo, para no depender de la red.
fn loro() -> iced::widget::image::Handle {
    const LADO: u32 = 32;
    let mut pixeles = Vec::with_capacity((LADO * LADO * 4) as usize);
    for y in 0..LADO {
        for x in 0..LADO {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            let dentro = dx * dx + dy * dy <= 15.5 * 15.5;
            pixeles.extend([
                (x * 8) as u8,
                200,
                (255 - y * 8) as u8,
                if dentro { 255 } else { 0 },
            ]);
        }
    }
    iced::widget::image::Handle::from_rgba(LADO, LADO, pixeles)
}

/// Miniatura de ejemplo: un boceto de compositor en grises.
fn boceto() -> iced::widget::image::Handle {
    let (ancho, alto) = (360u32, 160u32);
    let mut pixeles = Vec::with_capacity((ancho * alto * 4) as usize);
    for y in 0..alto {
        for x in 0..ancho {
            let borde = !(12..ancho - 12).contains(&x) || !(12..alto - 12).contains(&y);
            let barra = (24..40).contains(&y) && (24..200).contains(&x);
            let boton = (120..148).contains(&y) && (290..340).contains(&x);
            let gris = match (borde, barra, boton) {
                (true, ..) => 0xdd,
                (_, true, _) => 0xbb,
                _ => 0xf8,
            };
            let (r, g, b) = if boton {
                (0x00, 0x7a, 0x5a)
            } else {
                (gris, gris, gris)
            };
            pixeles.extend([r, g, b, 255]);
        }
    }
    iced::widget::image::Handle::from_rgba(ancho, alto, pixeles)
}

fn seccion(id: &str, tipo: TipoSeccion, nombre: &str, canales: &[&str]) -> Seccion {
    Seccion {
        id: id.into(),
        tipo,
        nombre: nombre.into(),
        emoji: String::new(),
        canales: canales.iter().map(|c| c.to_string()).collect(),
    }
}

/// Un workspace en segundo plano con menciones pendientes, para ver el rail.
fn segundo() -> Espacio {
    let canales = ["general", "soporte", "random"]
        .iter()
        .enumerate()
        .map(|(i, n)| serde_json::from_value(json!({"id": format!("C{i}"), "name": n})).unwrap())
        .collect();
    Espacio {
        sesion: Some(Sesion {
            user_id: "U9".into(),
            team: "Luna".into(),
        }),
        equipo: Some(Equipo {
            name: "Proyecto Luna".into(),
            domain: "proyecto-luna".into(),
            icon: Icono::default(),
        }),
        canales,
        contadores: HashMap::from([(
            "C1".to_string(),
            Contador {
                no_leidos: true,
                menciones: 3,
                ultimo: 0,
            },
        )]),
        ..Espacio::default()
    }
}
