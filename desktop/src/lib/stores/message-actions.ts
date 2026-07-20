import { get } from 'svelte/store';
import { topics } from './core-stores';
import {
  forgetDismissed, isMessageDismissed, rememberDismissed, rememberPurged, rememberRead,
  type MessageRef,
} from './message-state';
import { emitStateChanges, type SyncedChange } from './state-events';
import { persistState } from './state-persistence';
import {
  discardTrash as discardTrashLocal, emptyTrash as emptyTrashLocal, moveToTrash, takeFromTrash,
  trash, trashContains, trashRef,
} from './trash-store';

const refKey = (ref: MessageRef) => `${ref.server}\n${ref.topic}\n${ref.id}`;

function applyRead(refs: MessageRef[], sync: boolean) {
  if (!refs.length) return;
  const selected = new Set(refs.map(refKey));
  const restore = refs.filter(ref => isMessageDismissed(ref) || trashContains(ref));
  const restored = restore.length ? takeFromTrash(restore) : [];
  forgetDismissed(restore);
  rememberRead(refs);
  topics.update(items => items.map(topic => {
    const fromTrash = restored.filter(message =>
      message.server === topic.server && message.topicName === topic.name);
    const existing = new Set(topic.messages.map(message => message.id));
    let messages = topic.messages.map(message => selected.has(refKey({
      server: topic.server, topic: topic.name, id: message.id,
    })) ? { ...message, read: true } : message);
    messages = [...messages, ...fromTrash.filter(message => !existing.has(message.id)).map(message => ({
      id: message.id, time: message.time, title: message.title, message: message.message,
      priority: message.priority, tags: message.tags, category: message.category,
      popup: message.popup, bypassDnd: message.bypassDnd, read: true,
    }))].sort((a, b) => a.time - b.time).slice(-500);
    const unread = messages.reduce((count, message) => count + (message.read ? 0 : 1), 0);
    return messages !== topic.messages ? { ...topic, messages, unread } : topic;
  }));
  persistState();
  if (sync) emitStateChanges(refs, 'read');
}

function applyTrash(refs: MessageRef[], sync: boolean) {
  if (!refs.length) return;
  const selected = new Set(refs.map(refKey));
  const deletedAt = Date.now();
  moveToTrash(get(topics).flatMap(topic => topic.messages
    .filter(message => selected.has(refKey({ server: topic.server, topic: topic.name, id: message.id })))
    .map(message => ({ ...message, server: topic.server, topicName: topic.name, deletedAt }))));
  rememberDismissed(refs);
  topics.update(items => items.map(topic => {
    const messages = topic.messages.filter(message =>
      !selected.has(refKey({ server: topic.server, topic: topic.name, id: message.id })));
    return messages.length === topic.messages.length ? topic : {
      ...topic, messages, unread: messages.reduce((count, message) => count + (message.read ? 0 : 1), 0),
    };
  }));
  persistState();
  if (sync) emitStateChanges(refs, 'trash');
}

export function markRead(topicName: string, server?: string) {
  const refs = get(topics).filter(topic => topic.name === topicName && (!server || topic.server === server))
    .flatMap(topic => topic.messages.filter(message => !message.read)
      .map(message => ({ server: topic.server, topic: topic.name, id: message.id })));
  applyRead(refs, true);
}

export function markAllRead() {
  const refs = get(topics).flatMap(topic => topic.messages.filter(message => !message.read)
    .map(message => ({ server: topic.server, topic: topic.name, id: message.id })));
  applyRead(refs, true);
}

export function markMessagesRead(refs: MessageRef[]) { applyRead(refs, true); }
export function clearMessages(refs: MessageRef[]) { applyTrash(refs, true); }

export function clearTopicMessages(server: string, name: string) {
  const topic = get(topics).find(item => item.server === server && item.name === name);
  if (topic) applyTrash(topic.messages.map(message => ({ server, topic: name, id: message.id })), true);
}

export function clearAllMessages() {
  applyTrash(get(topics).flatMap(topic => topic.messages.map(message => ({
    server: topic.server, topic: topic.name, id: message.id,
  }))), true);
}

export function restoreTrashMessages(refs: MessageRef[]) { applyRead(refs, true); }

export function discardTrash(refs: MessageRef[]) {
  discardTrashLocal(refs);
  emitStateChanges(refs, 'purged');
}

export function emptyTrash() {
  const refs = get(trash).map(trashRef);
  emptyTrashLocal();
  emitStateChanges(refs, 'purged');
}

export function applyRemoteState(changes: SyncedChange[]) {
  const grouped = new Map<string, MessageRef[]>();
  for (const change of changes) {
    const refs = grouped.get(change.status) || [];
    refs.push(change);
    grouped.set(change.status, refs);
  }
  applyRead(grouped.get('read') || [], false);
  applyTrash(grouped.get('trash') || [], false);
  const purged = grouped.get('purged') || [];
  if (purged.length) {
    applyTrash(purged, false);
    rememberPurged(purged);
    discardTrashLocal(purged);
  }
}
