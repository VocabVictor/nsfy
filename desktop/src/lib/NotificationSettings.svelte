<script lang="ts">
  import {
    type NotificationMode, type PopupPosition, type WindowBehavior,
  } from './stores/nsfy';

  let {
    windowBehavior = $bindable(), doNotDisturb = $bindable(),
    dndAllowedPriorities = $bindable(),
    notificationMode = $bindable(), popupPosition = $bindable(), onchange,
  }: {
    windowBehavior: WindowBehavior;
    doNotDisturb: boolean;
    dndAllowedPriorities: number[];
    notificationMode: NotificationMode;
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
  const modes: { value: NotificationMode; label: string; detail: string }[] = [
    { value: 'silent', label: '静默', detail: '只进入收件箱' },
    { value: 'system', label: '系统通知', detail: '进入系统通知中心' },
    { value: 'temporary', label: '临时横幅', detail: '显示后自动消失' },
    { value: 'persistent', label: '持续提醒', detail: '手动关闭前保留' },
  ];
  const priorities = [
    { value: 5, label: '紧急' }, { value: 4, label: '高' },
    { value: 3, label: '普通' }, { value: 2, label: '低' },
    { value: 1, label: '最低' },
  ];

  function toggleDndPriority(value: number) {
    dndAllowedPriorities = dndAllowedPriorities.includes(value)
      ? dndAllowedPriorities.filter(item => item !== value)
      : [...dndAllowedPriorities, value].sort((a, b) => b - a);
    onchange();
  }
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
  <p class="hint">勿扰时仍接收消息；发送方指定无视勿扰的消息不受限制。</p>
  <div class="dnd-levels">
    <span>勿扰时仍允许弹窗</span>
    <div class="level-row">
      {#each priorities as priority}
        <button class:active={dndAllowedPriorities.includes(priority.value)}
          onclick={() => toggleDndPriority(priority.value)}>{priority.label}</button>
      {/each}
    </div>
    <small>只对发送时已勾选“弹窗通知”的消息生效。</small>
  </div>

  <div class="mode-label">通知样式</div>
  {#if windowBehavior === 'popup'}
    <p class="mode-note">当前会直接显示主窗口；以下样式用于驻留托盘模式。</p>
  {/if}
  <div class="notification-modes">
    {#each modes as mode}
      <button class:active={notificationMode === mode.value}
        onclick={() => { notificationMode = mode.value; onchange(); }}>
        <span>{mode.label}</span><small>{mode.detail}</small>
      </button>
    {/each}
  </div>
  {#if notificationMode === 'temporary' || notificationMode === 'persistent'}
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
  .dnd-levels { margin: 5px 0 8px 25px; color: var(--text-3); font-size: 11px; }
  .level-row { display: flex; gap: 5px; margin: 6px 0 4px; }
  .level-row button {
    padding: 5px 8px; border: 1px solid var(--border); border-radius: var(--r-sm);
    background: var(--bg-2); color: var(--text-3); cursor: pointer; font: inherit;
  }
  .level-row button.active { border-color: var(--accent); background: var(--accent-dim); color: var(--accent); }
  .dnd-levels small { color: var(--text-4); }
  .mode-label { margin-top: 12px; color: var(--text-2); font-size: 12px; font-weight: 600; }
  .mode-note { margin-top: 3px; color: var(--text-4); font-size: 10px; }
  .notification-modes { display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; margin-top: 10px; }
  .notification-modes button {
    display: flex; flex-direction: column; gap: 3px; padding: 9px; text-align: left;
    border: 1px solid var(--border); border-radius: var(--r-md); background: var(--bg-2); cursor: pointer;
  }
  .notification-modes button.active { border-color: var(--accent); background: var(--accent-dim); }
  .notification-modes span { color: var(--text-1); font-size: 12px; font-weight: 600; }
  .notification-modes small { color: var(--text-4); font-size: 10px; }
  .position-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px; margin: 4px 0 8px; }
  .position {
    padding: 8px 10px; border-radius: var(--r-sm); border: 1px solid var(--border);
    background: var(--bg-2); color: var(--text-2); font: inherit; font-size: 12px; cursor: pointer;
  }
  .position:hover { background: var(--bg-3); }
  .position.active { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }
  .position:last-child { grid-column: 1 / -1; }
</style>
