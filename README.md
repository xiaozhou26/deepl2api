# deepl2api

DeepL 非官方翻译 API —— 将 DeepL Chrome 扩展的内部翻译接口包装为标准 REST API，无需 DeepL API Key 即可调用。

## 快速开始

```bash
# 运行服务（默认监听 127.0.0.1:9000）
cargo run --release
```

```bash
# 翻译文本
curl 'http://127.0.0.1:9000/translate' \
  -H 'Content-Type: application/json' \
  -d '{"text":"Hello, world!","source_lang":"EN","target_lang":"ZH"}'

# 响应
{"code":200,"data":"你好，世界！","source_lang":"EN"}
```

## 功能特点

- **无需 API Key** —— 模拟 DeepL Chrome 扩展的行为，通过 DeepL 公开网页接口翻译
- **即用型 REST API** —— 单端点，JSON 输入输出
- **语言自动检测** —— 设置 `source_lang` 为 `"auto"` 或留空即可
- **代理支持** —— 通过 `PROXY_LIST` 环境变量配置 HTTP 代理
- **CORS 全开** —— 可在浏览器环境中直接调用

## 安装

### 前置依赖

- [Rust](https://www.rust-lang.org/) 2021 edition 或更新版本

### 构建

```bash
git clone https://github.com/xiaozhou26/deepl2api.git
cd deepl2api
cargo build --release
```

编译后的二进制文件位于 `target/release/deepl2api.exe`。

## 配置

### 环境变量

| 变量 | 说明 | 默认值 |
|---|---|---|
| `PROXY_LIST` | HTTP 代理地址，如 `http://127.0.0.1:7890` | 无（直连） |

### 服务器地址

服务硬编码监听 `127.0.0.1:9000`，如需更改请修改 `src/main.rs` 中的 `addr` 变量。

## 项目架构

```
deepl2api/
├── src/
│   └── main.rs          # 全部代码（~380行，单文件）
├── Cargo.toml           # 依赖及元信息
├── Cargo.lock
├── CLAUDE.md            # Claude Code 项目指南
└── API.md               # API 文档
```

代码结构一览：

1. **DeepLClient** —— 核心翻译客户端
   - 启动时访问 `https://www.deepl.com/translator` 预热 Cookie
   - 内部维护带 Cookie 存储的 HTTP 客户端
   - 支持免费（`oneshot-free`）和 Pro（`oneshot-pro`）两个端点

2. **请求伪装** —— 模拟 DeepL Chrome 扩展身份
   - 请求头携带 `Origin: chrome-extension://...`
   - 请求体包含 `app_information`（操作系统、应用版本、实例 ID 等）
   - 使用随机生成的 UUIDv4 作为实例标识

3. **语言映射** —— 标准 ISO 代码 ↔ DeepL 内部代码
   - 支持 35+ 种目标语言
   - 自动别名：`EN` → `EN-US`，`PT` → `PT-BR`，`ZH` → `ZH-HANS`

4. **Axum Web 服务器** —— 单路由 `POST /translate`
   - 全开 CORS（允许任意来源、方法、请求头）
   - 错误映射为合理 HTTP 状态码

## 限制

- 单次翻译文本最长 **1500 个字符**（Unicode 安全计数）
- 出站 HTTP 请求超时 **20 秒**
- 服务端无认证、无速率限制（依赖 DeepL 端限制）
- 当前始终使用免费端点（`oneshot-free`），未实现 Pro 会话逻辑

## 开发

```bash
# 构建
cargo build

# 运行（开发模式）
cargo run

# 使用代理
$env:PROXY_LIST="http://127.0.0.1:7890"; cargo run
```

## 许可

本项目仅供学习研究目的。使用 DeepL 服务请遵守 [DeepL 服务条款](https://www.deepl.com/pro-license)。
