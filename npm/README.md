# clickhousectl

The official ClickHouse CLI: local ClickHouse version manager and ClickHouse Cloud control plane.

```bash
npm install -g clickhousectl
clickhousectl --help
```

This npm package is a thin wrapper that downloads the prebuilt `clickhousectl` binary for your platform from [GitHub Releases](https://github.com/ClickHouse/clickhousectl/releases) at install time. It is published in lock-step with the release tag.

Supported platforms:

- Linux x86_64 (musl)
- Linux aarch64 (musl)
- macOS x86_64
- macOS arm64

For full documentation, see the [project repository](https://github.com/ClickHouse/clickhousectl).

## Installation flags

`clickhousectl` requires npm's postinstall step to fetch the binary. If you use `npm install --ignore-scripts`, the binary won't be downloaded and the `clickhousectl` command will fail with a helpful error. Re-run install without `--ignore-scripts`, or grab the binary directly from GitHub Releases.

## License

Apache-2.0
