# ov
- a implementation of zero2prod but in poem + sea_orm
  - poem supports built-in swagger
- `serde_aux` is not needed since `config-rs` support deserialization of unsigned integers since 0.13.2
  - [the issue](https://github.com/mehcode/config-rs/issues/357)

# run
## migration
- `sea_orm`'s migration must be on the root dir
```bash
# run a new migration
DATABASE_URL=<url> sea-orm-cli migrate

# update ORM struct def
sea-orm-cli generate entity -u <url> -o /path/to/dir

# rollback
DATABASE_URL=<url> sea-orm-cli migrate down
```
- you may want to refer [sea-query](https://github.com/SeaQL/sea-query) when writing migrations
  - raw sql is not compatible across databases

# test
- remember to shut down VPN/HTTP_PROXY when running mockwire test
- to show tracing during test, use `cargo test -- -nocapture`
