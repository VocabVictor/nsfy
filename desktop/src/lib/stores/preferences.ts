import { get, writable } from 'svelte/store';

export type PopupPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'center';
export type NotificationMode = 'silent' | 'system' | 'temporary' | 'persistent';
export type LayoutMode = 'split' | 'timeline';
export type WindowBehavior = 'popup' | 'resident';
export type ProxyMode = 'system' | 'direct';
export type TopicNotificationMode = 'normal' | 'high' | 'mute';

export interface TopicNotificationRule {
  mode: TopicNotificationMode;
  bypassDnd: boolean;
}

export interface AdvancedPreferences {
  autoStart: boolean;
  startMinimized: boolean;
  dndScheduleEnabled: boolean;
  dndStart: string;
  dndEnd: string;
  dndDays: number[];
  showPreview: boolean;
  soundEnabled: boolean;
  urgentSoundEnabled: boolean;
  retentionDays: number;
  trashRetentionDays: number;
  dndShortcut: string;
  showShortcut: string;
  proxyMode: ProxyMode;
  topicRules: Record<string, TopicNotificationRule>;
}

export const defaultAdvancedPreferences: AdvancedPreferences = {
  autoStart: false,
  startMinimized: true,
  dndScheduleEnabled: false,
  dndStart: '22:00',
  dndEnd: '08:00',
  dndDays: [1, 2, 3, 4, 5, 6, 7],
  showPreview: true,
  soundEnabled: true,
  urgentSoundEnabled: true,
  retentionDays: 30,
  trashRetentionDays: 30,
  dndShortcut: 'Ctrl+Alt+D',
  showShortcut: 'Ctrl+Alt+N',
  proxyMode: 'system',
  topicRules: {},
};

export const notificationMode = writable<NotificationMode>('system');
export const popupPosition = writable<PopupPosition>('top-right');
export const layoutMode = writable<LayoutMode>('split');
export const windowBehavior = writable<WindowBehavior>('resident');
export const doNotDisturb = writable(false);
export const dndAllowedPriorities = writable<number[]>([]);
export const advancedPreferences = writable<AdvancedPreferences>({ ...defaultAdvancedPreferences });

export interface PreferenceValues {
  layoutMode: LayoutMode;
  windowBehavior: WindowBehavior;
  doNotDisturb: boolean;
  dndAllowedPriorities: number[];
  notificationMode: NotificationMode;
  popupPosition: PopupPosition;
  advanced: AdvancedPreferences;
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
  advancedPreferences.set(values.advanced);
  persist();
}

export function isScheduledDnd(preferences: AdvancedPreferences, date = new Date()): boolean {
  if (!preferences.dndScheduleEnabled) return false;
  let day = date.getDay() === 0 ? 7 : date.getDay();
  const minutes = date.getHours() * 60 + date.getMinutes();
  const parse = (value: string) => {
    const [hour, minute] = value.split(':').map(Number);
    return hour * 60 + minute;
  };
  const start = parse(preferences.dndStart);
  const end = parse(preferences.dndEnd);
  if (start > end && minutes < end) day = day === 1 ? 7 : day - 1;
  if (!preferences.dndDays.includes(day)) return false;
  return start <= end ? minutes >= start && minutes < end : minutes >= start || minutes < end;
}

export function topicRuleKey(server: string, topic: string): string {
  return JSON.stringify([server, topic]);
}
