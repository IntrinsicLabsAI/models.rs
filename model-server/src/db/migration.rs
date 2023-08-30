use anyhow::{Context, Ok};
use rusqlite::Connection;

/// Database migration
pub trait Migration {
    fn forward(&self, conn: &Connection) -> anyhow::Result<()>;
}

/// List of migrations to be executed.
#[derive(Clone, Copy, Debug)]
pub struct V0;

impl Migration for V0 {
    fn forward(&self, conn: &Connection) -> anyhow::Result<()> {
        conn.execute_batch(
            r"
        begin;
        create table if not exists model (
            id          text not null,
            name        text unique,
            model_type  text not null,
            runtime     text not null,
            description text not null,

            primary key (id)
        );

        create table if not exists model_version (
            model_id    text not null,
            version     text not null,

            primary key (model_id, version),
            foreign key (model_id) references model(id)
        );

        create table if not exists import_metadata (
            model_id        text not null,
            model_version   text not null,
            source text     not null,
            imported_at     datetime not null,

            primary key (model_id, model_version),
            foreign key (model_id) references model(id),
            foreign key (model_version) references model_version(version)
        );

        create table if not exists model_params (
            model_id        text not null,
            model_version   text not null,
            params          text not null,

            primary key (model_id, model_version),
            foreign key (model_id) references model(id),
            foreign key (model_version) references model_version(version)
        );

        create table if not exists saved_experiments (
            id              text not null,
            model_id        text not null,
            model_version   text not null,
            temperature     float not null,
            tokens          integer not null,
            prompt          text not null,
            output          text not null,
            created_at      datetime not null,

            primary key (id),
            foreign key (model_id) references model(id),
            foreign key (model_version) references model(version)
        );

        commit;
    ",
        )
        .context("failed to execute migration v0 -- create initial tables")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{V0, Migration};

    #[test]
    fn test_migration() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        
        // Test migrations
        V0.forward(&db).unwrap();
    }
}
