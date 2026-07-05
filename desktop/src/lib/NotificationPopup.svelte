<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  let title = $state('');
  let body = $state('');

  $effect(() => {
    invoke<{ title: string; body: string } | null>('get_pending_notification').then((content) => {
      if (content) {
        title = content.title;
        body = content.body;
      }
    });
  });

  function openApp() {
    invoke('focus_main_window');
  }

  function dismiss(e: MouseEvent) {
    e.stopPropagation();
    getCurrentWindow().close();
  }
</script>

<div class="banner" onclick={openApp} role="button" tabindex="0"
  onkeydown={(e) => { if (e.key === 'Enter') openApp(); }}>
  <div class="icon">
    <svg viewBox="0 0 24 24" fill="none">
      <path d="M12 3a6 6 0 0 0-6 6v3l-2 4h16l-2-4V9a6 6 0 0 0-6-6Z" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round"/>
      <path d="M9.5 18a2.5 2.5 0 0 0 5 0" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
    </svg>
  </div>
  <div class="content">
    <div class="title">{title || 'nsfy'}</div>
    <div class="body">{body}</div>
  </div>
  <button class="close-btn" onclick={dismiss} aria-label="Dismiss">
    <svg viewBox="0 0 16 16" fill="none" width="11" height="11"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
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
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    height: 100vh;
    padding: 14px 16px;
    background: #17171bee;
    border: 1px solid #2c2c33;
    border-radius: 12px;
    cursor: pointer;
    color: #f0f0f2;
    -webkit-backdrop-filter: blur(12px);
    backdrop-filter: blur(12px);
  }
  .icon {
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    border-radius: 8px;
    background: #f2a93c;
    color: #1a1206;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .icon svg { width: 16px; height: 16px; }
  .content { flex: 1; min-width: 0; }
  .title {
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 3px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .body {
    font-size: 12px;
    color: #a3a3ab;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .close-btn {
    width: 20px;
    height: 20px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: #6e6e76;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .close-btn:hover { background: #1e1e23; color: #f0f0f2; }
</style>
