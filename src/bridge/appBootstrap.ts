import { invoke } from "@tauri-apps/api/core";

import type { AppBootstrapDto } from "./generated/AppBootstrapDto";

export type AppBootstrap = AppBootstrapDto;

export function getAppBootstrap(): Promise<AppBootstrapDto> {
  return invoke<AppBootstrapDto>("app_get_bootstrap");
}
