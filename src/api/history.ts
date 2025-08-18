import { invoke } from '@tauri-apps/api/core';

export type HistoryQueryParams = { from_ts: number; to_ts: number; limit?: number };

export async function historyQuery(params: HistoryQueryParams) {
  const { from_ts, to_ts, limit } = params;
  return await invoke('cmd_history_query', { fromTs: from_ts, toTs: to_ts, limit });
}

export async function stateLatest() {
  return await invoke('cmd_state_get_latest');
}
