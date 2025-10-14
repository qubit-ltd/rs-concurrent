# CircleCI Quick Start Guide

[简体中文](QUICKSTART.zh_CN.md) | English

## 🚀 5-Minute Quick Setup

### 1. Enable CircleCI (1 minute)

1. Visit https://circleci.com/
2. Login with GitHub
3. Select your project
4. Click "Set Up Project"
5. ✅ Done! Build starts automatically

### 2. Configure Coveralls (2 minutes, optional)

Enable coverage reporting service:

1. Visit [Coveralls.io](https://coveralls.io/)
2. Login with GitHub
3. Enable your repository
4. Copy `COVERALLS_REPO_TOKEN`
5. Add environment variable in CircleCI project settings:
   ```
   Name: COVERALLS_REPO_TOKEN
   Value: [your token]
   ```

**Detailed guide**: See [Coveralls Setup Guide](COVERALLS_SETUP.md)

### 3. Add Badges to README (1 minute)

Add to your project's `README.md`:

```markdown
[![CircleCI](https://circleci.com/gh/<org>/<repo>.svg?style=svg)](https://circleci.com/gh/<org>/<repo>)
[![Coverage Status](https://coveralls.io/repos/github/<org>/<repo>/badge.svg?branch=main)](https://coveralls.io/github/<org>/<repo>?branch=main)
```

### 4. Local Testing (3 minutes)

Run checks before committing:

```bash
./ci-check.sh
```

## 📊 CI Workflow Overview

```
Commit Code → GitHub
    ↓
    ├── ✨ Format check (30s)
    ├── 🔧 Lint check (30s)
    ↓
    └── 🔨 Build project (2min)
        ↓
        ├── 🧪 Run tests (1min)
        │   └── 📈 Code coverage (2min)
        │       └── 📤 Upload to Coveralls (10s)
        ├── 📚 Generate docs (1min)
        └── 🔒 Security audit (30s)
```

**Total Time**: ~8-10 minutes first run, ~2-3 minutes with cache

## 🛠️ Daily Usage

### Before Committing

```bash
# Quick check (recommended)
./ci-check.sh

# Or step by step
cargo fmt              # Format code
cargo clippy --fix     # Fix lint issues
cargo test             # Run tests
```

### View Build Status

- Online: https://app.circleci.com/pipelines/github/<org>/<repo>
- PR page shows check status
- Email notifications (if configured)

### Download Build Artifacts

1. Go to CircleCI project page
2. Select a workflow run
3. Click "Artifacts" tab
4. Download:
   - 📊 `coverage/lcov.info` - Coverage report
   - 📄 `coverage/coverage.txt` - Text coverage
   - 📚 `doc/` - API documentation

## ⚡ Common Commands Quick Reference

| Task | Local Command | Auto in CI |
|------|--------------|------------|
| Format | `cargo fmt` | ✅ |
| Format check | `cargo fmt -- --check` | ✅ |
| Lint | `cargo clippy` | ✅ |
| Build | `cargo build` | ✅ |
| Test | `cargo test` | ✅ |
| Coverage | `./coverage.sh` | ✅ |
| Docs | `cargo doc --open` | ✅ |
| Audit | `cargo audit` | ✅ Daily |

## 🐛 Quick Fixes

### ❌ Format check failed
```bash
cargo fmt
git add .
git commit -m "style: format code"
```

### ❌ Clippy warnings
```bash
cargo clippy --fix
# Or manually fix, then
git add .
git commit -m "fix: clippy warnings"
```

### ❌ Tests failed
```bash
# View details
RUST_BACKTRACE=1 cargo test

# After fixing
cargo test
git add .
git commit -m "fix: fix failing tests"
```

### ❌ Security audit failed
```bash
# Update dependencies
cargo update
cargo test  # Ensure working
git add Cargo.lock
git commit -m "chore: update dependencies for security"
```

## 🎯 Skip CI (docs-only changes)

```bash
git commit -m "docs: update README [ci skip]"
```

## 📱 Setup Notifications

1. Go to CircleCI project settings
2. Select "Notifications"
3. Configure:
   - ✉️ Email notifications
   - 💬 Slack notifications
   - 🔗 Webhooks

## 🔗 Important Links

- 📖 [Full Documentation](README.md)
- 🏠 [CircleCI Dashboard](https://app.circleci.com/pipelines/github/<org>/<repo>)
- 📚 [Project Documentation](https://github.com/<org>/<repo>)

## 💡 Best Practices

1. ✅ **Run** `./ci-check.sh` **before committing**
2. ✅ **Small commits** make debugging easier
3. ✅ **Check CI logs** to understand failures
4. ✅ **Update dependencies** regularly with `cargo update`
5. ✅ **Monitor security** audit results

## 🆘 Need Help?

- 📧 starfish.hu@gmail.com
- 🐛 [Submit Issue](https://github.com/<org>/<repo>/issues)
- 💬 [Discussion Forum](https://github.com/<org>/<repo>/discussions)

---

**Tip**: First build is slower (~10 min), subsequent builds are much faster (~2-3 min).

