package com.nsfy.app.cli

import android.app.Activity
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class CliReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val result = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                result.resultCode = Activity.RESULT_OK
                result.resultData = CliCommands(context).execute(intent)
            } catch (error: Exception) {
                result.resultCode = Activity.RESULT_CANCELED
                result.resultData = org.json.JSONObject()
                    .put("ok", false)
                    .put("error", error.message ?: error.javaClass.simpleName)
                    .toString()
            } finally {
                result.finish()
            }
        }
    }
}
