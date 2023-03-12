pub use sea_orm_migration::prelude::*;

pub mod m20220101_000001_create_subscription;
mod m20221214_000002_add_status_to_subscription;
mod m20221214_000003_make_status_not_null_in_subscription;
mod m20221225_000004_create_subscription_token_table;
mod m20230212_094638_create_user_table;
mod m20230309_131331_create_idempotency_table;
mod m20230312_033844_relax_null_checks_on_idempotency;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_subscription::Migration),
            Box::new(m20221214_000002_add_status_to_subscription::Migration),
            Box::new(m20221214_000003_make_status_not_null_in_subscription::Migration),
            Box::new(m20221225_000004_create_subscription_token_table::Migration),
            Box::new(m20230212_094638_create_user_table::Migration),
            Box::new(m20230309_131331_create_idempotency_table::Migration),
            Box::new(m20230312_033844_relax_null_checks_on_idempotency::Migration),
        ]
    }
}
