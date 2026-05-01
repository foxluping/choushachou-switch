// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const API_BASE_URL: &str = "https://api.choushachou.top";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    api_key: String,
    enabled: bool,
    default_model: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestResult {
    success: bool,
    message: String,
}

/// 获取 ~/.claude/settings.json 路径
fn get_claude_settings_path() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join(".claude").join("settings.json")
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
        })
    } else {
        Config {
            api_key: String::new(),
            enabled: false,
            default_model: "claude-sonnet-4-6".to_string(),
        }
    }
}

/// 保存配置并写入 Claude Code settings.json
#[tauri::command]
fn save_config(config: Config) -> Result<(), String> {
    // 1. 保存 app 自己的配置
    let app_path = get_app_config_path();
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&app_path, json).map_err(|e| format!("写入配置失败: {}", e))?;

    // 2. 修改 Claude Code 的 settings.json
    let claude_path = get_claude_settings_path();

    // 确保 ~/.claude 目录存在
    if let Some(parent) = claude_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    // 读取现有配置，如果没有就创建空对象
    let mut settings: serde_json::Value = if claude_path.exists() {
        let content = fs::read_to_string(&claude_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let obj = settings.as_object_mut().ok_or("settings.json 格式错误")?;

    if config.enabled {
        // 启用: 设置环境变量让 Claude Code 走我们的 API
        // 保留现有 env 中的其他配置
        let mut env = if let Some(existing_env) = obj.get("env") {
            existing_env.as_object().cloned().unwrap_or_default()
        } else {
            serde_json::Map::new()
        };
        // 同时设置 ANTHROPIC_API_KEY 和 ANTHROPIC_AUTH_TOKEN
        // ANTHROPIC_API_KEY: 作为 X-Api-Key 标头发送 (某些版本 Claude Code 启动时必须检测到)
        // ANTHROPIC_AUTH_TOKEN: 作为 Authorization: Bearer 标头发送
        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            serde_json::Value::String(API_BASE_URL.to_string()),
        );
        env.insert(
            "ANTHROPIC_API_KEY".to_string(),
            serde_json::Value::String(config.api_key.clone()),
        );
        env.insert(
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            serde_json::Value::String(config.api_key.clone()),
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
        // 禁用: 移除我们设置的环境变量
        if let Some(env_val) = obj.get_mut("env") {
            if let Some(env_obj) = env_val.as_object_mut() {
                env_obj.remove("ANTHROPIC_BASE_URL");
                env_obj.remove("ANTHROPIC_API_KEY");
                env_obj.remove("ANTHROPIC_AUTH_TOKEN");
                env_obj.remove("CLAUDE_CODE_USE_BEDROCK");
                env_obj.remove("CLAUDE_CODE_USE_VERTEX");
                env_obj.remove("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC");
                // 如果 env 为空，整个移除
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

/// 测试 API 连通性
#[tauri::command]
async fn test_connection(api_key: String) -> Result<TestResult, String> {
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/v1/models", API_BASE_URL))
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            test_connection
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
