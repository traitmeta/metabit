use super::*;
use datatypes::types;
use tokio::sync::RwLock;

pub struct UtxoUpdater {
    address: String,
    share_data: Arc<RwLock<Vec<types::Utxo>>>,
}

impl UtxoUpdater {
    pub fn new(cfg: &config::Config, data: Arc<RwLock<Vec<types::Utxo>>>) -> Self {
        Self {
            address: cfg.sign.receiver.clone(),
            share_data: data,
        }
    }

    pub async fn update_utxo(&self) -> Result<()> {
        let utxos = mempool::utxo::gets_uspent_utxo(&self.address).await?;
        if utxos.is_empty() {
            return Ok(());
        }

        info!("update utxo: {:?}", utxos);
        let mut share_data = self.share_data.write().await;
        *share_data = utxos;
        Ok(())
    }
}
