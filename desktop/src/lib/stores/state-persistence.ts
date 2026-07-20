import { invoke } from '@tauri-apps/api/core';
import { get } from 'svelte/store';
import { normalizeServerUrl } from '../server-url';
import { activeTopic, servers, topics } from './core-stores';
import {
  advancedPreferences, defaultAdvancedPreferences, dndAllowedPriorities, doNotDisturb,
  layoutMode, notificationMode, popupPosition,
  setPreferencePersistence, windowBehavior,
} from './preferences';
import { initializeMessageState, pruneTrashOlderThan } from './trash-store';

export async function loadState() {
  initializeMessageState();
  const raw = localStorage.getItem('nsfy-state');
  let localData: any = null;
  if (raw) {
    try { localData = JSON.parse(raw); } catch {}
  }
  let data = localData;
  try {
    const shared = await invoke<any | null>('load_shared_config');
    if (shared) data = shared;
  } catch {}
  if (data?.servers) servers.set(data.servers);
  if (data?.topics) {
    topics.set(data.topics.map((topic: any) => ({
      ...topic, connected: false, unread: 0, messages: [],
    })));
  }
  if (['silent', 'system', 'temporary', 'persistent'].includes(data?.notificationMode)) {
    notificationMode.set(data.notificationMode);
  } else if (data?.popupOnNotify === true) notificationMode.set('temporary');
  if (typeof data?.popupPosition === 'string') popupPosition.set(data.popupPosition);
  if (data?.layoutMode === 'split' || data?.layoutMode === 'timeline') layoutMode.set(data.layoutMode);
  if (data?.windowBehavior === 'popup' || data?.windowBehavior === 'resident') {
    windowBehavior.set(data.windowBehavior);
  }
  if (typeof data?.doNotDisturb === 'boolean') doNotDisturb.set(data.doNotDisturb);
  if (Array.isArray(data?.dndAllowedPriorities)) {
    dndAllowedPriorities.set(data.dndAllowedPriorities.filter((value: unknown) =>
      Number.isInteger(value) && Number(value) >= 1 && Number(value) <= 5));
  }
  if (data?.advanced && typeof data.advanced === 'object') {
    advancedPreferences.set({ ...defaultAdvancedPreferences, ...data.advanced });
  }
  pruneTrashOlderThan(get(advancedPreferences).trashRetentionDays);
  if (get(servers).length === 0) servers.set([{ url: 'http://localhost:8080', name: 'Local' }]);
  if (!data || data === localData) persistState();
}

export function persistState() {
  const state = {
    servers: get(servers),
    topics: get(topics).map(topic => ({
      name: topic.name, server: topic.server, unread: topic.unread,
      lastConnectedAt: topic.lastConnectedAt,
    })),
    notificationMode: get(notificationMode),
    popupOnNotify: ['temporary', 'persistent'].includes(get(notificationMode)),
    popupPosition: get(popupPosition),
    layoutMode: get(layoutMode),
    windowBehavior: get(windowBehavior),
    doNotDisturb: get(doNotDisturb),
    dndAllowedPriorities: get(dndAllowedPriorities),
    advanced: get(advancedPreferences),
  };
  localStorage.setItem('nsfy-state', JSON.stringify(state));
  invoke('save_shared_config', { config: state }).catch(() => {});
}

setPreferencePersistence(persistState);

export function addServer(url: string, name: string, token?: string) {
  url = normalizeServerUrl(url);
  servers.update(items => items.some(item => item.url === url)
    ? items : [...items, { url, name, token: token || undefined }]);
  persistState();
}

export function setServerToken(url: string, token: string) {
  normalizeServerUrl(url);
  servers.update(items => items.map(item => item.url === url
    ? { ...item, token: token || undefined } : item));
  persistState();
}

export function serverToken(url: string): string | undefined {
  return get(servers).find(server => server.url === url)?.token;
}

export function authHeaders(url: string): Record<string, string> {
  normalizeServerUrl(url);
  const token = serverToken(url);
  return token ? { Authorization: `Bearer ${token}` } : {};
}

export function removeServer(url: string) {
  servers.update(items => items.filter(item => item.url !== url));
  topics.update(items => items.filter(topic => topic.server !== url));
  if (!get(topics).some(topic => topic.name === get(activeTopic))) activeTopic.set(null);
  persistState();
}
