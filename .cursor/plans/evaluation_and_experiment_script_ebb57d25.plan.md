---
name: Evaluation and Experiment Script
overview: 基于 4 个 RQ 扩充论文实验部分(含占位表格),创建 Python 实验编排脚本与 LLM 配置文件,最后将 main.tex 按章节拆分为多个 .tex 文件.
todos:
  - id: expand-evaluation
    content: 重写并扩充 Evaluation 章节:RQ 定义、RQ1-RQ3 实验设计及表格(随机数据)、RQ4 Discussion、保留原有 Test Matrix 和 Pattern Descriptions
    status: completed
  - id: create-experiment-config
    content: 创建 experiments/config.toml 配置文件模板
    status: completed
  - id: create-experiment-script
    content: 创建 experiments/run_experiment.py 实验编排脚本
    status: completed
  - id: split-tex
    content: 将 main.tex 按章节拆分为 9 个 .tex 文件,main.tex 用 \input 引用
    status: completed
isProject: false
---

# 扩充实验章节、创建实验脚本、拆分论文

## 1. 论文实验部分重构 (Section 7 Evaluation)

当前 `paper/main.tex` 的 Evaluation 部分(约 line 2009-2221)只有一个 Test Matrix + Pattern Descriptions + Results 表格.需要基于以下 RQ 重新组织,并在已有内容基础上大幅扩充.

### RQ 定义

- **RQ1 (CIR Generation)**: 哪些 LLM 能生成语法正确的 CIR？需要几轮迭代？
- **RQ2 (Bug Detection & Repair)**: CVN 能否检测 CIR 中的 bug？CVN 规模、状态数、耗时如何？几轮修复可成功？修复过程是否引入新 bug？
- **RQ3 (Translation Correctness)**: CIR 到 CVN 翻译的正确性如何保证？(结合已有的 step correspondence theorem 和结构校验)
- **RQ4 (Code Faithfulness)**: 如何证明生成的代码遵循 CIR 语义？(Discussion 性质,无法形式化证明,cargo check 通过即认为可接受)

### 新增表格设计

**Table: RQ1 - CIR Generation Iterations by Model**

- 行:10 个并发模式
- 列:5 个模型 (GPT-4o, Claude 3.5 Sonnet, DeepSeek-V3, GPT-4o-mini, Gemini 1.5 Pro)
- 单元格:达到无语法错误 CIR 所需迭代次数(1/2/3/X 表示失败)
- 汇总行:成功率、平均迭代次数

**Table: RQ2 - CVN Bug Detection and Repair**

- 行:7 个 buggy 模式 x 5 个模型
- 列:CVN Places / CVN Transitions / 状态数 / 分析耗时(ms) / Bug 检出 / 修复迭代数 / 新 Bug 数
- 汇总:检出率、平均修复轮次、回归率

**Table: RQ3 - Translation Structural Invariants**

- 行:10 个模式
- 列:CIR 语句数 / CVN Places / CVN Transitions / 结构校验通过 / 翻译错误数

### 修改位置

在 `[paper/main.tex](paper/main.tex)` 中,替换当前 `\section{Evaluation}` (line 2009) 到 `\section{Related Work}` (line 2223) 之间的内容,保留 Test Matrix 和 Pattern Descriptions,在其后新增 RQ1-RQ3 小节和表格.RQ4 作为 Discussion 小节加在 RQ3 之后.

## 2. 实验脚本

创建 `experiments/` 目录,包含:

- `**experiments/config.toml` - LLM provider 配置文件(用户填写 API key)
- `**experiments/run_experiment.py` - 主实验编排脚本

### 配置文件格式 (`config.toml`)

```toml
[experiment]
max_gen_rounds = 5       # RQ1: CIR generation max iterations
max_repair_rounds = 5    # RQ2: repair loop max iterations
patterns_dir = "tests/e2e"
output_dir = "experiments/results"

[[models]]
name = "gpt-4o"
provider = "openai"
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com/v1"

[[models]]
name = "claude-3-5-sonnet"
provider = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"
base_url = "https://api.anthropic.com/v1"
# ... more models
```

### 脚本流程

1. 读取 `config.toml`,遍历每个模型
2. **RQ1 实验**: 对每个模式的源代码,调用 LLM 生成 CIR,运行 `cargo test` 做静态检查,记录迭代次数
3. **RQ2 实验**: 对每个 buggy CIR,translate → explore → analyze,记录 CVN 规模/状态/耗时/bug 类型;然后调用 LLM 修复,循环记录
4. 输出 CSV/JSON 结果文件

脚本调用 Rust 工具链(通过 `cargo run` 或编译后的二进制)完成 CIR 校验、翻译和分析.LLM 调用通过 Python `requests` 库直接完成.

## 3. 拆分 main.tex

将 `[paper/main.tex](paper/main.tex)` 拆分为以下文件:

| 新文件                                  | 对应章节                      | 原始行号范围 |
| --------------------------------------- | ----------------------------- | ------------ |
| `paper/sections/introduction.tex`       | Section 1 Introduction        | ~75-191      |
| `paper/sections/motivation.tex`         | Section 2 Motivation          | ~193-298     |
| `paper/sections/architecture.tex`       | Section 3 System Architecture | ~300-419     |
| `paper/sections/formal-definitions.tex` | Section 4 Formal Definitions  | ~421-1225    |
| `paper/sections/analysis-repair.tex`    | Section 5 Analysis and Repair | ~1227-1503   |
| `paper/sections/properties.tex`         | Section 6 Formal Properties   | ~1505-2005   |
| `paper/sections/evaluation.tex`         | Section 7 Evaluation (扩充后) | ~2009-?      |
| `paper/sections/related.tex`            | Section 8 Related Work        | 占位         |
| `paper/sections/conclusion.tex`         | Section 9 Conclusion          | 占位         |

`paper/main.tex` 保留 preamble(包、宏定义)、`\begin{document}`、title/author/abstract/keywords/maketitle,然后用 `\input{sections/xxx}` 引入各章节,最后 `\end{document}`.
