<script lang="ts">
  import {
    servers, topics, addServer, removeServer,
    popupOnNotify, popupPosition, setPopupOnNotify, setPopupPosition,
    type PopupPosition,
  } from './stores/nsfy';

  const positions: { value: PopupPosition; label: string }[] = [
    { value: 'top-left', label: 'Top left' },
    { value: 'top-right', label: 'Top right' },
    { value: 'bottom-left', label: 'Bottom left' },
    { value: 'bottom-right', label: 'Bottom right' },
    { value: 'center', label: 'Center' },
  ];

  let newUrl = $state('');
  let newName = $state('');
  let showAdd = $state(false);

  function focusOnMount(el: HTMLElement) {
    el.focus();
  }

  function submitAdd() {
    if (!newUrl || !newName) return;
    addServer(newUrl, newName);
    newUrl = ''; newName = ''; showAdd = false;
  }

  function confirmRemove(url: string, name: string) {
    const count = $topics.filter(t => t.server === url).length;
    const extra = count > 0 ? ` and its ${count} subscribed topic(s)` : '';
    if (confirm(`Remove server "${name}"${extra}? This can't be undone.`)) {
      removeServer(url);
    }
  }
</script>

<div class="page">
  <header>
    <h1>Settings</h1>
    <button class="add-btn" onclick={() => showAdd = !showAdd}>+</button>
  </header>

  {#if showAdd}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="add-form" onkeydown={(e) => {
      if (e.key === 'Escape') showAdd = false;
      if (e.key === 'Enter') submitAdd();
    }}>
      <input type="text" placeholder="Server name (e.g. Home VPS)" bind:value={newName} use:focusOnMount />
      <input type="text" placeholder="http://host:port" bind:value={newUrl} />
      <button class="btn-primary" disabled={!newUrl || !newName} onclick={submitAdd}>Add</button>
    </div>
  {/if}

  <div class="section">
    <h2>Servers</h2>
    {#each $servers as s (s.url)}
      <div class="server-item">
        <div class="server-info">
          <div class="server-name">{s.name}</div>
          <div class="server-url">{s.url}</div>
          <div class="server-topics">
            {$topics.filter(t => t.server === s.url).length} topic(s) subscribed
          </div>
        </div>
        <button class="del-btn" onclick={() => confirmRemove(s.url, s.name)} aria-label="Remove server">
          <svg viewBox="0 0 16 16" fill="none" width="13" height="13"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
        </button>
      </div>
    {/each}
  </div>

  <div class="section">
    <h2>Notifications</h2>
    <label class="toggle-row">
      <input
        type="checkbox"
        checked={$popupOnNotify}
        onchange={(e) => setPopupOnNotify(e.currentTarget.checked)}
      />
      <span>Show a banner window for high-priority messages</span>
    </label>
    {#if $popupOnNotify}
      <div class="position-grid">
        {#each positions as p}
          <button
            class="pos-btn"
            class:active={$popupPosition === p.value}
            onclick={() => setPopupPosition(p.value)}
          >
            {p.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="section">
    <h2>About nsfy</h2>
    <div class="about">
      <p>A minimal, high-performance pub-sub notification system.</p>
      <p class="version">Desktop v0.1.0 · Server nsfyd v0.1.0</p>
    </div>
  </div>
</div>

<style>
  .page {
    display: flex; flex-direction: column; height: 100%;
    padding: 24px; max-width: 600px; margin: 0 auto; width: 100%;
  }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
  header h1 { font-size: 18px; font-weight: 600; letter-spacing: -0.2px; color: var(--text-1); }
  .add-btn {
    width: 32px; height: 32px; border-radius: var(--r-md);
    border: 1px solid var(--border); background: var(--bg-2); color: var(--text-2);
    font-size: 18px; cursor: pointer; transition: all 0.12s;
  }
  .add-btn:hover { background: var(--bg-3); color: var(--text-1); border-color: var(--border-strong); }
  .add-form {
    display: flex; flex-direction: column; gap: 8px; padding: 12px;
    background: var(--bg-2); border-radius: var(--r-lg); border: 1px solid var(--border); margin-bottom: 20px;
  }
  .add-form input {
    background: var(--bg-1); border: 1px solid var(--border); border-radius: var(--r-sm);
    padding: 8px 12px; color: var(--text-1); font-size: 13px;
    font-family: inherit; outline: none;
  }
  .add-form input:focus { border-color: var(--accent); }
  .btn-primary {
    padding: 8px 16px; border: none; border-radius: var(--r-sm); background: var(--accent);
    color: var(--accent-ink); font-size: 13px; font-weight: 600; cursor: pointer; font-family: inherit;
  }
  .btn-primary:hover { background: var(--accent-hover); }
  .btn-primary:disabled { background: var(--bg-3); color: var(--text-4); cursor: default; }
  .section { margin-bottom: 24px; }
  .section h2 {
    font-size: 11px; font-weight: 600; color: var(--text-3);
    text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px;
  }
  .toggle-row {
    display: flex; align-items: center; gap: 10px;
    padding: 10px 0; font-size: 13px; color: var(--text-2); cursor: pointer;
  }
  .toggle-row input { accent-color: var(--accent); width: 15px; height: 15px; cursor: pointer; }
  .position-grid {
    display: grid; grid-template-columns: repeat(2, 1fr); gap: 6px;
    margin-top: 4px; margin-bottom: 8px;
  }
  .pos-btn {
    padding: 8px 10px; border-radius: var(--r-sm); border: 1px solid var(--border);
    background: var(--bg-2); color: var(--text-2); font-size: 12px; cursor: pointer;
    font-family: inherit; transition: all 0.12s;
  }
  .pos-btn:hover { background: var(--bg-3); color: var(--text-1); }
  .pos-btn.active { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }
  .pos-btn:last-child { grid-column: 1 / -1; }
  .server-item {
    display: flex; align-items: center; gap: 12px; padding: 12px 16px;
    background: var(--bg-2); border-radius: var(--r-lg); border: 1px solid var(--border); margin-bottom: 4px;
  }
  .server-info { flex: 1; }
  .server-name { font-weight: 600; font-size: 14px; color: var(--text-1); }
  .server-url { font-size: 12px; color: var(--text-3); font-family: monospace; }
  .server-topics { font-size: 11px; color: var(--text-4); margin-top: 2px; }
  .del-btn {
    width: 26px; height: 26px; border-radius: var(--r-sm);
    border: 1px solid var(--border); background: transparent; color: var(--text-3);
    cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.12s;
  }
  .del-btn:hover { background: var(--danger-bg); color: var(--danger); border-color: rgba(239,68,68,0.35); }
  .about p { font-size: 13px; color: var(--text-2); }
  .version { margin-top: 4px; color: var(--text-4); font-size: 12px; }
</style>
