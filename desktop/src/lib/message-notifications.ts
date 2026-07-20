import { invoke } from '@tauri-apps/api/core';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { get } from 'svelte/store';
import { selectPresentation, shouldPresentNotification } from './notification-policy';
import {
  advancedPreferences, dndAllowedPriorities, doNotDisturb, isScheduledDnd, notificationMode,
  popupPosition, topicRuleKey, topics, windowBehavior,
  type Message,
} from './stores/nsfy';

export function handleIncomingNotification(
  topicName: string,
  server: string,
  message: Message,
  permissionGranted: boolean,
  fresh: boolean,
) {
  const advanced = get(advancedPreferences);
  const rule = advanced.topicRules[topicRuleKey(server, topicName)]
    || { mode: 'normal', bypassDnd: false };
  if (rule.mode === 'mute' || (rule.mode === 'high' && message.priority < 4)) return;
  const dnd = get(doNotDisturb) || isScheduledDnd(advanced);
  const allowed = get(dndAllowedPriorities);
  const policy = { ...message, bypassDnd: message.bypassDnd || rule.bypassDnd };
  if (!fresh || !shouldPresentNotification(policy, dnd, allowed)) return;

  const presentation = selectPresentation(
    get(notificationMode), get(windowBehavior), permissionGranted);
  if (presentation === 'none') return;
  playSound(message.priority, advanced.soundEnabled, advanced.urgentSoundEnabled);
  if (presentation === 'main') {
    invoke('focus_main_window').catch(() => {});
    return;
  }
  if (presentation === 'system') {
    sendNotification({
      title: message.title || topicName,
      body: advanced.showPreview ? message.message : '有一条新消息',
    });
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
      body: advanced.showPreview ? item.message : '有一条新消息',
      time: item.time,
      priority: item.priority,
    }));

  invoke('show_notification_popup', {
    messages: recent.length ? recent : [{
      title: message.title || topicName,
      body: advanced.showPreview ? message.message : '有一条新消息',
      time: message.time,
      priority: message.priority,
    }],
    position: get(popupPosition),
    persistent: presentation === 'persistent',
  }).catch(() => {});
}

function playSound(priority: number, enabled: boolean, urgentEnabled: boolean) {
  if (!enabled || (priority >= 5 && !urgentEnabled)) return;
  try {
    const context = new AudioContext();
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    oscillator.frequency.value = priority >= 5 ? 880 : 560;
    gain.gain.setValueAtTime(0.08, context.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, context.currentTime + (priority >= 5 ? 0.35 : 0.18));
    oscillator.connect(gain).connect(context.destination);
    oscillator.start();
    oscillator.stop(context.currentTime + (priority >= 5 ? 0.35 : 0.18));
    oscillator.onended = () => void context.close();
  } catch {}
}
