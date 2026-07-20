import { get } from 'svelte/store';
import { activeTopic, topics } from './core-stores';
import {
  isMessageDismissed, isMessagePurged, isMessageRead, rememberDismissed, rememberReadSoon,
} from './message-state';
import { persistState } from './state-persistence';
import { advancedPreferences } from './preferences';
import { moveToTrash, trashContains } from './trash-store';
import type { Message } from './models';
import { emitStateChanges } from './state-events';

export function addTopic(server: string, name: string) {
  topics.update(items => items.some(topic => topic.server === server && topic.name === name)
    ? items : [...items, { name, server, messages: [], unread: 0, connected: false }]);
  persistState();
}

export function removeTopic(server: string, name: string) {
  const topic = get(topics).find(item => item.server === server && item.name === name);
  if (topic) rememberDismissed(topic.messages.map(message => ({ server, topic: name, id: message.id })));
  topics.update(items => items.filter(topic => !(topic.server === server && topic.name === name)));
  if (get(activeTopic) === name) activeTopic.set(null);
  persistState();
}

export function addMessage(topicName: string, server: string, message: Message) {
  const cutoff = Date.now() / 1000 - get(advancedPreferences).retentionDays * 86_400;
  if (message.time < cutoff) return;
  const ref = { server, topic: topicName, id: message.id };
  if (isMessageDismissed(ref)) {
    if (!isMessagePurged(ref) && !trashContains(ref)) moveToTrash([{
      ...message, read: true, server, topicName, deletedAt: Date.now(),
    }]);
    return;
  }
  if (get(activeTopic) === topicName && !isMessageRead(ref)) {
    rememberReadSoon(ref);
    emitStateChanges([ref], 'read');
  }
  topics.update(items => items.map(topic => {
    if (topic.server !== server || topic.name !== topicName) return topic;
    if (topic.messages.some(item => item.id === message.id)) return topic;
    const read = isMessageRead(ref);
    const messages = [...topic.messages, { ...message, read }].slice(-500);
    return { ...topic, messages, unread: read ? topic.unread : topic.unread + 1 };
  }));
}

export function setConnected(topicName: string, server: string, connected: boolean) {
  topics.update(items => items.map(topic => topic.server === server && topic.name === topicName
    ? { ...topic, connected, lastConnectedAt: connected ? Date.now() : topic.lastConnectedAt }
    : topic));
  if (connected) persistState();
}
