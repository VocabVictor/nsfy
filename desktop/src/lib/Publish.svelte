<script lang="ts">
  import { servers, topics } from './stores/nsfy';

  let { onclose }: { onclose?: () => void } = $props();

  let serverUrl = $state('');
  let topicName = $state('');
  let title = $state('');
  let message = $state('');
  let priority = $state(3);
  let scheduleAt = $state('');
  let attachName = $state('');
  let status = $state<'idle' | 'sending' | 'scheduled' | 'sent' | 'error'>('idle');
  let statusMsg = $state('');

  const serverTopics = $derived($topics.filter(t => t.server === serverUrl));

  const PRIORITIES = [
    { value: 5, label: '紧急' },
    { value: 4, label: '高' },
    { value: 3, label: '普通' },
    { value: 1, label: '低' },
  ];

  // Set default server on mount
  $effect(() => {
    if (!$servers[0]) return;
    serverUrl = $servers[0].url;
  });

  function onAttach(e: Event) {
    const input = e.target as HTMLInputElement;
    attachName = input.files?.[0]?.name || '';
  }

  async function post() {
    const t = topicName.trim() || 'default';
    const tags = attachName ? [`附件:${attachName}`] : [];
    const body = JSON.stringify({
      title: title.trim(),
      message: message.trim(),
      priority,
      tags,
    });
    const res = await fetch(`${serverUrl}/${t}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    if (!res.ok) throw new Error(`server returned ${res.status}`);
  }

  async function doPublish() {
    if (!message.trim() || !serverUrl) return;
    const delay = scheduleAt ? new Date(scheduleAt).getTime() - Date.now() : 0;
    if (delay > 0) {
      // Simple client-side scheduling: fires while the app stays open.
      status = 'scheduled';
      statusMsg = `已定时，${new Date(scheduleAt).toLocaleString('zh-CN')} 发送`;
      setTimeout(() => { post().catch(() => {}); }, delay);
      setTimeout(() => { status = 'idle'; onclose?.(); }, 1500);
      return;
    }
    status = 'sending';
    statusMsg = '发布中…';
    try {
      await post();
      status = 'sent'; statusMsg = '已发布';
      message = ''; title = ''; attachName = ''; scheduleAt = '';
      setTimeout(() => { status = 'idle'; onclose?.(); }, 1200);
    } catch (e) {
      status = 'error'; statusMsg = '发布失败';
    }
  }
</script>

<div class="page">
  <header><h1>发布消息</h1></header>
  <div class="form">
    <div class="row">
      <label for="pub-topic">主题</label>
      <div class="topic-row">
        <select id="pub-server" aria-label="服务器" bind:value={serverUrl}>
          {#each $servers as s}
            <option value={s.url}>{s.name} — {s.url}</option>
          {/each}
        </select>
        <input id="pub-topic" type="text" list="topic-list" placeholder="主题名" bind:value={topicName} />
        <datalist id="topic-list">
          {#each serverTopics as t}
            <option value={t.name}></option>
          {/each}
        </datalist>
      </div>
    </div>
    <div class="row">
      <label for="pub-title">标题</label>
      <input id="pub-title" type="text" placeholder="一句话说明发生了什么" bind:value={title} />
    </div>
    <div class="row">
      <label for="pub-message">内容</label>
      <textarea id="pub-message" placeholder="磁盘清理脚本已执行，/var 回落至 71%…" bind:value={message} rows="4"></textarea>
    </div>
    <div class="row">
      <span class="row-label">优先级</span>
      <div class="priority-row">
        {#each PRIORITIES as p}
          <button
            class="pri-btn" class:active={priority === p.value}
            class:danger={p.value === 5 && priority === 5}
            onclick={() => priority = p.value}
          >{p.label}</button>
        {/each}
      </div>
    </div>
    <div class="row inline-row">
      <div class="inline-field">
        <label for="pub-schedule">定时发送</label>
        <input id="pub-schedule" type="datetime-local" bind:value={scheduleAt} />
      </div>
      <div class="inline-field">
        <span class="row-label">附件</span>
        <label class="attach-btn">
          <svg viewBox="0 0 16 16" fill="none" width="13" height="13"><path d="M13 7.5 8.2 12.3a3.2 3.2 0 0 1-4.5-4.5L8.5 3a2.1 2.1 0 0 1 3 3l-4.6 4.6a1 1 0 0 1-1.5-1.5L9.6 5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/></svg>
          {attachName || '选择文件'}
          <input type="file" onchange={onAttach} hidden />
        </label>
      </div>
    </div>

    <div class="actions">
      {#if status !== 'idle'}
        <span class="status" class:err={status === 'error'}>{statusMsg}</span>
      {/if}
      <div class="spacer"></div>
      {#if onclose}
        <button class="btn-ghost" onclick={onclose}>取消</button>
      {/if}
      <button class="btn-primary" disabled={!message.trim() || status === 'sending'} onclick={doPublish}>
        发布
      </button>
    </div>
  </div>
</div>

<style>
  .page { padding: 22px 24px; }
  header { margin-bottom: 18px; }
  header h1 { font-size: 16px; font-weight: 700; letter-spacing: -0.2px; color: var(--text-1); }
  .form { display: flex; flex-direction: column; gap: 14px; }
  .row { display: flex; flex-direction: column; gap: 6px; }
  label, .row-label { font-size: 12px; font-weight: 600; color: var(--text-2); }
  .topic-row { display: flex; gap: 8px; }
  .topic-row select { flex: 1.4; min-width: 0; }
  .topic-row input { flex: 1; min-width: 0; }
  select, input, textarea {
    background: var(--bg-2); border: 1px solid var(--border); border-radius: var(--r-md);
    padding: 9px 12px; color: var(--text-1); font-size: 13px;
    font-family: inherit; outline: none; transition: border-color 0.12s; width: 100%;
  }
  select:focus, input:focus, textarea:focus { border-color: var(--accent); }
  textarea { resize: vertical; line-height: 1.5; }
  ::placeholder { color: var(--text-4); }

  .priority-row { display: flex; gap: 8px; }
  .pri-btn {
    flex: 1; padding: 8px 0; border-radius: var(--r-md); border: 1px solid var(--border);
    background: var(--bg-2); color: var(--text-2); font-size: 13px; cursor: pointer;
    font-family: inherit; transition: all 0.12s;
  }
  .pri-btn:hover { border-color: var(--border-strong); }
  .pri-btn.active {
    background: var(--accent); border-color: var(--accent);
    color: var(--accent-ink); font-weight: 600;
  }
  .pri-btn.danger { background: var(--danger); border-color: var(--danger); }

  .inline-row { flex-direction: row; gap: 12px; }
  .inline-field { flex: 1; display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .attach-btn {
    display: flex; align-items: center; gap: 6px;
    background: var(--bg-2); border: 1px solid var(--border); border-radius: var(--r-md);
    padding: 9px 12px; color: var(--text-2); font-size: 13px; cursor: pointer;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .attach-btn:hover { border-color: var(--border-strong); }

  .actions { display: flex; align-items: center; gap: 10px; margin-top: 4px; }
  .status { font-size: 12px; color: var(--success); }
  .status.err { color: var(--danger); }
  .spacer { flex: 1; }
  .btn-ghost {
    padding: 9px 18px; border-radius: var(--r-md); border: 1px solid var(--border);
    background: var(--bg-1); color: var(--text-2); font-size: 13px; cursor: pointer;
    font-family: inherit; transition: all 0.12s;
  }
  .btn-ghost:hover { background: var(--bg-3); }
  .btn-primary {
    padding: 9px 22px; border: none; border-radius: var(--r-md); background: var(--accent);
    color: var(--accent-ink); font-size: 13px; font-weight: 600; cursor: pointer;
    font-family: inherit; transition: background 0.12s;
  }
  .btn-primary:hover { background: var(--accent-hover); }
  .btn-primary:disabled { background: var(--bg-3); color: var(--text-4); cursor: default; }
</style>
