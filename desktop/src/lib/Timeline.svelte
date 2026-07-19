<script lang="ts">
  import {
    topics, activeTopic, layoutMode, markRead,
    markMessagesRead, clearMessages, messageKey,
    fmtTime, dateGroup, topicColor, priorityColor, priorityLabel,
    categoryOptions, matchesCategory,
    type Message,
  } from './stores/nsfy';

  type TimelineItem = Message & { topicName: string; server: string };

  let filter = $state<'all' | 'unread'>('all');
  let selectedCategory = $state('');
  let selecting = $state(false);
  let selected = $state<string[]>([]);

  // Flatten all topics' messages into one stream, newest first.
  const items = $derived(
    $topics
      .flatMap(t => t.messages.map(m => ({ ...m, topicName: t.name, server: t.server })))
      .sort((a, b) => b.time - a.time)
  );

  const unreadCount = $derived($topics.reduce((n, t) => n + t.unread, 0));
  const categoryChoices = $derived(categoryOptions(items));

  const visible = $derived(
    (filter === 'all' ? items : items.filter(m => !m.read))
      .filter(m => matchesCategory(m, selectedCategory))
  );
  const selectedSet = $derived(new Set(selected));
  const selectedItems = $derived(visible.filter(item => selectedSet.has(itemKey(item))));
  const allVisibleSelected = $derived(visible.length > 0 && selectedItems.length === visible.length);

  // Group into 今天 / 昨天 / 更早, preserving order.
  const groups = $derived.by(() => {
    const out: { label: string; items: TimelineItem[] }[] = [];
    for (const m of visible) {
      const label = dateGroup(m.time);
      const last = out[out.length - 1];
      if (last && last.label === label) last.items.push(m);
      else out.push({ label, items: [m] });
    }
    return out;
  });

  function itemKey(item: TimelineItem) {
    return messageKey({ server: item.server, topic: item.topicName, id: item.id });
  }

  function refs(items: TimelineItem[]) {
    return items.map(item => ({ server: item.server, topic: item.topicName, id: item.id }));
  }

  function toggle(item: TimelineItem) {
    const key = itemKey(item);
    selected = selected.includes(key) ? selected.filter(value => value !== key) : [...selected, key];
  }

  function toggleAll() {
    const visibleKeys = visible.map(itemKey);
    selected = allVisibleSelected
      ? selected.filter(key => !visibleKeys.includes(key))
      : [...new Set([...selected, ...visibleKeys])];
  }

  function readSelected() {
    markMessagesRead(refs(selectedItems));
    selected = [];
  }

  function deleteSelected() {
    if (selectedItems.length && confirm(`将选中的 ${selectedItems.length} 条消息移入回收站？`)) {
      clearMessages(refs(selectedItems));
      selected = [];
    }
  }

  function openTopic(item: TimelineItem) {
    if (selecting) {
      toggle(item);
      return;
    }
    markRead(item.topicName, item.server);
    $layoutMode = 'split';
    $activeTopic = item.topicName;
  }

</script>

<div class="page">
  <div class="filter-bar">
    <button class="chip" class:active={filter === 'all'} onclick={() => filter = 'all'}>全部</button>
    <button class="chip" class:active={filter === 'unread'} onclick={() => filter = 'unread'}>
      未读{#if unreadCount > 0}&nbsp;{unreadCount}{/if}
    </button>
    <select class="category-select" bind:value={selectedCategory} aria-label="按分类筛选">
      <option value="">全部分类</option>
      {#each categoryChoices as item}
        <option value={item.path}>{'—'.repeat(item.depth - 1)} {item.path.split('/').at(-1)}</option>
      {/each}
    </select>
    <div class="spacer"></div>
    {#if selecting}
      <button class="link-btn" onclick={toggleAll}>{allVisibleSelected ? '取消全选' : '全选'}</button>
      <button class="link-btn" disabled={!selectedItems.length} onclick={readSelected}>已读所选</button>
      <button class="link-btn danger" disabled={!selectedItems.length} onclick={deleteSelected}>移入回收站</button>
      <button class="link-btn" onclick={() => { selecting = false; selected = []; }}>完成</button>
    {:else}
      <button class="link-btn" disabled={!visible.length} onclick={() => selecting = true}>批量管理</button>
    {/if}
  </div>

  <div class="stream">
    {#if visible.length === 0}
      <div class="empty">
        <p>{filter === 'unread' ? '没有未读消息' : '暂无消息'}</p>
        <p class="hint">{filter === 'unread' ? '' : '订阅主题后，推送会按时间线汇总在这里'}</p>
      </div>
    {:else}
      {#each groups as g (g.label)}
        <div class="group-label">{g.label}</div>
        {#each g.items as msg (itemKey(msg))}
          <div
            class="msg-card" class:unread={!msg.read} class:selected={selectedSet.has(itemKey(msg))}
            role="button" tabindex="0" onclick={() => openTopic(msg)}
            onkeydown={(event) => { if (event.key === 'Enter' || event.key === ' ') openTopic(msg); }}
          >
            <div class="msg-meta">
              {#if selecting}
                <input
                  type="checkbox" checked={selectedSet.has(itemKey(msg))}
                  aria-label={`选择 ${msg.title || msg.message}`}
                  onclick={(event) => event.stopPropagation()} onchange={() => toggle(msg)}
                />
              {/if}
              <button class="topic-tag" style="color:{topicColor(msg.topicName)}"
                onclick={(event) => { event.stopPropagation(); openTopic(msg); }}>
                <span class="tag-dot" style="background:{topicColor(msg.topicName)}"></span>
                {msg.topicName}
              </button>
              {#if msg.priority >= 4}
                <span class="msg-priority" style="color:{priorityColor(msg.priority)}">
                  {priorityLabel(msg.priority)}
                </span>
              {/if}
              <span class="msg-time">{fmtTime(msg.time)}</span>
            </div>
            {#if msg.category?.length}
              <div class="category-path">{msg.category.join(' › ')}</div>
            {/if}
            {#if msg.title}
              <div class="msg-title">{msg.title}</div>
            {/if}
            <div class="msg-body">{msg.message}</div>
          </div>
        {/each}
      {/each}
    {/if}
  </div>
</div>

<style>
  .page {
    display: flex; flex-direction: column; height: 100%;
    padding: 16px 24px 24px; width: 100%;
  }
  .filter-bar { display: flex; align-items: center; gap: 6px; margin-bottom: 14px; }
  .chip {
    padding: 5px 14px; border-radius: 999px; border: 1px solid var(--border);
    background: var(--bg-2); color: var(--text-2); font-size: 12px; cursor: pointer;
    font-family: inherit; transition: all 0.12s;
  }
  .chip:hover { background: var(--bg-3); }
  .chip.active {
    background: var(--accent-dim); border-color: var(--accent);
    color: var(--accent-hover); font-weight: 600;
  }
  .category-select {
    max-width: 230px; padding: 5px 9px; border-radius: 999px;
    border: 1px solid var(--border); background: var(--bg-2); color: var(--text-2);
    font: inherit; font-size: 11px;
  }
  .spacer { flex: 1; }
  .link-btn {
    border: none; background: transparent; color: var(--accent-hover);
    font-size: 12px; cursor: pointer; font-family: inherit;
  }
  .link-btn:hover { text-decoration: underline; }
  .link-btn:disabled { color: var(--text-4); cursor: default; text-decoration: none; }
  .link-btn.danger { color: var(--danger); }

  .stream { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 4px; padding-right: 4px; }
  .group-label {
    font-size: 11px; font-weight: 600; color: var(--text-3);
    padding: 12px 2px 4px; letter-spacing: 0.3px;
  }
  .group-label:first-child { padding-top: 0; }

  .msg-card {
    padding: 12px 16px; border-radius: var(--r-lg); background: var(--bg-2);
    border: 1px solid var(--border); transition: border-color 0.12s;
    content-visibility: auto; contain-intrinsic-size: auto 72px;
  }
  .msg-card:hover { border-color: var(--border-strong); }
  .msg-card.unread { border-left: 3px solid var(--accent); padding-left: 14px; }
  .msg-card.selected { background: var(--accent-dim); border-color: var(--accent); }
  .msg-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .topic-tag {
    display: flex; align-items: center; gap: 5px;
    border: none; background: transparent; font-size: 11px; font-weight: 600;
    cursor: pointer; font-family: inherit; padding: 0;
  }
  .topic-tag:hover { text-decoration: underline; }
  .tag-dot { width: 7px; height: 7px; border-radius: 50%; }
  .msg-priority { font-size: 11px; font-weight: 600; }
  .msg-time { font-size: 11px; color: var(--text-3); margin-left: auto; }
  .msg-title { font-weight: 600; font-size: 14px; margin-bottom: 3px; color: var(--text-1); }
  .category-path { font-size: 10px; color: var(--accent-hover); margin-bottom: 4px; }
  .msg-body {
    font-size: 14px; color: var(--text-2); line-height: 1.5;
    white-space: pre-wrap; word-break: break-word;
  }
  .empty {
    display: flex; flex-direction: column; align-items: center;
    padding: 60px 0; color: var(--text-3); gap: 6px;
  }
  .hint { font-size: 13px; color: var(--text-4); }
</style>
