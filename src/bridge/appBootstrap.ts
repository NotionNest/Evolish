import { invoke } from "@tauri-apps/api/core";

import type { AppBootstrapDto } from "./generated/AppBootstrapDto";
import { IpcError, normalizeIpcError } from "./errors";

export type AppBootstrap = AppBootstrapDto;

export async function getAppBootstrap(): Promise<AppBootstrapDto> {
  try {
    return await invoke<AppBootstrapDto>("app_get_bootstrap");
  } catch (error: unknown) {
    throw new IpcError(normalizeIpcError(error));
  }
}
