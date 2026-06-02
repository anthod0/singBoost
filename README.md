# SingBoost

SingBoost 是一个极简 Windows 托盘启动器，用于管理同目录下的 `sing-box.exe`。

## 功能

- 启动/停止 sing-box 内核
- 查看运行日志
- Windows 系统托盘图标
- 设置开机自启

## 目标目录结构

将 `singboost.exe` 放到 sing-box 所在目录：

```text
D:\Program Files\sing-box\
  singboost.exe
  singboost.toml
  sing-box.exe
  wintun.dll
  config.json
  logs\
    singboost-runtime.log
```

首次运行时，如果 `singboost.toml` 不存在，SingBoost 会自动创建默认配置。

## 配置文件

配置文件路径：

```text
<app_dir>\singboost.toml
```

默认内容：

```toml
[app]
run_as_admin = false

[sing_box]
start_command = 'sing-box.exe -D "<app_dir>" -c "<app_dir>\config.json" run'
```

说明：

- `<app_dir>` 会在运行时替换为 `singboost.exe` 所在目录。
- `run_as_admin = true` 时，SingBoost 启动后会通过 UAC 重新拉起自身。
- `start_command` 可以由用户自定义。
- 如果配置文件已存在但 TOML 无法解析、字段缺失或 `start_command` 为空，SingBoost 不会启动 sing-box。

## 托盘菜单

右键托盘图标可使用：

- `启动` / `停止`：启动或停止 sing-box。
- `重启`：仅在 sing-box 运行中可用。
- `打开 UI`：从 sing-box `config.json` 的 `experimental.clash_api.external_controller` 解析 WebUI 地址并打开 `/ui/`；例如 `0.0.0.0:20123` 会打开 `http://127.0.0.1:20123/ui/`。
- `日志`：打开 PowerShell 窗口实时跟踪 `logs\singboost-runtime.log`。
- `以管理员身份运行`：切换管理员权限配置。
- `开机自启`：创建或删除 Windows 任务计划。
- `退出`：停止 sing-box、关闭由 SingBoost 打开的日志窗口并退出。

## 日志

SingBoost 每次启动时会重建：

```text
<app_dir>\logs\singboost-runtime.log
```

日志来源包括：

- sing-box stdout
- sing-box stderr
- SingBoost 自身事件和错误

点击托盘菜单 `日志` 会执行类似命令：

```powershell
Get-Content "<app_dir>\logs\singboost-runtime.log" -Wait
```

## 开机自启

SingBoost 使用 Windows 任务计划程序，而不是注册表 Run 项。

任务名：

```text
SingBoost
```

当 `run_as_admin = true` 时，任务会尝试以最高权限运行。

## 构建

安装 Windows target：

```bash
rustup target add x86_64-pc-windows-msvc
```

常用验证命令：

```bash
cargo fmt --check
cargo check
cargo test
cargo check --target x86_64-pc-windows-msvc
```

构建 Windows 版本：

```bash
cargo build --release --target x86_64-pc-windows-msvc
```

输出文件通常位于：

```text
target\x86_64-pc-windows-msvc\release\singboost.exe
```

## 平台说明

SingBoost 仅面向 Windows。非 Windows 平台可运行核心测试，但应用本体不会提供托盘功能。

## 非目标

SingBoost 不做以下事情：

- 不生成 sing-box 配置。
- 不编辑 `config.json`。
- 不管理订阅。
- 不解析节点。
- 不替代 Clash API UI。
- 不内置 sing-box 或 wintun.dll。
