#!/bin/bash

# MMY Todo 发布打包脚本
# 用法: ./release.sh
# 输出目录: releases/v{版本号}_{YYYYMMDD_HHMMSS}/

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# 获取版本号
get_version() {
    cd "$(dirname "$0")/desktop"
    version=$(node -p "require('./package.json').version")
    echo "$version"
}

# 获取时间戳
get_timestamp() {
    date +"%Y%m%d_%H%M%S"
}

# 检测系统
detect_os() {
    case "$(uname -s)" in
        Darwin*) echo "mac" ;;
        MINGW*|MSYS*|CYGWIN*) echo "win" ;;
        *) echo "unknown" ;;
    esac
}

# 创建发布目录
create_release_dir() {
    local version="$1"
    local timestamp="$2"
    local os="$3"

    local release_base="$(dirname "$0")/releases"
    local release_dir="${release_base}/v${version}_${timestamp}"

    mkdir -p "$release_dir"
    echo "$release_dir"
}

# 构建 macOS DMG
build_mac() {
    local release_dir="$1"

    log_step "开始构建 macOS DMG..."

    cd "$(dirname "$0")/desktop"

    # 安装依赖
    log_info "检查依赖..."
    npm install --prefer-offline

    # 构建
    log_info "构建前端和 Tauri 应用..."
    npm run tauri build -- --bundles dmg

    # 复制 DMG 到发布目录
    local dmg_path="src-tauri/target/release/bundle/dmg"
    if [ -d "$dmg_path" ]; then
        cp "$dmg_path"/*.dmg "$release_dir/" 2>/dev/null || true
        log_info "DMG 已复制到: $release_dir"
        ls -la "$release_dir"/*.dmg 2>/dev/null || true
    else
        log_error "未找到 DMG 文件"
        exit 1
    fi
}

# 构建 Windows 安装包和便携版
build_win() {
    local release_dir="$1"

    log_step "开始构建 Windows 安装包..."

    cd "$(dirname "$0")/desktop"

    # 安装依赖
    log_info "检查依赖..."
    npm install --prefer-offline

    # 构建 NSIS 安装包
    log_info "构建前端和 Tauri 应用..."
    npm run tauri build -- --bundles nsis

    # 复制安装包到发布目录
    local nsis_path="src-tauri/target/release/bundle/nsis"
    if [ -d "$nsis_path" ]; then
        # 复制安装程序
        cp "$nsis_path"/*_setup.exe "$release_dir/" 2>/dev/null || true

        # 如果有便携版，也复制
        if ls "$nsis_path"/*.exe 1>/dev/null 2>&1; then
            for exe in "$nsis_path"/*.exe; do
                if [[ ! "$exe" =~ _setup\.exe$ ]]; then
                    cp "$exe" "$release_dir/" 2>/dev/null || true
                fi
            done
        fi

        log_info "安装包已复制到: $release_dir"
        ls -la "$release_dir"/*.exe 2>/dev/null || true
    else
        log_error "未找到 NSIS 安装包"
        exit 1
    fi
}

# 生成发布说明
generate_release_notes() {
    local release_dir="$1"
    local version="$2"
    local timestamp="$3"
    local os="$4"

    local notes_file="${release_dir}/README.txt"

    cat > "$notes_file" << EOF
MMY TodoList v${version}
发布时间: ${timestamp}
构建系统: ${os}

文件列表:
EOF

    # 列出所有文件
    ls -la "$release_dir" | grep -v "^d" | grep -v "README.txt" | awk '{print "- " $NF}' >> "$notes_file"

    log_info "发布说明已生成: $notes_file"
}

# 主流程
main() {
    log_step "=== MMY Todo 发布打包流程 ==="

    # 获取参数
    local version=$(get_version)
    local timestamp=$(get_timestamp)
    local os=$(detect_os)

    log_info "版本号: $version"
    log_info "时间戳: $timestamp"
    log_info "构建系统: $os"

    # 创建发布目录
    local release_dir=$(create_release_dir "$version" "$timestamp" "$os")
    log_info "发布目录: $release_dir"

    # 根据系统构建
    case "$os" in
        mac)
            build_mac "$release_dir"
            ;;
        win)
            build_win "$release_dir"
            ;;
        *)
            log_error "未知系统: $os"
            exit 1
            ;;
    esac

    # 生成发布说明
    generate_release_notes "$release_dir" "$version" "$timestamp" "$os"

    log_step "=== 发布打包完成 ==="
    log_info "输出目录: $release_dir"
    echo ""
    echo "可用于分发的文件:"
    ls -la "$release_dir"
}

main "$@"