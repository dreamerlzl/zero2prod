# ov
- a implementation of [zero2prod](https://github.com/LukeMathWalker/zero-to-production) but in [poem](https://github.com/poem-web/poem) + [sea_orm](https://github.com/SeaQL/sea-orm)
  - poem supports built-in swagger/OpenAPI docs

# compared to Luke's implementation in details
- macro error setup
  - credit: [aptos-core](https://github.com/aptos-labs/aptos-core/blob/main/api/src/response.rs)
- less test setup boilerplate
  - `macro_rules`
  - `[sqlx::test]`
- a mix use of sea-orm and sqlx
  - why sqlx with sea-orm? -> psql's `create type` is not fully supported in sea-orm
- `serde_aux` is not needed since `config-rs` support deserialization of unsigned integers since 0.13.2
  - [the issue](https://github.com/mehcode/config-rs/issues/357)

# run
## migration
- `sea_orm`'s migration must be run under the project dir
```bash
# run a new migration
DATABASE_URL=<url> sea-orm-cli migrate

# update ORM struct def
sea-orm-cli generate entity -u <url> -o /path/to/dir  --ignore-tables idempotency,seaql_migrations

# rollback
DATABASE_URL=<url> sea-orm-cli migrate down
```
- you may want to refer [sea-query](https://github.com/SeaQL/sea-query) when writing migrations
  - raw sql is not compatible across databases

# test
- you need to setup a redis and a psql first; execute `scripts/init_redis.sh` and `scripts/init_db.sh` first
- remember to shut down global `VPN/HTTP_PROXY` when running mockwire test
- to show tracing during test, use `TEST_LOG=1 cargo test -- -nocapture`
