# npm releases

Use the staging helper in the repo root to generate npm tarballs for a release. For
example, to stage the Peezy Codex package family for version `0.130.0`:

```bash
./scripts/stage_npm_packages.py \
  --release-version 0.130.0 \
  --package codex \
  --package codex-responses-api-proxy \
  --package codex-sdk
```

This downloads the native artifacts once, hydrates `vendor/` for each package, and writes
tarballs to `dist/npm/`.

When `--package codex` is provided, the staging helper builds the lightweight
`@peezy.tech/codex` meta package plus all platform-native `@peezy.tech/codex` variants
that are later published under platform-specific dist-tags. The additional package
arguments stage `@peezy.tech/codex-responses-api-proxy` and `@peezy.tech/codex-sdk`;
the SDK package depends on the matching `@peezy.tech/codex` version.

If you need to invoke `build_npm_package.py` directly, run
`codex-cli/scripts/install_native_deps.py` first and pass `--vendor-src` pointing to the
directory that contains the populated `vendor/` tree.
