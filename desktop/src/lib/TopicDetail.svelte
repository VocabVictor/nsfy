<script lang="ts">
  import {
    topics, servers, activeTopic,
    markRead, fmtTime, priorityColor, type Message
  } from './stores/nsfy';

  let { onback }: { onback: () => void } = $props();

  const topic = $derived($topics.find(t => t.name === $activeTopic));
  const serverUrl = $derived(topic?.server || '');
  const serverName = $derived($servers.find(s => s.url === serverUrl)?.name || '');

  let newMsg = $state('');
  let replyTitle = $state('');

  $effect(() => {
    if ($activeTopic) markRead($activeTopic);
  });

  async function doPublish() {
    if (!newMsg.trim() || !serverUrl || !$activeTopic) return;
    try {
      await fetch(`${serverUrl}/${$activeTopic}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title: replyTitle, message: newMsg.trim(), priority: 3 }),
      });
      newMsg = '';
      replyTitle = '';
    } catch (e) {
      console.error('publish failed', e);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      doPublish();
    }
  }
</script>

<div class="page">
  <header class="detail-header">
    <button class="back-btn" onclick={onback}>←</button>
    <div class="header-info">
      <h1>{$activeTopic}</h1>
      <span class="server-tag">{serverName}</span>
    </div>
    <div class="conn-status" class:online={topic?.connected}>
      {topic?.connected ? '● live' : '○ offline'}
    </div>
  </header>

  <div class="msg-list">
    {#if !topic || topic.messages.length === 0}
      <div class="empty">
        <p>No messages yet</p>
        <p class="hint">Publish one below or wait for incoming</p>
      </div>
    {:else}
      {#each topic.messages as msg (msg.id)}
        <div class="msg-card">
          <div class="msg-meta">
            <span class="msg-time">{fmtTime(msg.time)}</span>
            {#if msg.priority >= 4}
              <span class="msg-priority" style="color:{priorityColor(msg.priority)}">
                {msg.priority >= 5 ? '⚡⚡' : '⚡'}
              </span>
            {/if}
          </div>
          {#if msg.title}
            <div class="msg-title">{msg.title}</div>
          {/if}
          <div class="msg-body">{msg.message}</div>
          {#if msg.tags.length > 0}
            <div class="msg-tags">
              {#each msg.tags as tag}
                <span class="tag">{tag}</span>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <div class="input-bar">
    <input class="title-input" type="text" placeholder="Title (optional)" bind:value={replyTitle} />
    <div class="msg-row">
      <input
        class="msg-input" type="text" placeholder="Type a message..."
        bind:value={newMsg} onkeydown={handleKeydown}
      />
      <button class="send-btn" disabled={!newMsg.trim()} onclick={doPublish}>↑</button>
    </div>
  </div>
</div>

<style>
  .page {
    display: flex; flex-direction: column; height: 100%;
    padding: 24px; max-width: 700px; margin: 0 auto; width: 100%;
  }
  .detail-header { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; }
  .back-btn {
    width: 32px; height: 32px; border-radius: 8px; border: 1px solid #2a2a2a;
    background: #1a1a1a; color: #888; font-size: 16px; cursor: pointer;
    display: flex; align-items: center; justify-content: center; font-family: inherit;
  }
  .back-btn:hover { background: #252525; color: #ccc; }
  .header-info { flex: 1; }
  .header-info h1 { font-size: 20px; font-weight: 700; letter-spacing: -0.3px; }
  .server-tag { font-size: 11px; color: #555; }
  .conn-status { font-size: 11px; color: #555; display: flex; align-items: center; gap: 4px; }
  .conn-status.online { color: #22c55e; }
  .msg-list {
    flex: 1; overflow-y: auto; display: flex;
    flex-direction: column; gap: 4px; padding-right: 4px;
  }
  .empty {
    display: flex; flex-direction: column; align-items: center;
    padding: 40px 0; color: #555; gap: 4px;
  }
  .hint { font-size: 13px; color: #444; }
  .msg-card {
    padding: 12px 16px; border-radius: 10px; background: #111111;
    border: 1px solid #1a1a1a; transition: border-color 0.15s;
  }
  .msg-card:hover { border-color: #27272a; }
  .msg-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .msg-time { font-size: 11px; color: #555; }
  .msg-priority { font-size: 12px; }
  .msg-title { font-weight: 600; font-size: 14px; margin-bottom: 3px; color: #d4d4d8; }
  .msg-body {
    font-size: 14px; color: #a1a1aa; line-height: 1.5;
    white-space: pre-wrap; word-break: break-word;
  }
  .msg-tags { display: flex; gap: 4px; margin-top: 6px; }
  .tag {
    font-size: 10px; padding: 2px 6px; border-radius: 4px;
    background: #1e1e2e; color: #7171a6;
  }
  .input-bar { margin-top: 16px; display: flex; flex-direction: column; gap: 8px; }
  .title-input {
    background: #111111; border: 1px solid #222; border-radius: 10px;
    padding: 8px 12px; color: #e5e5e5; font-size: 13px;
    font-family: inherit; outline: none; transition: border-color 0.15s;
  }
  .title-input:focus { border-color: #3a3a5a; }
  .title-input::placeholder { color: #444; }
  .msg-row { display: flex; gap: 8px; align-items: center; }
  .msg-input {
    flex: 1; background: #111111; border: 1px solid #222; border-radius: 10px;
    padding: 10px 14px; color: #e5e5e5; font-size: 14px;
    font-family: inherit; outline: none; transition: border-color 0.15s;
  }
  .msg-input:focus { border-color: #6366f1; }
  .msg-input::placeholder { color: #444; }
  .send-btn {
    width: 40px; height: 40px; border-radius: 10px; border: none;
    background: #6366f1; color: white; font-size: 20px; cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    transition: background 0.15s; font-family: inherit;
  }
  .send-btn:hover { background: #5558e6; }
  .send-btn:disabled { background: #2a2a3a; color: #555; cursor: default; }
</style>
