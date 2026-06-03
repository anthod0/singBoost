# SingBoost

SingBoost 是一个纯粹的 windows sing-box 内核启动器，提供托盘图标、日志查看和开机自启功能。

## 功能

- 启动/停止 sing-box 内核
- 查看运行日志
- Windows 系统托盘图标
- 设置开机自启

## 非目标

- 不生成、修改 sing-box 配置
- 不解析节点
- 不内置 sing-box 内核

## 前置条件

- 自行获取完整的 sing-box 配置。
- sing-box.exe
- wintun.dll（如果使用 tun 模式）。

## 目标目录结构

将 `singboost.exe` 放到 sing-box 所在目录：

```text
<app_dir>\
  singboost.exe
  sing-box.exe
  wintun.dll
  config.json
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
start_command = 'sing-box.exe -D . -c config.json run'
```

说明：

- `run_as_admin = true` 时，SingBoost 启动后会通过 UAC 重新拉起自身。
- `start_command` 可以由用户自定义。
- 如果配置文件已存在但 TOML 无法解析、字段缺失或 `start_command` 为空，SingBoost 不会启动 sing-box。

## 托盘菜单

右键托盘图标可使用：

- `启动` / `停止`：启动或停止 sing-box。
- `重启`：仅在 sing-box 运行中可用。
- `打开 UI`：打开配置文件中的Web UI。
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

点击托盘菜单 `日志` 可以查看实时日志输出。

## 编译

### Windows 本机编译

开发构建：

```powershell
cargo build
```

Release 构建：

```powershell
cargo build --release
```

编译产物：

```text
target\debug\singboost.exe
target\release\singboost.exe
```

正式使用建议使用 `target\release\singboost.exe`。

### Linux/macOS 编译说明

项目的托盘功能仅支持 Windows。在非 Windows 平台直接编译时，只会得到一个提示程序，不是实际可用的 Windows 托盘启动器。

如需在 Linux 上交叉编译 Windows exe，可安装 Windows target 后构建：

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

产物路径：

```text
target/x86_64-pc-windows-gnu/release/singboost.exe
```

更推荐在 Windows 上编译和验证托盘、任务计划、管理员权限等行为。

