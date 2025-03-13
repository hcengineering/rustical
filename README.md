# RustiCal

This is a fork of [RustiCal](https://github.com/lennart-k/rustical) to provide a CalDAV server for [Huly Platform](https://github.com/hcengineering/platform) calendars.

This branch supports only `/caldav` endpoint, other features are disabled.

To make it easier to run on cluster, this branch uses only env variables for configuration, no config file:

- `HTTP_HOST` - an address to listen on, default is `0.0.0.0`
- `HTTP_PORT` - a port to listen to, default is `9070`
- `ACCOUNTS_URL` - address of accounts service, default is `http://huly.local:3000`
- `LOG_LEVEL` - logging level `error`,`warn`,`info`,`debug`,`trace`,`off`, default is `warn`

Local run:

```bash
cargo build & ./target/debug/rustical
```

In a calendar application (e.g. Evolution on Linux) use such URL for registering an external calendar:

```
http://localhost:9070/caldav/user/${USER_EMAIL}/calendar/${WORKSPACE_NAME}
```

---

**Original readme:**

a CalDAV/CardDAV server

> [!CAUTION]
> RustiCal is **not production-ready!**
> There can be changes to the database without migrations and there's no guarantee that all endpoints are secured yet.
> If you still want to play around with it in its current state, absolutely feel free to do so but know that not even I use it productively yet.

## Features

- easy to backup, everything saved in one SQLite database
- [WebDAV Push](https://github.com/bitfireAT/webdav-push/) support, so near-instant synchronisation to DAVx5
- lightweight (the container image contains only one binary)
- adequately fast (I'd say blazingly fast™ :fire: if I did the benchmarks to back that claim up)
- deleted calendars are recoverable
- Nextcloud login flow (In DAVx5 you can login through the Nextcloud flow and automatically generate an app token)

## Installation

### Manual

```sh
cargo install --locked --git https://github.com/lennart-k/rustical
```

### Docker

```sh
docker run \
  -p 4000:4000 \
  -v YOUR_DATA_DIR:/var/lib/rustical/ \
  -v YOUR_CONFIG_TOML:/etc/rustical/config.toml \
  ghcr.io/lennart-k/rustical
```

## Configuration

RustiCal can either be configured using a TOML file or using environment variables.

You can generate a default `config.toml` with

```sh
rustical gen-config
```

> [!WARNING]
> The `rustical gen-config` command generates a random `frontend.secret_key`.
> This secret is used to generate session cookies so if it is leaked an attacker could use it to authenticate to against any endpoint (also when the frontend is disabled).

You'll have to set your database path to something like `/var/lib/rustical/db.sqlite3`.

### Environment variables

The options in `config.toml` can also be configured using environment variables. Names translate the following:

```toml
[data_store.toml]
path = "asd"
```

becomes `RUSTICAL_DATA_STORE__TOML__PATH`.
Every variable is

- uppercase
- prefixed by `RUSTICAL_`
- Dots become `__`

### Users and groups

Next, configure the principals by creating a file specified in `auth.toml.path` (by default `/etc/rustical/principals.toml`) and inserting your principals:

```toml
[[principals]]
id = "user"
displayname = "User"
password = "$argon2id$......."
app_tokens = [
  {name = "Token", token = "$pbkdf2-sha256$........"},
]
memberships = ["group:amazing_group"]

[[principals]]
id = "group:amazing_group"
user_type = "group"
displayname = "Amazing group"
```

Password hashes can be generated with

```sh
rustical pwhash
```

### Docker

You can also run the upper commands in Docker with

```sh
docker run --rm ghcr.io/lennart-k/rustical rustical gen-config
docker run -it --rm ghcr.io/lennart-k/rustical rustical pwhash
```

### Password vs app tokens

The password is meant as a password you use to log in to the frontend.
Since it's sensitive information,
the secure but slow hash algorithm `argon2` is chosen.

I recommend to generate random app tokens for each CalDAV/CardDAV client.
These can use the faster `pbkdf2` algorithm.

### WebDAV Push

RustiCal supports [WebDAV Push](https://github.com/bitfireAT/webdav-push/) which can notify compatible clients like DAVx5 about changed calendar/addressbook objects.
Since push messages are currently not encrypted you might potentially want to ensure that users only subscribe through your push server (e.g. [ntfy.sh](https://ntfy.sh/)), you can configure it the following:

```toml
[dav_push]
# Must strictly be the URL origin (so no trailing slashes)
allowed_push_servers = ["https://your-instance-ntfy.sh"]
```

## Debugging

Set the log level with following environment variables:

```sh
RUST_LOG="debug"
RUST_BACKTRACE=1
```

RustiCal also supports exporting opentelemetry traces to inspect with tools like [Jaeger](https://www.jaegertracing.io/).
To enable you need to compile with the `opentelemtry` (or `debug`) feature and enable opentelemetry in the config with

```toml
[tracing]
opentelemetry = true
```

## Relevant RFCs

- Versioning Extensions to WebDAV: [RFC 3253](https://datatracker.ietf.org/doc/html/rfc3253)
  - provides the REPORT method
- Calendaring Extensions to WebDAV (CalDAV): [RFC 4791](https://datatracker.ietf.org/doc/html/rfc4791)
- Scheduling Extensions to CalDAV: [RFC 6638](https://datatracker.ietf.org/doc/html/rfc6638)
  - not sure yet whether to implement this
- Collection Synchronization WebDAV [RFC 6578](https://datatracker.ietf.org/doc/html/rfc6578)
  - We need to implement sync-token, etc.
  - This is important for more efficient synchronisation
- iCalendar [RFC 2445](https://datatracker.ietf.org/doc/html/rfc2445#section-3.10)
