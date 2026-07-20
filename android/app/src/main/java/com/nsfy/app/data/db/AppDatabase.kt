package com.nsfy.app.data.db

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.room.migration.Migration
import androidx.sqlite.db.SupportSQLiteDatabase
import com.nsfy.app.data.model.MessageEntity
import com.nsfy.app.data.model.MessageStateEntity
import com.nsfy.app.data.model.TopicEntity

@Database(
    entities = [TopicEntity::class, MessageEntity::class, MessageStateEntity::class],
    version = 4,
    exportSchema = false,
)
abstract class AppDatabase : RoomDatabase() {
    abstract fun topicDao(): TopicDao
    abstract fun messageDao(): MessageDao
    abstract fun messageStateDao(): MessageStateDao

    companion object {
        @Volatile
        private var INSTANCE: AppDatabase? = null

        fun getInstance(context: Context): AppDatabase {
            return INSTANCE ?: synchronized(this) {
                INSTANCE ?: Room.databaseBuilder(
                    context.applicationContext,
                    AppDatabase::class.java,
                    "nsfy.db"
                ).addMigrations(MIGRATION_1_2, MIGRATION_2_3, MIGRATION_3_4)
                    .build().also { INSTANCE = it }
            }
        }

        private val MIGRATION_1_2 = object : Migration(1, 2) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL("ALTER TABLE messages ADD COLUMN category TEXT NOT NULL DEFAULT '[]'")
            }
        }

        private val MIGRATION_2_3 = object : Migration(2, 3) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL("ALTER TABLE messages ADD COLUMN popup INTEGER NOT NULL DEFAULT 0")
                db.execSQL("ALTER TABLE messages ADD COLUMN bypassDnd INTEGER NOT NULL DEFAULT 0")
                db.execSQL("UPDATE messages SET popup = priority >= 4")
            }
        }

        private val MIGRATION_3_4 = object : Migration(3, 4) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL("ALTER TABLE topics ADD COLUMN lastConnectedAt INTEGER NOT NULL DEFAULT 0")
                db.execSQL("ALTER TABLE messages ADD COLUMN read INTEGER NOT NULL DEFAULT 0")
                db.execSQL("ALTER TABLE messages ADD COLUMN deletedAt INTEGER")
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS message_states (" +
                        "key TEXT NOT NULL PRIMARY KEY, topicId TEXT NOT NULL, " +
                        "messageId TEXT NOT NULL, status TEXT NOT NULL, " +
                        "updatedAt INTEGER NOT NULL, pending INTEGER NOT NULL DEFAULT 0)",
                )
            }
        }
    }
}
