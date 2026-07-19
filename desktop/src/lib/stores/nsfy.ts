// Svelte stores for nsfy desktop
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { normalizeServerUrl } from '../server-url';
export { normalizeServerUrl } from '../server-url';

// --- Types ---
export interface Message {
  id: string;
  time: number;
  title: string;
  message: string;
  priority: number;
  tags: string[];
  category: string[];
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

// --- Stores ---
export const servers = writable<Server[]>([]);
export const topics = writable<Topic[]>([]);
export const activeTopic = writable<string | null>(null);
export const activeTab = writable<'topics' | 'publish' | 'settings'>('topics');

export type PopupPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'center';

// When true, a priority>=4 message also shows a small macOS-style banner
// window at popupPosition, on top of everything, in addition to the
// background OS notification — for whoever wants it impossible to miss.
export const popupOnNotify = writable<boolean>(false);
export const popupPosition = writable<PopupPosition>('top-right');

// Layout direction: 1a 分栏排版 (sidebar topic list + message stream) vs
// 1b 统一时间线 (single inbox + date grouping, topic as label).
export type LayoutMode = 'split' | 'timeline';
export const layoutMode = writable<LayoutMode>('split');

// --- Persistence ---
export async function loadState() {
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
      ...t, connected: false, unread: t.unread || 0, messages: [],
    })));
  }
  if (typeof data?.popupOnNotify === 'boolean') popupOnNotify.set(data.popupOnNotify);
  if (typeof data?.popupPosition === 'string') popupPosition.set(data.popupPosition);
  if (data?.layoutMode === 'split' || data?.layoutMode === 'timeline') layoutMode.set(data.layoutMode);
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
    popupOnNotify: get(popupOnNotify),
    popupPosition: get(popupPosition),
    layoutMode: get(layoutMode),
  };
  localStorage.setItem('nsfy-state', JSON.stringify(state));
  invoke('save_shared_config', { config: state }).catch(() => {});
}

// --- Actions ---
export function addTopic(server: string, name: string) {
  topics.update(ts => {
    if (ts.find(t => t.server === server && t.name === name)) return ts;
    return [...ts, { name, server, messages: [], unread: 0, connected: false }];
  });
  persistState();
}

export function removeTopic(server: string, name: string) {
  topics.update(ts => ts.filter(t => !(t.server === server && t.name === name)));
  if (get(activeTopic) === name) activeTopic.set(null);
  persistState();
}

export function addMessage(topicName: string, server: string, msg: Message) {
  topics.update(ts => ts.map(t => {
    if (t.server !== server || t.name !== topicName) return t;
    if (t.messages.find(m => m.id === msg.id)) return t;
    const msgs = [...t.messages, msg];
    if (msgs.length > 500) msgs.splice(0, msgs.length - 500);
    const unread = get(activeTopic) === topicName ? t.unread : t.unread + 1;
    return { ...t, messages: msgs, unread };
  }));
}

export function markRead(topicName: string) {
  if (!get(topics).some(t => t.name === topicName && t.unread !== 0)) return;
  topics.update(ts => ts.map(t =>
    t.name === topicName ? { ...t, unread: 0 } : t
  ));
  persistState();
}

export function markAllRead() {
  if (!get(topics).some(t => t.unread !== 0)) return;
  topics.update(ts => ts.map(t =>
    t.unread === 0 ? t : { ...t, unread: 0 }
  ));
  persistState();
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

export function setPopupOnNotify(value: boolean) {
  popupOnNotify.set(value);
  persistState();
}

export function setPopupPosition(value: PopupPosition) {
  popupPosition.set(value);
  persistState();
}

export function setLayoutMode(value: LayoutMode) {
  layoutMode.set(value);
  persistState();
}

// --- Formatting ---
// Chinese relative time, matching the design mockup.
export function fmtTime(ts: number): string {
  const now = new Date();
  const d = new Date(ts * 1000);
  const diff = now.getTime() - d.getTime();
  if (diff < 60_000) return '刚刚';
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`;
  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  const hm = `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (sameDay(d, yesterday)) return `昨天 ${hm}`;
  const dayDiff = Math.floor((now.getTime() - d.getTime()) / 86400_000);
  if (dayDiff < 7) {
    const week = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'][d.getDay()];
    return `${week} ${hm}`;
  }
  return `${d.getMonth() + 1}月${d.getDate()}日 ${hm}`;
}

// Which calendar group a timestamp falls into, for the 1b timeline.
export function dateGroup(ts: number): '今天' | '昨天' | '更早' {
  const now = new Date();
  const d = new Date(ts * 1000);
  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  if (sameDay(d, now)) return '今天';
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (sameDay(d, yesterday)) return '昨天';
  return '更早';
}

export function priorityColor(p: number): string {
  if (p >= 5) return '#ef4444';
  if (p >= 4) return '#f97316';
  if (p >= 3) return '#6b7280';
  return '#9ca3af';
}

// Named priority label, matching the design (紧急/高/普通/低).
export function priorityLabel(p: number): string {
  if (p >= 5) return '紧急';
  if (p >= 4) return '高';
  if (p >= 3) return '普通';
  return '低';
}

export function categoryOptions(messages: Message[]): { path: string; depth: number }[] {
  const paths = new Set<string>();
  for (const message of messages) {
    for (let depth = 1; depth <= (message.category || []).length; depth++) {
      paths.add(message.category.slice(0, depth).join('/'));
    }
  }
  return [...paths]
    .sort((a, b) => a.localeCompare(b, 'zh-CN'))
    .map(path => ({ path, depth: path.split('/').length }));
}

export function matchesCategory(message: Message, selected: string): boolean {
  if (!selected) return true;
  const path = (message.category || []).join('/');
  return path === selected || path.startsWith(`${selected}/`);
}

// Deterministic color for a topic, used as the topic dot/tag in 1b timeline
// and the topic list. Stable per topic name, no schema change needed.
const TOPIC_PALETTE = [
  '#ef4444', '#f97316', '#f59e0b', '#22c55e',
  '#14b8a6', '#0ea5e9', '#3b82f6', '#8b5cf6',
];
export function topicColor(name: string): string {
  let h = 0;
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0;
  return TOPIC_PALETTE[h % TOPIC_PALETTE.length];
}
