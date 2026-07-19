import { get, writable } from 'svelte/store';

export type PopupPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'center';
export type LayoutMode = 'split' | 'timeline';
export type WindowBehavior = 'popup' | 'resident';

export const popupOnNotify = writable(false);
export const popupPosition = writable<PopupPosition>('top-right');
export const layoutMode = writable<LayoutMode>('split');
export const windowBehavior = writable<WindowBehavior>('resident');
export const doNotDisturb = writable(false);

export interface PreferenceValues {
  layoutMode: LayoutMode;
  windowBehavior: WindowBehavior;
  doNotDisturb: boolean;
  popupOnNotify: boolean;
  popupPosition: PopupPosition;
}

let persist = () => {};

export function setPreferencePersistence(callback: () => void) {
  persist = callback;
}

export function setPopupOnNotify(value: boolean) {
  popupOnNotify.set(value);
  persist();
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
  popupOnNotify.set(values.popupOnNotify);
  popupPosition.set(values.popupPosition);
  persist();
}
