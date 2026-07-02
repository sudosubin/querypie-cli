# querypie-cli

Rust/Tauri CLI for querying QueryPie databases through the QueryPie web API.

## Authentication

The CLI uses a dedicated Tauri webview for QueryPie login. The webview runtime
cookie store is the source of truth for httpOnly QueryPie cookies; the CLI does
not store auth cookies in config files.

```bash
cargo run -- --host querypie.example.com auth login
```

When an access token expires, the CLI attempts a gRPC-Web refresh in the
background and writes rotated cookies back to the same webview cookie store. If
refresh fails or no login exists, the command exits with an auth error instead
of opening a login window.

## Usage

```bash
cargo run -- --host querypie.example.com connection list
cargo run -- --host querypie.example.com -c 'example-main [US]' --engine mysql database list
cargo run -- --host querypie.example.com -c 'example-main [US]' --engine mysql query 'select 1'
```

Optional config file:

```yaml
host: querypie.example.com
connection: example-main [US]
database: example_db
```

By default the config path is `~/.config/querypie/config.yml`.
