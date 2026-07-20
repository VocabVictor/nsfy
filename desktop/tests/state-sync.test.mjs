import assert from 'node:assert/strict';
import test from 'node:test';
import { dedupeStateChanges, groupStateChanges } from '../src/lib/state-sync-wire.ts';

const change = (server, topic, id, status) => ({ server, topic, id, status });

test('state queue keeps only the latest status for each message', () => {
  const result = dedupeStateChanges([
    change('a', 'alerts', 'one', 'read'),
    change('a', 'alerts', 'one', 'trash'),
    change('a', 'alerts', 'two', 'read'),
  ]);
  assert.deepEqual(result, [
    change('a', 'alerts', 'one', 'trash'),
    change('a', 'alerts', 'two', 'read'),
  ]);
});

test('state requests are isolated by server and topic', () => {
  const groups = groupStateChanges([
    change('a', 'alerts', 'one', 'read'),
    change('a', 'news', 'two', 'trash'),
    change('b', 'alerts', 'three', 'purged'),
  ]);
  assert.equal(groups.length, 3);
  assert.ok(groups.every(group => group.every(item =>
    item.server === group[0].server && item.topic === group[0].topic)));
});
