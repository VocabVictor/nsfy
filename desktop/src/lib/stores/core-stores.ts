import { writable } from 'svelte/store';
import type { Server, Topic } from './models';

export const servers = writable<Server[]>([]);
export const topics = writable<Topic[]>([]);
export const activeTopic = writable<string | null>(null);
export const activeTab = writable<'topics' | 'publish' | 'settings'>('topics');
