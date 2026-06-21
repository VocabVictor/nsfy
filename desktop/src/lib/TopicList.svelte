<script lang="ts">
  import {
    topics, servers, activeTopic,
    addTopic, addMessage, setConnected, fmtTime,
  } from './stores/nsfy';

  let newTopicName = $state('');
  let newTopicServer = $state('');
  let showAdd = $state(false);

  const sockets = new Map<string, WebSocket>();

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
        } catch {}
      };
      sockets.set(key, ws);
    } catch {}
  }

  function disconnectAll() {
    sockets.forEach(ws => ws.close());
    sockets.clear();
  }

  $effect(() => {
    for (const t of $topics) {
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
    <div class="add-form">
      <select bind:value={newTopicServer}>
        {#each $servers as s}
          <option value={s.url}>{s.name} ({s.url})</option>
        {/each}
      </select>
      <input type="text" placeholder="topic name" bind:value={newTopicName} />
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
        <div class="empty-icon">🔔</div>
        <p>No topics yet</p>
        <p class="hint">Click + to subscribe to a topic</p>
      </div>
    {:else}
      {#each $topics as t (t.server + '/' + t.name)}
        <button class="topic-item" onclick={() => { $activeTopic = t.name; t.unread = 0; }}>
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
        </button>
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
  header h1 { font-size: 22px; font-weight: 700; letter-spacing: -0.3px; }
  .add-btn {
    width: 36px; height: 36px; border-radius: 10px;
    border: 1px solid #2a2a2a; background: #1a1a1a; color: #888;
    font-size: 20px; cursor: pointer; display: flex;
    align-items: center; justify-content: center; transition: all 0.15s;
  }
  .add-btn:hover { background: #252525; color: #a5b4fc; border-color: #3a3a5a; }
  .add-form {
    display: flex; gap: 8px; margin-bottom: 16px; padding: 12px;
    background: #161616; border-radius: 12px; border: 1px solid #1e1e1e;
  }
  .add-form select, .add-form input {
    background: #0d0d0d; border: 1px solid #2a2a2a; border-radius: 8px;
    padding: 8px 12px; color: #e5e5e5; font-size: 13px; font-family: inherit;
  }
  .add-form input { flex: 1; }
  .btn-primary {
    padding: 8px 16px; border: none; border-radius: 8px; background: #6366f1;
    color: white; font-size: 13px; font-weight: 600; cursor: pointer;
    font-family: inherit; transition: background 0.15s; white-space: nowrap;
  }
  .btn-primary:hover { background: #5558e6; }
  .btn-primary:disabled { background: #2a2a3a; color: #555; cursor: default; }
  .topic-list { display: flex; flex-direction: column; gap: 4px; flex: 1; overflow-y: auto; }
  .empty {
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; padding: 60px 0; color: #555; gap: 8px;
  }
  .empty-icon { font-size: 40px; }
  .hint { font-size: 13px; color: #444; }
  .topic-item {
    display: flex; align-items: center; gap: 12px; padding: 12px 16px;
    border: 1px solid transparent; border-radius: 12px; background: #111111;
    cursor: pointer; transition: all 0.15s; text-align: left; font-family: inherit;
    color: inherit; width: 100%;
  }
  .topic-item:hover { background: #18181b; border-color: #27272a; }
  .topic-info { flex: 1; min-width: 0; }
  .topic-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .topic-name { font-weight: 600; font-size: 15px; }
  .topic-server {
    font-size: 11px; color: #555; background: #1a1a1a;
    padding: 1px 6px; border-radius: 4px;
  }
  .topic-last-msg {
    font-size: 13px; color: #888; white-space: nowrap;
    overflow: hidden; text-overflow: ellipsis;
  }
  .topic-last-msg.dim { color: #444; }
  .topic-time { font-size: 11px; color: #555; margin-top: 2px; }
  .topic-meta { display: flex; flex-direction: column; align-items: center; gap: 4px; flex-shrink: 0; }
  .conn-dot { width: 6px; height: 6px; border-radius: 50%; background: #333; }
  .conn-dot.online { background: #22c55e; }
  .badge {
    background: #6366f1; color: white; font-size: 10px; font-weight: 700;
    padding: 2px 6px; border-radius: 10px; min-width: 20px; text-align: center;
  }
</style>
