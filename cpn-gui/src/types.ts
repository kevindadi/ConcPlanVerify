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
    candidateJson: string | null;
    parseError: string | null;
    validationMessages: string[];
    verificationStatus: VerificationStatus | null;
    verificationMessages: string[];
    stateCount: number;
    analysisComplete: boolean;
    accepted: boolean;
    durationMs: number;
  }>;
};

export type VerificationStatus =
  | "invalid_model"
  | "translation_failed"
  | "analysis_incomplete"
  | "verified_safe"
  | "verified_unsafe"
  | "goals_unmet";

export type VerificationResult = {
  status: VerificationStatus;
  validation: {
    valid: boolean;
    diagnostics: Array<{
      code: string;
      severity: "error" | "warning";
      message: string;
      path?: string;
      fixHint?: string;
    }>;
  };
  translationErrors: string[];
  translationWarnings: string[];
  places: number;
  transitions: number;
  cvnDot: string | null;
  stateCount: number;
  analysisComplete: boolean;
  analysisError: string | null;
  bugs: BugReport[];
  unmetGoals: UnmetGoal[];
  goalWarnings: string[];
  declaredGoalCount: number;
  timings: {
    validationMs: number;
    translationMs: number;
    analysisMs: number;
    goalsMs: number;
    totalMs: number;
  };
};

export type BugReport = {
  kind: Record<string, unknown>;
  trace: Array<{
    transitionId: string;
    kind: string | Record<string, unknown>;
    anchorSids: string[];
    description: string;
  }>;
  finalMarkingSummary: string;
  summary: string;
  involvedResources: string[];
  involvedFunctions: string[];
  cirSlice: Array<{ sid: string; op: string; function: string }>;
  preservationConstraints: string[];
  repairHint: string | null;
};

export type UnmetGoal = {
  goal: {
    id: string;
    desc: string | null;
    predicates: Array<Record<string, unknown>>;
  };
  reason: string;
};

export type TranslateCirResponse = {
  success: boolean;
  cvnDot?: string;
  translateWarnings?: string[];
  errors?: string[];
};

export type AnalyzeResponse = {
  status: VerificationStatus;
  validation: VerificationResult["validation"];
  translationErrors: string[];
  translationWarnings: string[];
  places: number;
  transitions: number;
  cvnDot: string | null;
  stateCount: number;
  analysisComplete: boolean;
  analysisError: string | null;
  bugs: BugReport[];
  unmetGoals: UnmetGoal[];
  goalWarnings: string[];
  declaredGoalCount: number;
  timings: VerificationResult["timings"];
};

export type RepairResponse = {
  status: string;
  rounds: number;
  cirJson: string | null;
  lastReport: string | null;
  history: Array<{
    round: number;
    candidateCirJson: string | null;
    parseError: string | null;
    verification: VerificationResult | null;
    accepted: boolean;
    rejectionReason: string | null;
    durationMs: number;
  }>;
};

export type LlmConfigStatus = {
  path: string;
  exists: boolean;
  valid: boolean;
  error: string | null;
};
