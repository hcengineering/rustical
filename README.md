# RustiCal

This is a fork of [RustiCal](https://github.com/lennart-k/rustical) to provide a CalDAV server for [Huly Platform](https://github.com/hcengineering/platform) calendars.

This branch supports only `/caldav` endpoint, other features are disabled.

To make it easier to run on cluster, this branch uses only env variables for configuration, no config file:

- `HTTP_HOST` - an address to listen on, default is `0.0.0.0`
- `HTTP_PORT` - a port to listen to, default is `9070`
- `ACCOUNTS_URL` - address of the accounts service, default is `http://huly.local:3000`
- `TOKEN_EXPIRATION_SECS` - cached workspace token lifetime, default is `600`
- `LOG_LEVEL` - logging level `error`,`warn`,`info`,`debug`,`trace`,`off`, default is `warn`
- `SYNC_CACHE_PATH` - a path to file-based sync cache, default is none (don't use sync cache)
- `KV_URL` - an address of the huly key-value storage used for sync cache, default is none  (don't use sync cache)

## Sync cache

Sync cache is a storage used for saving synctokens (it's something like a calendar revision) and a set of event ids corresponding to that synctoken. Some clients do not use synctokens. E.g. Evolution or Thunderbird query hashes of all events (etags), then calculate the difference between locally saved etags, and fetch full events whose etags do not match. But the macOS Calendar app, for example, works differently. It stores the latest synctoken; the next time it request a synctoken from the CalDAV server. If it does not match the local one, it request a set of events that has been added and removed. To make it  working we have to store synctokens and correspondng event ids somewhere. This is a *sync cache* which can be file-based for local dev, or can be stored in a key-value storage.

## Local run:

```bash
cargo build & ./target/debug/rustical
```

In a calendar application (e.g. Evolution on Linux) use such URL for registering an external calendar:

```
http://localhost:9070/caldav/user/${HULY_ID}/calendar/${WORKSPACE_NAME}
```

Where `HULY_ID` is a Social ID with type `huly`. 

Before, the access to the user's calendar should be enabled in Huly: 

![](<screen_access.png>)

### Local testing macOS Calendar

It's a bit quirky to test it locally.

Normally, one should use the Manual mode when registergin a CalDAV server and provide the server address and credentials shown on the screenshot above.

![](<screen_macos_manual.png>)

For some good reason, this does not work with locally run dev CalDAV server. So for local testing, the Advanced mode should be used with these parameters:

- Server address: `huly.local`
- Server path: `/caldav/principal/${HULY_ID}`
- Port: `${HTTP_PORT}`
- Username: `${HULY_ID}`
- Password: *Copy from the Access dialog*
- Use SSL: *off*

And event then, it's better to disable authentication :) (see commented code in `crates/store/src/auth/middleware.rs`).

## Deployment

Add and push a version tag:

```
git tag -a v0.0.3
git push origin v0.0.3
```

Build should start on GitHub automatically. Target image has the `service_` prefix to mark it as "internal". Then update the version tag in the [platform](https://github.com/hcengineering/platform/tree/develop/pods/external/services.d). The sevice will be automatically pulled, retagged with the lates platform version, and deployed on the cluster.

<p>&nbsp;</p>
<p>&nbsp;</p>
<p>&nbsp;</p>

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
