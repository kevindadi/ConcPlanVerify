import { useMemo, useState } from "react";
import ExperimentReport from "./ExperimentReport";
import { FilterContext } from "./filter";

const SECTIONS = [
  { id: "s1", label: "1 检出" },
  { id: "s2", label: "2 基线" },
  { id: "s3", label: "3 Goals" },
  { id: "s4", label: "4 修复" },
  { id: "s5", label: "5 生成" },
  { id: "s6", label: "6 规模" },
  { id: "s7", label: "7 Scaling" },
  { id: "s8", label: "8 判卷" },
  { id: "s9", label: "9 Codegen" },
  { id: "s10", label: "10 Goals约束" },
  { id: "s11", label: "11 结论" },
];

export default function App() {
  const [query, setQuery] = useState("");
  const [dark, setDark] = useState(false);

  const filter = useMemo(() => query.trim().toLowerCase(), [query]);

  return (
    <div className={dark ? "app dark" : "app"}>
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark">CVN</span>
          <div>
            <div className="brand-title">实验报告</div>
            <div className="brand-sub">可提交 · 本地交互 · Vite</div>
          </div>
        </div>
        <label className="search">
          <span>筛选表格</span>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="case 名关键字…"
          />
        </label>
        <button type="button" className="theme-btn" onClick={() => setDark((d) => !d)}>
          {dark ? "浅色" : "深色"}
        </button>
      </header>

      <nav className="toc" aria-label="章节导航">
        {SECTIONS.map((s) => (
          <a key={s.id} href={`#${s.id}`}>
            {s.label}
          </a>
        ))}
      </nav>

      <main className="main">
        <FilterContext.Provider value={filter}>
          <ExperimentReport />
        </FilterContext.Provider>
      </main>
    </div>
  );
}
