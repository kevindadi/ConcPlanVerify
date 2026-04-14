export type GuiSettings = {
  llmConfigPath: string;
  providerOverride: string | null;
  modelOverride: string | null;
  generationMaxRounds: number;
  repairMaxRounds: number;
  nlSystemPromptOverride: string | null;
  analysisStrategy: string;
  analysisMaxStates: number;
  renderDotPreview: boolean;
};

/** 与 Rust `default_settings()` 一致；在无法 `invoke`（如仅用浏览器打开 Vite）时使用。 */
export function defaultGuiSettings(): GuiSettings {
  return {
    llmConfigPath: "uni-llm.toml",
    providerOverride: null,
    modelOverride: null,
    generationMaxRounds: 8,
    repairMaxRounds: 6,
    nlSystemPromptOverride: null,
    analysisStrategy: "bfs",
    analysisMaxStates: 100_000,
    renderDotPreview: true,
  };
}

export type GenerationResult = {
  cirJson: string;
  rounds: Array<{
    round: number;
    parseError: string | null;
    validationMessages: string[];
  }>;
};

export type TranslateCirResponse = {
  success: boolean;
  cvnDot?: string;
  translateWarnings?: string[];
  errors?: string[];
};

export type AnalyzeResponse = {
  translateWarnings: string[];
  cvnDot: string;
  stateCount: number;
  deadlockCount: number;
  deadlocks: Array<{
    kind: string;
    trace: Array<{ transitionId: string; anchorSids?: string[] }>;
    traceLen: number;
  }>;
  exploreError: string | null;
};

export type RepairResponse = {
  status: string;
  rounds: number;
  cirJson: string | null;
  lastReport: string | null;
};
