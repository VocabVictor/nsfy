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
        status = 'sent'; statusMsg = '✓ Published';
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
      <label>Server</label>
      <select bind:value={serverUrl}>
        {#each $servers as s}
          <option value={s.url}>{s.name} ({s.url})</option>
        {/each}
      </select>
    </div>
    <div class="row">
      <label>Topic</label>
      <input type="text" list="topic-list" placeholder="topic name" bind:value={topicName} />
      <datalist id="topic-list">
        {#each serverTopics as t}
          <option value={t.name} />
        {/each}
      </datalist>
    </div>
    <div class="row">
      <label>Title <span class="opt">optional</span></label>
      <input type="text" placeholder="Notification title" bind:value={title} />
    </div>
    <div class="row">
      <label>Message</label>
      <textarea placeholder="What do you want to send?" bind:value={message} rows="4"></textarea>
    </div>
    <div class="row">
      <label>Priority</label>
      <div class="priority-row">
        {#each [1,2,3,4,5] as p}
          <button class="pri-btn" class:active={priority === p} onclick={() => priority = p}>{p}</button>
        {/each}
      </div>
    </div>
    <div class="row">
      <label>Tags <span class="opt">comma-separated</span></label>
      <input type="text" placeholder="e.g. backup, db" bind:value={tags} />
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
  header { margin-bottom: 24px; }
  header h1 { font-size: 22px; font-weight: 700; letter-spacing: -0.3px; }
  .form { display: flex; flex-direction: column; gap: 16px; }
  .row { display: flex; flex-direction: column; gap: 6px; }
  label { font-size: 12px; font-weight: 600; color: #888; text-transform: uppercase; letter-spacing: 0.5px; }
  .opt { font-weight: 400; color: #555; text-transform: none; }
  select, input, textarea {
    background: #111111; border: 1px solid #222; border-radius: 10px;
    padding: 10px 14px; color: #e5e5e5; font-size: 14px;
    font-family: inherit; outline: none; transition: border-color 0.15s; resize: vertical;
  }
  select:focus, input:focus, textarea:focus { border-color: #6366f1; }
  .priority-row { display: flex; gap: 6px; }
  .pri-btn {
    width: 40px; height: 36px; border-radius: 8px;
    border: 1px solid #2a2a2a; background: #111111; color: #888;
    font-size: 14px; font-weight: 600; cursor: pointer;
    transition: all 0.15s; font-family: inherit;
  }
  .pri-btn:hover { background: #1a1a1a; color: #ccc; }
  .pri-btn.active { background: #6366f1; color: white; border-color: #6366f1; }
  .pub-btn {
    padding: 12px 24px; border: none; border-radius: 10px;
    background: #6366f1; color: white; font-size: 15px;
    font-weight: 600; cursor: pointer; transition: background 0.15s;
    font-family: inherit; margin-top: 8px;
  }
  .pub-btn:hover { background: #5558e6; }
  .pub-btn:disabled { background: #2a2a3a; color: #555; cursor: default; }
  .status-msg { font-size: 13px; padding: 8px 12px; border-radius: 8px; }
  .status-msg.success { color: #22c55e; background: #0a2a0a; }
  .status-msg.error { color: #ef4444; background: #2a0a0a; }
</style>
