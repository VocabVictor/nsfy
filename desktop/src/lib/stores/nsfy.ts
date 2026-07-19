import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { normalizeServerUrl } from '../server-url';
import {
  forgetDismissed, isMessageDismissed, isMessagePurged, isMessageRead, rememberDismissed,
  rememberRead, rememberReadSoon, type MessageRef,
} from './message-state';
import { initializeMessageState, moveToTrash, takeFromTrash, trashContains } from './trash-store';
import {
  dndAllowedPriorities, doNotDisturb, layoutMode, notificationMode, popupPosition, windowBehavior,
  setPreferencePersistence,
} from './preferences';
export { normalizeServerUrl } from '../server-url';
export * from '../message-format';
export { messageKey, type MessageRef, type TrashMessage } from './message-state';
export { discardTrash, emptyTrash, trash, trashRef } from './trash-store';
export * from './preferences';

export interface Message {
  id: string;
  time: number;
  title: string;
  message: string;
  priority: number;
  tags: string[];
  category: string[];
  popup: boolean;
  bypassDnd: boolean;
  read: boolean;
}

export interface Topic {
  name: string;
  server: string;
  messages: Message[];
  unread: number;
  connected: boolean;
}

export interface Server {
  url: string;
  name: string;
  // Optional auth token for servers started with --auth-token.
  token?: string;
}

export const servers = writable<Server[]>([]);
export const topics = writable<Topic[]>([]);
export const activeTopic = writable<string | null>(null);
export const activeTab = writable<'topics' | 'publish' | 'settings'>('topics');

export async function loadState() {
  initializeMessageState();
  const raw = localStorage.getItem('nsfy-state');
  let localData: any = null;
  if (raw) {
    try {
      localData = JSON.parse(raw);
    } catch {}
  }

  let data = localData;
  try {
    const shared = await invoke<any | null>('load_shared_config');
    if (shared) data = shared;
  } catch {}

  if (data?.servers) servers.set(data.servers);
  if (data?.topics) {
    topics.set(data.topics.map((t: any) => ({
      ...t, connected: false, unread: 0, messages: [],
    })));
  }
  if (['silent', 'system', 'temporary', 'persistent'].includes(data?.notificationMode)) {
    notificationMode.set(data.notificationMode);
  } else if (data?.popupOnNotify === true) {
    notificationMode.set('temporary');
  }
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
  if (get(servers).length === 0) {
    servers.set([{ url: 'http://localhost:8080', name: 'Local' }]);
  }
  if (!data || data === localData) persistState();
}

export function persistState() {
  const s = get(servers);
  const t = get(topics);
  const state = {
    servers: s,
    topics: t.map(t => ({ name: t.name, server: t.server, unread: t.unread })),
    notificationMode: get(notificationMode),
    popupOnNotify: ['temporary', 'persistent'].includes(get(notificationMode)),
    popupPosition: get(popupPosition),
    layoutMode: get(layoutMode),
    windowBehavior: get(windowBehavior),
    doNotDisturb: get(doNotDisturb),
    dndAllowedPriorities: get(dndAllowedPriorities),
  };
  localStorage.setItem('nsfy-state', JSON.stringify(state));
  invoke('save_shared_config', { config: state }).catch(() => {});
}

setPreferencePersistence(persistState);
export function addTopic(server: string, name: string) {
  topics.update(ts => {
    if (ts.find(t => t.server === server && t.name === name)) return ts;
    return [...ts, { name, server, messages: [], unread: 0, connected: false }];
  });
  persistState();
}

export function removeTopic(server: string, name: string) {
  const topic = get(topics).find(t => t.server === server && t.name === name);
  if (topic) rememberDismissed(topic.messages.map(m => ({ server, topic: name, id: m.id })));
  topics.update(ts => ts.filter(t => !(t.server === server && t.name === name)));
  if (get(activeTopic) === name) activeTopic.set(null);
  persistState();
}

export function addMessage(topicName: string, server: string, msg: Message) {
  const ref = { server, topic: topicName, id: msg.id };
  if (isMessageDismissed(ref)) {
    if (!isMessagePurged(ref) && !trashContains(ref)) moveToTrash([{
      ...msg, read: true, server, topicName, deletedAt: Date.now(),
    }]);
    return;
  }
  if (get(activeTopic) === topicName && !isMessageRead(ref)) rememberReadSoon(ref);
  topics.update(ts => ts.map(t => {
    if (t.server !== server || t.name !== topicName) return t;
    if (t.messages.find(m => m.id === msg.id)) return t;
    const read = isMessageRead(ref);
    const msgs = [...t.messages, { ...msg, read }];
    if (msgs.length > 500) msgs.splice(0, msgs.length - 500);
    const unread = read ? t.unread : t.unread + 1;
    return { ...t, messages: msgs, unread };
  }));
}

export function markRead(topicName: string, server?: string) {
  const matches = (t: Topic) => t.name === topicName && (!server || t.server === server);
  const refs = get(topics).filter(matches).flatMap(t =>
    t.messages.filter(m => !m.read).map(m => ({ server: t.server, topic: t.name, id: m.id }))
  );
  if (!refs.length && !get(topics).some(t => matches(t) && t.unread !== 0)) return;
  rememberRead(refs);
  topics.update(ts => ts.map(t => matches(t)
    ? { ...t, unread: 0, messages: t.messages.map(m => ({ ...m, read: true })) }
    : t
  ));
  persistState();
}

export function markAllRead() {
  if (!get(topics).some(t => t.unread !== 0)) return;
  rememberRead(get(topics).flatMap(t =>
    t.messages.filter(m => !m.read).map(m => ({ server: t.server, topic: t.name, id: m.id }))
  ));
  topics.update(ts => ts.map(t =>
    t.unread === 0 ? t : { ...t, unread: 0, messages: t.messages.map(m => ({ ...m, read: true })) }
  ));
  persistState();
}

export function markMessagesRead(refs: MessageRef[]) {
  const selected = new Set(refs.map(ref => `${ref.server}\n${ref.topic}\n${ref.id}`));
  rememberRead(refs);
  topics.update(ts => ts.map(t => {
    let changed = 0;
    const messages = t.messages.map(m => {
      const chosen = selected.has(`${t.server}\n${t.name}\n${m.id}`);
      if (!chosen || m.read) return m;
      changed++;
      return { ...m, read: true };
    });
    return changed ? { ...t, messages, unread: Math.max(0, t.unread - changed) } : t;
  }));
  persistState();
}

export function clearMessages(refs: MessageRef[]) {
  const selected = new Set(refs.map(ref => `${ref.server}\n${ref.topic}\n${ref.id}`));
  const deletedAt = Date.now();
  moveToTrash(get(topics).flatMap(t => t.messages
    .filter(m => selected.has(`${t.server}\n${t.name}\n${m.id}`))
    .map(m => ({ ...m, server: t.server, topicName: t.name, deletedAt }))
  ));
  rememberDismissed(refs);
  topics.update(ts => ts.map(t => {
    const removed = t.messages.filter(m => selected.has(`${t.server}\n${t.name}\n${m.id}`));
    if (!removed.length) return t;
    const unreadRemoved = removed.filter(m => !m.read).length;
    return {
      ...t,
      messages: t.messages.filter(m => !selected.has(`${t.server}\n${t.name}\n${m.id}`)),
      unread: Math.max(0, t.unread - unreadRemoved),
    };
  }));
  persistState();
}

export function clearTopicMessages(server: string, name: string) {
  const topic = get(topics).find(t => t.server === server && t.name === name);
  if (!topic) return;
  moveToTrash(topic.messages.map(m => ({
    ...m, server, topicName: name, deletedAt: Date.now(),
  })));
  rememberDismissed(topic.messages.map(m => ({ server, topic: name, id: m.id })));
  topics.update(ts => ts.map(t => t.server === server && t.name === name
    ? { ...t, messages: [], unread: 0 }
    : t
  ));
  persistState();
}

export function clearAllMessages() {
  const deletedAt = Date.now();
  const refs = get(topics).flatMap(t =>
    t.messages.map(m => ({ server: t.server, topic: t.name, id: m.id }))
  );
  moveToTrash(get(topics).flatMap(t => t.messages.map(m => ({
    ...m, server: t.server, topicName: t.name, deletedAt,
  }))));
  rememberDismissed(refs);
  topics.update(ts => ts.map(t => ({ ...t, messages: [], unread: 0 })));
  persistState();
}

export function restoreTrashMessages(refs: MessageRef[]) {
  const subscribed = get(topics);
  const restorable = refs.filter(ref =>
    subscribed.some(t => t.server === ref.server && t.name === ref.topic)
  );
  if (!restorable.length) return;
  const restored = takeFromTrash(restorable);
  forgetDismissed(restorable);
  rememberRead(restorable);
  topics.update(ts => ts.map(t => {
    const messages = restored.filter(m => m.server === t.server && m.topicName === t.name);
    if (!messages.length) return t;
    const existing = new Set(t.messages.map(m => m.id));
    const merged = [...t.messages, ...messages.filter(m => !existing.has(m.id)).map(m => ({
      id: m.id, time: m.time, title: m.title, message: m.message,
      priority: m.priority, tags: m.tags, category: m.category,
      popup: m.popup, bypassDnd: m.bypassDnd, read: true,
    }))].sort((a, b) => a.time - b.time).slice(-500);
    return { ...t, messages: merged };
  }));
}

export function setConnected(topicName: string, server: string, connected: boolean) {
  topics.update(ts => ts.map(t =>
    t.server === server && t.name === topicName ? { ...t, connected } : t
  ));
}

export function addServer(url: string, name: string, token?: string) {
  url = normalizeServerUrl(url);
  servers.update(s => {
    if (s.find(x => x.url === url)) return s;
    return [...s, { url, name, token: token || undefined }];
  });
  persistState();
}

export function setServerToken(url: string, token: string) {
  normalizeServerUrl(url);
  servers.update(s => s.map(x =>
    x.url === url ? { ...x, token: token || undefined } : x
  ));
  persistState();
}

export function serverToken(url: string): string | undefined {
  return get(servers).find(s => s.url === url)?.token;
}

// Credentials belong in the Authorization header, never in URLs where
// proxies, browser history, and diagnostics may record them.
export function authHeaders(serverUrl: string): Record<string, string> {
  normalizeServerUrl(serverUrl);
  const token = serverToken(serverUrl);
  return token ? { Authorization: `Bearer ${token}` } : {};
}

export function removeServer(url: string) {
  servers.update(s => s.filter(x => x.url !== url));
  topics.update(ts => ts.filter(t => t.server !== url));
  persistState();
}
