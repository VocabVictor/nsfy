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
    <button class="back-btn" onclick={onback} aria-label="Back">
      <svg viewBox="0 0 16 16" fill="none" width="15" height="15"><path d="M10 3 5 8l5 5" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>
    </button>
    <div class="header-info">
      <h1>{$activeTopic}</h1>
      <span class="server-tag">{serverName}</span>
    </div>
    <div class="conn-status" class:online={topic?.connected}>
      <span class="conn-dot"></span>
      {topic?.connected ? 'live' : 'offline'}
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
                <svg viewBox="0 0 12 12" fill="currentColor" width="11" height="11"><path d="M6.5 1 2 6.8h2.6L4 11l4.5-5.8H5.9L6.5 1Z"/></svg>
                {msg.priority >= 5 ? 'urgent' : 'high'}
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
      <button class="send-btn" disabled={!newMsg.trim()} onclick={doPublish} aria-label="Send">
        <svg viewBox="0 0 20 20" fill="none" width="17" height="17"><path d="M10 16V4M10 4L5 9M10 4l5 5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>
      </button>
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
    width: 30px; height: 30px; border-radius: var(--r-md); border: 1px solid var(--border);
    background: var(--bg-2); color: var(--text-2); cursor: pointer;
    display: flex; align-items: center; justify-content: center; font-family: inherit; transition: all 0.12s;
  }
  .back-btn:hover { background: var(--bg-3); color: var(--text-1); border-color: var(--border-strong); }
  .header-info { flex: 1; }
  .header-info h1 { font-size: 16px; font-weight: 600; letter-spacing: -0.2px; color: var(--text-1); }
  .server-tag { font-size: 11px; color: var(--text-3); }
  .conn-status { font-size: 11px; color: var(--text-3); display: flex; align-items: center; gap: 5px; }
  .conn-status.online { color: var(--success); }
  .conn-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--text-4); }
  .conn-status.online .conn-dot { background: var(--success); }
  .msg-list {
    flex: 1; overflow-y: auto; display: flex;
    flex-direction: column; gap: 4px; padding-right: 4px;
  }
  .empty {
    display: flex; flex-direction: column; align-items: center;
    padding: 40px 0; color: var(--text-3); gap: 4px;
  }
  .hint { font-size: 13px; color: var(--text-4); }
  .msg-card {
    padding: 12px 16px; border-radius: var(--r-lg); background: var(--bg-2);
    border: 1px solid var(--border); transition: border-color 0.12s;
  }
  .msg-card:hover { border-color: var(--border-strong); }
  .msg-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .msg-time { font-size: 11px; color: var(--text-3); }
  .msg-priority { font-size: 11px; display: flex; align-items: center; gap: 3px; font-weight: 600; }
  .msg-title { font-weight: 600; font-size: 14px; margin-bottom: 3px; color: var(--text-1); }
  .msg-body {
    font-size: 14px; color: var(--text-2); line-height: 1.5;
    white-space: pre-wrap; word-break: break-word;
  }
  .msg-tags { display: flex; gap: 4px; margin-top: 6px; }
  .tag {
    font-size: 10px; padding: 2px 6px; border-radius: 4px;
    background: var(--accent-dim); color: var(--accent);
  }
  .input-bar { margin-top: 16px; display: flex; flex-direction: column; gap: 8px; }
  .title-input {
    background: var(--bg-2); border: 1px solid var(--border); border-radius: var(--r-md);
    padding: 8px 12px; color: var(--text-1); font-size: 13px;
    font-family: inherit; outline: none; transition: border-color 0.12s;
  }
  .title-input:focus { border-color: var(--border-strong); }
  .title-input::placeholder { color: var(--text-4); }
  .msg-row { display: flex; gap: 8px; align-items: center; }
  .msg-input {
    flex: 1; background: var(--bg-2); border: 1px solid var(--border); border-radius: var(--r-md);
    padding: 10px 14px; color: var(--text-1); font-size: 14px;
    font-family: inherit; outline: none; transition: border-color 0.12s;
  }
  .msg-input:focus { border-color: var(--accent); }
  .msg-input::placeholder { color: var(--text-4); }
  .send-btn {
    width: 38px; height: 38px; border-radius: var(--r-md); border: none;
    background: var(--accent); color: var(--accent-ink); cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    transition: background 0.12s; font-family: inherit; flex-shrink: 0;
  }
  .send-btn:hover { background: var(--accent-hover); }
  .send-btn:disabled { background: var(--bg-3); color: var(--text-4); cursor: default; }
</style>
