# About

The backend of yuanyang.app 2025, deployed on alicloud.

# Development

## Credential

EXAMPLE `.env` (for local development only, the file is ignored in both .gitignore and .fcignore)

```
DATABASE_URL=postgres://<user>:<password>@<server>/<dbname>
REGISTER_TOKEN=
LOGIN_TOKEN=
COOKIE_TOKEN=
```

## Database

psql (PostgreSQL) 16.4 (Ubuntu 16.4-0ubuntu0.24.04.2) is used.


``` bash
cargo install diesel_cli --no-default-features --features postgres
diesel setup

```

migrate:

``` bash
diesel migration generate create_users_and_teams
```

example `up.sql`

```sql
-- teams table
CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR NOT NULL UNIQUE,
    members_count INTEGER DEFAULT 0,
    score INTEGER DEFAULT 0
);

-- users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR NOT NULL UNIQUE,
    password VARCHAR NOT NULL,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL
);
```

example `down.sql`

```sql
DROP TABLE users;
DROP TABLE teams;

```

Then, run

```bash
diesel migration run
```

```bash
RUST_LOG=info MODE=dev cargo run
```



# Building and Deploying

1. migrate the database.
2. `./build.sh`. The binary server will be copied into `./build`, where it is gitignored but not fcignored.
3. set environment variables in `s.yaml` (out of this repo), for AliCloud FC. `./run.sh` will be triggered. Additional environment variables may also be set here.
4. `cd .. && s deploy`.

