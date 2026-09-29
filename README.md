# env-manager

A focused command-line tool for managing `.env` files as named profiles and running commands with a selected profile.

## Quick start

```sh
cargo run -- init
cargo run -- set development API_URL http://localhost:3000
cargo run -- set staging API_URL https://staging.example.com
cargo run -- list
cargo run -- run development cargo test
```

Profiles are ordinary dotenv files in the selected directory:

```text
.env.development
.env.staging
.env.production
```

Use `-d, --directory` to manage profiles somewhere else. Values are passed directly to the child process; the manager does not mutate the shell that launched it. `ENV_PROFILE` is also set for the child process so applications can identify the active profile.

## Commands

| Command | Purpose |
| --- | --- |
| `init` | Create the three standard profiles without overwriting existing files |
| `list` | Show profiles, paths, and variable counts |
| `show <profile>` | Print a profile's variables |
| `get <profile> <key>` | Print one value |
| `set <profile> <key> <value>` | Add or replace a value |
| `unset <profile> <key>` | Remove a value |
| `run <profile> <command>...` | Run a command with profile variables |

Keep `.env.production` out of version control when it contains secrets. A typical `.gitignore` can ignore the profile files while committing safe templates such as `.env.example`.