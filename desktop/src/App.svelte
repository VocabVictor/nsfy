<script lang="ts">
  import TopicList from './lib/TopicList.svelte';
  import TopicDetail from './lib/TopicDetail.svelte';
  import PublishView from './lib/Publish.svelte';
  import Settings from './lib/Settings.svelte';
  import { activeTab, activeTopic, loadState } from './lib/stores/nsfy';

  let ready = $state(false);

  $effect(() => {
    loadState();
    ready = true;
  });
</script>

{#if !ready}
  <div class="splash">nsfy</div>
{:else}
  <div class="app">
    <nav class="sidebar">
      <div class="logo">nsfy</div>
      <div class="nav-tabs">
        <button
          class="nav-btn"
          class:active={$activeTab === 'topics'}
          onclick={() => $activeTab = 'topics'}
        >
          <svg class="icon" viewBox="0 0 20 20" fill="none"><rect x="3" y="4" width="14" height="3" rx="1" stroke="currentColor" stroke-width="1.6"/><rect x="3" y="9" width="14" height="3" rx="1" stroke="currentColor" stroke-width="1.6"/><rect x="3" y="14" width="9" height="3" rx="1" stroke="currentColor" stroke-width="1.6"/></svg>
          <span>Topics</span>
        </button>
        <button
          class="nav-btn"
          class:active={$activeTab === 'publish'}
          onclick={() => $activeTab = 'publish'}
        >
          <svg class="icon" viewBox="0 0 20 20" fill="none"><path d="M10 16V4M10 4L5 9M10 4l5 5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>
          <span>Publish</span>
        </button>
        <button
          class="nav-btn"
          class:active={$activeTab === 'settings'}
          onclick={() => $activeTab = 'settings'}
        >
          <svg class="icon" viewBox="0 0 20 20" fill="none"><circle cx="10" cy="10" r="2.6" stroke="currentColor" stroke-width="1.6"/><path d="M10 2.6v2M10 15.4v2M17.4 10h-2M4.6 10h-2M15.06 4.94l-1.42 1.42M6.36 13.64l-1.42 1.42M15.06 15.06l-1.42-1.42M6.36 6.36 4.94 4.94" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
          <span>Settings</span>
        </button>
      </div>
      <div class="status">
        <span class="dot"></span>
        <span>v0.1.0</span>
      </div>
    </nav>
    <main class="content">
      {#if $activeTab === 'topics'}
        {#if $activeTopic}
          <TopicDetail onback={() => $activeTopic = null} />
        {:else}
          <TopicList />
        {/if}
      {:else if $activeTab === 'publish'}
        <PublishView />
      {:else}
        <Settings />
      {/if}
    </main>
  </div>
{/if}

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }
  :global(:root) {
    /* Surfaces — a luminance ladder instead of shadows for depth */
    --bg-0: #0a0a0c;   /* app shell / sidebar */
    --bg-1: #0e0e11;   /* content canvas */
    --bg-2: #17171b;   /* cards, inputs */
    --bg-3: #1e1e23;   /* hover state */
    --border: #232328;
    --border-strong: #2c2c33;

    /* Text — off-white rather than pure white, softer on a dark canvas */
    --text-1: #f0f0f2;
    --text-2: #a3a3ab;
    --text-3: #6e6e76;
    --text-4: #47474d;

    /* One accent, used sparingly — warm amber instead of the ubiquitous
       indigo/violet "AI app" look. Dark ink on top since amber is light. */
    --accent: #f2a93c;
    --accent-hover: #ffbb55;
    --accent-dim: rgba(242, 169, 60, 0.14);
    --accent-ink: #1a1206;

    --success: #22c55e;
    --success-bg: rgba(34, 197, 94, 0.12);
    --danger: #ef4444;
    --danger-bg: rgba(239, 68, 68, 0.12);

    --r-sm: 6px;
    --r-md: 8px;
    --r-lg: 10px;
  }
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'SF Pro Text', system-ui, sans-serif;
    background: var(--bg-1);
    color: var(--text-1);
    overflow: hidden;
    height: 100vh;
    -webkit-font-smoothing: antialiased;
  }
  .splash {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    font-size: 32px;
    font-weight: 700;
    letter-spacing: -0.5px;
    color: var(--accent);
    background: var(--bg-1);
  }
  .app {
    display: flex;
    height: 100vh;
  }
  .sidebar {
    width: 200px;
    background: var(--bg-0);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    -webkit-app-region: drag;
  }
  .logo {
    font-size: 15px;
    font-weight: 700;
    letter-spacing: -0.2px;
    padding: 18px 20px 14px;
    color: var(--text-1);
  }
  .nav-tabs {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 0 8px;
    -webkit-app-region: no-drag;
  }
  .nav-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--text-3);
    font-size: 13px;
    cursor: pointer;
    transition: background 0.12s, color 0.12s, border-color 0.12s;
    text-align: left;
    font-family: inherit;
  }
  .nav-btn:hover { background: var(--bg-2); color: var(--text-2); }
  .nav-btn.active { background: var(--bg-2); color: var(--text-1); border-color: var(--border); }
  .nav-btn .icon { width: 16px; height: 16px; flex-shrink: 0; opacity: 0.9; }
  .status {
    padding: 12px 20px;
    font-size: 11px;
    color: var(--text-4);
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--success); flex-shrink: 0; }
  .content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    background: var(--bg-1);
  }
</style>
