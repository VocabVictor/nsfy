import { get, writable } from 'svelte/store';
import {
  emptyTrashState, loadMessageState, messageKey, rememberTrash, removeTrash,
  rememberPurged, type MessageRef, type TrashMessage,
} from './message-state';

export const trash = writable<TrashMessage[]>([]);

export function initializeMessageState() {
  trash.set(loadMessageState());
}

export function trashRef(message: TrashMessage): MessageRef {
  return { server: message.server, topic: message.topicName, id: message.id };
}

export function moveToTrash(messages: TrashMessage[]) {
  trash.set(rememberTrash(messages));
}

export function trashContains(ref: MessageRef): boolean {
  const key = messageKey(ref);
  return get(trash).some(message => messageKey(trashRef(message)) === key);
}

export function takeFromTrash(refs: MessageRef[]): TrashMessage[] {
  const selected = new Set(refs.map(messageKey));
  const messages = get(trash).filter(message => selected.has(messageKey(trashRef(message))));
  trash.set(removeTrash(refs));
  return messages;
}

export function discardTrash(refs: MessageRef[]) {
  rememberPurged(refs);
  trash.set(removeTrash(refs));
}

export function emptyTrash() {
  rememberPurged(get(trash).map(trashRef));
  trash.set(emptyTrashState());
}

export function pruneTrashOlderThan(days: number) {
  const cutoff = Date.now() - Math.max(1, days) * 86_400_000;
  const expired = get(trash).filter(message => message.deletedAt < cutoff).map(trashRef);
  if (expired.length) discardTrash(expired);
}
