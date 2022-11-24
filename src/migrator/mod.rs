use sea_orm_migration::prelude::*;

pub struct Migrator;

mod m20220101_000001_create_subscription;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20220101_000001_create_subscription::Migration)]
    }
}