package com.nsfy.app.ui.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.nsfy.app.NsfyApp
import com.nsfy.app.data.model.*
import com.nsfy.app.data.repository.NsfyRepository
import com.nsfy.app.service.WebSocketService
import kotlinx.coroutines.launch

@Composable
fun AndroidAdvancedSettings(value: MobilePreferences, onChange: (MobilePreferences) -> Unit) {
    val context = LocalContext.current
    val prefs = remember { context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE) }
    val repository = remember { NsfyRepository(NsfyApp.instance.database) }
    val topics by repository.getAllTopics().collectAsState(initial = emptyList())
    val scope = rememberCoroutineScope()
    var backupText by remember { mutableStateOf("") }
    var includeTokens by remember { mutableStateOf(false) }
    var backupError by remember { mutableStateOf("") }

    SectionTitle("启动")
    SettingSwitch("开机自动启动并接收消息", value.autoStart) { onChange(value.copy(autoStart = it)) }
    HorizontalDivider()

    SectionTitle("定时勿扰")
    SettingSwitch("按时间自动进入勿扰", value.dndScheduleEnabled) {
        onChange(value.copy(dndScheduleEnabled = it))
    }
    if (value.dndScheduleEnabled) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(value.dndStart, { onChange(value.copy(dndStart = it)) },
                label = { Text("开始") }, modifier = Modifier.weight(1f), singleLine = true)
            OutlinedTextField(value.dndEnd, { onChange(value.copy(dndEnd = it)) },
                label = { Text("结束") }, modifier = Modifier.weight(1f), singleLine = true)
        }
        Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(5.dp)) {
            (1..7).forEach { day ->
                FilterChip(day in value.dndDays, onClick = {
                    onChange(value.copy(dndDays = if (day in value.dndDays) value.dndDays - day else value.dndDays + day))
                }, label = { Text(listOf("一", "二", "三", "四", "五", "六", "日")[day - 1]) })
            }
        }
    }
    HorizontalDivider()

    SectionTitle("隐私与声音")
    SettingSwitch("通知中显示消息正文", value.showPreview) { onChange(value.copy(showPreview = it)) }
    SettingSwitch("播放通知声音", value.soundEnabled) { onChange(value.copy(soundEnabled = it)) }
    SettingSwitch("紧急消息使用明显声音", value.urgentSoundEnabled) {
        onChange(value.copy(urgentSoundEnabled = it))
    }
    Text("锁屏内容仍受 Android 系统隐私设置控制。", style = MaterialTheme.typography.labelSmall)
    HorizontalDivider()

    SectionTitle("消息保留")
    NumberSetting("收件箱保留天数", value.retentionDays) { onChange(value.copy(retentionDays = it)) }
    NumberSetting("回收站保留天数", value.trashRetentionDays) {
        onChange(value.copy(trashRetentionDays = it))
    }
    OutlinedButton(onClick = { scope.launch { repository.trashAll() } }) { Text("清理消息缓存到回收站") }
    HorizontalDivider()

    SectionTitle("网络与连接")
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        FilterChip(value.proxyMode == "system", { onChange(value.copy(proxyMode = "system")) }, { Text("跟随系统代理") })
        FilterChip(value.proxyMode == "direct", { onChange(value.copy(proxyMode = "direct")) }, { Text("直接连接") })
    }
    topics.forEach { topic ->
        Text("${if (topic.isConnected) "●" else "○"} ${topic.name} · ${if (topic.isConnected) "已连接" else "未连接"}" +
            if (topic.lastConnectedAt > 0) " · 最近 ${java.text.DateFormat.getDateTimeInstance().format(topic.lastConnectedAt)}" else "",
            style = MaterialTheme.typography.bodySmall)
    }
    OutlinedButton(onClick = {
        context.stopService(Intent(context, WebSocketService::class.java))
        val intent = Intent(context, WebSocketService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) context.startForegroundService(intent) else context.startService(intent)
    }) { Text("立即重新连接") }
    HorizontalDivider()

    if (topics.isNotEmpty()) {
        SectionTitle("主题通知规则")
        topics.forEach { topic ->
            val key = topicRuleKey(topic.serverUrl, topic.name)
            val rule = value.topicRules[key] ?: TopicRule()
            Card(Modifier.fillMaxWidth()) { Column(Modifier.padding(10.dp)) {
                Text(topic.name)
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf("normal" to "正常", "high" to "仅高优先级", "mute" to "静音").forEach { option ->
                        FilterChip(rule.mode == option.first, {
                            onChange(value.copy(topicRules = value.topicRules + (key to rule.copy(mode = option.first))))
                        }, { Text(option.second) })
                    }
                }
                SettingSwitch("可越过勿扰", rule.bypassDnd) {
                    onChange(value.copy(topicRules = value.topicRules + (key to rule.copy(bypassDnd = it))))
                }
            } }
        }
        HorizontalDivider()
    }

    SectionTitle("备份与恢复")
    SettingSwitch("导出时包含访问令牌", includeTokens) { includeTokens = it }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = {
            backupText = exportPreferences(prefs, includeTokens)
            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("nsfy config", backupText))
        }) { Text("导出并复制") }
        OutlinedButton(enabled = backupText.isNotBlank(), onClick = {
            try { importPreferences(prefs, backupText); backupError = "导入成功，请重新打开设置" }
            catch (error: Exception) { backupError = error.message ?: "导入失败" }
        }) { Text("导入") }
    }
    OutlinedTextField(backupText, { backupText = it }, modifier = Modifier.fillMaxWidth().heightIn(min = 90.dp),
        label = { Text("配置 JSON") })
    if (backupError.isNotEmpty()) Text(backupError, color = MaterialTheme.colorScheme.error)
    OutlinedButton(onClick = { onChange(MobilePreferences()) }) { Text("恢复高级设置默认值") }
    Text("Android 端不注册桌面式全局键盘快捷键。", style = MaterialTheme.typography.labelSmall)
}

@Composable private fun SectionTitle(text: String) = Text(text, style = MaterialTheme.typography.titleSmall,
    color = MaterialTheme.colorScheme.primary)

@Composable private fun SettingSwitch(text: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(text, modifier = Modifier.weight(1f)); Switch(checked, onChange)
    }
}

@Composable private fun NumberSetting(text: String, value: Int, onChange: (Int) -> Unit) {
    OutlinedTextField(value.toString(), { input -> input.toIntOrNull()?.let { onChange(it.coerceIn(1, 3650)) } },
        label = { Text(text) }, modifier = Modifier.fillMaxWidth(), singleLine = true)
}
