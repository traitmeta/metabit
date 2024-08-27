CREATE TABLE IF NOT EXISTS anchor_tx_out (
    tx_id VARCHAR(128) NOT NULL,
    vout INTEGER NOT NULL,
    value BIGINT NOT NULL,
    script_pubkey VARCHAR(128),
    unlock_info VARCHAR(128),
    spent BOOLEAN DEFAULT FALSE,
    confirmed_block_height BIGINT DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER trigger_update_updated_at BEFORE
UPDATE
    ON anchor_tx_out FOR EACH ROW EXECUTE FUNCTION update_updated_at_column ();

CREATE TABLE IF NOT EXISTS indexer (
    height BIGINT NOT NULL,
    hash VARCHAR(128) NOT NULL,
    chain_name VARCHAR(64) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER trigger_update_updated_at BEFORE
UPDATE
    ON indexer FOR EACH ROW EXECUTE FUNCTION update_updated_at_column ();