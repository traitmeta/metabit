pub mod btcrpc;
pub mod checker;
pub mod config;
pub mod lightning;
pub mod receiver;
pub mod sender;

use anyhow::{anyhow, Result};
use bitcoin::{consensus::deserialize, Transaction};
use std::{sync::Arc, time::Duration};
use tgbot::TgBot;
use tokio::{sync::broadcast::Receiver, time::sleep};
use tracing::{error, info, warn};
use zmq::{Context, Socket};