import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface Config {
  api_key: string;
  enabled: boolean;
  default_model: string;
  custom_path: string;
  api_url: string;
}

interface TestResult {
  success: boolean;
  message: string;
}

interface Preset {
  name: string;
  api_key: string;
  api_url: string;
  default_model: string;
}

interface PresetsStore {
  presets: Preset[];
  last_used: string | null;
}

const MODELS = [
  { id: "gpt-5.2", name: "GPT-5.2", desc: "旗舰版，最强推理", provider: "OpenAI" },
  { id: "gpt-5-mini", name: "GPT-5 Mini", desc: "轻量版，性价比之选", provider: "OpenAI" },
  { id: "gpt-5-nano", name: "GPT-5 Nano", desc: "极速版，响应最快", provider: "OpenAI" },
  { id: "o4-mini", name: "O4 Mini", desc: "推理模型，深度思考", provider: "OpenAI" },
  { id: "o3", name: "O3", desc: "推理模型，复杂问题", provider: "OpenAI" },
  { id: "claude-opus-4-6", name: "Claude Opus 4.6", desc: "Anthropic 旗舰", provider: "Anthropic" },
  { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6", desc: "均衡全能", provider: "Anthropic" },
  { id: "claude-haiku-4-5", name: "Claude Haiku 4.5", desc: "轻快日常", provider: "Anthropic" },
];

const DEFAULT_API_URL = "https://api.choushachou.top";

function App() {
  const [config, setConfig] = useState<Config>({
    api_key: "",
    enabled: false,
    default_model: "claude-sonnet-4-6",
    custom_path: "",
    api_url: DEFAULT_API_URL,
  });
  const [testResult, setTestResult] = useState<TestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [defaultPath, setDefaultPath] = useState("");
  const [detectedPaths, setDetectedPaths] = useState<string[]>([]);
  const [presetsStore, setPresetsStore] = useState<PresetsStore>({ presets: [], last_used: null });
  const [presetName, setPresetName] = useState("");
  const [showPresetInput, setShowPresetInput] = useState(false);

  useEffect(() => {
    loadConfig();
    loadPaths();
    loadPresets();
  }, []);

  async function loadConfig() {
    try {
      const cfg = await invoke<Config>("load_config");
      setConfig(cfg);
    } catch (e) {
      console.error("Failed to load config:", e);
    }
    setLoaded(true);
  }

  async function loadPaths() {
    try {
      const dp = await invoke<string>("get_default_path");
      setDefaultPath(dp);
      const paths = await invoke<string[]>("detect_settings_path");
      setDetectedPaths(paths);
    } catch (e) {
      console.error("Failed to detect paths:", e);
    }
  }

  async function loadPresets() {
    try {
      const store = await invoke<PresetsStore>("load_presets");
      setPresetsStore(store);
    } catch (e) {
      console.error("Failed to load presets:", e);
    }
  }

  async function saveConfig() {
    setSaving(true);
    try {
      await invoke("save_config", { config });
      setTestResult({ success: true, message: "配置已保存并应用到 Claude Code!" });
    } catch (e: any) {
      setTestResult({ success: false, message: `保存失败: ${e}` });
    }
    setSaving(false);
  }

  async function testConnection() {
    setTesting(true);
    setTestResult(null);
    try {
      const result = await invoke<TestResult>("test_connection", {
        apiKey: config.api_key,
        apiUrl: config.api_url || null,
      });
      setTestResult(result);
    } catch (e: any) {
      setTestResult({ success: false, message: `测试失败: ${e}` });
    }
    setTesting(false);
  }

  async function toggleEnabled() {
    const newConfig = { ...config, enabled: !config.enabled };
    setConfig(newConfig);
    setSaving(true);
    try {
      await invoke("save_config", { config: newConfig });
      setTestResult({
        success: true,
        message: newConfig.enabled
          ? "已启用，Claude Code 将使用自定义 API"
          : "已切换回 Anthropic 官方 API",
      });
    } catch (e: any) {
      setTestResult({ success: false, message: `切换失败: ${e}` });
    }
    setSaving(false);
  }

  async function browsePath() {
    try {
      const selected = await open({
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
        title: "选择 Claude Code settings.json 文件",
      });
      if (selected) {
        setConfig({ ...config, custom_path: selected as string });
      }
    } catch (e) {
      console.error("Browse failed:", e);
    }
  }

  function resetToDefault() {
    setConfig({ ...config, custom_path: "" });
  }

  async function saveAsPreset() {
    if (!presetName.trim()) return;
    try {
      const preset: Preset = {
        name: presetName.trim(),
        api_key: config.api_key,
        api_url: config.api_url,
        default_model: config.default_model,
      };
      const store = await invoke<PresetsStore>("save_preset", { preset });
      setPresetsStore(store);
      setPresetName("");
      setShowPresetInput(false);
      setTestResult({ success: true, message: `预设「${preset.name}」已保存` });
    } catch (e: any) {
      setTestResult({ success: false, message: `保存预设失败: ${e}` });
    }
  }

  async function applyPreset(preset: Preset) {
    const newConfig = {
      ...config,
      api_key: preset.api_key,
      api_url: preset.api_url,
      default_model: preset.default_model,
    };
    setConfig(newConfig);
    setTestResult({ success: true, message: `已加载预设「${preset.name}」，点击保存配置以应用` });
  }

  async function deletePreset(name: string) {
    try {
      const store = await invoke<PresetsStore>("delete_preset", { name });
      setPresetsStore(store);
      setTestResult({ success: true, message: `预设「${name}」已删除` });
    } catch (e: any) {
      setTestResult({ success: false, message: `删除预设失败: ${e}` });
    }
  }

  const displayPath = config.custom_path || defaultPath;

  if (!loaded) {
    return <div className="loading">加载中...</div>;
  }

  return (
    <div className="container">
      <header className="header">
        <h1 className="title">抽纱绸 AI</h1>
        <p className="subtitle">Claude Code 自定义 API 配置工具</p>
      </header>

      {/* 预设配置 */}
      {presetsStore.presets.length > 0 && (
        <section className="card">
          <h3>快速切换配置</h3>
          <p className="hint">选择已保存的预设配置，一键加载</p>
          <div className="presets-list">
            {presetsStore.presets.map((preset) => (
              <div key={preset.name} className="preset-item">
                <div className="preset-info" onClick={() => applyPreset(preset)}>
                  <div className="preset-name">{preset.name}</div>
                  <div className="preset-detail">
                    {preset.api_url} · {preset.default_model}
                  </div>
                </div>
                <button
                  className="btn-icon btn-delete"
                  onClick={() => deletePreset(preset.name)}
                  title="删除预设"
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* 开关 */}
      <section className="card">
        <div className="switch-row">
          <div>
            <h3>启用自定义 API</h3>
            <p className="hint">
              {config.enabled
                ? "当前: 使用自定义 API 服务"
                : "当前: 使用 Anthropic 官方服务"}
            </p>
          </div>
          <label className="switch">
            <input
              type="checkbox"
              checked={config.enabled}
              onChange={toggleEnabled}
              disabled={saving}
            />
            <span className="slider"></span>
          </label>
        </div>
      </section>

      {/* 配置文件路径 */}
      <section className="card">
        <h3>配置文件路径</h3>
        <p className="hint">Claude Code settings.json 位置</p>
        <div className="path-display">
          <input
            type="text"
            className="input path-input"
            value={displayPath}
            onChange={(e) => setConfig({ ...config, custom_path: e.target.value })}
            placeholder={defaultPath}
          />
          <button onClick={browsePath} className="btn btn-secondary btn-small">
            浏览
          </button>
          {config.custom_path && (
            <button onClick={resetToDefault} className="btn btn-secondary btn-small">
              重置
            </button>
          )}
        </div>
        {detectedPaths.length > 1 && (
          <div className="detected-paths">
            <p className="hint">检测到的路径:</p>
            {detectedPaths.map((p) => (
              <div
                key={p}
                className={`path-option ${displayPath === p ? "active" : ""}`}
                onClick={() => setConfig({ ...config, custom_path: p })}
              >
                {p}
              </div>
            ))}
          </div>
        )}
      </section>

      {/* API URL */}
      <section className="card">
        <h3>API 地址</h3>
        <p className="hint">自定义 API 服务地址（留空使用默认）</p>
        <div className="input-group">
          <input
            type="text"
            placeholder={DEFAULT_API_URL}
            value={config.api_url}
            onChange={(e) => setConfig({ ...config, api_url: e.target.value })}
            className="input"
          />
          {config.api_url !== DEFAULT_API_URL && (
            <button
              onClick={() => setConfig({ ...config, api_url: DEFAULT_API_URL })}
              className="btn btn-secondary btn-small"
            >
              默认
            </button>
          )}
        </div>
      </section>

      {/* API Key */}
      <section className="card">
        <h3>API 令牌</h3>
        <div className="input-group">
          <input
            type="password"
            placeholder="sk-xxxx"
            value={config.api_key}
            onChange={(e) => setConfig({ ...config, api_key: e.target.value })}
            className="input"
          />
          <button
            onClick={testConnection}
            disabled={testing || !config.api_key}
            className="btn btn-secondary"
          >
            {testing ? "测试中..." : "测试连通性"}
          </button>
        </div>
      </section>

      {/* 默认模型 */}
      <section className="card">
        <h3>默认模型</h3>
        <div className="models-grid">
          {MODELS.map((model) => (
            <div
              key={model.id}
              className={`model-item ${config.default_model === model.id ? "active" : ""}`}
              onClick={() => setConfig({ ...config, default_model: model.id })}
            >
              <div className="model-name">{model.name}</div>
              <div className="model-desc">{model.desc}</div>
              <span className={`model-badge ${model.provider === "OpenAI" ? "badge-openai" : "badge-anthropic"}`}>
                {model.provider}
              </span>
            </div>
          ))}
        </div>
      </section>

      {/* 状态消息 */}
      {testResult && (
        <div className={`toast ${testResult.success ? "toast-success" : "toast-error"}`}>
          {testResult.message}
        </div>
      )}

      {/* 操作按钮 */}
      <div className="action-buttons">
        <button
          onClick={saveConfig}
          disabled={saving || !config.api_key}
          className="btn btn-primary btn-full"
        >
          {saving ? "保存中..." : "保存配置"}
        </button>

        {/* 保存为预设 */}
        {!showPresetInput ? (
          <button
            onClick={() => setShowPresetInput(true)}
            disabled={!config.api_key}
            className="btn btn-secondary btn-full"
          >
            保存为预设
          </button>
        ) : (
          <div className="preset-save-row">
            <input
              type="text"
              placeholder="输入预设名称，如：公司、个人"
              value={presetName}
              onChange={(e) => setPresetName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && saveAsPreset()}
              className="input"
              autoFocus
            />
            <button onClick={saveAsPreset} disabled={!presetName.trim()} className="btn btn-primary btn-small">
              确定
            </button>
            <button onClick={() => { setShowPresetInput(false); setPresetName(""); }} className="btn btn-secondary btn-small">
              取消
            </button>
          </div>
        )}
      </div>

      <footer className="footer">
        <p>配置文件: {displayPath}</p>
        <p className="hint">保存后请重启 Claude Code 使配置生效</p>
      </footer>
    </div>
  );
}

export default App;
