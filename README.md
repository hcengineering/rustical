# RustiCal

This is a fork of [RustiCal](https://github.com/lennart-k/rustical) to provide a CalDAV server for [Huly Platform](https://github.com/hcengineering/platform) calendars.

This branch supports only `/caldav` endpoint, other features are disabled.

To make it easier to run on cluster, this branch uses only env variables for configuration, no config file:

- `HTTP_HOST` - an address to listen on, default is `0.0.0.0`
- `HTTP_PORT` - a port to listen to, default is `9070`
- `ACCOUNTS_URL` - address of accounts service, default is `http://huly.local:3000`
- `TOKEN_EXPIRATION_SECS` - cached workspace token lifetime, default is `600`
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
- OpenID Connect support (with option to disable password login)

## Getting Started

- Check out the [documentation](https://lennart-k.github.io/rustical/installation/)
