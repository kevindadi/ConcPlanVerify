import { useEffect, useRef, useState } from "react";
import { useApp } from "../context";
import * as api from "../api";
import type { AnalyzeResponse } from "../types";

export function Analyze() {
  const { settings, cirJson, setBusy, busy } = useApp();
  const [strategy, setStrategy] = useState(settings.analysisStrategy);
  const [maxStates, setMaxStates] = useState(settings.analysisMaxStates);
  const [renderDot, setRenderDot] = useState(settings.renderDotPreview);
  const [result, setResult] = useState<AnalyzeResponse | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const dotHost = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setStrategy(settings.analysisStrategy);
    setMaxStates(settings.analysisMaxStates);
    setRenderDot(settings.renderDotPreview);
  }, [settings]);

  useEffect(() => {
    const host = dotHost.current;
    if (!host || !result?.cvnDot || !renderDot) {
      if (host) host.innerHTML = "";
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const { instance } = await import("@viz-js/viz");
        const viz = await instance();
        if (cancelled) return;
        const el = viz.renderSVGElement(result.cvnDot);
        host.innerHTML = "";
        host.appendChild(el);
      } catch {
        if (!cancelled && host) {
          host.innerHTML = "";
          const pre = document.createElement("pre");
          pre.className = "trace";
          pre.textContent = result.cvnDot;
          host.appendChild(pre);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [result, renderDot]);

  async function run() {
    setErr(null);
    setResult(null);
    setBusy(true);
    try {
      const r = await api.analyzeCvn(cirJson, strategy, maxStates);
      setResult(r);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  function downloadDot() {
    if (!result?.cvnDot) return;
    const blob = new Blob([result.cvnDot], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "cvn.dot";
    a.click();
    URL.revokeObjectURL(url);
  }

  return (
    <main className="page">
      <section className="panel">
        <h2>分析参数</h2>
        <div className="row">
          <label>
            策略{" "}
            <select
              value={strategy}
              onChange={(e) => setStrategy(e.target.value)}
            >
              <option value="bfs">BFS</option>
              <option value="dfs">DFS</option>
            </select>
          </label>
          <label>
            max_states{" "}
            <input
              type="number"
              min={100}
              step={100}
              value={maxStates}
              onChange={(e) => setMaxStates(Number(e.target.value))}
              style={{ width: 120 }}
            />
          </label>
          <label>
            <input
              type="checkbox"
              checked={renderDot}
              onChange={(e) => setRenderDot(e.target.checked)}
            />{" "}
            预览 CVN 图（Graphviz WASM）
          </label>
          <button type="button" className="primary" disabled={busy} onClick={() => void run()}>
            翻译并分析
          </button>
        </div>
        <p className="msg" style={{ color: "#9aa3b2" }}>
          使用工作台中的 CIR JSON；本页独立运行状态空间探索。
        </p>
      </section>

      {err ? <p className="msg error">{err}</p> : null}

      {result ? (
        <>
          <section className="panel">
            <h2>结果摘要</h2>
            <p className="msg">
              状态数: {result.stateCount}，死锁反例数: {result.deadlockCount}
            </p>
            {result.exploreError ? (
              <p className="msg error">探索中止: {result.exploreError}</p>
            ) : null}
            {result.translateWarnings.length > 0 ? (
              <pre className="trace">
                翻译层警告:
                {"\n"}
                {result.translateWarnings.join("\n")}
              </pre>
            ) : null}
            <div className="row">
              <button type="button" onClick={downloadDot}>
                下载 cvn.dot
              </button>
            </div>
          </section>

          {result.deadlocks.length > 0 ? (
            <section className="panel">
              <h2>死锁反例</h2>
              {result.deadlocks.map((d, i) => (
                <div key={i} style={{ marginBottom: "1rem" }}>
                  <strong>
                    #{i + 1} {d.kind}
                  </strong>
                  <span style={{ marginLeft: 8, color: "#9aa3b2" }}>
                    trace 长度 {d.traceLen}
                  </span>
                  <pre className="trace">{JSON.stringify(d.trace, null, 2)}</pre>
                </div>
              ))}
            </section>
          ) : null}

          {renderDot ? (
            <section className="panel">
              <h2>CVN 结构（DOT）</h2>
              <div ref={dotHost} className="dot-preview" />
            </section>
          ) : null}
        </>
      ) : null}
    </main>
  );
}
