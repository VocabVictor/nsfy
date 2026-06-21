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
          <span class="icon">▤</span>
          <span>Topics</span>
        </button>
        <button
          class="nav-btn"
          class:active={$activeTab === 'publish'}
          onclick={() => $activeTab = 'publish'}
        >
          <span class="icon">↑</span>
          <span>Publish</span>
        </button>
        <button
          class="nav-btn"
          class:active={$activeTab === 'settings'}
          onclick={() => $activeTab = 'settings'}
        >
          <span class="icon">⚙</span>
          <span>Settings</span>
        </button>
      </div>
      <div class="status">
        <span class="dot">●</span>
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
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'SF Pro Text', system-ui, sans-serif;
    background: #0d0d0d;
    color: #e5e5e5;
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
    color: #6366f1;
  }
  .app {
    display: flex;
    height: 100vh;
  }
  .sidebar {
    width: 200px;
    background: #111111;
    border-right: 1px solid #1e1e1e;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    -webkit-app-region: drag;
  }
  .logo {
    font-size: 20px;
    font-weight: 800;
    letter-spacing: -0.5px;
    padding: 16px 20px;
    color: #6366f1;
  }
  .nav-tabs {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px;
    -webkit-app-region: no-drag;
  }
  .nav-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #888;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.15s;
    text-align: left;
    font-family: inherit;
  }
  .nav-btn:hover { background: #1a1a1a; color: #ccc; }
  .nav-btn.active { background: #1e1e2e; color: #a5b4fc; }
  .nav-btn .icon { font-size: 16px; }
  .status {
    padding: 12px 20px;
    font-size: 11px;
    color: #555;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .dot { font-size: 6px; color: #22c55e; }
  .content {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
</style>
