<script lang="ts">
  import { servers, topics } from './stores/nsfy';

  let serverUrl = $state('');
  let topicName = $state('');
  let title = $state('');
  let message = $state('');
  let priority = $state(3);
  let tags = $state('');
  let status = $state<'idle' | 'sending' | 'sent' | 'error'>('idle');
  let statusMsg = $state('');

  const serverTopics = $derived($topics.filter(t => t.server === serverUrl));

  // Set default server on mount
  $effect(() => {
    if (!$servers[0]) return;
    serverUrl = $servers[0].url;
  });

  async function doPublish() {
    if (!message.trim() || !serverUrl) return;
    const t = topicName.trim() || 'default';
    status = 'sending';
    try {
      const body = JSON.stringify({
        title: title.trim(),
        message: message.trim(),
        priority,
        tags: tags.split(',').map(s => s.trim()).filter(Boolean),
      });
      const res = await fetch(`${serverUrl}/${t}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      });
      if (res.ok) {
        status = 'sent'; statusMsg = 'Published';
        message = ''; title = ''; tags = '';
        setTimeout(() => status = 'idle', 2000);
      } else {
        status = 'error'; statusMsg = 'Failed to publish';
      }
    } catch (e) {
      status = 'error'; statusMsg = `Error: ${e}`;
    }
  }
</script>

<div class="page">
  <header><h1>Publish</h1></header>
  <div class="form">
    <div class="row">
      <label for="pub-server">Server</label>
      <select id="pub-server" bind:value={serverUrl}>
        {#each $servers as s}
          <option value={s.url}>{s.name} ({s.url})</option>
        {/each}
      </select>
    </div>
    <div class="row">
      <label for="pub-topic">Topic</label>
      <input id="pub-topic" type="text" list="topic-list" placeholder="topic name" bind:value={topicName} />
      <datalist id="topic-list">
        {#each serverTopics as t}
          <option value={t.name}></option>
        {/each}
      </datalist>
    </div>
    <div class="row">
      <label for="pub-title">Title <span class="opt">optional</span></label>
      <input id="pub-title" type="text" placeholder="Notification title" bind:value={title} />
    </div>
    <div class="row">
      <label for="pub-message">Message</label>
      <textarea id="pub-message" placeholder="What do you want to send?" bind:value={message} rows="4"></textarea>
    </div>
    <div class="row">
      <span class="row-label">Priority</span>
      <div class="priority-row">
        {#each [1,2,3,4,5] as p}
          <button class="pri-btn" class:active={priority === p} onclick={() => priority = p}>{p}</button>
        {/each}
      </div>
    </div>
    <div class="row">
      <label for="pub-tags">Tags <span class="opt">comma-separated</span></label>
      <input id="pub-tags" type="text" placeholder="e.g. backup, db" bind:value={tags} />
    </div>
    <button class="pub-btn" disabled={!message.trim() || status === 'sending'} onclick={doPublish}>
      {status === 'sending' ? 'Sending...' : 'Send Notification'}
    </button>
    {#if status === 'sent'}
      <div class="status-msg success">{statusMsg}</div>
    {:else if status === 'error'}
      <div class="status-msg error">{statusMsg}</div>
    {/if}
  </div>
</div>

<style>
  .page {
    display: flex; flex-direction: column; height: 100%;
    padding: 24px; max-width: 600px; margin: 0 auto; width: 100%;
  }
  header { margin-bottom: 20px; }
  header h1 { font-size: 18px; font-weight: 600; letter-spacing: -0.2px; color: var(--text-1); }
  .form { display: flex; flex-direction: column; gap: 16px; }
  .row { display: flex; flex-direction: column; gap: 6px; }
  label, .row-label { font-size: 11px; font-weight: 600; color: var(--text-3); text-transform: uppercase; letter-spacing: 0.5px; }
  .opt { font-weight: 400; color: var(--text-4); text-transform: none; }
  select, input, textarea {
    background: var(--bg-2); border: 1px solid var(--border); border-radius: var(--r-md);
    padding: 10px 14px; color: var(--text-1); font-size: 14px;
    font-family: inherit; outline: none; transition: border-color 0.12s; resize: vertical;
  }
  select:focus, input:focus, textarea:focus { border-color: var(--accent); }
  .priority-row { display: flex; gap: 6px; }
  .pri-btn {
    width: 38px; height: 34px; border-radius: var(--r-sm);
    border: 1px solid var(--border); background: var(--bg-2); color: var(--text-3);
    font-size: 13px; font-weight: 600; cursor: pointer;
    transition: all 0.12s; font-family: inherit;
  }
  .pri-btn:hover { background: var(--bg-3); color: var(--text-2); }
  .pri-btn.active { background: var(--accent); color: var(--accent-ink); border-color: var(--accent); }
  .pub-btn {
    padding: 11px 24px; border: none; border-radius: var(--r-md);
    background: var(--accent); color: var(--accent-ink); font-size: 14px;
    font-weight: 600; cursor: pointer; transition: background 0.12s;
    font-family: inherit; margin-top: 8px;
  }
  .pub-btn:hover { background: var(--accent-hover); }
  .pub-btn:disabled { background: var(--bg-3); color: var(--text-4); cursor: default; }
  .status-msg { font-size: 13px; padding: 8px 12px; border-radius: var(--r-sm); }
  .status-msg.success { color: var(--success); background: var(--success-bg); }
  .status-msg.error { color: var(--danger); background: var(--danger-bg); }
</style>
