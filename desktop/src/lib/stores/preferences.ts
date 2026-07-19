import { get, writable } from 'svelte/store';

export type PopupPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'center';
export type NotificationMode = 'silent' | 'system' | 'temporary' | 'persistent';
export type LayoutMode = 'split' | 'timeline';
export type WindowBehavior = 'popup' | 'resident';

export const notificationMode = writable<NotificationMode>('system');
export const popupPosition = writable<PopupPosition>('top-right');
export const layoutMode = writable<LayoutMode>('split');
export const windowBehavior = writable<WindowBehavior>('resident');
export const doNotDisturb = writable(false);
export const dndAllowedPriorities = writable<number[]>([]);

export interface PreferenceValues {
  layoutMode: LayoutMode;
  windowBehavior: WindowBehavior;
  doNotDisturb: boolean;
  dndAllowedPriorities: number[];
  notificationMode: NotificationMode;
  popupPosition: PopupPosition;
}

let persist = () => {};

export function setPreferencePersistence(callback: () => void) {
  persist = callback;
}

export function setPopupPosition(value: PopupPosition) {
  popupPosition.set(value);
  persist();
}

export function setLayoutMode(value: LayoutMode) {
  layoutMode.set(value);
  persist();
}

export function setWindowBehavior(value: WindowBehavior) {
  windowBehavior.set(value);
  persist();
}

export function setDoNotDisturb(value: boolean) {
  doNotDisturb.set(value);
  persist();
}

export function toggleDoNotDisturb() {
  setDoNotDisturb(!get(doNotDisturb));
}

export function savePreferences(values: PreferenceValues) {
  layoutMode.set(values.layoutMode);
  windowBehavior.set(values.windowBehavior);
  doNotDisturb.set(values.doNotDisturb);
  dndAllowedPriorities.set(values.dndAllowedPriorities);
  notificationMode.set(values.notificationMode);
  popupPosition.set(values.popupPosition);
  persist();
}
