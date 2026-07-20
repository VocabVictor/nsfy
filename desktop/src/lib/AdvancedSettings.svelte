<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { clearAllMessages, topics, topicRuleKey, type AdvancedPreferences } from './stores/nsfy';

  let {
    advanced = $bindable(), onchange,
  }: { advanced: AdvancedPreferences; onchange: () => void } = $props();
  let backupText = $state('');
  let includeTokens = $state(false);
  let backupStatus = $state('');
  const days = [
    { value: 1, label: '一' }, { value: 2, label: '二' }, { value: 3, label: '三' },
    { value: 4, label: '四' }, { value: 5, label: '五' }, { value: 6, label: '六' },
    { value: 7, label: '日' },
  ];

  function update(patch: Partial<AdvancedPreferences>) {
    advanced = { ...advanced, ...patch };
    onchange();
  }

  function toggleDay(day: number) {
    update({ dndDays: advanced.dndDays.includes(day)
      ? advanced.dndDays.filter(value => value !== day)
      : [...advanced.dndDays, day].sort() });
  }

  function updateTopicRule(server: string, topic: string, field: 'mode' | 'bypassDnd', value: string | boolean) {
    const key = topicRuleKey(server, topic);
    const current = advanced.topicRules[key] || { mode: 'normal', bypassDnd: false };
    update({ topicRules: { ...advanced.topicRules, [key]: { ...current, [field]: value } } });
  }

  async function exportBackup() {
    backupText = await invoke<string>('export_config', { includeTokens });
    await navigator.clipboard.writeText(backupText).catch(() => {});
    backupStatus = '配置已生成并复制到剪贴板';
  }

  async function importBackup() {
    try {
      await invoke('import_config', { content: backupText });
      backupStatus = '导入成功，正在重新加载';
      location.reload();
    } catch (error) {
      backupStatus = String(error);
    }
  }

  async function resetSettings() {
    if (!confirm('恢复默认设置？服务器和订阅不会删除。')) return;
    await invoke('reset_preferences');
    location.reload();
  }
</script>

<div class="section">
  <h2>启动</h2>
  <label class="toggle"><input type="checkbox" checked={advanced.autoStart}
    onchange={e => update({ autoStart: e.currentTarget.checked })} />登录系统后自动启动</label>
  <label class="toggle sub"><input type="checkbox" checked={advanced.startMinimized} disabled={!advanced.autoStart}
    onchange={e => update({ startMinimized: e.currentTarget.checked })} />自动启动时直接驻留托盘</label>
</div>

<div class="section">
  <h2>定时勿扰</h2>
  <label class="toggle"><input type="checkbox" checked={advanced.dndScheduleEnabled}
    onchange={e => update({ dndScheduleEnabled: e.currentTarget.checked })} />按时间自动进入勿扰</label>
  {#if advanced.dndScheduleEnabled}
    <div class="inline"><input type="time" value={advanced.dndStart} onchange={e => update({ dndStart: e.currentTarget.value })} />
      <span>至</span><input type="time" value={advanced.dndEnd} onchange={e => update({ dndEnd: e.currentTarget.value })} /></div>
    <div class="days">{#each days as day}<button class:active={advanced.dndDays.includes(day.value)}
      onclick={() => toggleDay(day.value)}>{day.label}</button>{/each}</div>
    <p class="hint">手动开启勿扰会立即生效；关闭手动勿扰不会跳过当前定时时段。</p>
  {/if}
</div>

<div class="section">
  <h2>隐私与声音</h2>
  <label class="toggle"><input type="checkbox" checked={advanced.showPreview}
    onchange={e => update({ showPreview: e.currentTarget.checked })} />通知中显示消息正文</label>
  <label class="toggle"><input type="checkbox" checked={advanced.soundEnabled}
    onchange={e => update({ soundEnabled: e.currentTarget.checked })} />播放普通通知声音</label>
  <label class="toggle"><input type="checkbox" checked={advanced.urgentSoundEnabled}
    onchange={e => update({ urgentSoundEnabled: e.currentTarget.checked })} />紧急消息使用更明显的声音</label>
  <p class="hint">锁屏是否显示通知正文仍受操作系统隐私设置控制。</p>
</div>

<div class="section">
  <h2>消息保留</h2>
  <div class="fields"><label>收件箱保留 <input type="number" min="1" max="3650" value={advanced.retentionDays}
    onchange={e => update({ retentionDays: Math.max(1, Number(e.currentTarget.value)) })} /> 天</label>
    <label>回收站保留 <input type="number" min="1" max="3650" value={advanced.trashRetentionDays}
    onchange={e => update({ trashRetentionDays: Math.max(1, Number(e.currentTarget.value)) })} /> 天</label></div>
  <button class="secondary" onclick={() => { if (confirm('清理本机消息缓存并同步到回收站？设置和订阅会保留。')) clearAllMessages(); }}>清理消息缓存</button>
</div>

<div class="section">
  <h2>快捷键</h2>
  <div class="fields"><label>切换勿扰<input value={advanced.dndShortcut} onchange={e => update({ dndShortcut: e.currentTarget.value })} /></label>
    <label>显示窗口<input value={advanced.showShortcut} onchange={e => update({ showShortcut: e.currentTarget.value })} /></label></div>
  <p class="hint">示例：Ctrl+Alt+D。保存时会检查格式、重复和系统占用。</p>
</div>

<div class="section">
  <h2>网络与连接</h2>
  <div class="choice"><label><input type="radio" name="proxy" checked={advanced.proxyMode === 'system'}
    onchange={() => update({ proxyMode: 'system' })} />跟随系统代理</label>
    <label><input type="radio" name="proxy" checked={advanced.proxyMode === 'direct'}
    onchange={() => update({ proxyMode: 'direct' })} />直接连接</label></div>
  <div class="connections">{#each $topics as topic (`${topic.server}/${topic.name}`)}
    <div><span class:online={topic.connected}></span><b>{topic.name}</b>
      <small>{topic.connected ? '已连接' : '未连接'}{topic.lastConnectedAt ? ` · 最近 ${new Date(topic.lastConnectedAt).toLocaleString('zh-CN')}` : ''}</small></div>
  {:else}<p class="hint">尚未订阅主题</p>{/each}</div>
  <button class="secondary" onclick={() => location.reload()}>立即重新连接</button>
</div>

{#if $topics.length}
  <div class="section"><h2>主题通知规则</h2>
    {#each $topics as topic (`rule-${topic.server}/${topic.name}`)}
      {@const key = topicRuleKey(topic.server, topic.name)}
      {@const rule = advanced.topicRules[key] || { mode: 'normal', bypassDnd: false }}
      <div class="topic-rule"><b>{topic.name}</b><select value={rule.mode}
        onchange={e => updateTopicRule(topic.server, topic.name, 'mode', e.currentTarget.value)}>
        <option value="normal">正常提醒</option><option value="high">仅高优先级</option><option value="mute">静音</option>
      </select><label><input type="checkbox" checked={rule.bypassDnd}
        onchange={e => updateTopicRule(topic.server, topic.name, 'bypassDnd', e.currentTarget.checked)} />可越过勿扰</label></div>
    {/each}
  </div>
{/if}

<div class="section"><h2>备份与恢复</h2>
  <div class="backup-actions"><label><input type="checkbox" bind:checked={includeTokens} />导出时包含访问令牌</label>
    <button class="secondary" onclick={exportBackup}>导出并复制</button></div>
  <textarea bind:value={backupText} placeholder="导出的配置会显示在这里；也可以粘贴配置后导入"></textarea>
  <div class="backup-actions"><button class="secondary" disabled={!backupText.trim()} onclick={importBackup}>导入配置</button>
    <button class="danger" onclick={resetSettings}>恢复默认设置</button></div>
  {#if backupStatus}<p class="hint">{backupStatus}</p>{/if}
</div>

<style>
  .section { margin-bottom: 24px; }
  h2 { font-size: 11px; font-weight: 600; color: var(--text-3); text-transform: uppercase; letter-spacing: .5px; margin-bottom: 8px; }
  .toggle, .choice label { display: flex; align-items: center; gap: 9px; padding: 7px 0; color: var(--text-2); font-size: 13px; }
  .toggle.sub { margin-left: 24px; color: var(--text-3); } input { accent-color: var(--accent); }
  .inline, .fields, .choice, .backup-actions { display: flex; align-items: center; gap: 10px; margin: 8px 0; }
  input[type="time"], input[type="number"], .fields input, select, textarea { border: 1px solid var(--border); border-radius: var(--r-sm); background: var(--bg-2); color: var(--text-1); padding: 7px 9px; font: inherit; }
  .fields label { flex: 1; display: flex; flex-direction: column; gap: 5px; color: var(--text-3); font-size: 11px; }
  .fields input { width: 100%; } .days { display: flex; gap: 5px; }
  .days button { width: 31px; height: 29px; border: 1px solid var(--border); border-radius: var(--r-sm); background: var(--bg-2); color: var(--text-3); }
  .days button.active { border-color: var(--accent); background: var(--accent-dim); color: var(--accent); }
  .hint { color: var(--text-4); font-size: 11px; margin-top: 5px; }
  button.secondary, button.danger { border: 1px solid var(--border); border-radius: var(--r-sm); padding: 7px 10px; background: var(--bg-2); color: var(--text-2); cursor: pointer; }
  button.danger { color: var(--danger); } button:disabled { opacity: .5; cursor: default; }
  .connections { border: 1px solid var(--border); border-radius: var(--r-md); overflow: hidden; }
  .connections div { display: flex; align-items: center; gap: 7px; padding: 8px 10px; border-bottom: 1px solid var(--border); }
  .connections div:last-child { border: none; } .connections span { width: 7px; height: 7px; border-radius: 50%; background: var(--text-4); }
  .connections span.online { background: #22c55e; } .connections b { font-size: 12px; } .connections small { margin-left: auto; color: var(--text-4); }
  .topic-rule { display: grid; grid-template-columns: 1fr 140px 110px; align-items: center; gap: 8px; padding: 8px 10px; border: 1px solid var(--border); border-radius: var(--r-md); margin-bottom: 5px; font-size: 12px; }
  .topic-rule label { color: var(--text-3); } textarea { width: 100%; min-height: 90px; resize: vertical; font-family: monospace; font-size: 11px; }
  .backup-actions { justify-content: flex-end; } .backup-actions label { margin-right: auto; color: var(--text-3); font-size: 11px; }
</style>
