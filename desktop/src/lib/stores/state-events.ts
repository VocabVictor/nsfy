import type { MessageRef } from './message-state';

export type SyncedStatus = 'read' | 'trash' | 'purged';
export interface SyncedChange extends MessageRef {
  status: SyncedStatus;
  updatedAt?: number;
}

let emitter: (changes: SyncedChange[]) => void = () => {};

export function setStateSyncEmitter(callback: (changes: SyncedChange[]) => void) {
  emitter = callback;
}

export function emitStateChanges(refs: MessageRef[], status: SyncedStatus) {
  if (refs.length) emitter(refs.map(ref => ({ ...ref, status })));
}
