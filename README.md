# Silly Fast Fresh Deploys with Rust

This was made for the [series of the same name on YouTube](https://www.youtube.com/playlist?list=PLxV2db94vuXkvZuZrx-iBvrO13dtIQ91k)

It shows how to use Rust to build a TCP proxy that does Blue/Green deploys of a Fresh/Deno app, which is stored in a Postgres instance (on Amazon RDS for me, but it could be anywhere). The whole thing runs on [Control Plane](https://controlplane.com), who sponsored this series (thanks!)

### Running this locally

Start the project:

```
deno task start
```

This will watch the project directory and restart as necessary.

See also the various tasks in `Justfile` - the fresh app itself isn't the most interesting part, the Rust code is.
