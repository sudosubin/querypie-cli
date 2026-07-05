<div align="center">

# querypie-cli

[![license](https://badgen.net/github/license/sudosubin/querypie-cli)](LICENSE)
[![release](https://badgen.net/github/release/sudosubin/querypie-cli)](https://github.com/sudosubin/querypie-cli/releases)
[![crates.io](https://badgen.net/crates/v/querypie-cli)](https://crates.io/crates/querypie-cli)
[![built with rust](https://badgen.net/badge/built%20with/Rust/orange)](https://www.rust-lang.org)

Query QueryPie databases from the terminal with webview authentication.

<video src="./docs/assets/querypie-cli-demo.webm" controls muted playsinline width="100%"></video>

</div>

## Quick Start

```sh
querypie --host querypie.example.com auth login
querypie --host querypie.example.com connection list
querypie --host querypie.example.com -c '<connection>' --engine mysql query 'select 1;'
```

## Installation

```sh
cargo install querypie-cli
```

Or download a binary from [GitHub Releases](https://github.com/sudosubin/querypie-cli/releases).

Build from source:

```sh
cargo build --release
```

Linux builds require the WebKitGTK and Tauri system packages used by the CI workflow.

## Commands

| Command | Purpose |
| --- | --- |
| `auth login` | Open the QueryPie WebView login |
| `auth status` | Show login status |
| `auth logout` | Clear the WebView profile for a host |
| `connection list` | List available QueryPie connections |
| `database list` | List databases for a connection |
| `schema list` | List schemas for a database |
| `table list` | List tables |
| `table describe <table>` | Show QueryPie table structure |
| `table ddl <table>` | Show QueryPie table DDL |
| `query <sql>` | Run SQL through QueryPie |
| `session list` | List cached database sessions |
| `session clear` | Clear cached database sessions |

## Examples

```sh
querypie --host querypie.example.com connection list
querypie --host querypie.example.com -c '<connection>' --engine mysql database list
querypie --host querypie.example.com -c '<connection>' --engine mysql --db example_db table list
querypie --host querypie.example.com -c '<connection>' --engine mysql --db example_db table describe users
querypie --host querypie.example.com -c '<connection>' --engine mysql --db example_db query 'select 1;'
```

Use `--output json` for machine-readable output.

```sh
querypie --host querypie.example.com connection list --output json
```

## Authentication

- Login uses a dedicated Tauri WebView.
- httpOnly QueryPie cookies stay in the WebView cookie store.
- Access token refresh runs automatically in the background.
- If refresh fails or no login exists, commands exit with an auth error.
- Commands do not open a login window automatically.

## Configuration

Default path:

```text
~/.config/querypie/config.json
```

Example:

```json
{
  "host": "querypie.example.com",
  "connection": "example-main",
  "database": "example_db"
}
```

CLI flags override config values.

## Output

- `--output text`: default human-readable output
- `--output json`: raw JSON output
- `--no-truncate`: do not shorten long table cells
- `QUERYPIE_NO_TRUNCATE=1`: disable truncation globally

`NULL` values are rendered distinctly in text output.

## How It Works

1. `auth login` opens QueryPie in a Tauri WebView.
2. The CLI reads httpOnly cookies through the same WebView profile.
3. API calls use QueryPie's gRPC-Web endpoints.
4. Expired access tokens are refreshed through the WebView cookie store.
5. Database sessions are cached under `~/.cache/querypie`.

## Troubleshooting

Check auth:

```sh
querypie --host querypie.example.com auth status
```

Clear cached database sessions:

```sh
querypie --host querypie.example.com session clear
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --all-features
```

## License

[MIT](LICENSE)
