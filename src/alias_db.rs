use std::path::Path;
use rusqlite::{ params, Connection, Result };

use crate::client::{Channel, Member};

pub struct Database {
    conn: Connection
}

impl Database {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Initialize tables
        Self::initialize_users_table(&conn)?;
        Self::initialize_channels_table(&conn)?;

        Ok(Database { conn })
    }

    fn initialize_users_table(conn: &Connection) -> Result<()> {
        conn.execute(
            "
            CREATE TABLE IF NOT EXISTS users (
                id text PRIMARY KEY,
                name TEXT NOT NULL,
                real_name TEXT,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            ",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_users_name on users(name);",
            [],
        )?;

        Ok(())
    }

    fn initialize_channels_table(conn: &Connection) -> Result<()> {
        conn.execute(
            "
            CREATE TABLE IF NOT EXISTS channels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                is_private INTEGER DEFAULT 0,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            ",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_channels_name on channels(name);",
            [],
        )?;

        Ok(())
    }

    fn insert_user(&self, id: &str, name: &str, real_name: Option<&str>) -> Result<()> {
        self.conn.execute(
            "
            INSERT OR REPLACE INTO users (id, name, real_name, updated_at)
            VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP);
            ",
            params![id, name, real_name],
        )?;
        Ok(())
    }

    pub fn insert_members(&self, members: Vec<Member>) -> Result<()> {
        for member in members.into_iter().filter(|m| !m.deleted) {
            self.insert_user(&member.user_id, &member.name, member.real_name.as_deref())?;
        }
        Ok(())
    }

    fn insert_channel(&self, id: &str, name: &str, is_private: bool) -> Result<()> {
        self.conn.execute(
            "
            INSERT OR REPLACE INTO channels (id, name, is_private, updated_at)
            VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP);
            ",
            params![id, name, is_private as u8]
            )?;
        Ok(())
    }

    pub fn insert_channels(&self, channels: Vec<Channel>) -> Result<()> {
        for channel in channels {
            self.insert_channel(&channel.channel_id, &channel.name, channel.is_private)?;
        }
        Ok(())
    }
}
