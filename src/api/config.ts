import { invoke } from '@tauri-apps/api/core';

export type AppConfig = {
  tray_bottom_mode?: string | null;
  tray_show_mem?: boolean;
  net_interfaces?: string[] | null;
  public_net_enabled?: boolean | null;
  public_net_api?: string | null;
  rtt_targets?: string[] | null;
  rtt_timeout_ms?: number | null;
  interval_ms?: number | null;
  pace_rtt_multi_every?: number | null;
  pace_net_if_every?: number | null;
  pace_logical_disk_every?: number | null;
  pace_smart_every?: number | null;
  top_n?: number | null;
};

export async function getConfig(): Promise<AppConfig> {
  return await invoke('get_config');
}

export async function setConfig(newCfg: AppConfig): Promise<void> {
  await invoke('set_config', { newCfg });
}

export async function cfgUpdate(patch: Partial<AppConfig>): Promise<AppConfig> {
  return await invoke('cmd_cfg_update', { patch });
}

export async function listNetInterfaces(): Promise<string[]> {
  try { return (await invoke('list_net_interfaces')) as string[]; } catch { return []; }
}
