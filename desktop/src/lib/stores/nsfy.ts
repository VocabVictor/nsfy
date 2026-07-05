// Svelte stores for nsfy desktop
import { writable, get } from 'svelte/store';

// --- Types ---
export interface Message {
  id: string;
  time: number;
  title: string;
  message: string;
  priority: number;
  tags: string[];
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

// --- Persistence ---
export function loadState() {
  const raw = localStorage.getItem('nsfy-state');
  if (raw) {
    try {
      const data = JSON.parse(raw);
      if (data.servers) servers.set(data.servers);
      if (data.topics) {
        topics.set(data.topics.map((t: any) => ({
          ...t, connected: false, unread: t.unread || 0, messages: [],
        })));
      }
      if (typeof data.popupOnNotify === 'boolean') popupOnNotify.set(data.popupOnNotify);
      if (typeof data.popupPosition === 'string') popupPosition.set(data.popupPosition);
    } catch {}
  }
  if (get(servers).length === 0) {
    servers.set([{ url: 'http://localhost:8080', name: 'Local' }]);
  }
}

export function persistState() {
  const s = get(servers);
  const t = get(topics);
  localStorage.setItem('nsfy-state', JSON.stringify({
    servers: s,
    topics: t.map(t => ({ name: t.name, server: t.server, unread: t.unread })),
    popupOnNotify: get(popupOnNotify),
    popupPosition: get(popupPosition),
  }));
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
  topics.update(ts => ts.map(t =>
    t.name === topicName ? { ...t, unread: 0 } : t
  ));
  persistState();
}

export function setConnected(topicName: string, server: string, connected: boolean) {
  topics.update(ts => ts.map(t =>
    t.server === server && t.name === topicName ? { ...t, connected } : t
  ));
}

export function addServer(url: string, name: string) {
  servers.update(s => {
    if (s.find(x => x.url === url)) return s;
    return [...s, { url, name }];
  });
  persistState();
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

// --- Formatting ---
export function fmtTime(ts: number): string {
  const d = new Date(ts * 1000);
  const now = new Date();
  const isToday = d.toDateString() === now.toDateString();
  if (isToday) {
    return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  }
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) +
    ' ' + d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}

export function priorityColor(p: number): string {
  if (p >= 5) return '#ef4444';
  if (p >= 4) return '#f97316';
  if (p >= 3) return '#eab308';
  return '#6b7280';
}
