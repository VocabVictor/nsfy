<script lang="ts">
  import { servers, topics, addServer, removeServer } from './stores/nsfy';

  let newUrl = $state('');
  let newName = $state('');
  let showAdd = $state(false);
</script>

<div class="page">
  <header>
    <h1>Settings</h1>
    <button class="add-btn" onclick={() => showAdd = !showAdd}>+</button>
  </header>

  {#if showAdd}
    <div class="add-form">
      <input type="text" placeholder="Server name (e.g. Home VPS)" bind:value={newName} />
      <input type="text" placeholder="http://host:port" bind:value={newUrl} />
      <button class="btn-primary" disabled={!newUrl || !newName} onclick={() => {
        addServer(newUrl, newName);
        newUrl = ''; newName = ''; showAdd = false;
      }}>Add</button>
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
        <button class="del-btn" onclick={() => removeServer(s.url)}>✕</button>
      </div>
    {/each}
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
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
  header h1 { font-size: 22px; font-weight: 700; letter-spacing: -0.3px; }
  .add-btn {
    width: 36px; height: 36px; border-radius: 10px;
    border: 1px solid #2a2a2a; background: #1a1a1a; color: #888;
    font-size: 20px; cursor: pointer;
  }
  .add-btn:hover { background: #252525; color: #a5b4fc; }
  .add-form {
    display: flex; flex-direction: column; gap: 8px; padding: 12px;
    background: #161616; border-radius: 12px; margin-bottom: 20px;
  }
  .add-form input {
    background: #0d0d0d; border: 1px solid #2a2a2a; border-radius: 8px;
    padding: 8px 12px; color: #e5e5e5; font-size: 13px;
    font-family: inherit; outline: none;
  }
  .add-form input:focus { border-color: #6366f1; }
  .btn-primary {
    padding: 8px 16px; border: none; border-radius: 8px; background: #6366f1;
    color: white; font-size: 13px; font-weight: 600; cursor: pointer; font-family: inherit;
  }
  .btn-primary:disabled { background: #2a2a3a; color: #555; cursor: default; }
  .section { margin-bottom: 24px; }
  .section h2 {
    font-size: 11px; font-weight: 600; color: #666;
    text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px;
  }
  .server-item {
    display: flex; align-items: center; gap: 12px; padding: 12px 16px;
    background: #111111; border-radius: 10px; border: 1px solid #1a1a1a; margin-bottom: 4px;
  }
  .server-info { flex: 1; }
  .server-name { font-weight: 600; font-size: 14px; }
  .server-url { font-size: 12px; color: #666; font-family: monospace; }
  .server-topics { font-size: 11px; color: #555; margin-top: 2px; }
  .del-btn {
    width: 28px; height: 28px; border-radius: 6px;
    border: 1px solid #2a2a2a; background: transparent; color: #555;
    font-size: 12px; cursor: pointer;
  }
  .del-btn:hover { background: #2a0a0a; color: #ef4444; border-color: #4a1515; }
  .about p { font-size: 13px; color: #888; }
  .version { margin-top: 4px; color: #555; font-size: 12px; }
</style>
