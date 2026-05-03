#!/bin/bash

# MMY Todo 一键打包脚本
# 用法: ./build.sh [platform]
#   platform: mac | win | all (默认检测当前系统)

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 进入 desktop 目录
cd "$(dirname "$0")/desktop"

# 检测系统
detect_os() {
    case "$(uname -s)" in
        Darwin*) echo "mac" ;;
        MINGW*|MSYS*|CYGWIN*) echo "win" ;;
        *) echo "unknown" ;;
    esac
}

# 构建 macOS DMG
build_mac() {
    log_info "开始构建 macOS DMG..."

    # 检查是否在 macOS 上
    if [ "$(detect_os)" != "mac" ]; then
        log_error "DMG 只能在 macOS 上构建"
        exit 1
    fi

    # 安装依赖
    log_info "安装依赖..."
    npm install

    # 构建
    log_info "构建前端和 Tauri 应用..."
    npm run tauri build -- --bundles dmg

    # 输出路径
    local dmg_path="src-tauri/target/release/bundle/dmg"
    if [ -d "$dmg_path" ]; then
        log_info "构建完成！DMG 文件位于: $dmg_path"
        ls -la "$dmg_path"/*.dmg 2>/dev/null || true
    fi
}

# 构建 Windows 便携版
build_win() {
    log_info "开始构建 Windows 便携版..."

    # 检查是否在 Windows 上
    if [ "$(detect_os)" != "win" ]; then
        log_error "Windows 版本只能在 Windows 上构建"
        exit 1
    fi

    # 安装依赖
    log_info "安装依赖..."
    npm install

    # 构建 NSIS 便携版
    log_info "构建前端和 Tauri 应用..."
    npm run tauri build -- --bundles nsis

    # 输出路径
    local nsis_path="src-tauri/target/release/bundle/nsis"
    if [ -d "$nsis_path" ]; then
        log_info "构建完成！安装包位于: $nsis_path"
        ls -la "$nsis_path"/*.exe 2>/dev/null || true
    fi
}

# 显示帮助
show_help() {
    echo "MMY Todo 一键打包脚本"
    echo ""
    echo "用法: $0 [platform]"
    echo ""
    echo "参数:"
    echo "  mac   - 构建 macOS DMG 安装包"
    echo "  win   - 构建 Windows 便携版 EXE"
    echo "  all   - 构建当前系统支持的版本"
    echo ""
    echo "示例:"
    echo "  $0 mac    # 在 macOS 上构建 DMG"
    echo "  $0 win    # 在 Windows 上构建便携版"
    echo ""
}

# 主逻辑
main() {
    local platform="${1:-auto}"
    local current_os=$(detect_os)

    case "$platform" in
        mac)
            build_mac
            ;;
        win)
            build_win
            ;;
        all|auto)
            case "$current_os" in
                mac) build_mac ;;
                win) build_win ;;
                *)
                    log_error "未知系统: $current_os"
                    show_help
                    exit 1
                    ;;
            esac
            ;;
        -h|--help|help)
            show_help
            ;;
        *)
            log_error "未知参数: $platform"
            show_help
            exit 1
            ;;
    esac
}

main "$@"