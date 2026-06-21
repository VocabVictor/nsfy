package com.nsfy.app

import android.app.Application
import com.nsfy.app.data.db.AppDatabase

class NsfyApp : Application() {
    val database by lazy { AppDatabase.getInstance(this) }

    override fun onCreate() {
        super.onCreate()
        instance = this
    }

    companion object {
        lateinit var instance: NsfyApp
            private set
    }
}
