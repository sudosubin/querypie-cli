<div align="center">

# querypie-cli

[![version](https://badgen.net/github/release/sudosubin/querypie-cli?label=version)](https://github.com/sudosubin/querypie-cli/releases)
[![QueryPie](https://badgen.net/badge/QueryPie/11.5.4/blue?icon=data:image/svg%2Bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9Ii0xMCAtMTAgODggODgiPjxwYXRoIGZpbGw9IiNmZmYiIGQ9Ik00OC4wOCA1LjgzYTE5LjkgMTkuOSAwIDAgMC0yOC4xNiAwTDUuODMgMTkuOTJhMTkuOSAxOS45IDAgMCAwIDAgMjguMTZsMTQuMDkgMTQuMDlhMTkuOSAxOS45IDAgMCAwIDI4LjE2IDBsMy4wOC0zLjA4LTcuMDEtNy4wMS0xLjcgMS43YTExLjk1IDExLjk1IDAgMCAxLTE2LjkgMEwxNC4yOCA0Mi41YTExLjk1IDExLjk1IDAgMCAxIDAtMTYuOWwxMS4yNy0xMS4yN2ExMS45NSAxMS45NSAwIDAgMSAxNi45IDBMNTMuNzIgMjUuNmExMS45NSAxMS45NSAwIDAgMSAwIDE2LjlsLS44NC44NCA3IDcuMDEgMi4yOS0yLjI4YTE5LjkgMTkuOSAwIDAgMCAwLTI4LjE2em0tOS45NCAxOC4yM2E2IDYgMCAwIDAtOC40NSAwbC01LjYzIDUuNjNhNiA2IDAgMCAwIDAgOC40NWw1LjYzIDUuNjNhNiA2IDAgMCAwIDguNDUgMGw1LjYzLTUuNjNhNiA2IDAgMCAwIDAtOC40NXoiLz48L3N2Zz4=)](https://docs.querypie.com)
[![license](https://badgen.net/github/license/sudosubin/querypie-cli?color=green)](LICENSE)

Query QueryPie databases from the terminal with webview authentication.

<a href="./docs/assets/querypie-cli-demo.avif">
  <img src="./docs/assets/querypie-cli-demo.avif" alt="querypie-cli demo" width="1200" />
</a>

</div>

## Quick Start

```sh
querypie --host querypie.example.com auth login
querypie --host querypie.example.com connection list
querypie --host querypie.example.com query -c '<connection>' 'select 1;'
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
| `auth logout` | Clear the WebView profile for a host |
| `auth status` | Show login status |
| `completion <shell>` | Generate shell completions |
| `connection list` | List available QueryPie connections |
| `database list` | List databases for a connection |
| `query <sql>` | Run SQL through QueryPie |
| `schema list` | List schemas for a database |
| `session clear` | Clear cached database sessions |
| `session list` | List cached database sessions |
| `table ddl <table>` | Show QueryPie table DDL |
| `table describe <table>` | Show QueryPie table structure |
| `table list` | List tables |

## Examples

```sh
querypie --host querypie.example.com connection list
querypie --host querypie.example.com database list -c '<connection>'
querypie --host querypie.example.com query -c '<connection>' --db example_db 'select 1;'
querypie --host querypie.example.com query -c '<connection>' --db example_db --limit 100 'select * from users;'
querypie --host querypie.example.com table describe -c '<connection>' --db example_db users
querypie --host querypie.example.com table list -c '<connection>' --db example_db
```

Use `--output json` for machine-readable output.

```sh
querypie --host querypie.example.com connection list --output json
```

## Authentication

- Login uses a dedicated Tauri WebView.
- httpOnly QueryPie cookies stay in the WebView cookie store.
- Access token refresh runs automatically in the background.
- If refresh fails for a previously authenticated host, commands open the login WebView and continue after login.
- If no login exists, commands exit with an auth error.

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

## Shell Completions

Generate completions for your shell:

```sh
querypie completion bash > querypie.bash
querypie completion elvish > querypie.elv
querypie completion fish > ~/.config/fish/completions/querypie.fish
querypie completion powershell > _querypie.ps1
querypie completion zsh > "${fpath[1]}/_querypie"
```

Completions can query your logged-in QueryPie host for connection, engine, database,
schema, and table candidates. If authentication or network access is unavailable,
dynamic candidates are skipped silently.

## Output

- `--color auto|always|never`: control ANSI color (`auto` colors terminal output unless `NO_COLOR` is set)
- `--limit <N>`: maximum query rows to fetch, defaults to `1000` and must be at least `1`
- `--no-truncate`: do not shorten long table cells
- `--output json`: raw JSON output
- `--output text`: default human-readable output
- `QUERYPIE_NO_TRUNCATE=1`: disable truncation globally

`NULL` values are rendered distinctly in colored text output. JSON output never includes ANSI escape sequences.
Text query output reports `(N rows, limit reached)` when the result reaches the configured row limit. JSON query output includes `limit` and `limit_reached` fields alongside `columns` and `rows`.

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
cargo build --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test --all-features
```

## License

MIT, see [LICENSE](./LICENSE).
