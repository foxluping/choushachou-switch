#!/bin/bash
# 一键生成 choushachou-switch DMG 安装包
# 在系统 Terminal.app 中运行: bash build-dmg.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_PATH="$SCRIPT_DIR/src-tauri/target/release/bundle/macos/choushachou-switch.app"
DMG_OUTPUT="$SCRIPT_DIR/choushachou-switch_1.0.0_aarch64.dmg"
STAGING_DIR="/tmp/choushachou-dmg-staging"

echo "🔨 开始打包 choushachou-switch DMG..."

# 检查 .app 是否存在
if [ ! -d "$APP_PATH" ]; then
    echo "❌ 错误: 找不到 $APP_PATH"
    echo "请先运行: cd $SCRIPT_DIR && source ~/.cargo/env && npx tauri build --bundles app"
    exit 1
fi

# 清理旧文件
rm -rf "$STAGING_DIR"
rm -f "$DMG_OUTPUT"

# 准备 staging 目录
mkdir -p "$STAGING_DIR"
cp -R "$APP_PATH" "$STAGING_DIR/"
ln -sf /Applications "$STAGING_DIR/Applications"

# 创建 DMG
echo "📦 正在创建 DMG..."
hdiutil create -volname "choushachou-switch" \
    -srcfolder "$STAGING_DIR" \
    -ov -format UDZO \
    "$DMG_OUTPUT"

# 清理
rm -rf "$STAGING_DIR"

echo ""
echo "✅ DMG 打包完成!"
echo "📍 文件位置: $DMG_OUTPUT"
echo "📏 文件大小: $(du -h "$DMG_OUTPUT" | cut -f1)"
echo ""
echo "使用方式: 双击 DMG → 将 App 拖入 Applications 文件夹"
