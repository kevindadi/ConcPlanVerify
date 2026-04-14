import { useRef } from "react";
import { useApp } from "../context";
import * as api from "../api";

export function Settings() {
  const fileRef = useRef<HTMLInputElement>(null);
  const {
    settings,
    setSettingsLocal,
    saveSettings,
    resetSettings,
    busy,
    setBusy,
  } = useApp();

  async function pickToml() {
    setBusy(true);
    try {
      const p = await api.pickLlmConfigFile();
      if (p) {
        setSettingsLocal({ ...settings, llmConfigPath: p });
      }
    } finally {
      setBusy(false);
    }
  }

  async function exportJson() {
    const blob = new Blob([JSON.stringify(settings, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "cpn-gui-settings.json";
    a.click();
    URL.revokeObjectURL(url);
  }

  function importJson(file: File) {
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const parsed = JSON.parse(String(reader.result)) as typeof settings;
        setSettingsLocal({ ...settings, ...parsed });
      } catch {
        alert("无效的 JSON");
      }
    };
    reader.readAsText(file);
  }

  return (
    <main className="page">
      <section className="panel">
        <h2>LLM（uni-llm）</h2>
        <div className="row" style={{ alignItems: "stretch", flexDirection: "column" }}>
          <label>配置文件路径（可用 ~ 表示家目录）</label>
          <div className="row">
            <input
              style={{ flex: 1, minWidth: 200 }}
              value={settings.llmConfigPath}
              onChange={(e) =>
                setSettingsLocal({ ...settings, llmConfigPath: e.target.value })
              }
            />
            <button type="button" disabled={busy} onClick={() => void pickToml()}>
              浏览…
            </button>
          </div>
        </div>
        <div className="row">
          <label>
            provider 覆盖{" "}
            <input
              value={settings.providerOverride ?? ""}
              placeholder="（留空=用 toml 默认）"
              onChange={(e) =>
                setSettingsLocal({
                  ...settings,
                  providerOverride: e.target.value || null,
                })
              }
            />
          </label>
        </div>
        <div className="row">
          <label>
            model 覆盖{" "}
            <input
              value={settings.modelOverride ?? ""}
              placeholder="（留空=用 toml 默认）"
              onChange={(e) =>
                setSettingsLocal({
                  ...settings,
                  modelOverride: e.target.value || null,
                })
              }
            />
          </label>
        </div>
      </section>

      <section className="panel">
        <h2>轮次与默认分析</h2>
        <div className="row">
          <label>
            NL 生成最大轮次{" "}
            <input
              type="number"
              min={1}
              max={64}
              value={settings.generationMaxRounds}
              onChange={(e) =>
                setSettingsLocal({
                  ...settings,
                  generationMaxRounds: Number(e.target.value),
                })
              }
            />
          </label>
          <label>
            修复最大轮次{" "}
            <input
              type="number"
              min={1}
              max={64}
              value={settings.repairMaxRounds}
              onChange={(e) =>
                setSettingsLocal({
                  ...settings,
                  repairMaxRounds: Number(e.target.value),
                })
              }
            />
          </label>
        </div>
        <div className="row">
          <label>
            默认搜索策略{" "}
            <select
              value={settings.analysisStrategy}
              onChange={(e) =>
                setSettingsLocal({ ...settings, analysisStrategy: e.target.value })
              }
            >
              <option value="bfs">bfs</option>
              <option value="dfs">dfs</option>
            </select>
          </label>
          <label>
            默认 max_states{" "}
            <input
              type="number"
              min={100}
              step={100}
              value={settings.analysisMaxStates}
              onChange={(e) =>
                setSettingsLocal({
                  ...settings,
                  analysisMaxStates: Number(e.target.value),
                })
              }
            />
          </label>
          <label>
            <input
              type="checkbox"
              checked={settings.renderDotPreview}
              onChange={(e) =>
                setSettingsLocal({
                  ...settings,
                  renderDotPreview: e.target.checked,
                })
              }
            />{" "}
            分析页默认开启 DOT 预览
          </label>
        </div>
      </section>

      <section className="panel">
        <h2>NL 系统提示词（可选）</h2>
        <p className="msg" style={{ color: "#9aa3b2" }}>
          留空则使用内置 `generation_nl_prompt.md`。可在此粘贴自定义 system prompt。
        </p>
        <textarea
          className="raw"
          style={{ minHeight: 220 }}
          value={settings.nlSystemPromptOverride ?? ""}
          placeholder="（内置默认）"
          onChange={(e) =>
            setSettingsLocal({
              ...settings,
              nlSystemPromptOverride: e.target.value.trim()
                ? e.target.value
                : null,
            })
          }
        />
      </section>

      <div className="row">
        <button type="button" className="primary" disabled={busy} onClick={() => void saveSettings()}>
          保存到应用配置目录
        </button>
        <button type="button" disabled={busy} onClick={() => void resetSettings()}>
          恢复默认
        </button>
        <button type="button" onClick={exportJson}>
          导出 JSON
        </button>
        <input
          ref={fileRef}
          type="file"
          accept="application/json,.json"
          style={{ display: "none" }}
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) importJson(f);
            e.target.value = "";
          }}
        />
        <button type="button" onClick={() => fileRef.current?.click()}>
          导入 JSON
        </button>
      </div>
    </main>
  );
}
