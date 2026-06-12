use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tauri::{AppHandle, Emitter, Manager};

const OLLAMA: &str = "http://localhost:11434";

// ---------- 시스템 정보 / 메모리 단계 ----------

#[derive(Serialize)]
struct SystemInfo {
    total_gb: f64,
    available_gb: f64,
    tier: u8,
    tier_label: String,
    tier_desc: String,
}

fn tier_for(total_gb: f64) -> (u8, &'static str, &'static str) {
    if total_gb < 12.0 {
        (1, "절약", "가벼운 소형 모델 1개를 순차 실행합니다.")
    } else if total_gb < 32.0 {
        (2, "균형", "중형 모델로 여러 에이전트를 함께 돌립니다.")
    } else {
        (3, "고성능", "대형 모델로 다중 에이전트를 병렬 실행합니다.")
    }
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    let mut sys = System::new();
    sys.refresh_memory();
    let total_gb = sys.total_memory() as f64 / 1_073_741_824.0;
    let available_gb = sys.available_memory() as f64 / 1_073_741_824.0;
    let (tier, label, desc) = tier_for(total_gb);
    SystemInfo {
        total_gb,
        available_gb,
        tier,
        tier_label: label.to_string(),
        tier_desc: desc.to_string(),
    }
}

// ---------- 단계별 추천 모델 ----------

#[derive(Serialize)]
struct ModelSpec {
    name: String,
    size: String,
    note: String,
}

fn spec(name: &str, size: &str, note: &str) -> ModelSpec {
    ModelSpec {
        name: name.to_string(),
        size: size.to_string(),
        note: note.to_string(),
    }
}

#[tauri::command]
fn get_tier_models(tier: u8) -> Vec<ModelSpec> {
    match tier {
        1 => vec![
            spec("llama3.2:3b", "~2.0GB", "가장 가벼운 범용 모델"),
            spec("qwen2.5:3b", "~1.9GB", "한국어·코드에 강함"),
            spec("gemma2:2b", "~1.6GB", "초경량, 빠른 응답"),
        ],
        2 => vec![
            spec("qwen2.5:7b", "~4.7GB", "균형형 주력 모델"),
            spec("llama3.1:8b", "~4.9GB", "범용 추론"),
            spec("qwen2.5:14b", "~9.0GB", "더 깊은 추론 (여유 있을 때)"),
        ],
        _ => vec![
            spec("qwen2.5:32b", "~20GB", "고품질 추론"),
            spec("llama3.3:70b", "~43GB", "최상위 품질 (32GB+ 권장)"),
            spec("mixtral:8x7b", "~26GB", "MoE, 빠르고 강력"),
        ],
    }
}

// ---------- 전체 모델 카탈로그 ----------

#[derive(Serialize)]
struct CatalogModel {
    name: String,
    size: String,
    note: String,
    category: String,
    tier: u8,
}

fn cat(name: &str, size: &str, note: &str, category: &str, tier: u8) -> CatalogModel {
    CatalogModel {
        name: name.to_string(),
        size: size.to_string(),
        note: note.to_string(),
        category: category.to_string(),
        tier,
    }
}

#[tauri::command]
fn get_catalog() -> Vec<CatalogModel> {
    vec![
        // 범용
        cat("llama3.2:1b", "~1.3GB", "초경량 범용", "범용", 1),
        cat("llama3.2:3b", "~2.0GB", "가벼운 범용", "범용", 1),
        cat("llama3.1:8b", "~4.9GB", "범용 추론", "범용", 2),
        cat("llama3.3:70b", "~43GB", "최상위 품질", "범용", 3),
        cat("qwen2.5:0.5b", "~0.4GB", "극소형", "범용", 1),
        cat("qwen2.5:3b", "~1.9GB", "한국어·코드 강함", "범용", 1),
        cat("qwen2.5:7b", "~4.7GB", "균형 주력", "범용", 2),
        cat("qwen2.5:14b", "~9.0GB", "깊은 추론", "범용", 2),
        cat("qwen2.5:32b", "~20GB", "고품질", "범용", 3),
        cat("qwen2.5:72b", "~47GB", "최상위", "범용", 3),
        cat("gemma2:2b", "~1.6GB", "구글 초경량", "범용", 1),
        cat("gemma2:9b", "~5.4GB", "구글 범용", "범용", 2),
        cat("gemma2:27b", "~16GB", "구글 고품질", "범용", 3),
        cat("mistral:7b", "~4.1GB", "빠른 범용", "범용", 2),
        cat("mixtral:8x7b", "~26GB", "MoE 강력", "범용", 3),
        cat("phi3.5:3.8b", "~2.2GB", "MS 소형 강자", "범용", 1),
        cat("phi3:14b", "~7.9GB", "MS 중형", "범용", 2),
        cat("hermes3:8b", "~4.7GB", "대화·지시 특화", "범용", 2),
        cat("hermes3:70b", "~40GB", "대형 대화", "범용", 3),
        cat("aya:8b", "~4.8GB", "다국어(한국어)", "범용", 2),
        cat("aya:35b", "~20GB", "다국어 대형", "범용", 3),
        // 코딩
        cat("qwen2.5-coder:1.5b", "~1.0GB", "경량 코딩", "코딩", 1),
        cat("qwen2.5-coder:7b", "~4.7GB", "코딩 주력", "코딩", 2),
        cat("qwen2.5-coder:32b", "~20GB", "최상위 코딩", "코딩", 3),
        cat("codellama:7b", "~3.8GB", "코드 생성", "코딩", 2),
        cat("codellama:13b", "~7.4GB", "코드 중형", "코딩", 2),
        cat("deepseek-coder-v2:16b", "~9.0GB", "강력 코딩", "코딩", 2),
        cat("starcoder2:3b", "~1.7GB", "코드 자동완성", "코딩", 1),
        cat("codegemma:7b", "~5.0GB", "구글 코딩", "코딩", 2),
        // 추론
        cat("deepseek-r1:7b", "~4.7GB", "추론 특화", "추론", 2),
        cat("deepseek-r1:8b", "~4.9GB", "추론", "추론", 2),
        cat("deepseek-r1:14b", "~9.0GB", "추론 중형", "추론", 2),
        cat("deepseek-r1:32b", "~20GB", "추론 대형", "추론", 3),
        // 비전 (이미지 이해)
        cat("moondream:1.8b", "~1.7GB", "초경량 비전", "비전", 1),
        cat("llava:7b", "~4.5GB", "이미지 이해", "비전", 2),
        cat("llava:13b", "~8.0GB", "이미지 이해 중형", "비전", 2),
        cat("llama3.2-vision:11b", "~7.9GB", "이미지+텍스트", "비전", 2),
        cat("minicpm-v:8b", "~5.5GB", "이미지 이해", "비전", 2),
        // 임베딩 (검색·RAG)
        cat("nomic-embed-text", "~0.3GB", "임베딩(검색)", "임베딩", 1),
        cat("mxbai-embed-large", "~0.7GB", "고품질 임베딩", "임베딩", 1),
        cat("bge-m3", "~1.2GB", "다국어 임베딩", "임베딩", 1),
        // 경량
        cat("tinyllama:1.1b", "~0.6GB", "초경량", "경량", 1),
        cat("smollm2:1.7b", "~1.1GB", "소형", "경량", 1),
    ]
}

// ---------- Ollama 연동 ----------

#[tauri::command]
async fn check_ollama() -> bool {
    reqwest::Client::new()
        .get(format!("{OLLAMA}/api/version"))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[derive(Deserialize)]
struct TagsResp {
    models: Vec<TagModel>,
}
#[derive(Deserialize)]
struct TagModel {
    name: String,
}

#[tauri::command]
async fn list_installed_models() -> Result<Vec<String>, String> {
    let resp = reqwest::Client::new()
        .get(format!("{OLLAMA}/api/tags"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<TagsResp>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.models.into_iter().map(|m| m.name).collect())
}

// 모델 삭제
#[tauri::command]
async fn delete_model(model: String) -> Result<(), String> {
    let resp = reqwest::Client::new()
        .delete(format!("{OLLAMA}/api/delete"))
        .json(&serde_json::json!({ "model": model, "name": model }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(format!("삭제 실패: HTTP {}", resp.status()))
    }
}

// 모델 다운로드: 진행률을 "pull-progress" 이벤트로 흘려보냄
#[tauri::command]
async fn pull_model(app: AppHandle, model: String) -> Result<(), String> {
    let resp = reqwest::Client::new()
        .post(format!("{OLLAMA}/api/pull"))
        .json(&serde_json::json!({ "model": model, "stream": true }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(idx) = buf.find('\n') {
            let line = buf[..idx].trim().to_string();
            buf = buf[idx + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                let _ = app.emit("pull-progress", serde_json::json!({ "model": model, "data": v }));
            }
        }
    }
    let _ = app.emit("pull-progress", serde_json::json!({ "model": model, "done": true }));
    Ok(())
}

// 단일 생성 (stream=false)
async fn generate(client: &reqwest::Client, model: &str, prompt: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct GenResp {
        response: String,
    }
    let r = client
        .post(format!("{OLLAMA}/api/generate"))
        .json(&serde_json::json!({ "model": model, "prompt": prompt, "stream": false }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<GenResp>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(r.response.trim().to_string())
}

// 스트리밍 생성: 토큰을 `event`로 흘려보내고 전체 텍스트를 반환
async fn generate_stream(
    app: &AppHandle,
    client: &reqwest::Client,
    model: &str,
    prompt: &str,
    event: &str,
) -> Result<String, String> {
    let resp = client
        .post(format!("{OLLAMA}/api/generate"))
        .json(&serde_json::json!({ "model": model, "prompt": prompt, "stream": true }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut full = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(idx) = buf.find('\n') {
            let line = buf[..idx].trim().to_string();
            buf = buf[idx + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(tok) = v.get("response").and_then(|x| x.as_str()) {
                    full.push_str(tok);
                    let _ = app.emit(event, tok);
                }
            }
        }
    }
    Ok(full.trim().to_string())
}

// ---------- 저장소 (대화 기억 + 작업 캐시) ----------

fn data_dir(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("local-ai-harness"));
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[derive(Serialize, Deserialize, Clone)]
struct Conversation {
    time: u64,
    prompt: String,
    summary: String,
    result: String,
}

fn conv_path(app: &AppHandle) -> PathBuf {
    data_dir(app).join("conversations.json")
}

fn load_conversations(app: &AppHandle) -> Vec<Conversation> {
    std::fs::read_to_string(conv_path(app))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_conversation(app: &AppHandle, conv: Conversation) {
    let mut all = load_conversations(app);
    all.push(conv);
    if let Ok(s) = serde_json::to_string_pretty(&all) {
        std::fs::write(conv_path(app), s).ok();
    }
}

// 이전 대화 요약 전부를 맥락 블록으로 구성
fn memory_context(app: &AppHandle) -> (usize, String) {
    let convs = load_conversations(app);
    if convs.is_empty() {
        return (0, String::new());
    }
    let lines: Vec<String> = convs
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {}", i + 1, c.summary))
        .collect();
    let block = format!(
        "다음은 사용자와의 이전 대화 요약입니다. 답변 시 이 맥락을 반드시 참고하세요.\n{}\n\n",
        lines.join("\n")
    );
    (convs.len(), block)
}

// 작업 캐시 (model+prompt 해시 키)
async fn generate_cached(
    app: &AppHandle,
    client: &reqwest::Client,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    let mut h = DefaultHasher::new();
    model.hash(&mut h);
    prompt.hash(&mut h);
    let key = format!("{:x}", h.finish());
    let cache = data_dir(app).join("cache");
    std::fs::create_dir_all(&cache).ok();
    let file = cache.join(format!("{key}.txt"));
    if let Ok(s) = std::fs::read_to_string(&file) {
        return Ok(s);
    }
    let out = generate(client, model, prompt).await?;
    std::fs::write(&file, &out).ok();
    Ok(out)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[tauri::command]
fn list_conversations(app: AppHandle) -> Vec<Conversation> {
    load_conversations(&app)
}

#[tauri::command]
fn clear_conversations(app: AppHandle) -> Result<(), String> {
    std::fs::remove_file(conv_path(&app)).ok();
    std::fs::remove_dir_all(data_dir(&app).join("cache")).ok();
    Ok(())
}

// ---------- 디자인: 이미지 생성 (Stable Diffusion 자동 감지) + 비전 ----------

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use std::time::Duration;

const A1111: &str = "http://127.0.0.1:7860";
const COMFY: &str = "http://127.0.0.1:8188";

async fn probe(client: &reqwest::Client, url: &str) -> bool {
    client
        .get(url)
        .timeout(Duration::from_millis(900))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[tauri::command]
async fn check_image_backends() -> serde_json::Value {
    let c = reqwest::Client::new();
    let a = probe(&c, &format!("{A1111}/sdapi/v1/sd-models")).await;
    let f = probe(&c, &format!("{COMFY}/system_stats")).await;
    serde_json::json!({ "a1111": a, "comfy": f })
}

// Automatic1111 / Forge txt2img → 순수 base64 PNG
async fn a1111_txt2img(client: &reqwest::Client, prompt: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct R {
        images: Vec<String>,
    }
    let r = client
        .post(format!("{A1111}/sdapi/v1/txt2img"))
        .json(&serde_json::json!({
            "prompt": prompt, "steps": 25, "width": 768, "height": 768, "cfg_scale": 7
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<R>()
        .await
        .map_err(|e| e.to_string())?;
    let b64 = r.images.into_iter().next().ok_or("이미지를 받지 못했습니다.")?;
    Ok(b64.split(',').last().unwrap_or("").to_string())
}

// ComfyUI txt2img (기본 워크플로, 첫 체크포인트 사용)
async fn comfy_txt2img(client: &reqwest::Client, prompt: &str) -> Result<String, String> {
    let info: serde_json::Value = client
        .get(format!("{COMFY}/object_info/CheckpointLoaderSimple"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let ckpt = info["CheckpointLoaderSimple"]["input"]["required"]["ckpt_name"][0][0]
        .as_str()
        .ok_or("ComfyUI에 SD 체크포인트(모델)가 없습니다. 모델을 먼저 넣어주세요.")?
        .to_string();
    let seed = now_ms();
    let workflow = serde_json::json!({
        "3": {"class_type": "KSampler", "inputs": {"seed": seed, "steps": 20, "cfg": 7.0, "sampler_name": "euler", "scheduler": "normal", "denoise": 1.0, "model": ["4", 0], "positive": ["6", 0], "negative": ["7", 0], "latent_image": ["5", 0]}},
        "4": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": ckpt}},
        "5": {"class_type": "EmptyLatentImage", "inputs": {"width": 768, "height": 768, "batch_size": 1}},
        "6": {"class_type": "CLIPTextEncode", "inputs": {"text": prompt, "clip": ["4", 1]}},
        "7": {"class_type": "CLIPTextEncode", "inputs": {"text": "", "clip": ["4", 1]}},
        "8": {"class_type": "VAEDecode", "inputs": {"samples": ["3", 0], "vae": ["4", 2]}},
        "9": {"class_type": "SaveImage", "inputs": {"filename_prefix": "harness", "images": ["8", 0]}}
    });
    let resp: serde_json::Value = client
        .post(format!("{COMFY}/prompt"))
        .json(&serde_json::json!({ "prompt": workflow, "client_id": "harness" }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let pid = resp["prompt_id"]
        .as_str()
        .ok_or("ComfyUI가 작업을 받지 못했습니다.")?
        .to_string();
    for _ in 0..180 {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let hist: serde_json::Value = client
            .get(format!("{COMFY}/history/{pid}"))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        let img = &hist[&pid]["outputs"]["9"]["images"][0];
        if let Some(fname) = img["filename"].as_str() {
            let sub = img["subfolder"].as_str().unwrap_or("");
            let bytes = client
                .get(format!("{COMFY}/view?filename={fname}&subfolder={sub}&type=output"))
                .send()
                .await
                .map_err(|e| e.to_string())?
                .bytes()
                .await
                .map_err(|e| e.to_string())?;
            return Ok(STANDARD.encode(&bytes));
        }
    }
    Err("ComfyUI 생성이 시간 초과되었습니다.".into())
}

#[tauri::command]
async fn generate_image(app: AppHandle, prompt: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let (b64, backend) = if probe(&client, &format!("{A1111}/sdapi/v1/sd-models")).await {
        (a1111_txt2img(&client, &prompt).await?, "Automatic1111")
    } else if probe(&client, &format!("{COMFY}/system_stats")).await {
        (comfy_txt2img(&client, &prompt).await?, "ComfyUI")
    } else {
        return Err(
            "로컬 이미지 생성 엔진을 찾지 못했습니다. Automatic1111(:7860) 또는 ComfyUI(:8188)를 실행하세요.".into(),
        );
    };
    let bytes = STANDARD.decode(&b64).map_err(|e| e.to_string())?;
    let dir = data_dir(&app).join("images");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join(format!("img_{}.png", now_ms()));
    std::fs::write(&path, &bytes).ok();
    Ok(serde_json::json!({ "b64": b64, "path": path.to_string_lossy(), "backend": backend }))
}

// 비전: 이미지(base64) 여러 장 + 질문 → 설명/분석 (Ollama 비전 모델, 스트리밍)
#[tauri::command]
async fn analyze_image(
    app: AppHandle,
    model: String,
    images: Vec<String>,
    prompt: String,
) -> Result<String, String> {
    let imgs: Vec<String> = images
        .iter()
        .map(|i| i.split(',').last().unwrap_or("").to_string())
        .collect();
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{OLLAMA}/api/generate"))
        .json(&serde_json::json!({ "model": model, "prompt": prompt, "images": imgs, "stream": true }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut full = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(idx) = buf.find('\n') {
            let line = buf[..idx].trim().to_string();
            buf = buf[idx + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(tok) = v.get("response").and_then(|x| x.as_str()) {
                    full.push_str(tok);
                    let _ = app.emit("final-token", tok);
                }
            }
        }
    }
    Ok(full.trim().to_string())
}

fn emit_step(app: &AppHandle, phase: &str, status: &str, detail: &str) {
    let _ = app.emit(
        "harness-step",
        serde_json::json!({ "phase": phase, "status": status, "detail": detail }),
    );
}

// ---------- 하네스 오케스트레이션 ----------
// 기억 주입 → 분해 → 작업(병렬·캐시) → 검토 → 종합(스트리밍) → 요약 저장

// 단일 실행: 모델 하나가 한 번에 답 (분해 없음) + 기억 주입/저장
#[tauri::command]
async fn run_single(app: AppHandle, prompt: String, model: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let (mem_count, mem) = memory_context(&app);
    let _ = app.emit("memory-info", serde_json::json!({ "count": mem_count }));
    let full = format!("{mem}{prompt}");
    let answer = generate_stream(&app, &client, &model, &full, "final-token").await?;
    let summary_prompt = format!(
        "다음 요청과 답변을 한국어 1~2문장으로 요약하라. 핵심만.\n\n요청: {prompt}\n\n답변: {answer}"
    );
    let summary = generate(&client, &model, &summary_prompt)
        .await
        .unwrap_or_else(|_| prompt.chars().take(60).collect());
    save_conversation(
        &app,
        Conversation {
            time: now_ms(),
            prompt: prompt.clone(),
            summary,
            result: answer.clone(),
        },
    );
    let _ = app.emit("memory-saved", serde_json::json!({}));
    Ok(answer)
}

// 하네스 실행: 모델 하나로 분해·작업·검토·종합 전부 수행
#[tauri::command]
async fn run_harness(app: AppHandle, prompt: String, model: String) -> Result<String, String> {
    let client = reqwest::Client::new();

    // 0) 이전 대화 기억 불러오기
    let (mem_count, mem) = memory_context(&app);
    let _ = app.emit("memory-info", serde_json::json!({ "count": mem_count }));

    // 1) 분해
    emit_step(&app, "decompose", "start", "요청을 하위 작업으로 분해 중...");
    let decompose_prompt = format!(
        "{mem}다음 요청을 해결하기 위한 하위 작업을 2~4개로 나눠라. 각 줄에 하나씩 번호와 함께 간결하게만 출력하라. 다른 설명은 쓰지 마라.\n\n요청: {prompt}"
    );
    let raw = generate(&client, &model, &decompose_prompt).await?;
    let mut subtasks: Vec<String> = raw
        .lines()
        .map(|l| {
            l.trim()
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == '-' || c == ' ')
                .trim()
                .to_string()
        })
        .filter(|l| !l.is_empty())
        .collect();
    if subtasks.is_empty() {
        subtasks.push(prompt.clone());
    }
    subtasks.truncate(4);
    emit_step(&app, "decompose", "done", &format!("{}개 하위 작업으로 분해", subtasks.len()));
    // 분해된 하위 작업 목록을 프런트에 전달 → 작업 단계가 동적으로 그려짐
    let _ = app.emit("harness-subtasks", serde_json::json!({ "tasks": &subtasks }));

    // 2) 작업 (병렬 + 캐시) — 각 하위 작업이 끝나는 대로 work-item 이벤트
    emit_step(&app, "work", "start", &format!("{}개 하위 작업 병렬 수행 중...", subtasks.len()));
    let mut futures = Vec::new();
    for (i, st) in subtasks.iter().enumerate() {
        let c = client.clone();
        let a = app.clone();
        let m = model.clone();
        let p = prompt.clone();
        let task = st.clone();
        let ctx = mem.clone();
        let idx = i;
        futures.push(async move {
            let wp = format!(
                "{ctx}당신은 전문가입니다. 아래 하위 작업을 충실하고 구체적으로 수행해 결과만 제시하세요.\n\n전체 목표: {p}\n하위 작업: {task}"
            );
            let res = generate_cached(&a, &c, &m, &wp).await;
            let _ = a.emit("work-item", serde_json::json!({ "index": idx, "ok": res.is_ok() }));
            (task, res)
        });
    }
    let results = futures_util::future::join_all(futures).await;
    let mut work_outputs = Vec::new();
    for (task, res) in results {
        let out = res?;
        work_outputs.push(format!("[하위 작업] {task}\n{out}"));
    }
    let joined = work_outputs.join("\n\n---\n\n");
    emit_step(&app, "work", "done", "");

    // 3) 검토
    emit_step(&app, "review", "start", "결과 교차 검증 중...");
    let review_prompt = format!(
        "다음은 한 요청에 대한 여러 부분 작업 결과입니다. 사실 오류, 누락, 모순을 짧게 지적하세요.\n\n요청: {prompt}\n\n결과들:\n{joined}"
    );
    let review = generate(&client, &model, &review_prompt).await?;
    emit_step(&app, "review", "done", &review);

    // 4) 종합 (스트리밍)
    emit_step(&app, "synthesize", "start", "최종 결과물 종합 중...");
    let synth_prompt = format!(
        "{mem}다음 부분 결과와 검토 의견을 바탕으로, 원래 요청에 대한 하나의 완성되고 매끄러운 최종 답변을 작성하세요.\n\n요청: {prompt}\n\n부분 결과:\n{joined}\n\n검토 의견:\n{review}"
    );
    let final_answer =
        generate_stream(&app, &client, &model, &synth_prompt, "final-token").await?;
    emit_step(&app, "synthesize", "done", "완료");

    // 5) 대화 요약 저장
    let summary_prompt = format!(
        "다음 요청과 답변을 한국어 1~2문장으로 요약하라. 핵심 주제와 결론만, 군더더기 없이.\n\n요청: {prompt}\n\n답변: {final_answer}"
    );
    let summary = generate(&client, &model, &summary_prompt)
        .await
        .unwrap_or_else(|_| prompt.chars().take(60).collect());
    save_conversation(
        &app,
        Conversation {
            time: now_ms(),
            prompt: prompt.clone(),
            summary,
            result: final_answer.clone(),
        },
    );
    let _ = app.emit("memory-saved", serde_json::json!({}));

    Ok(final_answer)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_tier_models,
            get_catalog,
            check_ollama,
            list_installed_models,
            pull_model,
            delete_model,
            run_harness,
            run_single,
            list_conversations,
            clear_conversations,
            check_image_backends,
            generate_image,
            analyze_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
