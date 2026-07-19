import assert from 'node:assert/strict';
import test from 'node:test';
import { selectPresentation, shouldPresentNotification } from '../src/lib/notification-policy.ts';

const message = { priority: 4, popup: true, bypassDnd: false };

test('messages not marked for popup stay silent', () => {
  assert.equal(shouldPresentNotification({ ...message, popup: false }, false, []), false);
});

test('do not disturb permits configured priorities', () => {
  assert.equal(shouldPresentNotification(message, true, [4, 5]), true);
  assert.equal(shouldPresentNotification(message, true, [5]), false);
});

test('sender can explicitly bypass do not disturb', () => {
  assert.equal(shouldPresentNotification({ ...message, bypassDnd: true }, true, []), true);
});

test('all four notification modes select a distinct presentation', () => {
  assert.equal(selectPresentation('silent', 'resident', true), 'none');
  assert.equal(selectPresentation('system', 'resident', true), 'system');
  assert.equal(selectPresentation('temporary', 'resident', true), 'temporary');
  assert.equal(selectPresentation('persistent', 'resident', true), 'persistent');
});

test('main-window popup mode overrides non-silent notification styles', () => {
  assert.equal(selectPresentation('temporary', 'popup', true), 'main');
  assert.equal(selectPresentation('silent', 'popup', true), 'none');
});
