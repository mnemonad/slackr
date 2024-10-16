use std::path::Path;
use rusqlite::{ params, Connection, Result };

use crate::client::{Channel, Member, SlackClient};

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

    /// Pass SlackClient to method to insert/update user & channel aliases to initialized sqlite database.
    pub async fn setup(&self, client: &SlackClient) -> Result<(), Box<dyn std::error::Error>> {
        let members = client.get_user_list().await?;
        self.insert_members(members)?;

        let channels = client.get_channel_list().await?;
        self.insert_channels(channels)?;

        Ok(())
    }

    /// Get human-readable slack user name from id
    pub fn get_real_name(&self, user_id: &str) -> Result<String> {
        let mut stmt = self.conn.prepare("SELECT real_name FROM users WHERE id = ?1")?;
        let real_name: String = stmt.query_row([user_id], |row| row.get(0))?;
        Ok(real_name.to_string())
    }

    /// Get human-readable slack channel name from id
    pub fn get_channel_from_id(&self, channel_id: &str) -> Result<String> {
        let mut stmt = self.conn.prepare("SELECT name FROM channels WHERE id = ?1")?;
        let channel_name: String = stmt.query_row([channel_id], |row| row.get(0))?;
        Ok(channel_name.to_string())
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
            "CREATE INDEX IF NOT EXISTS idx_users_name on users(id);",
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
            "CREATE INDEX IF NOT EXISTS idx_channels_name on channels(id);",
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

    fn insert_members(&self, members: Vec<Member>) -> Result<()> {
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

    fn insert_channels(&self, channels: Vec<Channel>) -> Result<()> {
        for channel in channels {
            self.insert_channel(&channel.channel_id, &channel.name, channel.is_private)?;
        }
        Ok(())
    }


}
