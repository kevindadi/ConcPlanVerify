import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import type { GuiSettings } from "./types";
import { defaultGuiSettings } from "./types";
import * as api from "./api";

const DEFAULT_CIR = `{
  "program": "",
  "resources": [],
  "protection": [],
  "functions": [],
  "fn_summaries": [],
  "entry": "main"
}`;

type AppContextValue = {
  settings: GuiSettings;
  /** 使用内置默认设置时的说明（例如未在 Tauri 内运行） */
  settingsLoadNotice: string | null;
  setSettingsLocal: (s: GuiSettings) => void;
  saveSettings: () => Promise<void>;
  resetSettings: () => Promise<void>;
  reloadSettings: () => Promise<void>;
  cirJson: string;
  setCirJson: (s: string) => void;
  busy: boolean;
  setBusy: (b: boolean) => void;
};

const AppContext = createContext<AppContextValue | null>(null);

export function AppProvider({ children }: { children: ReactNode }) {
  const [settings, setSettingsLocal] = useState<GuiSettings | null>(null);
  const [settingsLoadNotice, setSettingsLoadNotice] = useState<string | null>(
    null,
  );
  const [cirJson, setCirJson] = useState(DEFAULT_CIR);
  const [busy, setBusy] = useState(false);

  const reloadSettings = useCallback(async () => {
    try {
      const s = await api.getSettings();
      setSettingsLocal(s);
      setSettingsLoadNotice(null);
    } catch (e) {
      console.warn("get_settings failed, using defaults", e);
      setSettingsLocal(defaultGuiSettings());
      setSettingsLoadNotice(
        "未能从 Tauri 读取设置（若你只用浏览器打开了 Vite 预览，这是正常现象）。已载入内置默认；完整功能请运行：cargo tauri dev --manifest-path cpn-gui/src-tauri/Cargo.toml",
      );
    }
  }, []);

  useEffect(() => {
    void reloadSettings();
  }, [reloadSettings]);

  const saveSettings = useCallback(async () => {
    if (!settings) return;
    await api.setSettings(settings);
    await reloadSettings();
  }, [settings, reloadSettings]);

  const resetSettings = useCallback(async () => {
    try {
      const s = await api.resetSettings();
      setSettingsLocal(s);
      setSettingsLoadNotice(null);
    } catch (e) {
      console.warn("reset_settings failed", e);
      setSettingsLocal(defaultGuiSettings());
      setSettingsLoadNotice(
        "无法调用 Tauri 重置设置，已恢复为内置默认（请使用 cargo tauri dev 运行桌面端）。",
      );
    }
  }, []);

  const value = useMemo<AppContextValue | null>(() => {
    if (!settings) return null;
    return {
      settings,
      settingsLoadNotice,
      setSettingsLocal,
      saveSettings,
      resetSettings,
      reloadSettings,
      cirJson,
      setCirJson,
      busy,
      setBusy,
    };
  }, [
    settings,
    settingsLoadNotice,
    saveSettings,
    resetSettings,
    reloadSettings,
    cirJson,
    busy,
  ]);

  if (!value) {
    return <div className="page">正在加载设置…</div>;
  }

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useApp(): AppContextValue {
  const v = useContext(AppContext);
  if (!v) throw new Error("useApp outside AppProvider");
  return v;
}
