import { normalizeServerUrl } from '../server-url';
import { applyRemoteState } from './message-actions';
import { authHeaders, serverToken } from './state-persistence';
import type { SyncedChange, SyncedStatus } from './state-events';
import { dedupeStateChanges, groupStateChanges } from '../state-sync-wire';
import { connectSocket, type SocketConnection } from '../socket-client';
import { advancedPreferences } from './preferences';
import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

interface WireChange {
  id: string;
  status: SyncedStatus;
  updated_at?: number;
}

interface WireEvent {
  type: 'snapshot' | 'state';
  updates: WireChange[];
}

const QUEUE_KEY = 'nsfy-state-sync-queue';
let queue: SyncedChange[] = [];
let loaded = false;
let flushTimer: ReturnType<typeof setTimeout> | null = null;

function ensureLoaded() {
  if (loaded) return;
  loaded = true;
  try {
    const saved = JSON.parse(localStorage.getItem(QUEUE_KEY) || '[]');
    if (Array.isArray(saved)) queue = saved.slice(-5000);
  } catch {}
}

function saveQueue() {
  localStorage.setItem(QUEUE_KEY, JSON.stringify(queue.slice(-5000)));
}

export function queueStateUpdates(changes: SyncedChange[]) {
  ensureLoaded();
  queue = dedupeStateChanges([...queue, ...changes]);
  saveQueue();
  if (!flushTimer) flushTimer = setTimeout(() => {
    flushTimer = null;
    void flushStateQueue();
  }, 100);
}

export async function flushStateQueue(server?: string, topic?: string) {
  ensureLoaded();
  const selected = queue.filter(change => {
    if (server && change.server !== server) return false;
    if (topic && change.topic !== topic) return false;
    return true;
  });
  for (const changes of groupStateChanges(selected)) {
    const first = changes[0];
    try {
      const updates = changes.slice(0, 500).map(change => ({ id: change.id, status: change.status }));
      const direct = get(advancedPreferences).proxyMode === 'direct';
      const ok = direct
        ? await invoke<boolean>('post_state_direct', {
            server: first.server, topic: first.topic, updates,
          })
        : (await fetch(`${normalizeServerUrl(first.server)}/${first.topic}/state`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', ...authHeaders(first.server) },
            body: JSON.stringify({ updates }),
          })).ok;
      if (!ok) continue;
      const sent = new Set(changes.slice(0, 500).map(change =>
        `${change.server}\n${change.topic}\n${change.id}\n${change.status}`));
      queue = queue.filter(change => !sent.has(
        `${change.server}\n${change.topic}\n${change.id}\n${change.status}`));
      saveQueue();
      if (changes.length > 500) void flushStateQueue(first.server, first.topic);
    } catch {}
  }
}

export function connectStateSocket(
  server: string,
  topic: string,
  onClose: () => void,
): Promise<SocketConnection> {
  const url = normalizeServerUrl(server).replace(/^http/, 'ws') + `/${topic}/state/ws`;
  return connectSocket(
    url, get(advancedPreferences).proxyMode === 'direct', serverToken(server),
    {
      open: () => void flushStateQueue(server, topic),
      message: text => {
        try {
          const payload = JSON.parse(text) as WireEvent;
          if (!Array.isArray(payload.updates)) return;
          applyRemoteState(payload.updates.map(update => ({
            server, topic, id: update.id, status: update.status, updatedAt: update.updated_at,
          })));
        } catch {}
      },
      close: onClose,
    },
  );
}
