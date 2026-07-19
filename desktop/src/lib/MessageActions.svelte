<script lang="ts">
  import {
    clearAllMessages, discardTrash, doNotDisturb, emptyTrash, markAllRead,
    restoreTrashMessages, topics, trash, trashRef, type TrashMessage,
    toggleDoNotDisturb,
  } from './stores/nsfy';

  let showTrash = $state(false);
  const unread = $derived($topics.reduce((sum, topic) => sum + topic.unread, 0));
  const messageCount = $derived($topics.reduce((sum, topic) => sum + topic.messages.length, 0));

  function clearAll() {
    if (confirm('将桌面端的全部消息移入回收站？此操作不会删除服务器上的消息。')) {
      clearAllMessages();
    }
  }

  function restore(message: TrashMessage) {
    restoreTrashMessages([trashRef(message)]);
  }

  function restoreAll() {
    restoreTrashMessages($trash.map(trashRef));
  }

  function discard(message: TrashMessage) {
    if (confirm('永久删除这条消息？删除后无法恢复。')) discardTrash([trashRef(message)]);
  }

  function discardAll() {
    if (confirm('永久清空回收站？删除后无法恢复。')) emptyTrash();
  }
</script>

<div class="actions">
  <button class:active={$doNotDisturb} title="快捷键：Ctrl+Alt+D" onclick={toggleDoNotDisturb}>
    {$doNotDisturb ? '勿扰中' : '勿扰'}
  </button>
  <button disabled={unread === 0} onclick={markAllRead}>全部已读</button>
  <button onclick={() => showTrash = true}>回收站{#if $trash.length} {$trash.length}{/if}</button>
  <button class="clear" disabled={messageCount === 0} onclick={clearAll}>清空全部</button>
</div>

{#if showTrash}
  <div class="overlay" role="presentation" onclick={(event) => {
    if (event.target === event.currentTarget) showTrash = false;
  }}>
    <dialog open class="trash-panel" aria-label="回收站">
      <header>
        <div>
          <h2>回收站</h2>
          <p>消息仅从这台电脑隐藏，服务器内容不受影响</p>
        </div>
        <button class="close" aria-label="关闭回收站" onclick={() => showTrash = false}>×</button>
      </header>

      <div class="trash-toolbar">
        <span>{$trash.length} 条消息</span>
        <div class="spacer"></div>
        <button disabled={!$trash.length} onclick={restoreAll}>全部恢复</button>
        <button class="danger" disabled={!$trash.length} onclick={discardAll}>清空回收站</button>
      </div>

      <div class="trash-list">
        {#each [...$trash].reverse() as message (`${message.server}/${message.topicName}/${message.id}`)}
          <article>
            <div class="meta">
              <span class="topic">{message.topicName}</span>
              <span>{new Date(message.deletedAt).toLocaleString('zh-CN')}</span>
            </div>
            {#if message.title}<h3>{message.title}</h3>{/if}
            <p class="body">{message.message}</p>
            <div class="item-actions">
              <button onclick={() => restore(message)}>恢复</button>
              <button class="danger" onclick={() => discard(message)}>永久删除</button>
            </div>
          </article>
        {:else}
          <div class="empty">回收站为空</div>
        {/each}
      </div>
    </dialog>
  </div>
{/if}

<style>
  .actions { display: flex; align-items: center; gap: 6px; margin-left: auto; margin-right: 6px; }
  button {
    padding: 7px 10px; border: 1px solid var(--border); border-radius: var(--r-sm);
    background: var(--bg-1); color: var(--text-2); font: inherit; font-size: 12px;
    cursor: pointer; white-space: nowrap;
  }
  button:hover:not(:disabled) { background: var(--bg-3); color: var(--text-1); }
  button.active { background: var(--accent-dim); border-color: var(--accent); color: var(--accent-hover); }
  button.clear:hover:not(:disabled), button.danger:hover:not(:disabled) {
    background: var(--danger-bg); border-color: rgba(239, 68, 68, 0.35); color: var(--danger);
  }
  button:disabled { color: var(--text-4); cursor: default; opacity: 0.6; }
  .overlay {
    position: fixed; inset: 0; z-index: 70; background: rgba(17, 24, 39, 0.4);
    display: flex; align-items: center; justify-content: center;
  }
  .trash-panel {
    width: 600px; max-width: calc(100vw - 48px); height: min(680px, calc(100vh - 48px));
    display: flex; flex-direction: column; background: var(--bg-1); border-radius: 12px;
    border: 1px solid var(--border); box-shadow: 0 20px 50px rgba(17, 24, 39, 0.25);
    position: relative; margin: 0; padding: 0; color: inherit;
  }
  header { display: flex; align-items: flex-start; padding: 20px 22px 14px; border-bottom: 1px solid var(--border); }
  h2 { font-size: 17px; }
  header p { margin-top: 4px; color: var(--text-3); font-size: 12px; }
  .close { margin-left: auto; padding: 0; width: 30px; height: 30px; font-size: 20px; border: none; }
  .trash-toolbar { display: flex; align-items: center; gap: 6px; padding: 10px 22px; color: var(--text-3); font-size: 12px; }
  .spacer { flex: 1; }
  .trash-list { flex: 1; overflow-y: auto; padding: 0 22px 20px; display: flex; flex-direction: column; gap: 8px; }
  article { padding: 12px 14px; border: 1px solid var(--border); border-radius: var(--r-lg); background: var(--bg-2); }
  .meta { display: flex; justify-content: space-between; color: var(--text-3); font-size: 10px; }
  .topic { color: var(--accent-hover); font-weight: 600; }
  h3 { margin-top: 6px; font-size: 13px; }
  .body { margin-top: 4px; color: var(--text-2); font-size: 13px; line-height: 1.45; white-space: pre-wrap; word-break: break-word; }
  .item-actions { display: flex; justify-content: flex-end; gap: 6px; margin-top: 9px; }
  .item-actions button { padding: 4px 8px; font-size: 11px; }
  .empty { display: grid; place-items: center; height: 100%; color: var(--text-4); font-size: 13px; }
</style>
