import { invoke } from '@tauri-apps/api/core';
import { get } from 'svelte/store';
import {
  advancedPreferences, authHeaders, normalizeServerUrl,
} from './stores/nsfy';

interface DirectResponse {
  ok: boolean;
  status: number;
  body: string;
}

export async function postMessage(server: string, topic: string, body: object): Promise<void> {
  if (get(advancedPreferences).proxyMode === 'direct') {
    const response = await invoke<DirectResponse>('post_message_direct', { server, topic, body });
    if (!response.ok) throw new Error(`server returned ${response.status}: ${response.body}`);
    return;
  }
  const response = await fetch(`${normalizeServerUrl(server)}/${topic}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders(server) },
    body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error(`server returned ${response.status}`);
}
