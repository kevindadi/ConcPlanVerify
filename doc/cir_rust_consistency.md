# CIR ↔ 参考 Rust 一致性检查

benchmark 中每个 case 同时携带 CIR(`benchmarks/cir/`、`tests/e2e/`)与参考Rust(`benchmarks/rust/`),两者必须在**语句级**语义一致,否则外部基线(Miri / Lockbud,跑 Rust)与 CVN(跑 CIR)的对比失去意义。本文给出对照表与审计流程;同一张表也用于审计 codegen 产物(verified CIR → 生成 Rust)。

## 1. 构件对照表

| CIR 构件 | 参考 Rust 形态 | 一致性要求 |
| --- | --- | --- |
| `Mutex` 资源 | `Arc<Mutex<T>>`(跨线程)或 `Mutex<T>` | 每个 CIR Mutex 恰有一个 Rust Mutex 对象 |
| `Condvar` 资源 | `Arc<(Mutex<T>, Condvar)>` 或独立 `Condvar` | 与 CIR `wait` 声明的配对 mutex 一致 |
| `Channel` 资源 | `std::sync::mpsc::channel/sync_channel` | 容量语义一致(0 容量 ↔ rendezvous) |
| `Semaphore` 资源 | Mutex+Condvar 许可计数器 | 初始许可数与 CIR `init` 一致 |
| `Var`(受保护) | 锁内数据(`Mutex<T>` 的 `T`)或锁下访问的字段 | 只能在持有 protection 声明的锁时访问 |
| `Atomic` | `std::sync::atomic::AtomicX` | 不得降级为普通变量,也不得反向升级 |
| 非 main 函数(closure) | **一个独立的** `thread::spawn` 闭包 | 一个 CIR 函数 = 一个闭包定义(不得把同构 worker 折叠成参数化共享函数,见 §3) |
| `spawn f` / `join f` | `let h = thread::spawn(...)` / `h.join()` | 数量、顺序一致 |
| `res_op l lock` / `drop` | guard 作用域边界;提前释放用显式 `drop(guard)` | **获取顺序逐语句一致**(锁序类 case 的本体) |
| `res_op cv wait mtx` | `while !pred { guard = cv.wait(guard) }` | 循环重检查形态;配对 mutex 一致 |
| `notify` / `notify_all` | `cv.notify_one()` / `cv.notify_all()` | 单发/广播不可互换 |
| `send` / `recv` | `tx.send(..)` / `rx.recv()` | 阻塞语义一致 |
| `write v` / `read v` | 锁下赋值 / 读取 | 写入常量值一致(goals 依赖具体值) |
| `branch` transfer | `if`/`else`,同一条件、同一共享变量 | 分支结构 1:1,不得合并或消除 |
| `cas` | `compare_exchange`,对**返回值**分支 | true 臂 = CAS 成功臂(LLM judge 曾在此误读) |
| `goals` | 末尾 `assert!`(动态基线可见的近似) | assert 编码的谓词与 goal 谓词一致 |

## 2. 审计流程(每 case 一次)

1. **函数清单**:CIR `functions` 与 Rust 的 `main` + 各 spawn 闭包一一对应。
2. **语句走查**:按 sid 顺序在 Rust 中找到对应行,记录 `sid ↔ 行号`;
   重点核对每个函数内的**锁获取序列**(名称与顺序)与**分支条件**。
3. **资源清单**:CIR resources 与 Rust 对象一一对应,protection 关系体现为
   "数据在锁内"或注释声明。
4. **缺陷本体**:buggy 与 fixed 的差异必须恰好等于 manifest `notes` 描述的
   缺陷(例如 deep_lock_chain_4x3:仅 w3 else 臂的 m2/m1 顺序不同)。
5. **goals**:CIR goal 谓词与 Rust 末尾 assert(若有)一致。

## 3. 已知偏差类别

| 类别 | 判定 | 说明 |
| --- | --- | --- |
| codegen 把同构 worker 折叠为参数化共享函数 | ⚠ 语义等价但非语句级 1:1 | 已在 codegen 实验观察到;对照表第 7 行显式禁止,后续可作为 codegen prompt 约束或验收检查 |
| 修复实验中 `Var` → `Atomic` 漂移 | ✗ 违反资源对照 | repair oracle 不可见;局部重生成(`repair_local`)通过冻结非切片部分结构性消除 |
| 参考 Rust 用 `assert!` 近似 EF goal | ⚠ 接受 | 动态工具只能看终态,EF(某调度可达)无法直接断言;以注释注明 |
