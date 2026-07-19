export interface MessageRef {
  server: string;
  topic: string;
  id: string;
}

export interface TrashMessage {
  server: string;
  topicName: string;
  id: string;
  time: number;
  title: string;
  message: string;
  priority: number;
  tags: string[];
  category: string[];
  read: boolean;
  deletedAt: number;
}

interface SavedMessageState {
  read: string[];
  dismissed: string[];
  purged: string[];
  trash: TrashMessage[];
}

const STORAGE_KEY = 'nsfy-message-state';
const MAX_ENTRIES = 5000;
const MAX_TRASH_CHARS = 2_000_000;
let storage: Pick<Storage, 'getItem' | 'setItem'> | null = null;
let readKeys = new Set<string>();
let dismissedKeys = new Set<string>();
let purgedKeys = new Set<string>();
let trashMessages: TrashMessage[] = [];
let saveTimer: ReturnType<typeof setTimeout> | null = null;

export function messageKey(ref: MessageRef): string {
  return JSON.stringify([ref.server, ref.topic, ref.id]);
}

export function loadMessageState(
  source: Pick<Storage, 'getItem' | 'setItem'> = localStorage,
): TrashMessage[] {
  storage = source;
  readKeys = new Set();
  dismissedKeys = new Set();
  purgedKeys = new Set();
  trashMessages = [];
  try {
    const saved = JSON.parse(source.getItem(STORAGE_KEY) || '') as SavedMessageState;
    if (Array.isArray(saved.read)) readKeys = new Set(saved.read.slice(-MAX_ENTRIES));
    if (Array.isArray(saved.dismissed)) dismissedKeys = new Set(saved.dismissed.slice(-MAX_ENTRIES));
    if (Array.isArray(saved.purged)) purgedKeys = new Set(saved.purged.slice(-MAX_ENTRIES));
    if (Array.isArray(saved.trash)) trashMessages = saved.trash.slice(-MAX_ENTRIES);
  } catch {}
  return [...trashMessages];
}

function addBounded(target: Set<string>, keys: string[]) {
  for (const key of keys) {
    target.delete(key);
    target.add(key);
  }
  while (target.size > MAX_ENTRIES) {
    const oldest = target.values().next().value;
    if (oldest === undefined) break;
    target.delete(oldest);
  }
}

function save() {
  while (trashMessages.length > 1 && JSON.stringify(trashMessages).length > MAX_TRASH_CHARS) {
    trashMessages.shift();
  }
  try {
    storage?.setItem(STORAGE_KEY, JSON.stringify({
      read: [...readKeys], dismissed: [...dismissedKeys], purged: [...purgedKeys], trash: trashMessages,
    }));
  } catch {}
}

function saveSoon() {
  if (saveTimer) return;
  saveTimer = setTimeout(() => {
    saveTimer = null;
    save();
  }, 250);
}

export function isMessageRead(ref: MessageRef): boolean {
  return readKeys.has(messageKey(ref));
}

export function isMessageDismissed(ref: MessageRef): boolean {
  return dismissedKeys.has(messageKey(ref));
}

export function isMessagePurged(ref: MessageRef): boolean {
  return purgedKeys.has(messageKey(ref));
}

export function rememberRead(refs: MessageRef[]) {
  addBounded(readKeys, refs.map(messageKey));
  save();
}

export function rememberReadSoon(ref: MessageRef) {
  addBounded(readKeys, [messageKey(ref)]);
  saveSoon();
}

export function rememberDismissed(refs: MessageRef[]) {
  const keys = refs.map(messageKey);
  addBounded(dismissedKeys, keys);
  for (const key of keys) readKeys.delete(key);
  save();
}

export function forgetDismissed(refs: MessageRef[]) {
  for (const ref of refs) {
    dismissedKeys.delete(messageKey(ref));
    purgedKeys.delete(messageKey(ref));
  }
  save();
}

export function rememberPurged(refs: MessageRef[]) {
  addBounded(purgedKeys, refs.map(messageKey));
  save();
}

function trashKey(message: TrashMessage) {
  return messageKey({ server: message.server, topic: message.topicName, id: message.id });
}

export function rememberTrash(messages: TrashMessage[]): TrashMessage[] {
  const byKey = new Map(trashMessages.map(message => [trashKey(message), message]));
  for (const message of messages) {
    const key = trashKey(message);
    byKey.delete(key);
    byKey.set(key, message);
  }
  trashMessages = [...byKey.values()].slice(-MAX_ENTRIES);
  save();
  return [...trashMessages];
}

export function removeTrash(refs: MessageRef[]): TrashMessage[] {
  const selected = new Set(refs.map(messageKey));
  trashMessages = trashMessages.filter(message => !selected.has(trashKey(message)));
  save();
  return [...trashMessages];
}

export function emptyTrashState(): TrashMessage[] {
  trashMessages = [];
  save();
  return [];
}
