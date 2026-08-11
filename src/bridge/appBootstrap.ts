import { invoke } from "@tauri-apps/api/core";

export type AppBootstrap = {
  name: string;
  version: string;
  platform: string;
  architecture: string;
};

export function getAppBootstrap(): Promise<AppBootstrap> {
  return invoke<AppBootstrap>("get_app_bootstrap");
}
