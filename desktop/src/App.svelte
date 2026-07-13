<script lang="ts">
  import { get } from 'svelte/store';
  import { invoke } from '@tauri-apps/api/core';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
  import TopicDetail from './lib/TopicDetail.svelte';
  import PublishView from './lib/Publish.svelte';
  import Settings from './lib/Settings.svelte';
  import Timeline from './lib/Timeline.svelte';
  import {
    topics, servers, activeTopic, layoutMode, popupOnNotify, popupPosition,
    loadState, addTopic, removeTopic, addMessage, setConnected,
    fmtTime, topicColor,
  } from './lib/stores/nsfy';

  let ready = $state(false);
  let showSettings = $state(false);
  let showPublish = $state(false);
  let showAdd = $state(false);
  let newTopicName = $state('');
  let newTopicServer = $state('');

  $effect(() => {
    loadState();
    ready = true;
  });

  function focusOnMount(el: HTMLElement) {
    el.focus();
  }

  // --- WebSocket connections (top-level: alive regardless of view) ---
  const sockets = new Map<string, WebSocket>();
  let notifyPermission = $state(false);

  (async () => {
    notifyPermission = await isPermissionGranted();
    if (!notifyPermission) {
      notifyPermission = (await requestPermission()) === 'granted';
    }
  })();

  function connectTopic(server: string, name: string) {
    const key = `${server}/${name}`;
    if (sockets.has(key)) return;
    const url = server.replace(/^http/, 'ws') + `/${name}/ws`;
    try {
      const ws = new WebSocket(url);
      ws.onopen = () => setConnected(name, server, true);
      ws.onclose = () => {
        setConnected(name, server, false);
        sockets.delete(key);
        // Reconnect with 5s delay, as long as the topic is still subscribed.
        setTimeout(() => {
          if (get(topics).some(t => t.server === server && t.name === name)) {
            connectTopic(server, name);
          }
        }, 5000);
      };
      ws.onerror = () => { setConnected(name, server, false); };
      ws.onmessage = (e) => {
        try {
          const msg = JSON.parse(e.data);
          addMessage(name, server, msg);
          if (msg.priority >= 4) {
            // Native OS notification, seen even while the window is
            // hidden in the tray.
            if (notifyPermission) {
              sendNotification({ title: msg.title || name, body: msg.message });
            }
            // Optional compact notification-center window, on top of
            // everything, at whichever corner the user picked in Settings.
            if (get(popupOnNotify)) {
              // Latest high-priority message per topic, newest first, max 3.
              const recent = get(topics)
                .flatMap(t => {
                  const hi = t.messages.filter(m => m.priority >= 4);
                  return hi.length ? [{ ...hi[hi.length - 1], topicName: t.name }] : [];
                })
                .sort((a, b) => b.time - a.time)
                .slice(0, 3)
                .map(m => ({
                  title: m.title || m.topicName,
                  body: m.message,
                  time: m.time,
                  priority: m.priority,
                }));
              invoke('show_notification_popup', {
                messages: recent.length ? recent : [{
                  title: msg.title || name, body: msg.message,
                  time: msg.time, priority: msg.priority,
                }],
                position: get(popupPosition),
              }).catch(() => {});
            }
          }
        } catch {}
      };
      sockets.set(key, ws);
    } catch {}
  }

  function disconnectAll() {
    sockets.forEach(ws => ws.close());
    sockets.clear();
  }

  function unsubscribe(server: string, name: string) {
    const key = `${server}/${name}`;
    sockets.get(key)?.close();
    sockets.delete(key);
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
    if (t) t.unread = 0;
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
        <button class="btn-primary publish-btn" onclick={() => showPublish = true}>发布</button>
      </header>

      <div class="main-body">
        {#if showSettings}
          <Settings />
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

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }
  :global(:root) {
    /* Surfaces — light luminance ladder, per the 信鸽 design mockup */
    --bg-0: #f0f4f8;   /* app shell / sidebar */
    --bg-1: #ffffff;   /* content canvas */
    --bg-2: #f6f7f9;   /* cards, inputs */
    --bg-3: #eef2f6;   /* hover state */
    --border: #e5e7eb;
    --border-strong: #d1d5db;

    /* Text — near-black on a light canvas */
    --text-1: #111827;
    --text-2: #4b5563;
    --text-3: #6b7280;
    --text-4: #9ca3af;

    /* One accent — sky blue, matching the mockup. White ink on top. */
    --accent: #0ea5e9;
    --accent-hover: #0284c7;
    --accent-dim: rgba(14, 165, 233, 0.12);
    --accent-ink: #ffffff;

    --success: #22c55e;
    --success-bg: rgba(34, 197, 94, 0.12);
    --danger: #ef4444;
    --danger-bg: rgba(239, 68, 68, 0.12);

    --r-sm: 6px;
    --r-md: 8px;
    --r-lg: 10px;
  }
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'SF Pro Text', system-ui, sans-serif;
    background: var(--bg-1);
    color: var(--text-1);
    overflow: hidden;
    height: 100vh;
    -webkit-font-smoothing: antialiased;
  }
  .splash {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.5px;
    color: var(--text-3);
  }
  .app { display: flex; height: 100vh; }

  /* --- Sidebar --- */
  .sidebar {
    width: 220px; flex-shrink: 0; background: var(--bg-0);
    border-right: 1px solid var(--border);
    display: flex; flex-direction: column; padding: 16px 10px 12px;
  }
  .logo {
    font-size: 15px; font-weight: 700; letter-spacing: 0.5px;
    color: var(--text-1); padding: 0 8px 14px;
  }
  .topic-nav { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 2px; }
  .topic-row {
    display: flex; align-items: center; gap: 8px;
    padding: 7px 8px; border-radius: var(--r-md); cursor: pointer;
    color: var(--text-2); font-size: 13px; transition: background 0.12s;
  }
  .topic-row:hover { background: var(--bg-3); }
  .topic-row.active { background: var(--accent-dim); color: var(--accent-hover); font-weight: 600; }
  .topic-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .topic-row-name {
    flex: 1; min-width: 0; white-space: nowrap;
    overflow: hidden; text-overflow: ellipsis;
  }
  .conn-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--border-strong); flex-shrink: 0; }
  .conn-dot.online { background: var(--success); }
  .badge {
    background: var(--accent); color: var(--accent-ink); font-size: 10px; font-weight: 700;
    padding: 1px 6px; border-radius: 10px; min-width: 18px; text-align: center; flex-shrink: 0;
  }
  .unsub-btn {
    width: 18px; height: 18px; border-radius: var(--r-sm); border: none;
    background: transparent; color: var(--text-4); cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    opacity: 0; transition: all 0.12s; flex-shrink: 0;
  }
  .topic-row:hover .unsub-btn { opacity: 1; }
  .unsub-btn:hover { background: var(--danger-bg); color: var(--danger); }
  .sidebar-empty { padding: 20px 8px; color: var(--text-3); font-size: 13px; }
  .sidebar-empty .hint { font-size: 11px; color: var(--text-4); margin-top: 4px; }
  .timeline-side { flex: 1; padding: 4px 8px; }
  .side-note { font-size: 13px; font-weight: 600; color: var(--text-2); }
  .side-unread { font-size: 12px; color: var(--accent-hover); margin-top: 6px; }

  .add-form {
    display: flex; flex-direction: column; gap: 6px; margin: 8px 0; padding: 10px;
    background: var(--bg-2); border-radius: var(--r-lg); border: 1px solid var(--border);
  }
  .add-form select, .add-form input {
    background: var(--bg-1); border: 1px solid var(--border); border-radius: var(--r-sm);
    padding: 7px 10px; color: var(--text-1); font-size: 12px; font-family: inherit;
    width: 100%;
  }
  .btn-primary {
    padding: 8px 16px; border: none; border-radius: var(--r-sm); background: var(--accent);
    color: var(--accent-ink); font-size: 13px; font-weight: 600; cursor: pointer;
    font-family: inherit; transition: background 0.12s; white-space: nowrap;
  }
  .btn-primary:hover { background: var(--accent-hover); }
  .btn-primary:disabled { background: var(--bg-3); color: var(--text-4); cursor: default; }

  .sidebar-footer {
    display: flex; flex-direction: column; gap: 2px;
    padding-top: 10px; border-top: 1px solid var(--border);
  }
  .foot-btn {
    display: flex; align-items: center; gap: 8px;
    padding: 7px 8px; border: none; border-radius: var(--r-md);
    background: transparent; color: var(--text-2); font-size: 13px;
    cursor: pointer; font-family: inherit; text-align: left; transition: background 0.12s;
  }
  .foot-btn:hover { background: var(--bg-3); color: var(--text-1); }
  .foot-btn.active { background: var(--accent-dim); color: var(--accent-hover); }
  .foot-plus { font-size: 15px; font-weight: 600; width: 14px; text-align: center; }
  .status {
    display: flex; align-items: center; gap: 6px;
    padding: 8px 8px 0; font-size: 11px; color: var(--text-4);
  }
  .dot { width: 5px; height: 5px; border-radius: 50%; background: var(--success); }

  /* --- Main --- */
  .content { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .main-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 14px 24px; border-bottom: 1px solid var(--border); flex-shrink: 0;
  }
  .header-left { display: flex; align-items: baseline; gap: 10px; min-width: 0; }
  .main-header h1 { font-size: 17px; font-weight: 700; letter-spacing: -0.2px; color: var(--text-1); }
  .sub { font-size: 12px; color: var(--text-3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .publish-btn { flex-shrink: 0; }
  .main-body { flex: 1; min-height: 0; overflow: hidden; }

  .empty {
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; height: 100%; color: var(--text-3); gap: 10px;
  }
  .empty-icon { width: 40px; height: 40px; color: var(--text-4); }
  .hint { font-size: 13px; color: var(--text-4); }

  /* --- Publish modal --- */
  .overlay {
    position: fixed; inset: 0; background: rgba(17, 24, 39, 0.4);
    display: flex; align-items: center; justify-content: center; z-index: 50;
  }
  .modal {
    background: var(--bg-1); border-radius: 12px; border: 1px solid var(--border);
    width: 460px; max-width: calc(100vw - 48px); max-height: calc(100vh - 48px);
    overflow-y: auto; box-shadow: 0 20px 50px rgba(17, 24, 39, 0.25);
  }
</style>
