import { NavLink, Outlet } from "react-router-dom";
import { useApp } from "./context";

export function Layout() {
  const { settingsLoadNotice } = useApp();
  return (
    <div className="app-shell">
      {settingsLoadNotice ? (
        <div
          className="msg"
          style={{
            margin: 0,
            padding: "0.5rem 1.25rem",
            background: "#3a2f1a",
            color: "#ffd89a",
            borderBottom: "1px solid #5a4a2a",
            fontSize: "0.85rem",
          }}
        >
          {settingsLoadNotice}
        </div>
      ) : null}
      <header className="top-nav">
        <h1>CPN Guide — CIR / CVN / LLM</h1>
        <nav>
          <NavLink end to="/" className={({ isActive }) => (isActive ? "active" : "")}>
            工作台
          </NavLink>
          <NavLink to="/analyze" className={({ isActive }) => (isActive ? "active" : "")}>
            CVN 分析
          </NavLink>
          <NavLink to="/settings" className={({ isActive }) => (isActive ? "active" : "")}>
            设置
          </NavLink>
        </nav>
      </header>
      <Outlet />
    </div>
  );
}
