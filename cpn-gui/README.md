# CPN GUI（Tauri）

## 开发

在 **`cpn-gui` 目录内**：

```bash
npm install
cargo tauri dev
```

不要从本目录使用路径 `cpn-gui/src-tauri/Cargo.toml`（那是给**仓库根目录**用的相对路径）。

在 **仓库根目录**时（Tauri CLI 2.x 用 `-c` 指向配置，没有 `--manifest-path`）：

```bash
cargo tauri dev -c cpn-gui/src-tauri/tauri.conf.json
```

## 常见错误

- `unexpected argument '--manifest-path'`：当前 `cargo tauri dev` 不支持该 flag，请用上一节的 `-c .../tauri.conf.json`，或在 `cpn-gui/` 下直接 `cargo tauri dev`。

- `manifest path ... does not exist`：多为在 `cpn-gui/` 里误传了 `cpn-gui/src-tauri/Cargo.toml` 给 **cargo**（路径重复）。在 `cpn-gui/` 下请只运行 `cargo tauri dev`，不要带该路径。
