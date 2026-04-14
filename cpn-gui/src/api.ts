import { invoke } from "@tauri-apps/api/core";
import type {
  AnalyzeResponse,
  GenerationResult,
  GuiSettings,
  RepairResponse,
  TranslateCirResponse,
} from "./types";

export async function getSettings(): Promise<GuiSettings> {
  return invoke("get_settings");
}

export async function setSettings(s: GuiSettings): Promise<void> {
  return invoke("set_settings", { value: s });
}

export async function resetSettings(): Promise<GuiSettings> {
  return invoke("reset_settings");
}

export async function pickLlmConfigFile(): Promise<string | null> {
  return invoke("pick_llm_config_file");
}

export async function validateCir(cirJson: string): Promise<unknown> {
  return invoke("validate_cir", { cirJson });
}

export async function translateCir(
  cirJson: string,
): Promise<TranslateCirResponse> {
  return invoke("translate_cir", { cirJson });
}

export async function analyzeCvn(
  cirJson: string,
  strategy: string,
  maxStates: number,
): Promise<AnalyzeResponse> {
  return invoke("analyze_cvn", { cirJson, strategy, maxStates });
}

export async function repairCir(
  cirJson: string,
  settings: GuiSettings,
): Promise<RepairResponse> {
  return invoke("repair_cir", { cirJson, settings });
}

export async function generateCirNl(
  requirements: string,
  settings: GuiSettings,
): Promise<GenerationResult> {
  return invoke("generate_cir_nl", { requirements, settings });
}
