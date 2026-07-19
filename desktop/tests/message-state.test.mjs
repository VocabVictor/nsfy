import assert from 'node:assert/strict';
import test from 'node:test';
import {
  isMessageDismissed,
  isMessagePurged,
  isMessageRead,
  loadMessageState,
  messageKey,
  rememberTrash,
  removeTrash,
  rememberDismissed,
  rememberRead,
  rememberPurged,
} from '../src/lib/stores/message-state.ts';

class MemoryStorage {
  values = new Map();

  getItem(key) {
    return this.values.get(key) ?? null;
  }

  setItem(key, value) {
    this.values.set(key, value);
  }
}

const first = { server: 'https://one.example', topic: 'alerts', id: 'same-id' };
const second = { server: 'https://two.example', topic: 'alerts', id: 'same-id' };

test('message identity includes server, topic, and id', () => {
  assert.notEqual(messageKey(first), messageKey(second));
});

test('read state persists and remains scoped to one message', () => {
  const storage = new MemoryStorage();
  loadMessageState(storage);
  rememberRead([first]);

  loadMessageState(storage);
  assert.equal(isMessageRead(first), true);
  assert.equal(isMessageRead(second), false);
});

test('dismissed state persists and removes redundant read state', () => {
  const storage = new MemoryStorage();
  loadMessageState(storage);
  rememberRead([first]);
  rememberDismissed([first]);

  loadMessageState(storage);
  assert.equal(isMessageDismissed(first), true);
  assert.equal(isMessageRead(first), false);
});

test('malformed saved state is ignored', () => {
  const storage = new MemoryStorage();
  storage.setItem('nsfy-message-state', '{bad json');
  loadMessageState(storage);

  assert.equal(isMessageRead(first), false);
  assert.equal(isMessageDismissed(first), false);
});

test('saved state is bounded to the newest 5000 entries', () => {
  const storage = new MemoryStorage();
  loadMessageState(storage);
  const refs = Array.from({ length: 5002 }, (_, index) => ({
    server: 'https://one.example', topic: 'bulk', id: String(index),
  }));
  rememberRead(refs);

  assert.equal(isMessageRead(refs[0]), false);
  assert.equal(isMessageRead(refs[1]), false);
  assert.equal(isMessageRead(refs[2]), true);
  assert.equal(JSON.parse(storage.getItem('nsfy-message-state')).read.length, 5000);
});

test('trash keeps full message data and survives reload', () => {
  const storage = new MemoryStorage();
  loadMessageState(storage);
  const deleted = {
    ...first, topicName: first.topic, time: 1, title: 'title', message: 'body',
    priority: 3, tags: [], category: ['work'], read: true, deletedAt: 2,
  };
  delete deleted.topic;
  rememberTrash([deleted]);

  const restored = loadMessageState(storage);
  assert.deepEqual(restored, [deleted]);
});

test('removing trash does not remove its replay tombstone', () => {
  const storage = new MemoryStorage();
  loadMessageState(storage);
  const deleted = {
    server: first.server, topicName: first.topic, id: first.id,
    time: 1, title: '', message: 'body', priority: 3, tags: [], category: [],
    read: false, deletedAt: 2,
  };
  rememberDismissed([first]);
  rememberTrash([deleted]);
  removeTrash([first]);

  assert.equal(loadMessageState(storage).length, 0);
  assert.equal(isMessageDismissed(first), true);
});

test('purged messages remain permanently suppressed', () => {
  const storage = new MemoryStorage();
  loadMessageState(storage);
  rememberDismissed([first]);
  rememberPurged([first]);

  loadMessageState(storage);
  assert.equal(isMessageDismissed(first), true);
  assert.equal(isMessagePurged(first), true);
});
