<script lang="ts">
  import {
    servers, topics, addServer, removeServer, setServerToken,
    doNotDisturb, layoutMode, popupOnNotify, popupPosition, windowBehavior,
    savePreferences, type LayoutMode, type PopupPosition, type WindowBehavior,
  } from './stores/nsfy';
  import NotificationSettings from './NotificationSettings.svelte';

  let newUrl = $state('');
  let newName = $state('');
  let newToken = $state('');
  let showAdd = $state(false);
  let addError = $state('');
  let editTokenUrl = $state<string | null>(null);
  let editTokenValue = $state('');
  let draftLayout = $state<LayoutMode>('split');
  let draftWindow = $state<WindowBehavior>('resident');
  let draftDnd = $state(false);
  let draftBanner = $state(false);
  let draftPosition = $state<PopupPosition>('top-right');
  let dirty = $state(false);
  let saved = $state(false);

  $effect(() => {
    if (dirty) return;
    draftLayout = $layoutMode;
    draftWindow = $windowBehavior;
    draftDnd = $doNotDisturb;
    draftBanner = $popupOnNotify;
    draftPosition = $popupPosition;
  });

  function changed() {
    dirty = true;
    saved = false;
  }

  function saveSettings() {
    savePreferences({
      layoutMode: draftLayout, windowBehavior: draftWindow,
      doNotDisturb: draftDnd, popupOnNotify: draftBanner,
      popupPosition: draftPosition,
    });
    dirty = false;
    saved = true;
  }

  function focusOnMount(el: HTMLElement) {
    el.focus();
  }

  function submitAdd() {
    if (!newUrl || !newName) return;
    try {
      addServer(newUrl, newName, newToken.trim());
      newUrl = ''; newName = ''; newToken = ''; addError = ''; showAdd = false;
    } catch (error) {
      addError = error instanceof Error ? error.message : '服务器地址无效';
    }
  }

  function openTokenEditor(url: string, current: string | undefined) {
    editTokenUrl = url;
    editTokenValue = current || '';
  }

  function saveToken() {
    if (editTokenUrl === null) return;
    setServerToken(editTokenUrl, editTokenValue.trim());
    editTokenUrl = null;
    editTokenValue = '';
  }

  function confirmRemove(url: string, name: string) {
    const count = $topics.filter(t => t.server === url).length;
    const extra = count > 0 ? `及其 ${count} 个已订阅主题` : '';
    if (confirm(`移除服务器「${name}」${extra}?此操作不可撤销。`)) {
      removeServer(url);
    }
  }
</script>

<div class="page">
  <div class="section">
    <h2>布局</h2>
    <div class="layout-row">
      <button
        class="layout-btn" class:active={draftLayout === 'split'}
        onclick={() => { draftLayout = 'split'; changed(); }}
      >
        <span class="layout-name">分栏排版</span>
        <span class="layout-desc">主题侧栏在左，右侧消息流</span>
      </button>
      <button
        class="layout-btn" class:active={draftLayout === 'timeline'}
        onclick={() => { draftLayout = 'timeline'; changed(); }}
      >
        <span class="layout-name">统一时间线</span>
        <span class="layout-desc">单一收件箱，按日期分组</span>
      </button>
    </div>
  </div>

  <div class="section">
    <div class="section-head">
      <h2>服务器</h2>
      <button class="add-btn" onclick={() => showAdd = !showAdd} aria-label="添加服务器">+</button>
    </div>

    {#if showAdd}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="add-form" onkeydown={(e) => {
        if (e.key === 'Escape') showAdd = false;
        if (e.key === 'Enter') submitAdd();
      }}>
        <input type="text" placeholder="服务器名称（如:家里 VPS）" bind:value={newName} use:focusOnMount />
        <input type="text" placeholder="https://host:port" bind:value={newUrl} />
        <input type="password" placeholder="访问令牌（可选，服务器开启鉴权时填写）" bind:value={newToken} />
        {#if addError}<div class="form-error">{addError}</div>{/if}
        <button class="btn-primary" disabled={!newUrl || !newName} onclick={submitAdd}>添加</button>
      </div>
    {/if}

    {#each $servers as s (s.url)}
      <div class="server-item">
        <div class="server-info">
          <div class="server-name">{s.name}</div>
          <div class="server-url">{s.url}</div>
          <div class="server-topics">
            已订阅 {$topics.filter(t => t.server === s.url).length} 个主题
            {#if s.token}· 已配置令牌{/if}
          </div>
          {#if editTokenUrl === s.url}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="token-edit" onkeydown={(e) => {
              if (e.key === 'Escape') editTokenUrl = null;
              if (e.key === 'Enter') saveToken();
            }}>
              <input type="password" placeholder="访问令牌（留空清除）"
                bind:value={editTokenValue} use:focusOnMount />
              <button class="btn-primary" onclick={saveToken}>保存</button>
            </div>
          {/if}
        </div>
        <button class="token-btn" onclick={() => openTokenEditor(s.url, s.token)} aria-label="配置令牌">
          <svg viewBox="0 0 16 16" fill="none" width="13" height="13"><path d="M9.5 6.5a3 3 0 1 0-3 3L3 13v-2h2v-2h2l2.5-2.5Z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/><circle cx="10.5" cy="5.5" r="0.8" fill="currentColor"/></svg>
        </button>
        <button class="del-btn" onclick={() => confirmRemove(s.url, s.name)} aria-label="移除服务器">
          <svg viewBox="0 0 16 16" fill="none" width="13" height="13"><path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
        </button>
      </div>
    {/each}
  </div>

  <NotificationSettings
    bind:windowBehavior={draftWindow} bind:doNotDisturb={draftDnd}
    bind:popupOnNotify={draftBanner} bind:popupPosition={draftPosition}
    onchange={changed}
  />

  <div class="section">
    <h2>关于信鸽</h2>
    <div class="about">
      <p>订阅主题，接收服务器推送。</p>
      <p class="version">桌面端 v0.1.0 · 服务端 nsfyd v0.1.0</p>
    </div>
  </div>

  <div class="save-bar">
    {#if saved}<span>设置已保存</span>{/if}
    <button class="btn-primary" disabled={!dirty} onclick={saveSettings}>保存设置</button>
  </div>
</div>

<style>
  .page {
    display: flex; flex-direction: column; height: 100%;
    padding: 24px; max-width: 600px; margin: 0 auto; width: 100%; overflow-y: auto;
  }
  .section-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .section-head h2 { margin-bottom: 0; }
  .layout-row { display: flex; gap: 8px; }
  .layout-btn {
    flex: 1; display: flex; flex-direction: column; gap: 3px; text-align: left;
    padding: 12px 14px; border-radius: var(--r-lg); border: 1px solid var(--border);
    background: var(--bg-2); cursor: pointer; font-family: inherit; transition: all 0.12s;
  }
  .layout-btn:hover { border-color: var(--border-strong); }
  .layout-btn.active { border-color: var(--accent); background: var(--accent-dim); }
  .layout-name { font-size: 13px; font-weight: 600; color: var(--text-1); }
  .layout-btn.active .layout-name { color: var(--accent-hover); }
  .layout-desc { font-size: 11px; color: var(--text-3); }
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
  .server-item {
    display: flex; align-items: center; gap: 12px; padding: 12px 16px;
    background: var(--bg-2); border-radius: var(--r-lg); border: 1px solid var(--border); margin-bottom: 4px;
  }
  .server-info { flex: 1; }
  .server-name { font-weight: 600; font-size: 14px; color: var(--text-1); }
  .server-url { font-size: 12px; color: var(--text-3); font-family: monospace; }
  .server-topics { font-size: 11px; color: var(--text-4); margin-top: 2px; }
  .del-btn, .token-btn {
    width: 26px; height: 26px; border-radius: var(--r-sm);
    border: 1px solid var(--border); background: transparent; color: var(--text-3);
    cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.12s;
    flex-shrink: 0;
  }
  .del-btn:hover { background: var(--danger-bg); color: var(--danger); border-color: rgba(239,68,68,0.35); }
  .token-btn:hover { background: var(--accent-dim); color: var(--accent-hover); border-color: var(--accent); }
  .token-edit { display: flex; gap: 6px; margin-top: 8px; }
  .token-edit input {
    flex: 1; background: var(--bg-1); border: 1px solid var(--border); border-radius: var(--r-sm);
    padding: 7px 10px; color: var(--text-1); font-size: 12px; font-family: inherit; outline: none;
  }
  .token-edit input:focus { border-color: var(--accent); }
  .about p { font-size: 13px; color: var(--text-2); }
  .version { margin-top: 4px; color: var(--text-4); font-size: 12px; }
  .save-bar {
    position: sticky; bottom: -24px; display: flex; justify-content: flex-end;
    align-items: center; gap: 10px; margin-top: auto; padding: 14px 0 24px;
    background: linear-gradient(transparent, var(--bg-1) 22%);
  }
  .save-bar span { color: var(--success); font-size: 12px; }
</style>
