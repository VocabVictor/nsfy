<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  type PopupMsg = { title: string; body: string; time: number; priority: number };

  let messages = $state<PopupMsg[]>([]);
  let tab = $state<'receiving' | 'dnd'>('receiving');

  $effect(() => {
    invoke<PopupMsg[] | null>('get_pending_notification').then((content) => {
      if (content) messages = content;
    });
  });

  function fmtClock(ts: number): string {
    const now = Date.now();
    const diff = now - ts * 1000;
    if (diff < 60_000) return '刚刚';
    if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
    const d = new Date(ts * 1000);
    return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
  }

  function priColor(p: number): string {
    if (p >= 5) return '#ef4444';
    if (p >= 4) return '#f97316';
    return '#9ca3af';
  }

  function priLabel(p: number): string {
    if (p >= 5) return '紧急';
    if (p >= 4) return '高';
    return '';
  }

  function openApp() {
    invoke('focus_main_window');
  }

  function dismiss(e: MouseEvent) {
    e.stopPropagation();
    getCurrentWindow().close();
  }
</script>

<div class="panel">
  <div class="head">
    <div class="brand">
      <span class="brand-icon">
        <svg viewBox="0 0 24 24" fill="none" width="12" height="12">
          <path d="M12 3a6 6 0 0 0-6 6v3l-2 4h16l-2-4V9a6 6 0 0 0-6-6Z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/>
          <path d="M9.5 18a2.5 2.5 0 0 0 5 0" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
        </svg>
      </span>
      信鸽
    </div>
    <div class="tabs">
      <button class="tab" class:active={tab === 'receiving'} onclick={() => tab = 'receiving'}>接收中</button>
      <button class="tab" class:active={tab === 'dnd'} onclick={() => tab = 'dnd'}>勿扰</button>
    </div>
    <button class="close-btn" onclick={dismiss} aria-label="关闭">
      <svg viewBox="0 0 16 16" fill="none" width="10" height="10"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
    </button>
  </div>

  <div class="list">
    {#if tab === 'dnd'}
      <div class="dnd-note">勿扰模式下不弹出横幅</div>
    {:else if messages.length === 0}
      <div class="dnd-note">暂无新消息</div>
    {:else}
      {#each messages as m}
        <div class="row" onclick={openApp} role="button" tabindex="0"
          onkeydown={(e) => { if (e.key === 'Enter') openApp(); }}>
          <div class="row-top">
            {#if priLabel(m.priority)}
              <span class="pri" style="color:{priColor(m.priority)}">{priLabel(m.priority)}</span>
            {/if}
            <span class="row-title">{m.title}</span>
            <span class="row-time">{fmtClock(m.time)}</span>
          </div>
          {#if m.body}
            <div class="row-body">{m.body}</div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <button class="footer" onclick={openApp}>
    <svg viewBox="0 0 16 16" fill="none" width="12" height="12"><rect x="2.5" y="2.5" width="11" height="11" rx="2" stroke="currentColor" stroke-width="1.4"/><path d="M6 8h4M8 6v4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
    打开主窗口
  </button>
</div>

<style>
  :global(*) { margin: 0; padding: 0; box-sizing: border-box; }
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'SF Pro Text', system-ui, sans-serif;
    background: transparent;
    overflow: hidden;
    -webkit-font-smoothing: antialiased;
  }
  .panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100vh;
    background: #ffffffee;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    color: #111827;
    overflow: hidden;
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid #eef2f6;
    flex-shrink: 0;
  }
  .brand {
    display: flex; align-items: center; gap: 6px;
    font-size: 13px; font-weight: 700;
  }
  .brand-icon {
    width: 20px; height: 20px; border-radius: 6px;
    background: #0ea5e9; color: #fff;
    display: flex; align-items: center; justify-content: center;
  }
  .tabs { display: flex; gap: 2px; margin-left: auto; }
  .tab {
    padding: 4px 10px; border-radius: 999px; border: none;
    background: transparent; color: #6b7280; font-size: 11px;
    cursor: pointer; font-family: inherit;
  }
  .tab.active { background: rgba(14, 165, 233, 0.12); color: #0284c7; font-weight: 600; }
  .close-btn {
    width: 18px; height: 18px; border-radius: 6px; border: none;
    background: transparent; color: #9ca3af; cursor: pointer;
    display: flex; align-items: center; justify-content: center; flex-shrink: 0;
  }
  .close-btn:hover { background: #eef2f6; color: #111827; }

  .list { flex: 1; overflow-y: auto; padding: 4px 6px; }
  .row {
    padding: 8px 8px; border-radius: 8px; cursor: pointer;
  }
  .row:hover { background: #f6f7f9; }
  .row-top { display: flex; align-items: center; gap: 6px; }
  .pri { font-size: 10px; font-weight: 700; flex-shrink: 0; }
  .row-title {
    font-size: 12px; font-weight: 600; flex: 1; min-width: 0;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .row-time { font-size: 10px; color: #9ca3af; flex-shrink: 0; }
  .row-body {
    font-size: 11px; color: #6b7280; line-height: 1.4; margin-top: 2px;
    display: -webkit-box; line-clamp: 2; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
  }
  .dnd-note {
    padding: 20px 0; text-align: center;
    font-size: 12px; color: #9ca3af;
  }

  .footer {
    display: flex; align-items: center; justify-content: center; gap: 6px;
    padding: 9px 0; border: none; border-top: 1px solid #eef2f6;
    background: transparent; color: #0284c7; font-size: 12px; font-weight: 600;
    cursor: pointer; font-family: inherit; flex-shrink: 0;
  }
  .footer:hover { background: #f6f7f9; }
</style>
