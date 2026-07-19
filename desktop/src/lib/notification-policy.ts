export interface DeliveryPolicy {
  priority: number;
  popup: boolean;
  bypassDnd: boolean;
}

export type Presentation = 'none' | 'main' | 'system' | 'temporary' | 'persistent';

export function selectPresentation(
  mode: 'silent' | 'system' | 'temporary' | 'persistent',
  windowBehavior: 'popup' | 'resident',
  systemPermission: boolean,
): Presentation {
  if (mode === 'silent') return 'none';
  if (windowBehavior === 'popup') return 'main';
  if (mode === 'system') return systemPermission ? 'system' : 'none';
  return mode;
}

export function shouldPresentNotification(
  message: DeliveryPolicy,
  doNotDisturb: boolean,
  allowedPriorities: number[],
): boolean {
  if (!message.popup) return false;
  return !doNotDisturb
    || message.bypassDnd
    || allowedPriorities.includes(message.priority);
}
