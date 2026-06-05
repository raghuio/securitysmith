-- Migration 003: Recovery phrase envelope storage
-- Stores the encrypted recovery envelope inside the vault (supplementary to recovery_envelope.bin).

CREATE TABLE IF NOT EXISTS recovery (
    id                 INTEGER PRIMARY KEY CHECK(id = 1),
    encrypted_envelope BLOB NOT NULL,
    envelope_salt        BLOB NOT NULL,
    created_at         INTEGER NOT NULL DEFAULT (unixepoch())
);
