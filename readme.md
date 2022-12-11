# ov
- `serde_aux` is not needed since `config-rs` support deserialization of unsigned integers since 0.13.2
  - [the issue](https://github.com/mehcode/config-rs/issues/357)

# run
- `sea_orm`'s migration must be on the root dir

# test
- remember to shut down VPN/HTTP_PROXY when running mockwire test
- to show tracing during test, use `cargo test -- -nocapture`
