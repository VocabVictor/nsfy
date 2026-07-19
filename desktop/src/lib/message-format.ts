import type { Message } from './stores/nsfy';

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

export function priorityColor(priority: number): string {
  if (priority >= 5) return '#ef4444';
  if (priority >= 4) return '#f97316';
  if (priority >= 3) return '#6b7280';
  return '#9ca3af';
}

export function priorityLabel(priority: number): string {
  if (priority >= 5) return '紧急';
  if (priority >= 4) return '高';
  if (priority >= 3) return '普通';
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

const TOPIC_PALETTE = [
  '#ef4444', '#f97316', '#f59e0b', '#22c55e',
  '#14b8a6', '#0ea5e9', '#3b82f6', '#8b5cf6',
];

export function topicColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = (hash * 31 + name.charCodeAt(i)) >>> 0;
  return TOPIC_PALETTE[hash % TOPIC_PALETTE.length];
}
