use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use axum::http::HeaderName;
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use wreq::Client;
use wreq_util::Emulation;

// ═══════════════════════════════════════════════════════════════
//  Constants
// ═══════════════════════════════════════════════════════════════

const ONESHOT_FREE: &str = "https://oneshot-free.www.deepl.com/v1/translate";
const ONESHOT_PRO: &str = "https://oneshot-pro.www.deepl.com/v1/translate";
const MAX_TEXT_LENGTH: usize = 1500;

// ═══════════════════════════════════════════════════════════════
//  Language maps
// ═══════════════════════════════════════════════════════════════

fn target_lang_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    for (k, v) in [
        ("AR", "ar"), ("BG", "bg"), ("CS", "cs"), ("DA", "da"), ("DE", "de"),
        ("EL", "el"), ("EN-GB", "en-GB"), ("EN-US", "en-US"), ("ES", "es"),
        ("ES-419", "es-419"), ("ET", "et"), ("FI", "fi"), ("FR", "fr"),
        ("HE", "he"), ("HU", "hu"), ("ID", "id"), ("IT", "it"), ("JA", "ja"),
        ("KO", "ko"), ("LT", "lt"), ("LV", "lv"), ("NB", "nb"), ("NL", "nl"),
        ("PL", "pl"), ("PT-BR", "pt-BR"), ("PT-PT", "pt-PT"), ("RO", "ro"),
        ("RU", "ru"), ("SK", "sk"), ("SL", "sl"), ("SV", "sv"), ("TR", "tr"),
        ("UK", "uk"), ("VI", "vi"), ("ZH", "zh"), ("ZH-HANS", "zh-Hans"),
        ("ZH-HANT", "zh-Hant"),
    ] {
        m.insert(k, v);
    }
    m
}

fn resolve_target_lang(code: &str) -> Result<String, String> {
    let code = code.to_uppercase().replace('_', "-");
    let code = match code.as_str() {
        "EN" => "EN-US".to_string(),
        "PT" => "PT-BR".to_string(),
        "ZH" => "ZH-HANS".to_string(),
        c => c.to_string(),
    };
    let map = target_lang_map();
    map.get(code.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("unsupported target language: {}", code))
}

fn resolve_source_lang(code: &str) -> Result<Option<String>, String> {
    let c = code.to_uppercase().replace('_', "-");
    if c.is_empty() || c == "AUTO" {
        return Ok(None);
    }
    let mut map = target_lang_map();
    map.insert("EN", "en");
    map.insert("PT", "pt");
    map.get(c.as_str())
        .map(|v| Some(v.to_string()))
        .ok_or_else(|| format!("unsupported source language: {}", c))
}

// ═══════════════════════════════════════════════════════════════
//  Instance ID
// ═══════════════════════════════════════════════════════════════

fn new_instance_id() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15],
    )
}

static INSTANCE_ID: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(new_instance_id);

// ═══════════════════════════════════════════════════════════════
//  Request / Response types
// ═══════════════════════════════════════════════════════════════

#[derive(Serialize)]
struct AppInformation {
    os: String,
    os_version: String,
    app_version: String,
    app_build: String,
    instance_id: String,
}

#[derive(Serialize)]
struct OneshotRequest {
    text: Vec<String>,
    target_lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_lang: Option<String>,
    usage_type: String,
    app_information: AppInformation,
}

#[derive(Deserialize)]
struct TranslationItem {
    text: String,
    #[serde(default)]
    detected_source_language: String,
}

#[derive(Deserialize)]
struct OneshotResponse {
    translations: Vec<TranslationItem>,
}

// ── API request/response types ──

#[derive(Deserialize)]
struct TranslateRequest {
    text: String,
    source_lang: Option<String>,
    target_lang: String,
    #[allow(dead_code)]
    quality: Option<String>,
}

#[derive(Serialize)]
struct TranslateResponse {
    code: u16,
    data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_lang: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
}

// ═══════════════════════════════════════════════════════════════
//  Client
// ═══════════════════════════════════════════════════════════════

struct DeepLClient {
    client: Client,
    dl_session: Option<String>,
}

impl DeepLClient {
    async fn new(proxy: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let builder = Client::builder()
            .emulation(Emulation::Chrome130)
            .cookie_store(true)
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(20));

        let builder = if let Some(ref p) = proxy {
            if !p.is_empty() {
                builder.proxy(wreq::Proxy::https(p)?)
            } else {
                builder
            }
        } else {
            builder
        };

        let client = builder.build()?;
        let deepl = DeepLClient {
            client,
            dl_session: None,
        };

        deepl.warmup_cookies().await?;
        Ok(deepl)
    }

    async fn warmup_cookies(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .get("https://www.deepl.com/translator")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .send()
            .await?;
        Ok(())
    }

    async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
        if text.trim().is_empty() {
            return Err("text cannot be empty".into());
        }
        if text.chars().count() > MAX_TEXT_LENGTH {
            return Err(format!("text exceeds {} characters", MAX_TEXT_LENGTH).into());
        }

        let target = resolve_target_lang(target_lang)
            .map_err(|e| format!("target_lang: {}", e))?;
        let source = resolve_source_lang(source_lang)
            .map_err(|e| format!("source_lang: {}", e))?;

        let endpoint = if self.dl_session.is_some() {
            ONESHOT_PRO
        } else {
            ONESHOT_FREE
        };

        let auth_value = match &self.dl_session {
            Some(s) => format!("Bearer {}", s),
            None => "None".to_string(),
        };

        let body = OneshotRequest {
            text: vec![text.to_string()],
            target_lang: target,
            source_lang: source,
            usage_type: "Translate".to_string(),
            app_information: AppInformation {
                os: "brex_macOS".to_string(),
                os_version: "brex_chrome_120.0.0.0".to_string(),
                app_version: "1.86.0".to_string(),
                app_build: "chrome_web_store".to_string(),
                instance_id: INSTANCE_ID.clone(),
            },
        };

        let resp = self
            .client
            .post(endpoint)
            .header("Authorization", &auth_value)
            .header("Origin", "chrome-extension://cofdbpoegempjloogbagkncekinflcnj")
            .header("Accept", "*/*")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Sec-Fetch-Site", "cross-site")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Dest", "empty")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let body_bytes = resp.bytes().await?;

        if status == 429 {
            return Err("429: too many requests".into());
        }
        if !status.is_success() {
            let body_str = String::from_utf8_lossy(&body_bytes);
            return Err(format!("HTTP {}: {}", status, body_str).into());
        }

        let result: OneshotResponse = serde_json::from_slice(&body_bytes)
            .map_err(|e| format!("json parse: {}", e))?;

        if result.translations.is_empty() {
            return Err("no translations in response".into());
        }

        let translated = result.translations[0].text.clone();
        let detected = result.translations[0].detected_source_language.clone();
        let detected = if detected.is_empty() { None } else { Some(detected) };

        Ok((translated, detected))
    }
}

// ═══════════════════════════════════════════════════════════════
//  Axum handler
// ═══════════════════════════════════════════════════════════════

#[derive(Clone)]
struct AppState {
    client: Arc<DeepLClient>,
}

async fn handle_translate(
    State(state): State<AppState>,
    Json(req): Json<TranslateRequest>,
) -> Result<Json<TranslateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let source = req.source_lang.as_deref().unwrap_or("auto");

    match state
        .client
        .translate(&req.text, source, &req.target_lang)
        .await
    {
        Ok((data, detected)) => Ok(Json(TranslateResponse {
            code: 200,
            data,
            source_lang: detected,
        })),
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("429") {
                StatusCode::TOO_MANY_REQUESTS
            } else if msg.contains("exceeds") {
                StatusCode::PAYLOAD_TOO_LARGE
            } else if msg.contains("unsupported") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::BAD_GATEWAY
            };
            Err((
                status,
                Json(ErrorResponse {
                    code: status.as_u16(),
                    message: msg,
                }),
            ))
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  Main
// ═══════════════════════════════════════════════════════════════

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proxy = std::env::var("PROXY_LIST").ok();

    println!("[*] Initializing DeepL client...");
    let client = DeepLClient::new(proxy).await?;
    println!("[*] Client ready (cookies warmed)");

    let state = AppState {
        client: Arc::new(client),
    };

    let referrer = SetResponseHeaderLayer::appending(
        HeaderName::from_static("referrer-policy"),
        axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/translate", post(handle_translate))
        .layer(referrer)
        .layer(cors)
        .with_state(state);

    let addr = "127.0.0.1:9000";

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
