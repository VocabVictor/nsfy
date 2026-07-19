<script lang="ts">
  import {
    topics, activeTopic,
    markRead, clearTopicMessages, fmtTime, priorityColor, priorityLabel, authHeaders,
    categoryOptions, matchesCategory,
  } from './stores/nsfy';

  const topic = $derived($topics.find(t => t.name === $activeTopic));
  const serverUrl = $derived(topic?.server || '');

  let newMsg = $state('');
  let replyTitle = $state('');
  let replyCategory = $state('');
  let selectedCategory = $state('');

  const categoryChoices = $derived(categoryOptions(topic?.messages || []));
  const visibleMessages = $derived(
    (topic?.messages || []).filter(msg => matchesCategory(msg, selectedCategory))
  );

  $effect(() => {
    if ($activeTopic && topic) markRead($activeTopic, topic.server);
  });

  function clearTopic() {
    if (topic && confirm(`将「${topic.name}」的消息移入回收站？`)) {
      clearTopicMessages(topic.server, topic.name);
    }
  }

  async function doPublish() {
    if (!newMsg.trim() || !serverUrl || !$activeTopic) return;
    try {
      await fetch(`${serverUrl}/${$activeTopic}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...authHeaders(serverUrl) },
        body: JSON.stringify({
          title: replyTitle,
          message: newMsg.trim(),
          priority: 3,
          category: replyCategory.split('/').map(s => s.trim()).filter(Boolean),
        }),
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
  <div class="detail-toolbar">
    <select class="category-select" bind:value={selectedCategory} aria-label="按分类筛选">
      <option value="">全部分类</option>
      {#each categoryChoices as item}
        <option value={item.path}>{'—'.repeat(item.depth - 1)} {item.path.split('/').at(-1)}</option>
      {/each}
    </select>
    <div class="toolbar-actions">
      <button class="clear-btn" disabled={!topic?.messages.length} onclick={clearTopic}>清空本主题</button>
      <div class="conn-status" class:online={topic?.connected}>
        <span class="conn-dot"></span>
        {topic?.connected ? '已连接' : '离线'}
      </div>
    </div>
  </div>

  <div class="msg-list">
    {#if !topic || visibleMessages.length === 0}
      <div class="empty">
        <p>暂无消息</p>
        <p class="hint">在下方发布，或等待服务器推送</p>
      </div>
    {:else}
      {#each visibleMessages as msg (msg.id)}
        <div class="msg-card">
          <div class="msg-meta">
            <span class="msg-time">{fmtTime(msg.time)}</span>
            {#if msg.priority >= 4}
              <span class="msg-priority" style="color:{priorityColor(msg.priority)}">
                <svg viewBox="0 0 12 12" fill="currentColor" width="11" height="11"><path d="M6.5 1 2 6.8h2.6L4 11l4.5-5.8H5.9L6.5 1Z"/></svg>
                {priorityLabel(msg.priority)}
              </span>
            {/if}
          </div>
          {#if msg.category?.length}
            <div class="category-path">{msg.category.join(' › ')}</div>
          {/if}
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
    <div class="input-meta">
      <input class="title-input" type="text" placeholder="标题（可选）" bind:value={replyTitle} />
      <input class="category-input" type="text" placeholder="分类，如 工作/Agent" bind:value={replyCategory} />
    </div>
    <div class="msg-row">
      <input
        class="msg-input" type="text" placeholder="发送通知…"
        bind:value={newMsg} onkeydown={handleKeydown}
      />
      <button class="send-btn" disabled={!newMsg.trim()} onclick={doPublish} aria-label="发送">
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
  .detail-toolbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
  .category-select {
    max-width: 260px; padding: 5px 9px; border-radius: var(--r-sm);
    border: 1px solid var(--border); background: var(--bg-2); color: var(--text-2);
    font: inherit; font-size: 11px;
  }
  .toolbar-actions { display: flex; align-items: center; gap: 12px; }
  .clear-btn {
    border: none; background: transparent; color: var(--text-3);
    font: inherit; font-size: 11px; cursor: pointer;
  }
  .clear-btn:hover:not(:disabled) { color: var(--danger); }
  .clear-btn:disabled { color: var(--text-4); cursor: default; }
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
    content-visibility: auto; contain-intrinsic-size: auto 72px;
  }
  .msg-card:hover { border-color: var(--border-strong); }
  .msg-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .msg-time { font-size: 11px; color: var(--text-3); }
  .msg-priority { font-size: 11px; display: flex; align-items: center; gap: 3px; font-weight: 600; }
  .msg-title { font-weight: 600; font-size: 14px; margin-bottom: 3px; color: var(--text-1); }
  .category-path { font-size: 10px; color: var(--accent-hover); margin-bottom: 4px; }
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
  .input-meta { display: flex; gap: 8px; }
  .title-input, .category-input {
    flex: 1;
    background: var(--bg-2); border: 1px solid var(--border); border-radius: var(--r-md);
    padding: 8px 12px; color: var(--text-1); font-size: 13px;
    font-family: inherit; outline: none; transition: border-color 0.12s;
  }
  .title-input:focus, .category-input:focus { border-color: var(--border-strong); }
  .title-input::placeholder, .category-input::placeholder { color: var(--text-4); }
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
