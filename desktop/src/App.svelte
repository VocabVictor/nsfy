<script lang="ts">
  import './app.css';
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
  import TopicDetail from './lib/TopicDetail.svelte';
  import PublishView from './lib/Publish.svelte';
  import Settings from './lib/Settings.svelte';
  import Timeline from './lib/Timeline.svelte';
  import MessageActions from './lib/MessageActions.svelte';
  import { handleIncomingNotification } from './lib/message-notifications';
  import { connectStateSocket, queueStateUpdates } from './lib/stores/state-sync-client';
  import { setStateSyncEmitter } from './lib/stores/state-events';
  import { connectSocket, type SocketConnection } from './lib/socket-client';
  import {
    topics, servers, activeTopic, layoutMode,
    loadState, addTopic, removeTopic, addMessage, setConnected, markRead,
    advancedPreferences, topicColor, serverToken, normalizeServerUrl, setDoNotDisturb,
  } from './lib/stores/nsfy';

  let ready = $state(false);
  let showSettings = $state(false);
  let showPublish = $state(false);
  let showAdd = $state(false);
  let newTopicName = $state('');
  let newTopicServer = $state('');

  $effect(() => {
    let active = true;
    loadState().finally(() => {
      if (active) ready = true;
    });
    return () => { active = false; };
  });

  function focusOnMount(el: HTMLElement) {
    el.focus();
  }

  // --- WebSocket connections (top-level: alive regardless of view) ---
  const sockets = new Map<string, SocketConnection>();
  const stateSockets = new Map<string, SocketConnection>();
  let notifyPermission = $state(false);

  setStateSyncEmitter(queueStateUpdates);

  onMount(() => {
    const syncDnd = (event: Event) => setDoNotDisturb((event as CustomEvent<boolean>).detail);
    window.addEventListener('nsfy-dnd-changed', syncDnd);
    return () => window.removeEventListener('nsfy-dnd-changed', syncDnd);
  });

  (async () => {
    notifyPermission = await isPermissionGranted();
    if (!notifyPermission) {
      notifyPermission = (await requestPermission()) === 'granted';
    }
  })();

  async function connectTopicState(server: string, name: string) {
    const key = `${server}/${name}`;
    if (stateSockets.has(key)) return;
    const socket = await connectStateSocket(server, name, () => {
      stateSockets.delete(key);
      setTimeout(() => {
        if (get(topics).some(t => t.server === server && t.name === name)) {
          connectTopicState(server, name);
        }
      }, 5000);
    });
    stateSockets.set(key, socket);
  }

  async function connectTopic(server: string, name: string) {
    const key = `${server}/${name}`;
    connectTopicState(server, name);
    if (sockets.has(key)) return;
    try {
      let connectedAt = Date.now() / 1000;
      const base = normalizeServerUrl(server);
      const url = base.replace(/^http/, 'ws') + `/${name}/ws`;
      const ws = await connectSocket(url, $advancedPreferences.proxyMode === 'direct', serverToken(server), {
        open: () => { connectedAt = Date.now() / 1000; setConnected(name, server, true); },
        close: () => {
          setConnected(name, server, false); sockets.delete(key);
          setTimeout(() => { if (get(topics).some(t => t.server === server && t.name === name)) void connectTopic(server, name); }, 5000);
        },
        error: () => setConnected(name, server, false),
        message: text => { try {
          const msg = JSON.parse(text);
          if (!Array.isArray(msg.category)) msg.category = [];
          if (typeof msg.popup !== 'boolean') msg.popup = msg.priority >= 4;
          if (typeof msg.bypassDnd !== 'boolean') msg.bypassDnd = false;
          addMessage(name, server, msg);
          handleIncomingNotification(name, server, msg, notifyPermission, msg.time >= connectedAt - 2);
        } catch {} },
      });
      sockets.set(key, ws);
    } catch {}
  }

  function disconnectAll() {
    sockets.forEach(ws => ws.close());
    stateSockets.forEach(ws => ws.close());
    sockets.clear();
    stateSockets.clear();
  }

  function unsubscribe(server: string, name: string) {
    const key = `${server}/${name}`;
    sockets.get(key)?.close();
    stateSockets.get(key)?.close();
    sockets.delete(key);
    stateSockets.delete(key);
    removeTopic(server, name);
  }

  // Reactive dependency is just the *set* of subscribed topics (server+name),
  // not the full topic objects — those also carry messages/unread/connected,
  // which change on every incoming message or connect/disconnect event. If
  // the effect depended on $topics directly, each of those updates would
  // rerun it: cleanup tears down every socket, then the body reopens them
  // all, whose onopen fires setConnected() and triggers the same effect
  // again — an infinite reconnect loop that never stays connected long
  // enough to receive anything.
  const subscriptionKey = $derived($topics.map(t => `${t.server}::${t.name}`).join('|'));

  $effect(() => {
    subscriptionKey;
    for (const t of get(topics)) {
      connectTopic(t.server, t.name);
    }
    return () => disconnectAll();
  });

  function submitAdd() {
    if (!newTopicName) return;
    addTopic(newTopicServer, newTopicName);
    connectTopic(newTopicServer, newTopicName);
    newTopicName = '';
    showAdd = false;
  }

  function openTopic(name: string) {
    $activeTopic = name;
    showSettings = false;
    const t = $topics.find(t => t.name === name);
    if (t) markRead(t.name, t.server);
  }

  const totalUnread = $derived($topics.reduce((n, t) => n + t.unread, 0));
</script>

{#if !ready}
  <div class="splash">信鸽 · 推送提醒</div>
{:else}
  <div class="app">
    <nav class="sidebar">
      <div class="logo">信鸽</div>

      {#if $layoutMode === 'split'}
        <div class="topic-nav">
          {#each $topics as t (t.server + '/' + t.name)}
            <div
              class="topic-row" class:active={$activeTopic === t.name && !showSettings}
              role="button" tabindex="0"
              onclick={() => openTopic(t.name)}
              onkeydown={(e) => { if (e.key === 'Enter') openTopic(t.name); }}
            >
              <span class="topic-dot" style="background:{topicColor(t.name)}"></span>
              <span class="topic-row-name">{t.name}</span>
              <span class="conn-dot" class:online={t.connected}></span>
              {#if t.unread > 0}
                <span class="badge">{t.unread}</span>
              {/if}
              <button
                class="unsub-btn" aria-label="退订"
                onclick={(e) => { e.stopPropagation(); if (confirm(`退订「${t.name}」?其消息记录将被清除。`)) unsubscribe(t.server, t.name); }}
              >
                <svg viewBox="0 0 16 16" fill="none" width="11" height="11"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
              </button>
            </div>
          {:else}
            <div class="sidebar-empty">
              <p>暂无主题</p>
              <p class="hint">点击下方「新建订阅」</p>
            </div>
          {/each}
        </div>
      {:else}
        <div class="timeline-side">
          <div class="side-note">收件箱</div>
          {#if totalUnread > 0}
            <div class="side-unread">未读 {totalUnread}</div>
          {/if}
        </div>
      {/if}

      {#if showAdd}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="add-form" onkeydown={(e) => {
          if (e.key === 'Escape') { showAdd = false; }
          if (e.key === 'Enter') submitAdd();
        }}>
          <select bind:value={newTopicServer}>
            {#each $servers as s}
              <option value={s.url}>{s.name} ({s.url})</option>
            {/each}
          </select>
          <input type="text" placeholder="主题名" bind:value={newTopicName} use:focusOnMount />
          <button class="btn-primary" disabled={!newTopicName} onclick={submitAdd}>订阅</button>
        </div>
      {/if}

      <div class="sidebar-footer">
        <button class="foot-btn" onclick={() => { showAdd = !showAdd; newTopicServer = $servers[0]?.url || ''; }}>
          <span class="foot-plus">+</span> 新建订阅
        </button>
        <button class="foot-btn" class:active={showSettings} onclick={() => showSettings = !showSettings}>
          <svg class="icon" viewBox="0 0 20 20" fill="none" width="14" height="14"><circle cx="10" cy="10" r="2.6" stroke="currentColor" stroke-width="1.6"/><path d="M10 2.6v2M10 15.4v2M17.4 10h-2M4.6 10h-2M15.06 4.94l-1.42 1.42M6.36 13.64l-1.42 1.42M15.06 15.06l-1.42-1.42M6.36 6.36 4.94 4.94" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
          设置
        </button>
        <div class="status">
          <span class="dot"></span>
          <span>v0.1.0</span>
        </div>
      </div>
    </nav>

    <main class="content">
      <header class="main-header">
        <div class="header-left">
          {#if showSettings}
            <h1>设置</h1>
          {:else if $layoutMode === 'timeline'}
            <h1>收件箱</h1>
            <span class="sub">{totalUnread > 0 ? `未读 ${totalUnread}` : '全部已读'}</span>
          {:else if $activeTopic}
            {@const t = $topics.find(t => t.name === $activeTopic)}
            <h1>{$activeTopic}</h1>
            <span class="sub">
              {$servers.find(s => s.url === t?.server)?.name || t?.server || ''}
              {#if t}· {t.messages.length} 条消息{/if}
            </span>
          {:else}
            <h1>信鸽</h1>
            <span class="sub">订阅主题，接收服务器推送</span>
          {/if}
        </div>
        <MessageActions />
        <button class="btn-primary publish-btn" onclick={() => showPublish = true}>发布</button>
      </header>

      <div class="main-body">
        {#if showSettings}
          <Settings onsaved={() => showSettings = false} />
        {:else if $layoutMode === 'timeline'}
          <Timeline />
        {:else if $activeTopic}
          <TopicDetail />
        {:else}
          <div class="empty">
            <svg class="empty-icon" viewBox="0 0 48 48" fill="none">
              <path d="M24 6a12 12 0 0 0-12 12v6l-4 8h32l-4-8v-6A12 12 0 0 0 24 6Z" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/>
              <path d="M19 36a5 5 0 0 0 10 0" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
            <p>在左侧选择主题</p>
            <p class="hint">或点击「新建订阅」开始接收推送</p>
          </div>
        {/if}
      </div>
    </main>

    {#if showPublish}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="overlay" onclick={(e) => { if (e.target === e.currentTarget) showPublish = false; }}
        onkeydown={(e) => { if (e.key === 'Escape') showPublish = false; }}>
        <div class="modal">
          <PublishView onclose={() => showPublish = false} />
        </div>
      </div>
    {/if}
  </div>
{/if}
