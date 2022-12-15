pub use sea_orm_migration::prelude::*;

pub mod m20220101_000001_create_subscription;
mod m20221214_000002_add_status_to_subscription;
mod m20221214_000003_make_status_not_null_in_subscription;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_subscription::Migration),
            Box::new(m20221214_000002_add_status_to_subscription::Migration),
            Box::new(m20221214_000003_make_status_not_null_in_subscription::Migration),
        ]
    }
}
