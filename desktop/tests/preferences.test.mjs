import assert from 'node:assert/strict';
import test from 'node:test';
import { get } from 'svelte/store';
import {
  doNotDisturb,
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
    popupOnNotify: true, popupPosition: 'bottom-right',
  });

  assert.equal(saves, 1);
});
