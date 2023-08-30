//! Package for MigrationManager, which depends on a set of `Migration`s

use log::info;
use std::{sync::Arc, fmt};

use anyhow::Context;
use rusqlite::Transaction;

use super::migration::Migration;

pub trait MigrationManager<'a> {
    fn register_migration(&mut self, migration: Arc<dyn Migration>);

    /// Initialize the migration system in the database
    fn initialize(&self, conn: &'a Transaction) -> anyhow::Result<()>;

    /// Get the current schema version number from the DB, if present at all
    fn get_current_schema_version(&self, conn: &'a Transaction) -> anyhow::Result<u64>;

    fn get_target_schema_version(&self) -> u64;

    fn upgrade_schema(&self, conn: &'a Transaction, from: u64, to: u64) -> anyhow::Result<()>;
}

pub struct LinearMigrationManager {
    pub migrations: Vec<Arc<dyn Migration>>,
}

impl LinearMigrationManager {
    pub fn new() -> Self {
        LinearMigrationManager {
            migrations: Vec::new(),
        }
    }
}

impl<'a> MigrationManager<'a> for LinearMigrationManager {
    fn register_migration(&mut self, migration: Arc<dyn Migration>) {
        // Get a reference to the migration and attempt to copy it
        self.migrations.push(Arc::clone(&migration));
    }

    fn initialize(&self, conn: &'a Transaction) -> anyhow::Result<()> {
        conn.execute_batch(
            r"
            create table if not exist schema_versions (
                version INTEGER NOT NULL,
                is_current INTEGER NOT NULL,
                PRIMARY KEY (version)
            )
        ",
        )?;

        Ok(())
    }

    fn get_current_schema_version(&self, conn: &'a Transaction) -> anyhow::Result<u64> {
        let version = conn.query_row(
            "select version from schema_versions where is_current = 1",
            [],
            |row| {
                row.get_ref(0).map(|version| {
                    version
                        .as_i64()
                        .context("could not cast value to u64")
                        .unwrap()
                })
            },
        )?;

        Ok(version as u64)
    }

    fn get_target_schema_version(&self) -> u64 {
        self.migrations.len() as u64
    }

    fn upgrade_schema(&self, conn: &'a Transaction, from: u64, to: u64) -> anyhow::Result<()> {
        info!("Executing upgrade from {} to {}", from, to);
        // Enforce version ranges are valid
        if from <= 0 || from >= (self.migrations.len() as u64) {
            return anyhow::Result::Err(MigrationError::InvalidSchemaVersion.into());
        }

        if to < from {
            return anyhow::Result::Err(MigrationError::InvalidSchemaRange.into());
        }

        for i in from..to {
            // Get those schema versions
            let migration = self.migrations.get(i as usize).unwrap();
            info!("starting migration {}", &i);
            migration.forward(&conn)?;
            info!("migration {} complete", &i);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum MigrationError {
    InvalidSchemaVersion,
    InvalidSchemaRange,
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}


impl Drop for LinearMigrationManager {
    fn drop(&mut self) {
        self.migrations.clear();
    }
}
