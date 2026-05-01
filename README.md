# 抽纱绸 AI Switch

Claude Code 自定义 API 配置工具。一键将 Claude Code 切换到抽纱绸 AI 服务。

## 功能

- 输入 API Key，自动写入 Claude Code 配置
- 可视化选择默认模型
- 一键测试 API 连通性
- 开关切换（抽纱绸 AI ↔ Anthropic 官方）

## 开发

### 环境要求

- Node.js 18+
- Rust (rustup)
- Tauri CLI (`cargo install tauri-cli`)

### 安装依赖

```bash
npm install
```

### 开发运行

```bash
npm run tauri dev
```

### 构建发布

```bash
npm run tauri build
```

构建产物在 `src-tauri/target/release/bundle/` 下：
- macOS: `.dmg`
- Windows: `.exe` (NSIS installer)
- Linux: `.AppImage`, `.deb`

## 技术栈

- 前端: React + TypeScript + Vite
- 后端: Tauri 2 (Rust)
- 打包: ~10MB

## 原理

应用将以下环境变量写入 `~/.claude/settings.json`：

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.choushachou.top/v1",
    "ANTHROPIC_API_KEY": "sk-xxxx"
  }
}
```

Claude Code 启动时会读取此文件，自动使用抽纱绸 AI 的 API 端点。
