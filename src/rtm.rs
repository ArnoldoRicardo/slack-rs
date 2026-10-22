//! Eventos en tiempo real por el websocket de RTM, con reconexión automática.

use std::time::Duration;

use futures::channel::mpsc;
use futures::{SinkExt, Stream, StreamExt};
use iced::Subscription;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message as Ws;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

use crate::slack::{Cliente, Credenciales, Mensaje};

#[derive(Debug, Clone)]
pub enum Evento {
    Conectado,
    Desconectado(String),
    Nuevo {
        canal: String,
        mensaje: Mensaje,
    },
    Editado {
        canal: String,
        mensaje: Mensaje,
    },
    Borrado {
        canal: String,
        ts: String,
    },
    Reaccion {
        canal: String,
        ts: String,
        nombre: String,
        usuario: String,
        agregada: bool,
    },
    /// Por dónde mandarle cosas al websocket abierto; llega al conectar.
    Canal(Enviador),
    Presencia {
        usuarios: Vec<String>,
        activo: bool,
    },
    /// Se leyó en otro dispositivo (o aquí mismo): cuánto queda sin leer.
    Leido {
        canal: String,
        sin_leer: u32,
        menciones: u32,
    },
    /// Alguien está escribiendo; Slack lo repite cada pocos segundos mientras siga.
    Escribiendo {
        canal: String,
        usuario: String,
    },
}

/// Lo que la app manda por el websocket.
#[derive(Debug, Clone, PartialEq)]
pub enum Saliente {
    /// Pedir la presencia de estas personas (las de las DMs).
    Presencia(Vec<String>),
    /// Estás escribiendo en este canal.
    Escribiendo(String),
}

impl Saliente {
    fn json(&self, id: u64) -> Value {
        match self {
            Saliente::Presencia(ids) => json!({ "type": "presence_sub", "ids": ids }),
            Saliente::Escribiendo(canal) => json!({ "id": id, "type": "typing", "channel": canal }),
        }
    }
}

/// Extremo para mandar `Saliente`s a la conexión del workspace.
#[derive(Debug, Clone)]
pub struct Enviador(pub mpsc::Sender<Saliente>);

/// Espera antes del reintento `n` (desde 0): 5 s, 10 s, 20 s… hasta un minuto.
fn espera(intento: u32) -> Duration {
    Duration::from_secs((5u64 << intento.min(4)).min(60))
}

/// Sin pong en más de dos pings y medio, la conexión está muerta aunque no se cerró.
fn sin_respuesta(desde_el_ultimo: Duration) -> bool {
    desde_el_ultimo > Duration::from_secs(75)
}

pub fn suscripcion(cred: Credenciales) -> Subscription<Evento> {
    Subscription::run_with(cred, escuchar)
}

fn escuchar(cred: &Credenciales) -> impl Stream<Item = Evento> + use<> {
    let cliente = Cliente::new(cred.clone());
    iced::stream::channel(100, async move |mut salida: mpsc::Sender<Evento>| {
        let mut intento = 0;
        loop {
            let motivo = match sesion(&cliente, &mut salida, &mut intento).await {
                Ok(()) => "el servidor cerró la conexión".to_string(),
                Err(e) => e,
            };
            let _ = salida.send(Evento::Desconectado(motivo)).await;
            tokio::time::sleep(espera(intento)).await;
            intento += 1;
        }
    })
}

/// Una conexión, hasta que se cae. `intento` vuelve a 0 cuando el servidor saluda.
async fn sesion(
    cliente: &Cliente,
    salida: &mut mpsc::Sender<Evento>,
    intento: &mut u32,
) -> Result<(), String> {
    let url = cliente.url_tiempo_real().await?;
    let mut peticion = url.into_client_request().map_err(|e| e.to_string())?;
    let cookie = format!("d={}", cliente.credenciales().cookie);
    peticion.headers_mut().insert(
        "Cookie",
        HeaderValue::from_str(&cookie).map_err(|e| e.to_string())?,
    );
    let (ws, _) = tokio_tungstenite::connect_async(peticion)
        .await
        .map_err(|e| format!("websocket: {e}"))?;
    let (mut escritura, mut lectura) = ws.split();
    let (enviador, mut pendientes) = mpsc::channel::<Saliente>(32);
    if salida
        .send(Evento::Canal(Enviador(enviador)))
        .await
        .is_err()
    {
        return Ok(());
    }

    let mut latido = tokio::time::interval(Duration::from_secs(30));
    let mut id = 0u64;
    // Cualquier cosa que llegue (pong incluido) cuenta como señal de vida.
    let mut ultimo = tokio::time::Instant::now();
    loop {
        tokio::select! {
            _ = latido.tick() => {
                if sin_respuesta(ultimo.elapsed()) {
                    return Err("el servidor dejó de responder".into());
                }
                id += 1;
                let ping = json!({ "id": id, "type": "ping" }).to_string();
                escritura.send(Ws::text(ping)).await.map_err(|e| e.to_string())?;
            }
            Some(saliente) = pendientes.next() => {
                id += 1;
                let texto = saliente.json(id).to_string();
                escritura.send(Ws::text(texto)).await.map_err(|e| e.to_string())?;
            }
            entrante = lectura.next() => {
                let Some(entrante) = entrante else { return Ok(()) };
                ultimo = tokio::time::Instant::now();
                match entrante.map_err(|e| e.to_string())? {
                    Ws::Text(texto) => {
                        let Ok(valor) = serde_json::from_str::<Value>(&texto) else { continue };
                        let evento = interpretar(&valor);
                        if matches!(evento, Some(Evento::Conectado)) {
                            *intento = 0;
                        }
                        if let Some(evento) = evento
                            && salida.send(evento).await.is_err() {
                                return Ok(());
                            }
                    }
                    Ws::Close(_) => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

fn interpretar(v: &Value) -> Option<Evento> {
    match v["type"].as_str()? {
        "hello" => Some(Evento::Conectado),
        "message" => {
            let canal = v["channel"].as_str()?.to_string();
            match v["subtype"].as_str() {
                Some("message_changed") => {
                    let mensaje = serde_json::from_value(v["message"].clone()).ok()?;
                    Some(Evento::Editado { canal, mensaje })
                }
                Some("message_deleted") => {
                    let ts = v["deleted_ts"].as_str()?.to_string();
                    Some(Evento::Borrado { canal, ts })
                }
                _ => {
                    let mensaje = serde_json::from_value(v.clone()).ok()?;
                    Some(Evento::Nuevo { canal, mensaje })
                }
            }
        }
        "presence_change" => {
            let usuarios = match v["users"].as_array() {
                Some(lista) => lista
                    .iter()
                    .filter_map(|u| u.as_str().map(str::to_string))
                    .collect(),
                None => vec![v["user"].as_str()?.to_string()],
            };
            Some(Evento::Presencia {
                usuarios,
                activo: v["presence"].as_str()? == "active",
            })
        }
        "channel_marked" | "im_marked" | "group_marked" | "mpim_marked" => Some(Evento::Leido {
            canal: v["channel"].as_str()?.to_string(),
            sin_leer: v["unread_count_display"].as_u64().unwrap_or(0) as u32,
            menciones: v["mention_count_display"].as_u64().unwrap_or(0) as u32,
        }),
        "user_typing" => Some(Evento::Escribiendo {
            canal: v["channel"].as_str()?.to_string(),
            usuario: v["user"].as_str()?.to_string(),
        }),
        tipo @ ("reaction_added" | "reaction_removed") => Some(Evento::Reaccion {
            canal: v["item"]["channel"].as_str()?.to_string(),
            ts: v["item"]["ts"].as_str()?.to_string(),
            nombre: v["reaction"].as_str()?.to_string(),
            usuario: v["user"].as_str()?.to_string(),
            agregada: tipo == "reaction_added",
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evento(v: Value) -> Option<Evento> {
        interpretar(&v)
    }

    #[test]
    fn hola_conecta() {
        assert!(matches!(
            evento(json!({ "type": "hello" })),
            Some(Evento::Conectado)
        ));
    }

    #[test]
    fn mensaje_nuevo_editado_y_borrado() {
        let nuevo =
            evento(json!({ "type": "message", "channel": "C1", "ts": "1.0", "text": "hola" }));
        assert!(
            matches!(nuevo, Some(Evento::Nuevo { canal, mensaje }) if canal == "C1" && mensaje.text == "hola")
        );

        let editado = evento(json!({
            "type": "message", "subtype": "message_changed", "channel": "C1",
            "message": { "ts": "1.0", "text": "adiós" }
        }));
        assert!(
            matches!(editado, Some(Evento::Editado { mensaje, .. }) if mensaje.text == "adiós")
        );

        let borrado = evento(json!({
            "type": "message", "subtype": "message_deleted", "channel": "C1", "deleted_ts": "1.0"
        }));
        assert!(
            matches!(borrado, Some(Evento::Borrado { canal, ts }) if canal == "C1" && ts == "1.0")
        );
    }

    #[test]
    fn reacciones() {
        let v = |tipo| {
            json!({
                "type": tipo, "user": "U1", "reaction": "eyes",
                "item": { "type": "message", "channel": "C1", "ts": "1.0" }
            })
        };
        let puesta = evento(v("reaction_added"));
        assert!(
            matches!(puesta, Some(Evento::Reaccion { agregada: true, ref nombre, ref usuario, .. }) if nombre == "eyes" && usuario == "U1")
        );
        assert!(matches!(
            evento(v("reaction_removed")),
            Some(Evento::Reaccion {
                agregada: false,
                ..
            })
        ));
    }

    #[test]
    fn alguien_escribe() {
        let e = evento(json!({ "type": "user_typing", "channel": "C1", "user": "U2" }));
        assert!(matches!(
            e,
            Some(Evento::Escribiendo { canal, usuario }) if canal == "C1" && usuario == "U2"
        ));
    }

    #[test]
    fn presencia_de_uno_o_de_varios() {
        let uno = evento(json!({ "type": "presence_change", "user": "U1", "presence": "active" }));
        assert!(matches!(
            uno,
            Some(Evento::Presencia { usuarios, activo: true }) if usuarios == ["U1"]
        ));
        let varios =
            evento(json!({ "type": "presence_change", "users": ["U1", "U2"], "presence": "away" }));
        assert!(matches!(
            varios,
            Some(Evento::Presencia { usuarios, activo: false }) if usuarios == ["U1", "U2"]
        ));
    }

    #[test]
    fn leido_en_otro_dispositivo() {
        for tipo in ["channel_marked", "im_marked", "group_marked", "mpim_marked"] {
            let e = evento(json!({ "type": tipo, "channel": "C1", "ts": "2.0",
                                   "unread_count_display": 0, "mention_count_display": 0 }));
            assert!(
                matches!(
                    e,
                    Some(Evento::Leido { canal, sin_leer: 0, menciones: 0 }) if canal == "C1"
                ),
                "{tipo}"
            );
        }
    }

    #[test]
    fn lo_que_se_manda_por_el_websocket() {
        let p = Saliente::Presencia(vec!["U1".into(), "U2".into()]).json(7);
        assert_eq!(p, json!({ "type": "presence_sub", "ids": ["U1", "U2"] }));
        let t = Saliente::Escribiendo("C1".into()).json(7);
        assert_eq!(t, json!({ "id": 7, "type": "typing", "channel": "C1" }));
    }

    #[test]
    fn reintentos_cada_vez_mas_espaciados() {
        let s = |n| espera(n).as_secs();
        assert_eq!(
            [s(0), s(1), s(2), s(3), s(4), s(10)],
            [5, 10, 20, 40, 60, 60]
        );
    }

    #[test]
    fn sin_pong_se_da_por_caida() {
        assert!(!sin_respuesta(Duration::from_secs(45)));
        assert!(sin_respuesta(Duration::from_secs(76)));
    }

    #[test]
    fn descarta_lo_demas() {
        assert!(evento(json!({ "type": "pong", "reply_to": 3 })).is_none());
        assert!(evento(json!({ "type": "user_typing", "channel": "C1" })).is_none());
        assert!(evento(json!({ "type": "message" })).is_none());
        assert!(evento(json!({ "reply_to": 1 })).is_none());
    }
}
