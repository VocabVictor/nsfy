import type { SyncedChange } from './stores/state-events';

export function stateChangeKey(change: SyncedChange): string {
  return `${change.server}\n${change.topic}\n${change.id}`;
}

export function dedupeStateChanges(changes: SyncedChange[], limit = 5000): SyncedChange[] {
  const latest = new Map<string, SyncedChange>();
  for (const change of changes) {
    const key = stateChangeKey(change);
    latest.delete(key);
    latest.set(key, change);
  }
  return [...latest.values()].slice(-limit);
}

export function groupStateChanges(changes: SyncedChange[]): SyncedChange[][] {
  const groups = new Map<string, SyncedChange[]>();
  for (const change of changes) {
    const key = `${change.server}\n${change.topic}`;
    const items = groups.get(key) || [];
    items.push(change);
    groups.set(key, items);
  }
  return [...groups.values()];
}
