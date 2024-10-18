//! # Slackr
//!
//! Slackr is a library for creating Slack bots in Rust.
//! Slackr aims to provide a simple and intuitive interface for creating bots using both
//! the Slack Web API and Slack's Socket Mode.
//!
//! Credentials for the Slack Web API and Socket Mode are required to use this library.
//! The API keys should be provided in a .env file in the root of your project.
//!
//!
//! ### Note
//! While the [alias_db] module is included in the library, it is not necessary for
//! basic functionality. It is included as an extension to make referring to users
//! and channels by human-readable names easier.

pub mod client;
pub mod event_handler;
pub mod alias_db;

