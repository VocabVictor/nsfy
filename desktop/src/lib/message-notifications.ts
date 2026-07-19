import { invoke } from '@tauri-apps/api/core';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { get } from 'svelte/store';
import { selectPresentation, shouldPresentNotification } from './notification-policy';
import {
  dndAllowedPriorities, doNotDisturb, notificationMode, popupPosition, topics, windowBehavior,
  type Message,
} from './stores/nsfy';

export function handleIncomingNotification(
  topicName: string,
  message: Message,
  permissionGranted: boolean,
  fresh: boolean,
) {
  const dnd = get(doNotDisturb);
  const allowed = get(dndAllowedPriorities);
  if (!fresh || !shouldPresentNotification(message, dnd, allowed)) return;

  const presentation = selectPresentation(
    get(notificationMode), get(windowBehavior), permissionGranted);
  if (presentation === 'none') return;
  if (presentation === 'main') {
    invoke('focus_main_window').catch(() => {});
    return;
  }
  if (presentation === 'system') {
    sendNotification({ title: message.title || topicName, body: message.message });
    return;
  }

  const recent = get(topics)
    .flatMap(topic => {
      const alerting = topic.messages.filter(item =>
        shouldPresentNotification(item, dnd, allowed));
      return alerting.length ? [{ ...alerting[alerting.length - 1], topicName: topic.name }] : [];
    })
    .sort((a, b) => b.time - a.time)
    .slice(0, 3)
    .map(item => ({
      title: item.title || item.topicName,
      body: item.message,
      time: item.time,
      priority: item.priority,
    }));

  invoke('show_notification_popup', {
    messages: recent.length ? recent : [{
      title: message.title || topicName,
      body: message.message,
      time: message.time,
      priority: message.priority,
    }],
    position: get(popupPosition),
    persistent: presentation === 'persistent',
  }).catch(() => {});
}
