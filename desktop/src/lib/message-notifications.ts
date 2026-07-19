import { invoke } from '@tauri-apps/api/core';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { get } from 'svelte/store';
import {
  doNotDisturb, popupOnNotify, popupPosition, topics, windowBehavior,
  type Message,
} from './stores/nsfy';

export function handleIncomingNotification(
  topicName: string,
  message: Message,
  permissionGranted: boolean,
  fresh: boolean,
) {
  if (get(doNotDisturb)) return;

  if (fresh && get(windowBehavior) === 'popup') {
    invoke('focus_main_window').catch(() => {});
  }

  if (!fresh || message.priority < 4) return;
  if (permissionGranted) {
    sendNotification({ title: message.title || topicName, body: message.message });
  }
  if (!get(popupOnNotify) || get(windowBehavior) === 'popup') return;

  const recent = get(topics)
    .flatMap(topic => {
      const high = topic.messages.filter(item => item.priority >= 4);
      return high.length ? [{ ...high[high.length - 1], topicName: topic.name }] : [];
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
  }).catch(() => {});
}
