<script lang="ts">
  import { get } from 'svelte/store';
  import { invoke } from '@tauri-apps/api/core';
  import {
    topics, servers, activeTopic, popupOnNotify, popupPosition,
    addTopic, removeTopic, addMessage, setConnected, fmtTime,
  } from './stores/nsfy';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';

  let newTopicName = $state('');
  let newTopicServer = $state('');
  let showAdd = $state(false);

  function focusOnMount(el: HTMLElement) {
    el.focus();
  }

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
      ws.onclose = () => { setConnected(name, server, false); sockets.delete(key); };
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
            // Optional macOS-style banner window, on top of everything,
            // at whichever corner the user picked in Settings.
            if (get(popupOnNotify)) {
              invoke('show_notification_popup', {
                title: msg.title || name,
                body: msg.message,
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
</script>

<div class="page">
  <header>
    <h1>Topics</h1>
    <button class="add-btn" onclick={() => { showAdd = !showAdd; newTopicServer = $servers[0]?.url || ''; }}>
      +
    </button>
  </header>

  {#if showAdd}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="add-form" onkeydown={(e) => {
      if (e.key === 'Escape') { showAdd = false; }
      if (e.key === 'Enter' && newTopicName) {
        addTopic(newTopicServer, newTopicName);
        connectTopic(newTopicServer, newTopicName);
        newTopicName = '';
        showAdd = false;
      }
    }}>
      <select bind:value={newTopicServer}>
        {#each $servers as s}
          <option value={s.url}>{s.name} ({s.url})</option>
        {/each}
      </select>
      <input type="text" placeholder="topic name" bind:value={newTopicName} use:focusOnMount />
      <button class="btn-primary" disabled={!newTopicName} onclick={() => {
        if (!newTopicName) return;
        addTopic(newTopicServer, newTopicName);
        connectTopic(newTopicServer, newTopicName);
        newTopicName = '';
        showAdd = false;
      }}>Subscribe</button>
    </div>
  {/if}

  <div class="topic-list">
    {#if $topics.length === 0}
      <div class="empty">
        <svg class="empty-icon" viewBox="0 0 48 48" fill="none">
          <path d="M24 6a12 12 0 0 0-12 12v6l-4 8h32l-4-8v-6A12 12 0 0 0 24 6Z" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/>
          <path d="M19 36a5 5 0 0 0 10 0" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <p>No topics yet</p>
        <p class="hint">Click + to subscribe to a topic</p>
      </div>
    {:else}
      {#each $topics as t (t.server + '/' + t.name)}
        <div
          class="topic-item" role="button" tabindex="0"
          onclick={() => { $activeTopic = t.name; t.unread = 0; }}
          onkeydown={(e) => { if (e.key === 'Enter') { $activeTopic = t.name; t.unread = 0; } }}
        >
          <div class="topic-info">
            <div class="topic-header">
              <span class="topic-name">{t.name}</span>
              <span class="topic-server">
                {$servers.find(s => s.url === t.server)?.name || t.server}
              </span>
            </div>
            {#if t.messages.length > 0}
              {@const last = t.messages[t.messages.length - 1]}
              <div class="topic-last-msg">{last.title || last.message}</div>
              <div class="topic-time">{fmtTime(last.time)}</div>
            {:else}
              <div class="topic-last-msg dim">No messages yet</div>
            {/if}
          </div>
          <div class="topic-meta">
            <span class="conn-dot" class:online={t.connected}></span>
            {#if t.unread > 0}
              <span class="badge">{t.unread}</span>
            {/if}
          </div>
          <button
            class="unsub-btn" aria-label="Unsubscribe"
            onclick={(e) => { e.stopPropagation(); if (confirm(`Unsubscribe from "${t.name}"? Its message history will be lost.`)) unsubscribe(t.server, t.name); }}
          >
            <svg viewBox="0 0 16 16" fill="none" width="12" height="12"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
          </button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .page {
    display: flex; flex-direction: column; height: 100%;
    padding: 24px; max-width: 700px; margin: 0 auto; width: 100%;
  }
  header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 20px;
  }
  header h1 { font-size: 18px; font-weight: 600; letter-spacing: -0.2px; color: var(--text-1); }
  .add-btn {
    width: 32px; height: 32px; border-radius: var(--r-md);
    border: 1px solid var(--border); background: var(--bg-2); color: var(--text-2);
    font-size: 18px; cursor: pointer; display: flex;
    align-items: center; justify-content: center; transition: all 0.12s;
  }
  .add-btn:hover { background: var(--bg-3); color: var(--text-1); border-color: var(--border-strong); }
  .add-form {
    display: flex; gap: 8px; margin-bottom: 16px; padding: 12px;
    background: var(--bg-2); border-radius: var(--r-lg); border: 1px solid var(--border);
  }
  .add-form select, .add-form input {
    background: var(--bg-1); border: 1px solid var(--border); border-radius: var(--r-sm);
    padding: 8px 12px; color: var(--text-1); font-size: 13px; font-family: inherit;
  }
  .add-form input { flex: 1; }
  .btn-primary {
    padding: 8px 16px; border: none; border-radius: var(--r-sm); background: var(--accent);
    color: var(--accent-ink); font-size: 13px; font-weight: 600; cursor: pointer;
    font-family: inherit; transition: background 0.12s; white-space: nowrap;
  }
  .btn-primary:hover { background: var(--accent-hover); }
  .btn-primary:disabled { background: var(--bg-3); color: var(--text-4); cursor: default; }
  .topic-list { display: flex; flex-direction: column; gap: 4px; flex: 1; overflow-y: auto; }
  .empty {
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; padding: 60px 0; color: var(--text-3); gap: 10px;
  }
  .empty-icon { width: 40px; height: 40px; color: var(--text-4); }
  .hint { font-size: 13px; color: var(--text-4); }
  .topic-item {
    display: flex; align-items: center; gap: 12px; padding: 12px 16px;
    border: 1px solid var(--border); border-radius: var(--r-lg); background: var(--bg-2);
    cursor: pointer; transition: all 0.12s; text-align: left; font-family: inherit;
    color: inherit; width: 100%;
  }
  .topic-item:hover { background: var(--bg-3); border-color: var(--border-strong); }
  .unsub-btn {
    width: 22px; height: 22px; border-radius: var(--r-sm); border: none;
    background: transparent; color: var(--text-4); cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    opacity: 0; transition: all 0.12s; flex-shrink: 0;
  }
  .topic-item:hover .unsub-btn { opacity: 1; }
  .unsub-btn:hover { background: var(--danger-bg); color: var(--danger); }
  .topic-info { flex: 1; min-width: 0; }
  .topic-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .topic-name { font-weight: 600; font-size: 14px; color: var(--text-1); }
  .topic-server {
    font-size: 11px; color: var(--text-3); background: var(--bg-3);
    padding: 1px 6px; border-radius: 4px;
  }
  .topic-last-msg {
    font-size: 13px; color: var(--text-2); white-space: nowrap;
    overflow: hidden; text-overflow: ellipsis;
  }
  .topic-last-msg.dim { color: var(--text-4); }
  .topic-time { font-size: 11px; color: var(--text-3); margin-top: 2px; }
  .topic-meta { display: flex; flex-direction: column; align-items: center; gap: 4px; flex-shrink: 0; }
  .conn-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--border-strong); }
  .conn-dot.online { background: var(--success); }
  .badge {
    background: var(--accent); color: var(--accent-ink); font-size: 10px; font-weight: 700;
    padding: 2px 6px; border-radius: 10px; min-width: 20px; text-align: center;
  }
</style>
