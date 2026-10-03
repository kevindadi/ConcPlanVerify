# 匿名投稿源码包

本分支保留 Python 编排代码、仓库内的 Rust `ConcIR/`、benchmark 输入、
重跑配置和脚本。没有保存历史实验结果、LLM 响应、审阅记录或 API key。
完整的评审使用说明见 [README.md](README.md)。

基本顺序：

1. 安装 Python 依赖和固定版本的 Rust 工具链，运行 `python reproduce.py build`。
2. G2 对照需要额外安装 Lockbud 和 Miri，具体命令见英文说明。
3. 复制 `.env.example` 为 `.env`，填写自己的 DeepSeek/OpenCode key。
4. 用 `plan` 查看规模，再用 `generate` 发起真实模型调用。
5. 用 `score` 对各方法最后产生的代码进行统一需求评分。

生成任务共 24 个，Simple、Medium、Complex 各 8 个。DeepSeek 的配置为
每方法 24 任务 × 3 重复；GPT/GLM 为每模型每方法 24 任务 × 1 次。
生成批次默认并发 3；同时启动多个批次会增加总并发。

所有新结果写入 `results/`，不会自动进入 Git。运行脚本会产生新的结果，
不能保证与论文的历史数值相同。模型接受、需求满足、未知检查结果和缺失
token 用量均分别记录。

投稿分支为 `artifact-anonymous`，只向匿名平台提供该分支。开发历史仍在
原 `main` 分支，不属于本源码包。
