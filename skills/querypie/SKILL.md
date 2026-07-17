---
name: querypie
description: Runs SQL and inspects schemas, tables, and DDL against QueryPie-managed databases from the terminal. Use whenever a database is reachable only through a QueryPie connection. Typical requests include "query the <name> db", "show the schema of <table>", "list the QueryPie connections", and "/querypie". Auto-installs the querypie CLI if missing.
---

# QueryPie

## Prerequisites

Install querypie-cli if absent (Linux/macOS); ensure `~/.local/bin` is on PATH:

```sh
command -v querypie >/dev/null || { curl -fsSL https://raw.githubusercontent.com/sudosubin/querypie-cli/main/scripts/install.sh | sh; export PATH="$HOME/.local/bin:$PATH"; }
```

## Config

Set defaults once in `~/.config/querypie-cli/config.json` to avoid repeating flags:

```json
{ "host": "querypie.example.com", "connection": "example-main [US]", "database": "example_db" }
```

Anything not in config is passed per command: `--host`, `-c/--connection`, `--engine`, `-d/--db`, `--schema`.

## Auth

Webview login; do this first (and whenever a command reports "not logged in"):

```sh
querypie auth status || querypie auth login
```

## Explore

```sh
querypie connection list
querypie -c '<conn>' --engine mysql database list
querypie -c '<conn>' --engine mysql -d <db> schema list
querypie -c '<conn>' --engine mysql -d <db> --schema <schema> table list
querypie -c '<conn>' --engine mysql -d <db> table describe <table>
querypie -c '<conn>' --engine mysql -d <db> table ddl <table>
```

## Query

```sh
querypie -c '<conn>' --engine mysql -d <db> query 'select 1;'
querypie -c '<conn>' --engine mysql -d <db> query --limit 50 --output json 'select * from <table>;'
querypie -c '<conn>' --engine mysql -d <db> query --file ./query.sql
echo 'select 1;' | querypie -c '<conn>' --engine mysql -d <db> query -
```

- Row cap defaults to `--limit 1000`; raise it explicitly when needed.
- Use `--output json` when parsing results programmatically; `--no-truncate` for wide tables.

## Notes

- Sessions are cached: `querypie session list`, `querypie session clear`.
- Quote connection names with spaces/brackets: `-c 'example-main [US]'`.
