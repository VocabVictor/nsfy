import assert from 'node:assert/strict';
import test from 'node:test';
import { get } from 'svelte/store';
import {
  dndAllowedPriorities,
  doNotDisturb,
  notificationMode,
  defaultAdvancedPreferences,
  isScheduledDnd,
  setDoNotDisturb,
  setPreferencePersistence,
  savePreferences,
  setWindowBehavior,
  toggleDoNotDisturb,
  windowBehavior,
} from '../src/lib/stores/preferences.ts';

test('window behavior changes are persisted', () => {
  let saves = 0;
  setPreferencePersistence(() => saves++);
  setWindowBehavior('popup');

  assert.equal(get(windowBehavior), 'popup');
  assert.equal(saves, 1);
});

test('do not disturb supports direct and shortcut-style toggles', () => {
  let saves = 0;
  setPreferencePersistence(() => saves++);
  setDoNotDisturb(false);
  toggleDoNotDisturb();

  assert.equal(get(doNotDisturb), true);
  assert.equal(saves, 2);
});

test('saving a settings draft persists once', () => {
  let saves = 0;
  setPreferencePersistence(() => saves++);
  savePreferences({
    layoutMode: 'timeline', windowBehavior: 'resident', doNotDisturb: false,
    dndAllowedPriorities: [4, 5],
    notificationMode: 'persistent', popupPosition: 'bottom-right',
    advanced: defaultAdvancedPreferences,
  });

  assert.equal(saves, 1);
  assert.deepEqual(get(dndAllowedPriorities), [4, 5]);
  assert.equal(get(notificationMode), 'persistent');
});

test('scheduled do not disturb carries overnight ranges into the next day', () => {
  const settings = {
    ...defaultAdvancedPreferences,
    dndScheduleEnabled: true,
    dndStart: '22:00',
    dndEnd: '08:00',
    dndDays: [1],
  };
  assert.equal(isScheduledDnd(settings, new Date('2026-07-20T23:00:00')), true);
  assert.equal(isScheduledDnd(settings, new Date('2026-07-21T01:00:00')), true);
  assert.equal(isScheduledDnd(settings, new Date('2026-07-21T23:00:00')), false);
});
