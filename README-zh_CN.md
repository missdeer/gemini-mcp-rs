# gemini-mcp-rs

[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust Version](https://img.shields.io/badge/rust-1.77.2%2B-blue.svg)](https://www.rust-lang.org)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

[English](README.md)

高性能的 Rust 实现的 MCP（Model Context Protocol）服务器，封装 Gemini CLI 以支持 AI 驱动的任务。

## 快速开始

使用 gemini-mcp-rs 最简单的方式是通过 npx，无需手动安装：

```bash
npx @missdeer/gemini-mcp-rs
```

此命令会自动下载适合您平台的二进制文件并运行。添加到 Claude Code：

```bash
claude mcp add gemini-rs -s user --transport stdio -- npx @missdeer/gemini-mcp-rs
```

完成！MCP 服务器现已在 Claude Code 中可用。

## 特性

- **MCP 协议支持**：使用 Rust SDK 实现官方 Model Context Protocol
- **Gemini 集成**：封装 Gemini CLI，通过 MCP 启用 AI 驱动的任务
- **会话管理**：通过会话 ID 支持多轮对话
- **沙箱安全**：可配置的沙箱模式，用于隔离执行
- **异步运行时**：基于 Tokio 构建，实现高效的异步 I/O
- **跨平台**：支持 Windows、Linux 和 macOS（x64 和 arm64）

## 前置要求

- Rust 1.77.2+（Windows 批处理文件安全修复所需，参见 [CVE-2024-24576](https://blog.rust-lang.org/2024/04/09/cve-2024-24576.html)）
- 已安装并配置 [Gemini CLI](https://github.com/google-gemini/gemini-cli)
- Claude Code 或其他 MCP 客户端

## 构建

```bash
# 调试构建
cargo build

# 发布构建
cargo build --release
```

## 运行

服务器通过 stdio 传输进行通信：

```bash
cargo run
```

或在构建后：

```bash
./target/release/gemini-mcp-rs
```

### 命令行选项

```bash
# 显示帮助信息
./target/release/gemini-mcp-rs --help

# 显示版本信息
./target/release/gemini-mcp-rs --version
```

`--help` 标志提供完整的文档，包括：
- 环境变量
- MCP 客户端配置示例
- 所有支持的工具参数
- GEMINI.md 配置文件支持
- 返回结构格式
- 最佳实践和安全信息

## 安装

### 方式一：NPX（推荐）

使用 npx 直接运行，无需安装：

```bash
npx @missdeer/gemini-mcp-rs
```

或全局安装：

```bash
npm install -g @missdeer/gemini-mcp-rs
```

然后添加到您的 Claude MCP 配置：

```bash
claude mcp add gemini-rs -s user --transport stdio -- npx @missdeer/gemini-mcp-rs
```

### 方式二：快速安装（Linux/macOS）

使用单条命令安装最新版本：

```bash
curl -sSL https://raw.githubusercontent.com/missdeer/gemini-mcp-rs/master/scripts/install.sh | bash
```

或安装指定版本：

```bash
curl -sSL https://raw.githubusercontent.com/missdeer/gemini-mcp-rs/master/scripts/install.sh | bash -s v0.1.0
```

此脚本将：
- 检测您的平台和架构
- 从 GitHub releases 下载适合的二进制文件
- 安装到 `~/.local/bin`（或 `/usr/local/bin`，如需要）
- 自动添加到您的 Claude MCP 配置

### 方式三：从源码构建

```bash
git clone https://github.com/missdeer/gemini-mcp-rs.git
cd gemini-mcp-rs
cargo build --release
claude mcp add gemini-rs -s user --transport stdio -- $(pwd)/target/release/gemini-mcp-rs
```

### 方式四：从 Release 安装

从 [releases 页面](https://github.com/missdeer/gemini-mcp-rs/releases) 下载适合您平台的二进制文件：

| 平台 | 架构 | 文件 |
|------|------|------|
| Linux | x64 | `gemini-mcp-rs_Linux_x86_64.tar.gz` |
| Linux | arm64 | `gemini-mcp-rs_Linux_arm64.tar.gz` |
| macOS | Universal (x64 + arm64) | `gemini-mcp-rs_Darwin_universal.tar.gz` |
| Windows | x64 | `gemini-mcp-rs_Windows_x86_64.zip` |
| Windows | arm64 | `gemini-mcp-rs_Windows_arm64.zip` |

解压并添加到您的 MCP 配置：

```bash
claude mcp add gemini-rs -s user --transport stdio -- /path/to/gemini-mcp-rs
```

## 工具使用

服务器提供一个 `gemini` 工具，具有以下参数：

### 必需参数

- `PROMPT`（字符串）：发送给 gemini 的任务指令

### 可选参数

- `sandbox`（布尔值）：以沙箱模式运行。默认为 `False`
- `SESSION_ID`（字符串）：恢复指定的 gemini 会话。默认为空字符串，开始新会话
- `return_all_messages`（布尔值）：返回 gemini 会话的所有消息（如推理、工具调用等）。默认为 `False`，仅返回代理的最终回复消息
- `model`（字符串）：用于 gemini 会话的模型。如未指定，使用 `GEMINI_FORCE_MODEL` 环境变量或 Gemini CLI 默认值
- `timeout_secs`（整数）：gemini 执行的超时时间（秒）（1-3600）。默认为 `GEMINI_DEFAULT_TIMEOUT` 环境变量或 600 秒（10 分钟）

### 返回结构

**成功：**
```json
{
  "success": true,
  "SESSION_ID": "session-uuid",
  "agent_messages": "Gemini 的回复内容..."
}
```

**启用 return_all_messages 时：**
```json
{
  "success": true,
  "SESSION_ID": "session-uuid",
  "agent_messages": "Gemini 的回复内容...",
  "all_messages": [...]
}
```

**失败：**
```json
{
  "success": false,
  "error": "错误描述"
}
```

## 最佳实践

- 始终捕获并重用 `SESSION_ID` 进行多轮交互
- 当需要隔离文件修改时启用 `sandbox` 模式
- 仅在需要详细执行跟踪时使用 `return_all_messages`（会增加负载大小）
- 仅在用户明确请求特定模型时传递 `model`

## 配置

### 环境变量

- `GEMINI_BIN`：覆盖 Gemini CLI 二进制文件路径。默认情况下，服务器使用 PATH 中的 `gemini`。适用于：
  - 使用特定的 Gemini 安装位置
  - 使用自定义二进制文件进行测试
  - 具有多个 Gemini 版本的开发环境

  **示例：**
  ```bash
  export GEMINI_BIN=/usr/local/bin/gemini-custom
  cargo run
  ```

- `GEMINI_DEFAULT_TIMEOUT`：gemini 执行的默认超时时间（秒）（1-3600）。如未设置，默认为 600 秒（10 分钟）。可通过 `timeout_secs` 参数按请求覆盖。

  **示例：**
  ```bash
  export GEMINI_DEFAULT_TIMEOUT=300  # 5 分钟
  cargo run
  ```

- `GEMINI_FORCE_MODEL`：当请求中未提供 `model` 参数时使用的默认模型。会被显式的 `model` 参数覆盖。

  **示例：**
  ```bash
  export GEMINI_FORCE_MODEL=gemini-2.0-flash
  cargo run
  ```

## 测试

```bash
# 运行所有测试
cargo test

# 带输出运行
cargo test -- --nocapture

# 使用自定义 Gemini 二进制文件测试
GEMINI_BIN=/path/to/gemini cargo test
```

## 架构

项目遵循模块化架构：

- `src/main.rs`：入口点，解析 CLI 参数并启动 MCP 服务器
- `src/lib.rs`：库根，导出模块
- `src/server.rs`：MCP 服务器实现和工具处理程序
- `src/gemini.rs`：Gemini CLI 执行和结果解析

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 许可证

本项目采用双重许可：

### 非商业/个人使用 - GNU 通用公共许可证 v3.0

个人项目、教育目的、开源项目和非商业用途免费使用。完整的 GPLv3 许可证文本请参见 [LICENSE](LICENSE)。

### 商业/工作场所使用 - 需要商业许可证

**如果您在商业环境、工作场所或任何商业目的中使用 gemini-mcp-rs，必须获得商业许可证。**

包括但不限于：
- 在工作中使用本软件（任何组织）
- 集成到商业产品或服务中
- 用于客户工作或咨询
- 作为 SaaS/云服务的一部分提供

**联系方式**：missdeer@gmail.com 咨询商业许可。

更多详情请参见 [LICENSE-COMMERCIAL](LICENSE-COMMERCIAL)。

## Star 历史

[![Star History Chart](https://api.star-history.com/svg?repos=missdeer/gemini-mcp-rs&type=Date)](https://starchart.cc/missdeer/gemini-mcp-rs)
