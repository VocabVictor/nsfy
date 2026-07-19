<script lang="ts">
  import {
    type PopupPosition, type WindowBehavior,
  } from './stores/nsfy';

  let {
    windowBehavior = $bindable(), doNotDisturb = $bindable(),
    popupOnNotify = $bindable(), popupPosition = $bindable(), onchange,
  }: {
    windowBehavior: WindowBehavior;
    doNotDisturb: boolean;
    popupOnNotify: boolean;
    popupPosition: PopupPosition;
    onchange: () => void;
  } = $props();

  const positions: { value: PopupPosition; label: string }[] = [
    { value: 'top-left', label: '左上' },
    { value: 'top-right', label: '右上' },
    { value: 'bottom-left', label: '左下' },
    { value: 'bottom-right', label: '右下' },
    { value: 'center', label: '居中' },
  ];
</script>

<div class="section">
  <h2>窗口与通知</h2>
  <div class="mode-row">
    <button class="mode" class:active={windowBehavior === 'resident'} onclick={() => { windowBehavior = 'resident'; onchange(); }}>
      <span>驻留托盘</span>
      <small>后台接收消息，不抢占当前窗口</small>
    </button>
    <button class="mode" class:active={windowBehavior === 'popup'} onclick={() => { windowBehavior = 'popup'; onchange(); }}>
      <span>消息时弹出</span>
      <small>新消息到达时显示主窗口</small>
    </button>
  </div>

  <label class="toggle-row">
    <input type="checkbox" checked={doNotDisturb}
      onchange={(event) => { doNotDisturb = event.currentTarget.checked; onchange(); }} />
    <span>勿扰模式</span>
    <kbd>Ctrl+Alt+D</kbd>
  </label>
  <p class="hint">勿扰时仍接收消息，但不弹窗、不显示横幅，也不发送系统通知。</p>

  <label class="toggle-row">
    <input type="checkbox" checked={popupOnNotify}
      onchange={(event) => { popupOnNotify = event.currentTarget.checked; onchange(); }} />
    <span>高优先级消息弹出横幅窗口</span>
  </label>
  {#if popupOnNotify}
    <div class="position-grid">
      {#each positions as position}
        <button class="position" class:active={popupPosition === position.value}
          onclick={() => { popupPosition = position.value; onchange(); }}>
          {position.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .section { margin-bottom: 24px; }
  h2 {
    font-size: 11px; font-weight: 600; color: var(--text-3);
    text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px;
  }
  .mode-row { display: flex; gap: 8px; margin-bottom: 6px; }
  .mode {
    flex: 1; display: flex; flex-direction: column; gap: 3px; text-align: left;
    padding: 12px 14px; border-radius: var(--r-lg); border: 1px solid var(--border);
    background: var(--bg-2); cursor: pointer; font-family: inherit;
  }
  .mode:hover { border-color: var(--border-strong); }
  .mode.active { border-color: var(--accent); background: var(--accent-dim); }
  .mode span { color: var(--text-1); font-size: 13px; font-weight: 600; }
  .mode.active span { color: var(--accent-hover); }
  .mode small { color: var(--text-3); font-size: 11px; }
  .toggle-row {
    display: flex; align-items: center; gap: 10px;
    padding: 10px 0; font-size: 13px; color: var(--text-2); cursor: pointer;
  }
  .toggle-row input { accent-color: var(--accent); width: 15px; height: 15px; cursor: pointer; }
  kbd { margin-left: auto; padding: 2px 6px; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-2); font-size: 10px; }
  .hint { margin: -5px 0 4px 25px; color: var(--text-4); font-size: 11px; }
  .position-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px; margin: 4px 0 8px; }
  .position {
    padding: 8px 10px; border-radius: var(--r-sm); border: 1px solid var(--border);
    background: var(--bg-2); color: var(--text-2); font: inherit; font-size: 12px; cursor: pointer;
  }
  .position:hover { background: var(--bg-3); }
  .position.active { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }
  .position:last-child { grid-column: 1 / -1; }
</style>
