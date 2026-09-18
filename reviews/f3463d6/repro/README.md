# 本轮反例

`probe.py` 使用 Python 标准库调用后端 CLI。设置 `CONCIR_REVIEW_BIN` 指向要审阅的二进制即可运行。脚本会在自身目录重写 JSON 和运行输出；请先复制到临时目录，保留本审阅记录。

`rendezvous_probe.rs` 使用公开的两套引擎接口，从初始状态 BFS 到两个 receiver 等待的可达状态，并输出该状态的后继数。实际结果保存在上一级 `rendezvous-probe-summary.txt`，两引擎均为 1；默认无等待线程 FIFO 保证的语义应允许 2 个匹配。

在临时目录建立 Cargo 项目，将该 Rust 文件复制为 `src/main.rs`，使用以下 Cargo.toml：

```toml
[package]
name = "concir-audit-rendezvous"
version = "0.1.0"
edition = "2021"

[dependencies]
concir = { path = "/Users/kevin/local-repos/ConcIR" }
serde_json = "1"
```

实际执行的同类命令：

```sh
env CARGO_TARGET_DIR=/private/tmp/concir-audit-round2-target cargo run --offline --quiet --manifest-path /private/tmp/concir-audit-round2/rendezvous-crate/Cargo.toml -- /private/tmp/concir-audit-round2/r2_rendezvous_two_waiters.json
```

复现时按临时目录修改 manifest 和模型路径。不要把观察到的错误 PASS／repaired／单分支结果写成正确性测试的预期。
