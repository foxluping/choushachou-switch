// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_API_BASE_URL: &str = "https://api.choushachou.top";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    api_key: String,
    enabled: bool,
    default_model: String,
    #[serde(default)]
    custom_path: String,
    #[serde(default = "default_api_url")]
    api_url: String,
}

fn default_api_url() -> String {
    DEFAULT_API_BASE_URL.to_string()
}

/// 预设配置
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Preset {
    name: String,
    api_key: String,
    api_url: String,
    default_model: String,
}

/// 预设配置列表
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct PresetsStore {
    presets: Vec<Preset>,
    last_used: Option<String>, // 上次使用的预设名称
}

#[derive(Debug, Serialize, Deserialize)]
struct TestResult {
    success: bool,
    message: String,
}

/// 获取 ~/.claude/settings.json 的默认路径
fn get_default_claude_settings_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".claude").join("settings.json")
}

/// 获取 ~/.claude.json 路径 (Claude Code 的 onboarding 状态文件)
fn get_claude_json_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".claude.json")
}

/// 获取 Claude Code settings.json 路径（优先使用自定义路径）
fn get_claude_settings_path(custom_path: &str) -> PathBuf {
    if custom_path.is_empty() {
        get_default_claude_settings_path()
    } else {
        PathBuf::from(custom_path)
    }
}

/// 获取我们自己的配置文件路径（存储 API Key 等）
fn get_app_config_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    let dir = home.join(".choushachou-switch");
    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }
    dir.join("config.json")
}

/// 获取预设配置存储路径
fn get_presets_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    let dir = home.join(".choushachou-switch");
    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }
    dir.join("presets.json")
}

/// 确保 ~/.claude.json 存在且包含 hasCompletedOnboarding: true
fn ensure_claude_json() {
    let path = get_claude_json_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(obj) = val.as_object_mut() {
                if !obj.contains_key("hasCompletedOnboarding") {
                    obj.insert(
                        "hasCompletedOnboarding".to_string(),
                        serde_json::Value::Bool(true),
                    );
                    if let Ok(output) = serde_json::to_string_pretty(&val) {
                        fs::write(&path, output).ok();
                    }
                }
            }
        }
    } else {
        let val = serde_json::json!({
            "hasCompletedOnboarding": true
        });
        if let Ok(output) = serde_json::to_string_pretty(&val) {
            fs::write(&path, output).ok();
        }
    }
}

/// 获取默认路径（返回给前端展示）
#[tauri::command]
fn get_default_path() -> String {
    get_default_claude_settings_path()
        .to_string_lossy()
        .to_string()
}

/// 检测路径是否有效
#[tauri::command]
fn detect_settings_path() -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();

    let default_path = get_default_claude_settings_path();
    if default_path.exists() {
        paths.push(default_path.to_string_lossy().to_string());
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(home) = dirs::home_dir() {
            let alt_paths = vec![
                home.join(".claude").join("settings.json"),
                home.join("AppData")
                    .join("Roaming")
                    .join("claude")
                    .join("settings.json"),
                home.join("AppData")
                    .join("Local")
                    .join("claude")
                    .join("settings.json"),
            ];
            for p in alt_paths {
                let s = p.to_string_lossy().to_string();
                if p.exists() && !paths.contains(&s) {
                    paths.push(s);
                }
            }
        }
    }

    if paths.is_empty() {
        paths.push(default_path.to_string_lossy().to_string());
    }

    paths
}

/// 加载 app 配置
#[tauri::command]
fn load_config() -> Config {
    let path = get_app_config_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(Config {
            api_key: String::new(),
            enabled: false,
            default_model: "claude-sonnet-4-6".to_string(),
            custom_path: String::new(),
            api_url: DEFAULT_API_BASE_URL.to_string(),
        })
    } else {
        Config {
            api_key: String::new(),
            enabled: false,
            default_model: "claude-sonnet-4-6".to_string(),
            custom_path: String::new(),
            api_url: DEFAULT_API_BASE_URL.to_string(),
        }
    }
}

/// 保存配置并写入 Claude Code settings.json
#[tauri::command]
fn save_config(config: Config) -> Result<(), String> {
    // 0. 确保 ~/.claude.json 存在
    ensure_claude_json();

    // 1. 保存 app 自己的配置
    let app_path = get_app_config_path();
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&app_path, json).map_err(|e| format!("写入配置失败: {}", e))?;

    // 2. 修改 Claude Code 的 settings.json
    let claude_path = get_claude_settings_path(&config.custom_path);

    if let Some(parent) = claude_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    let mut settings: serde_json::Value = if claude_path.exists() {
        let content = fs::read_to_string(&claude_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let obj = settings.as_object_mut().ok_or("settings.json 格式错误")?;

    // 使用配置中的 api_url，如果为空则用默认值
    let api_url = if config.api_url.is_empty() {
        DEFAULT_API_BASE_URL.to_string()
    } else {
        config.api_url.clone()
    };

    if config.enabled {
        let mut env = if let Some(existing_env) = obj.get("env") {
            existing_env.as_object().cloned().unwrap_or_default()
        } else {
            serde_json::Map::new()
        };

        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            serde_json::Value::String(api_url),
        );
        env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            serde_json::Value::String(config.api_key.clone()),
        );
        // 移除 ANTHROPIC_API_KEY 避免冲突
        env.remove("ANTHROPIC_API_KEY");

        env.insert(
            "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
            serde_json::Value::String(config.default_model.clone()),
        );
        env.insert(
            "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
            serde_json::Value::String(config.default_model.clone()),
        );
        env.insert(
            "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
            serde_json::Value::String(config.default_model.clone()),
        );

        env.insert(
            "CLAUDE_CODE_USE_BEDROCK".to_string(),
            serde_json::Value::String("0".to_string()),
        );
        env.insert(
            "CLAUDE_CODE_USE_VERTEX".to_string(),
            serde_json::Value::String("0".to_string()),
        );
        env.insert(
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
            serde_json::Value::String("1".to_string()),
        );

        obj.insert("env".to_string(), serde_json::Value::Object(env));
    } else {
        if let Some(env_val) = obj.get_mut("env") {
            if let Some(env_obj) = env_val.as_object_mut() {
                env_obj.remove("ANTHROPIC_BASE_URL");
                env_obj.remove("ANTHROPIC_API_KEY");
                env_obj.remove("ANTHROPIC_AUTH_TOKEN");
                env_obj.remove("ANTHROPIC_DEFAULT_HAIKU_MODEL");
                env_obj.remove("ANTHROPIC_DEFAULT_SONNET_MODEL");
                env_obj.remove("ANTHROPIC_DEFAULT_OPUS_MODEL");
                env_obj.remove("CLAUDE_CODE_USE_BEDROCK");
                env_obj.remove("CLAUDE_CODE_USE_VERTEX");
                env_obj.remove("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC");
                if env_obj.is_empty() {
                    obj.remove("env");
                }
            }
        }
    }

    let output = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&claude_path, output).map_err(|e| format!("写入 Claude settings 失败: {}", e))?;

    Ok(())
}

/// 测试 API 连通性（使用自定义 URL）
#[tauri::command]
async fn test_connection(api_key: String, api_url: Option<String>) -> Result<TestResult, String> {
    let base_url = api_url
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| DEFAULT_API_BASE_URL.to_string());

    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/v1/models", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = resp.status();

    if status.is_success() {
        Ok(TestResult {
            success: true,
            message: "连接成功! API 令牌有效".to_string(),
        })
    } else if status.as_u16() == 401 {
        Ok(TestResult {
            success: false,
            message: "API 令牌无效，请检查".to_string(),
        })
    } else {
        let body = resp.text().await.unwrap_or_default();
        Ok(TestResult {
            success: false,
            message: format!("服务器返回错误 ({}): {}", status.as_u16(), body),
        })
    }
}

/// 加载预设列表
#[tauri::command]
fn load_presets() -> PresetsStore {
    let path = get_presets_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        PresetsStore::default()
    }
}

/// 保存预设
#[tauri::command]
fn save_preset(preset: Preset) -> Result<PresetsStore, String> {
    let path = get_presets_path();
    let mut store = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        PresetsStore::default()
    };

    // 如果同名预设已存在，替换之
    if let Some(existing) = store.presets.iter_mut().find(|p| p.name == preset.name) {
        *existing = preset.clone();
    } else {
        store.presets.push(preset.clone());
    }

    store.last_used = Some(preset.name);

    let json = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("保存预设失败: {}", e))?;

    Ok(store)
}

/// 删除预设
#[tauri::command]
fn delete_preset(name: String) -> Result<PresetsStore, String> {
    let path = get_presets_path();
    let mut store = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        PresetsStore::default()
    };

    store.presets.retain(|p| p.name != name);
    if store.last_used.as_deref() == Some(&name) {
        store.last_used = None;
    }

    let json = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("删除预设失败: {}", e))?;

    Ok(store)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            test_connection,
            get_default_path,
            detect_settings_path,
            load_presets,
            save_preset,
            delete_preset
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
