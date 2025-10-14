# Generic CircleCI Configuration

This CircleCI configuration is designed to be **project-agnostic** and can be reused for any Rust project.

## Key Features

### ✅ No Hardcoded Project Names

The configuration automatically detects the project name from `Cargo.toml`:

```bash
PACKAGE_NAME=$(grep "^name = " Cargo.toml | head -n 1 | sed 's/name = "\(.*\)"/\1/')
```

### ✅ Relative Paths Only

All paths are relative to the project root (where `Cargo.toml` is located):

- `Cargo.lock` - for cache keys
- `target/` - for build artifacts
- `coverage.lcov` - for coverage reports

### ✅ Standard Cargo Commands

All commands work from the project root without `cd` commands:

```yaml
- cargo fmt -- --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo build --verbose
- cargo test --verbose
- cargo doc --no-deps --verbose
- cargo audit
```

## How to Use

### For This Project (prism3-rust-core)

The configuration is already set up. CircleCI will automatically:
1. Detect it's in the `prism3-rust-core` directory
2. Use the package name from `Cargo.toml`
3. Run all checks from the project root

### For Other Rust Projects

Simply copy the `.circleci/` directory to any Rust project:

```bash
cp -r .circleci/ /path/to/another-rust-project/
```

**Requirements:**
- Project must have `Cargo.toml` in the root
- Standard Rust project structure
- Optional: Set `COVERALLS_REPO_TOKEN` for coverage reporting

### Multi-Project Workspace

For Cargo workspaces with multiple crates, you may need to:

1. Specify the package explicitly:
   ```yaml
   cargo test --package your-package-name
   ```

2. Or test all workspace members:
   ```yaml
   cargo test --workspace
   ```

## Configuration Customization

### Rust Version

Update the Docker image version:

```yaml
executors:
  rust-executor:
    docker:
      - image: cimg/rust:1.75  # Change version here
```

### Resource Class

Adjust compute resources:

```yaml
executors:
  rust-executor:
    resource_class: large  # Options: small, medium, large, xlarge
```

### Workflow Schedule

Modify the nightly security audit schedule:

```yaml
triggers:
  - schedule:
      cron: "0 0 * * *"  # Daily at midnight UTC
```

## What's Generic?

| Component | Generic? | Notes |
|-----------|----------|-------|
| Executors | ✅ Yes | Standard Rust Docker image |
| Cache keys | ✅ Yes | Based on `Cargo.lock` checksum |
| Commands | ✅ Yes | Standard Cargo commands |
| Paths | ✅ Yes | All relative to project root |
| Package detection | ✅ Yes | Auto-detected from `Cargo.toml` |
| Workflows | ✅ Yes | Standard build → test → coverage flow |
| Coveralls setup | ⚠️ Partial | Requires `COVERALLS_REPO_TOKEN` per project |

## Environment Variables Required

Only one environment variable is needed (optional):

- **`COVERALLS_REPO_TOKEN`**: For uploading coverage to Coveralls.io
  - Get from: https://coveralls.io/
  - Add in: CircleCI Project Settings → Environment Variables

## Benefits of This Approach

1. **Reusable**: Copy to any Rust project
2. **Maintainable**: Update in one place, works everywhere
3. **Consistent**: Same CI checks across all projects
4. **Simple**: No project-specific configuration needed

## Documentation

- [SETUP.md](SETUP.md) - Initial setup instructions
- [README.md](README.md) - Full English documentation
- [README.zh_CN.md](README.zh_CN.md) - Full Chinese documentation
- [QUICKSTART.zh_CN.md](QUICKSTART.zh_CN.md) - Quick start guide (Chinese)
- [COVERALLS_SETUP.zh_CN.md](COVERALLS_SETUP.zh_CN.md) - Coveralls integration guide (Chinese)

---

**Created**: 2025-10-13
**Version**: 1.0 (Generic)

