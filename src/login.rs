//! Inicio de sesión con el navegador: abre una ventana de Chrome con un perfil
//! temporal, el usuario entra a Slack como siempre y, cuando aparece la sesión, se
//! leen el token (`localStorage.localConfig_v2`) y la cookie `d` por el protocolo de
//! depuración (CDP). Las credenciales se guardan en `~/.config/slack-rs/`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message as Ws;

use crate::slack::{Cliente, Credenciales, Resultado};

const URL_SLACK: &str = "https://app.slack.com/client";
const ORIGEN_SLACK: &str = "https://app.slack.com/";
const LIMITE: Duration = Duration::from_secs(15 * 60);
const NAVEGADORES: [&str; 4] = [
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
];

pub fn archivo() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("slack-rs").join("credenciales.json")
}

/// Sesiones guardadas por inicios de sesión anteriores; ninguna si no hay archivo.
pub fn cargar_de(ruta: &Path) -> Vec<Credenciales> {
    std::fs::read_to_string(ruta)
        .map(|texto| leer_guardadas(&texto))
        .unwrap_or_default()
}

/// Lista de sesiones, o una sola (el formato anterior del archivo).
fn leer_guardadas(texto: &str) -> Vec<Credenciales> {
    serde_json::from_str(texto)
        .or_else(|_| serde_json::from_str(texto).map(|una| vec![una]))
        .unwrap_or_default()
}

/// Suma una sesión a la lista: reemplaza la del mismo workspace o el mismo token.
pub fn agregar(lista: &mut Vec<Credenciales>, cred: Credenciales) {
    let misma = lista
        .iter()
        .position(|c| c.token == cred.token || c.equipo.is_some() && c.equipo == cred.equipo);
    match misma {
        Some(i) => lista[i] = cred,
        None => lista.push(cred),
    }
}

/// Agrega la sesión al archivo de `ruta`, conservando las demás.
pub fn recordar_en(ruta: &Path, cred: Credenciales) -> Resultado<()> {
    let mut lista = cargar_de(ruta);
    agregar(&mut lista, cred);
    guardar_en(ruta, &lista)
}

/// Quita del archivo la sesión con ese token (cerrar sesión en un workspace).
pub fn olvidar_en(ruta: &Path, token: &str) -> Resultado<()> {
    let mut lista = cargar_de(ruta);
    let antes = lista.len();
    lista.retain(|c| c.token != token);
    if lista.len() == antes {
        return Ok(());
    }
    guardar_en(ruta, &lista)
}

/// Escribe las credenciales legibles solo por el usuario (0600, carpeta 0700).
fn guardar_en(ruta: &Path, lista: &[Credenciales]) -> Resultado<()> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

    if let Some(dir) = ruta.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(ruta)
        .map_err(|e| format!("{}: {e}", ruta.display()))?;
    let json = serde_json::to_string_pretty(lista).expect("credenciales serializables");
    f.write_all(json.as_bytes())
        .map_err(|e| format!("{}: {e}", ruta.display()))
}

/// Las sesiones con que arranca la app: las de `SLACK_TOKEN`/`SLACK_COOKIE` si están
/// definidas y luego las guardadas que no repitan token.
pub fn iniciales(
    leer: impl Fn(&str) -> Option<String>,
    guardadas: Vec<Credenciales>,
) -> Resultado<Vec<Credenciales>> {
    let definida = |var: &str| leer(var).is_some_and(|v| !v.trim().is_empty());
    if !definida("SLACK_TOKEN") && !definida("SLACK_COOKIE") {
        return Ok(guardadas);
    }
    let mut todas = Credenciales::desde(leer)?;
    for cred in guardadas {
        if !todas.iter().any(|c| c.token == cred.token) {
            todas.push(cred);
        }
    }
    Ok(todas)
}

/// Borra los perfiles temporales de inicios de sesión de procesos que ya no existen
/// (la app se cerró o se reinició con la ventana abierta). Guardan cookies de Slack.
fn limpiar_perfiles(dir: &Path, propio: u32, vivo: impl Fn(u32) -> bool) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entradas.flatten() {
        let nombre = e.file_name().to_string_lossy().into_owned();
        let pid = nombre
            .strip_prefix("slack-rs-login-")
            .and_then(|resto| resto.split('-').next())
            .and_then(|pid| pid.parse::<u32>().ok());
        if pid.is_some_and(|p| p != propio && !vivo(p)) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

pub async fn con_navegador() -> Resultado<Credenciales> {
    let vivo = |pid: u32| Path::new(&format!("/proc/{pid}")).exists();
    limpiar_perfiles(&std::env::temp_dir(), std::process::id(), vivo);
    esperar_sesion(URL_SLACK, ORIGEN_SLACK, false, |cred| async move {
        Cliente::new(cred).sesion().await.map(|_| ())
    })
    .await
}

/// Chrome lanzado por nosotros. Al soltarlo (también si se cancela el inicio de
/// sesión) se cierra y se borra su perfil temporal.
struct Navegador {
    hijo: tokio::process::Child,
    perfil: PathBuf,
}

impl Drop for Navegador {
    fn drop(&mut self) {
        let _ = self.hijo.start_kill();
        let perfil = self.perfil.clone();
        // Chrome tarda un momento en soltar sus archivos.
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            let _ = std::fs::remove_dir_all(perfil);
        });
    }
}

impl Navegador {
    fn abrir(url: &str, oculto: bool) -> Resultado<Self> {
        let binario = NAVEGADORES
            .iter()
            .find(|b| buscar_en_path(b))
            .ok_or("no encontré Chrome ni Chromium instalado")?;
        // El perfil guarda la sesión de Slack mientras dura el login: solo para el usuario.
        let perfil =
            std::env::temp_dir().join(format!("slack-rs-login-{}-{}", std::process::id(), nanos()));
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&perfil)
                .map_err(|e| format!("{}: {e}", perfil.display()))?;
        }

        let mut cmd = tokio::process::Command::new(binario);
        cmd.arg(format!("--user-data-dir={}", perfil.display()))
            // Puerto 0: Chrome elige uno libre y lo escribe en DevToolsActivePort.
            .arg("--remote-debugging-port=0")
            .args([
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-sync",
            ])
            .kill_on_drop(true)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if oculto {
            cmd.args(["--headless=new", url]);
        } else {
            cmd.arg(format!("--app={url}")).arg("--window-size=520,760");
        }
        let hijo = cmd
            .spawn()
            .map_err(|e| format!("no pude abrir {binario}: {e}"))?;
        Ok(Self { hijo, perfil })
    }

    fn puerto(&self) -> Option<u16> {
        let texto = std::fs::read_to_string(self.perfil.join("DevToolsActivePort")).ok()?;
        texto.lines().next()?.trim().parse().ok()
    }

    fn sigue_abierto(&mut self) -> bool {
        matches!(self.hijo.try_wait(), Ok(None))
    }

    /// Cierre ordenado: espera a que Chrome termine y borra el perfil antes de
    /// seguir, para que la sesión copiada no quede en disco si la app sale enseguida.
    async fn cerrar(mut self) {
        let _ = self.hijo.kill().await;
        let _ = std::fs::remove_dir_all(&self.perfil);
    }
}

/// Abre `url` y revisa cada segundo las pestañas cuyo URL empieza con `origen` hasta
/// encontrar token y cookie.
async fn esperar_sesion<F, R>(
    url: &str,
    origen: &str,
    oculto: bool,
    valida: F,
) -> Resultado<Credenciales>
where
    F: Fn(Credenciales) -> R,
    R: std::future::Future<Output = Resultado<()>>,
{
    let mut navegador = Navegador::abrir(url, oculto)?;
    let http = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let inicio = Instant::now();
    // Por qué Slack rechazó la última sesión leída, para decirlo si se cierra la ventana.
    let mut rechazo: Option<String> = None;

    while inicio.elapsed() < LIMITE {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if !navegador.sigue_abierto() {
            return Err(match rechazo {
                Some(r) => format!("se cerró la ventana y Slack no aceptó la sesión ({r})"),
                None => "se cerró la ventana del navegador antes de iniciar sesión".into(),
            });
        }
        let Some(puerto) = navegador.puerto() else {
            continue;
        };
        let Ok(pestanas) = pestanas(&http, puerto).await else {
            continue;
        };
        for p in pestanas
            .iter()
            .filter(|p| p.kind == "page" && p.url.starts_with(origen))
        {
            // Slack a veces escribe el token antes de terminar el inicio de sesión
            // (verificación en dos pasos, cookie que cambia): solo se acepta lo que
            // Slack ya reconoce; si no, se sigue esperando con la ventana abierta.
            let Ok(Some(cred)) = leer_sesion(&p.web_socket_debugger_url, origen).await else {
                continue;
            };
            match valida(cred.clone()).await {
                Ok(()) => {
                    navegador.cerrar().await;
                    return Ok(cred);
                }
                Err(e) => {
                    if rechazo.as_ref() != Some(&e) {
                        // Sin tokens: solo el motivo y en qué página va el inicio de sesión.
                        let pagina = p.url.split('?').next().unwrap_or_default();
                        eprintln!(
                            "inicio de sesión: Slack todavía no acepta la sesión ({e}) en {pagina}"
                        );
                    }
                    rechazo = Some(e);
                }
            }
        }
    }
    Err("se acabó el tiempo para iniciar sesión".into())
}

#[derive(Deserialize)]
struct Pestana {
    #[serde(rename = "type")]
    kind: String,
    url: String,
    #[serde(rename = "webSocketDebuggerUrl", default)]
    web_socket_debugger_url: String,
}

async fn pestanas(http: &reqwest::Client, puerto: u16) -> Resultado<Vec<Pestana>> {
    http.get(format!("http://127.0.0.1:{puerto}/json/list"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn leer_sesion(ws_url: &str, origen: &str) -> Resultado<Option<Credenciales>> {
    let respuestas = cdp(
        ws_url,
        &[
            (
                "Runtime.evaluate",
                json!({"expression": "localStorage.getItem('localConfig_v2')", "returnByValue": true}),
            ),
            ("Network.getCookies", json!({"urls": [origen]})),
        ],
    )
    .await?;
    let token = respuestas[0]["result"]["value"].as_str().and_then(token_de);
    let cookie = respuestas[1]["cookies"]
        .as_array()
        .and_then(|c| cookie_d(c));
    Ok(token
        .zip(cookie)
        .map(|((equipo, token), cookie)| Credenciales {
            token,
            cookie,
            equipo: Some(equipo),
        }))
}

/// Manda los comandos por el websocket de la pestaña y devuelve sus `result` en orden.
async fn cdp(ws_url: &str, comandos: &[(&str, Value)]) -> Resultado<Vec<Value>> {
    let conectar = tokio_tungstenite::connect_async(ws_url);
    let (mut ws, _) = tokio::time::timeout(Duration::from_secs(5), conectar)
        .await
        .map_err(|_| "CDP: tiempo de conexión agotado")?
        .map_err(|e| format!("CDP: {e}"))?;
    for (i, (metodo, params)) in comandos.iter().enumerate() {
        let msg = json!({"id": i + 1, "method": metodo, "params": params});
        ws.send(Ws::text(msg.to_string()))
            .await
            .map_err(|e| e.to_string())?;
    }
    let mut resultados = vec![Value::Null; comandos.len()];
    let mut faltan = comandos.len();
    let leer = async {
        while faltan > 0 {
            let Some(msg) = ws.next().await else { break };
            let Ws::Text(texto) = msg.map_err(|e| e.to_string())? else {
                continue;
            };
            let v: Value = serde_json::from_str(&texto).map_err(|e| e.to_string())?;
            if let Some(id) = v["id"]
                .as_u64()
                .map(|i| i as usize)
                .filter(|i| (1..=comandos.len()).contains(i))
            {
                resultados[id - 1] = v["result"].clone();
                faltan -= 1;
            }
        }
        Ok::<_, String>(())
    };
    tokio::time::timeout(Duration::from_secs(5), leer)
        .await
        .map_err(|_| "CDP: sin respuesta")??;
    let _ = ws.close(None).await;
    Ok(resultados)
}

/// Workspace activo dentro de `localConfig_v2` y su token `xoxc-`; si el activo no
/// tiene uno, el primero que lo tenga.
fn token_de(config: &str) -> Option<(String, String)> {
    let v: Value = serde_json::from_str(config).ok()?;
    let equipos = v["teams"].as_object()?;
    let activo = v["lastActiveTeamId"]
        .as_str()
        .and_then(|id| equipos.get_key_value(id));
    activo
        .into_iter()
        .chain(equipos.iter())
        .find_map(|(id, e)| {
            let token = e["token"].as_str().filter(|t| t.starts_with("xoxc-"))?;
            Some((id.clone(), token.to_string()))
        })
}

fn cookie_d(cookies: &[Value]) -> Option<String> {
    cookies
        .iter()
        .find(|c| c["name"] == "d")
        .and_then(|c| c["value"].as_str())
        .filter(|v| v.starts_with("xoxd-"))
        .map(str::to_string)
}

fn buscar_en_path(binario: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(binario).is_file()))
}

fn nanos() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cred(equipo: Option<&str>, token: &str) -> Credenciales {
        Credenciales {
            token: token.into(),
            cookie: format!("xoxd-{token}"),
            equipo: equipo.map(String::from),
        }
    }

    fn tokens(lista: &[Credenciales]) -> Vec<&str> {
        lista.iter().map(|c| c.token.as_str()).collect()
    }

    fn entorno(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |var| vars.iter().find(|(k, _)| k == var).map(|(_, v)| v.clone())
    }

    #[test]
    fn limpia_los_perfiles_de_procesos_muertos() {
        let dir = std::env::temp_dir().join(format!(
            "slack-rs-limpiar-{}-{}",
            std::process::id(),
            nanos()
        ));
        for nombre in [
            "slack-rs-login-100-1",
            "slack-rs-login-200-2",
            "slack-rs-login-300-3",
            "otra-cosa-100-1",
        ] {
            std::fs::create_dir_all(dir.join(nombre)).unwrap();
        }
        // 100 es este proceso, 200 sigue vivo, 300 ya no existe.
        limpiar_perfiles(&dir, 100, |pid| pid == 200);
        let mut quedan: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        quedan.sort();
        assert_eq!(
            quedan,
            [
                "otra-cosa-100-1",
                "slack-rs-login-100-1",
                "slack-rs-login-200-2"
            ]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn elige_token_del_workspace_activo() {
        let config = r#"{"lastActiveTeamId":"T2","teams":{"T1":{"token":"xoxc-uno"},"T2":{"token":"xoxc-dos"}}}"#;
        let dos = (String::from("T2"), String::from("xoxc-dos"));
        assert_eq!(token_de(config), Some(dos));
        let sin_activo = r#"{"teams":{"T1":{"token":"xoxc-uno"}}}"#;
        let uno = (String::from("T1"), String::from("xoxc-uno"));
        assert_eq!(token_de(sin_activo), Some(uno));
        assert_eq!(token_de(r#"{"teams":{}}"#), None);
        assert_eq!(token_de("null"), None);
    }

    #[test]
    fn lee_la_lista_y_el_formato_de_una_sola_sesion() {
        let lista = r#"[{"token":"xoxc-a","cookie":"xoxd-a","equipo":"T1"},{"token":"xoxc-b","cookie":"xoxd-b"}]"#;
        let leidas = leer_guardadas(lista);
        assert_eq!(tokens(&leidas), ["xoxc-a", "xoxc-b"]);
        assert_eq!(leidas[0].equipo.as_deref(), Some("T1"));
        let una = r#"{"token":"xoxc-a","cookie":"xoxd-a"}"#;
        assert_eq!(tokens(&leer_guardadas(una)), ["xoxc-a"]);
        assert!(leer_guardadas("basura").is_empty());
    }

    #[test]
    fn agregar_suma_otro_workspace_y_reemplaza_el_mismo() {
        let mut lista = vec![cred(Some("T1"), "xoxc-a")];
        agregar(&mut lista, cred(Some("T2"), "xoxc-b"));
        assert_eq!(tokens(&lista), ["xoxc-a", "xoxc-b"]);
        // Volver a entrar al mismo workspace cambia su sesión en su lugar.
        agregar(&mut lista, cred(Some("T1"), "xoxc-a2"));
        assert_eq!(tokens(&lista), ["xoxc-a2", "xoxc-b"]);
        // El mismo token no se repite aunque no se sepa el workspace.
        agregar(&mut lista, cred(None, "xoxc-b"));
        assert_eq!(tokens(&lista), ["xoxc-a2", "xoxc-b"]);
    }

    #[test]
    fn olvidar_quita_solo_esa_sesion() {
        let dir = std::env::temp_dir().join(format!(
            "slack-rs-olvidar-{}-{}",
            std::process::id(),
            nanos()
        ));
        let ruta = dir.join("credenciales.json");
        recordar_en(&ruta, cred(Some("T1"), "xoxc-a")).unwrap();
        recordar_en(&ruta, cred(Some("T2"), "xoxc-b")).unwrap();
        olvidar_en(&ruta, "xoxc-a").unwrap();
        assert_eq!(tokens(&cargar_de(&ruta)), ["xoxc-b"]);
        olvidar_en(&ruta, "xoxc-z").unwrap();
        assert_eq!(tokens(&cargar_de(&ruta)), ["xoxc-b"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn sin_entorno_quedan_las_guardadas() {
        let guardadas = vec![cred(Some("T1"), "xoxc-a")];
        let todas = iniciales(entorno(&[]), guardadas).unwrap();
        assert_eq!(tokens(&todas), ["xoxc-a"]);
        assert!(iniciales(entorno(&[]), vec![]).unwrap().is_empty());
    }

    #[test]
    fn el_entorno_va_primero_sin_repetir() {
        let guardadas = vec![cred(Some("T1"), "xoxc-a"), cred(Some("T2"), "xoxc-b")];
        let leer = entorno(&[("SLACK_TOKEN", "xoxc-b,xoxc-c"), ("SLACK_COOKIE", "xoxd-z")]);
        let todas = iniciales(leer, guardadas).unwrap();
        assert_eq!(tokens(&todas), ["xoxc-b", "xoxc-c", "xoxc-a"]);
    }

    #[test]
    fn un_entorno_a_medias_es_error() {
        let leer = entorno(&[("SLACK_TOKEN", "xoxc-b")]);
        assert!(
            iniciales(leer, vec![])
                .unwrap_err()
                .contains("SLACK_COOKIE")
        );
    }

    #[test]
    fn guarda_solo_para_el_usuario_y_lo_vuelve_a_leer() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!(
            "slack-rs-prueba-{}-{}",
            std::process::id(),
            nanos()
        ));
        let ruta = dir.join("slack-rs").join("credenciales.json");
        assert!(cargar_de(&ruta).is_empty());
        recordar_en(&ruta, cred(Some("T1"), "xoxc-a")).unwrap();
        recordar_en(&ruta, cred(Some("T2"), "xoxc-b")).unwrap();
        recordar_en(&ruta, cred(Some("T1"), "xoxc-a2")).unwrap();
        assert_eq!(tokens(&cargar_de(&ruta)), ["xoxc-a2", "xoxc-b"]);
        let modo = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(modo(&ruta), 0o600);
        assert_eq!(modo(ruta.parent().unwrap()), 0o700);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn encuentra_cookie_d() {
        let cookies = [
            json!({"name": "b", "value": "x"}),
            json!({"name": "d", "value": "xoxd-abc%2F"}),
        ];
        assert_eq!(cookie_d(&cookies).as_deref(), Some("xoxd-abc%2F"));
        assert_eq!(cookie_d(&[json!({"name": "d", "value": "otra"})]), None);
    }

    /// Flujo completo con un Chrome real sin ventana contra una página local que, como
    /// Slack al terminar el login, deja token y cookie un momento después de cargar.
    /// Requiere Chrome instalado: `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn lee_la_sesion_de_un_chrome_real() {
        let _turno = UN_CHROME.lock().await;
        let origen = pagina_que_inicia_sesion();
        let antes = perfiles_temporales();
        let cred = esperar_sesion(&origen, &origen, true, |_| async { Ok(()) })
            .await
            .unwrap();
        assert_eq!(cred.token, "xoxc-dos");
        assert_eq!(cred.equipo.as_deref(), Some("T2"));
        assert_eq!(cred.cookie, "xoxd-prueba%2Fabc");
        // El perfil con la sesión copiada ya no debe existir al devolver.
        assert_eq!(perfiles_temporales(), antes);
    }

    /// Si Slack todavía no acepta la sesión (falta un paso del inicio, la cookie
    /// cambia), la ventana sigue abierta y se vuelve a leer hasta que sirva.
    /// Requiere Chrome instalado: `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn espera_a_que_slack_acepte_la_sesion() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let _turno = UN_CHROME.lock().await;
        let origen = pagina_que_inicia_sesion();
        let intentos = Arc::new(AtomicUsize::new(0));
        let contador = intentos.clone();
        let cred = esperar_sesion(&origen, &origen, true, move |_| {
            let n = contador.fetch_add(1, Ordering::SeqCst);
            async move {
                if n >= 2 {
                    Ok(())
                } else {
                    Err("auth.test: invalid_auth".to_string())
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(cred.token, "xoxc-dos");
        assert_eq!(
            intentos.load(Ordering::SeqCst),
            3,
            "dos rechazos y la buena"
        );
    }

    /// Las pruebas con Chrome cuentan los perfiles temporales del proceso: de a una.
    static UN_CHROME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    /// Página local que, como Slack al terminar el login, deja token y cookie un
    /// momento después de cargar. Devuelve su origen.
    fn pagina_que_inicia_sesion() -> String {
        use std::io::{Read, Write};

        let servidor = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origen = format!(
            "http://127.0.0.1:{}/",
            servidor.local_addr().unwrap().port()
        );
        std::thread::spawn(move || {
            let pagina = r#"<html><body><script>
                setTimeout(() => {
                    localStorage.setItem('localConfig_v2', JSON.stringify(
                        {lastActiveTeamId: 'T2', teams: {T1: {token: 'xoxc-uno'}, T2: {token: 'xoxc-dos'}}}));
                    document.cookie = 'd=xoxd-prueba%2Fabc; path=/';
                }, 1500);
            </script></body></html>"#;
            for conexion in servidor.incoming() {
                let Ok(mut c) = conexion else { continue };
                let mut buf = [0u8; 4096];
                let _ = c.read(&mut buf);
                let _ = write!(
                    c,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{pagina}",
                    pagina.len()
                );
            }
        });
        origen
    }

    fn perfiles_temporales() -> usize {
        let propio = format!("slack-rs-login-{}-", std::process::id());
        std::fs::read_dir(std::env::temp_dir())
            .map(|d| {
                d.flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with(&propio))
                    .count()
            })
            .unwrap_or(0)
    }
}
