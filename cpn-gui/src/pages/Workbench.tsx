import CodeMirror from "@uiw/react-codemirror";
import { json } from "@codemirror/lang-json";
import { useState } from "react";
import { useApp } from "../context";
import * as api from "../api";

export function Workbench() {
  const {
    settings,
    cirJson,
    setCirJson,
    busy,
    setBusy,
  } = useApp();
  const [requirements, setRequirements] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [validationText, setValidationText] = useState("");

  async function onGenerate() {
    setErr(null);
    setMsg(null);
    setBusy(true);
    try {
      const r = await api.generateCirNl(requirements, settings);
      setCirJson(r.cirJson);
      setMsg(
        `生成完成（共 ${r.rounds.length} 轮记录，见下方 rounds）。`,
      );
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onValidate() {
    setErr(null);
    setMsg(null);
    setBusy(true);
    try {
      const report = await api.validateCir(cirJson);
      setValidationText(JSON.stringify(report, null, 2));
      setMsg("已运行 CIR 校验。");
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onRepair() {
    setErr(null);
    setMsg(null);
    setBusy(true);
    try {
      const r = await api.repairCir(cirJson, settings);
      if (r.status === "fixed" && r.cirJson) {
        setCirJson(r.cirJson);
        setMsg(`修复成功，共 ${r.rounds} 轮。`);
      } else {
        setErr(
          r.lastReport ??
            `未修复（${r.rounds} 轮），status=${r.status}`,
        );
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  function downloadCir() {
    const blob = new Blob([cirJson], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "cir.json";
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <main className="page">
      <div className="grid-2">
        <section className="panel">
          <h2>自然语言需求</h2>
          <textarea
            className="raw"
            value={requirements}
            onChange={(e) => setRequirements(e.target.value)}
            placeholder="描述并发场景、线程、锁、通道等…"
          />
          <div className="row">
            <button
              type="button"
              className="primary"
              disabled={busy || !requirements.trim()}
              onClick={() => void onGenerate()}
            >
              用 LLM 生成 CIR
            </button>
          </div>
        </section>
        <section className="panel">
          <h2>校验报告</h2>
          <textarea
            className="raw"
            readOnly
            value={validationText || "（点击「校验 CIR」）"}
            style={{ minHeight: 200 }}
          />
          <div className="row">
            <button type="button" disabled={busy} onClick={() => void onValidate()}>
              校验 CIR
            </button>
            <button type="button" disabled={busy} onClick={() => void onRepair()}>
              LLM 修复（CVN 反例循环）
            </button>
            <button type="button" onClick={downloadCir}>
              导出 cir.json
            </button>
          </div>
        </section>
      </div>

      <section className="panel">
        <h2>CIR JSON</h2>
        <div style={{ border: "1px solid #2a2f3a", borderRadius: 6, overflow: "hidden" }}>
          <CodeMirror
            value={cirJson}
            height="360px"
            theme="dark"
            extensions={[json()]}
            onChange={(v) => setCirJson(v)}
          />
        </div>
      </section>

      {msg ? <p className="msg ok">{msg}</p> : null}
      {err ? <p className="msg error">{err}</p> : null}
    </main>
  );
}
