<script lang="ts">
  import { servers, topics } from './stores/nsfy';
  import { postMessage } from './post-json';

  let { onclose }: { onclose?: () => void } = $props();

  let serverUrl = $state('');
  let topicName = $state('');
  let title = $state('');
  let message = $state('');
  let categoryPath = $state('');
  let priority = $state(3);
  let popup = $state(false);
  let bypassDnd = $state(false);
  let status = $state<'idle' | 'sending' | 'sent' | 'error'>('idle');
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

  async function post() {
    const t = topicName.trim() || 'default';
    const body = {
      title: title.trim(),
      message: message.trim(),
      priority,
      tags: [],
      category: categoryPath.split('/').map(s => s.trim()).filter(Boolean),
      popup,
      bypassDnd,
    };
    await postMessage(serverUrl, t, body);
  }

  async function doPublish() {
    if (!message.trim() || !serverUrl) return;
    status = 'sending';
    statusMsg = '发布中…';
    try {
      await post();
      status = 'sent'; statusMsg = '已发布';
      message = ''; title = ''; categoryPath = ''; popup = false; bypassDnd = false;
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
            <option value={s.url}>{s.name}</option>
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
      <label for="pub-category">多级分类</label>
      <input id="pub-category" type="text" placeholder="工作/Agent/Codex" bind:value={categoryPath} />
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
    <div class="delivery-row">
      <label><input type="checkbox" checked={popup}
        onchange={(event) => { popup = event.currentTarget.checked; if (!popup) bypassDnd = false; }} /> 弹窗通知</label>
      <label class:disabled={!popup}>
        <input type="checkbox" bind:checked={bypassDnd} disabled={!popup} /> 无视勿扰模式
      </label>
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
  .delivery-row { display: flex; gap: 20px; padding: 2px 0; }
  .delivery-row label { display: flex; align-items: center; gap: 7px; cursor: pointer; }
  .delivery-row input { width: 15px; padding: 0; accent-color: var(--accent); }
  .delivery-row label.disabled { color: var(--text-4); cursor: default; }

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
