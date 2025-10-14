# CircleCI Configuration Guide

[简体中文](README.zh_CN.md) | English

This directory contains the CircleCI continuous integration configuration for Rust projects.

## 📋 Configuration Overview

### Executor

Uses the `cimg/rust:1.70` Docker image, which is the official Rust CircleCI image with Rust 1.70+ toolchain.

### Workflows

#### Main Workflow (build_and_test)

Automatically runs on every code commit, including the following jobs:

| Job | Description | Dependencies |
|-----|-------------|--------------|
| **check_format** | Code format check (cargo fmt) | None |
| **lint** | Code quality check (cargo clippy) | None |
| **build** | Build debug and release versions | check_format, lint |
| **test** | Run all tests | build |
| **coverage** | Generate code coverage report | test |
| **doc** | Generate API documentation | build |
| **security_audit** | Security vulnerability audit | build |

#### Scheduled Workflow (nightly_security)

Runs security audit daily at 00:00 UTC, only on `main` or `master` branches.

### Job Details

#### 1. check_format - Code Formatting

```bash
cargo fmt -- --check
```

- ✅ Checks if code format complies with Rust standards
- ❌ Build fails if format is incorrect
- 💡 Local fix: `cargo fmt`

#### 2. lint - Code Quality Check

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

- ✅ Checks code quality issues and potential bugs
- ❌ All warnings are treated as errors
- 💡 Local fix: `cargo clippy --fix`

#### 3. build - Build Project

```bash
cargo build --verbose
cargo build --release --verbose
```

- 🔨 Builds debug and release versions
- 💾 Build artifacts are cached for subsequent jobs
- ⚡ Uses Cargo cache to speed up builds

#### 4. test - Run Tests

```bash
cargo test --verbose
```

- 🧪 Runs all unit and integration tests
- 📊 Shows detailed test output
- ✅ All tests must pass

#### 5. coverage - Code Coverage

```bash
cargo llvm-cov --package <package-name> --lcov --output-path coverage.lcov
cargo llvm-cov --package <package-name>
```

- 📈 Generates code coverage report
- 📄 Output formats: LCOV (machine-readable) and text (human-readable)
- 💾 Reports saved as CircleCI artifacts
- 🎯 Automatically detects package name from `Cargo.toml`

#### 6. doc - Generate Documentation

```bash
cargo doc --no-deps --verbose
```

- 📚 Generates API documentation
- 💾 Documentation saved as CircleCI artifacts
- 🌐 Viewable and downloadable in CircleCI interface

#### 7. security_audit - Security Audit

```bash
cargo audit
```

- 🔒 Checks for known security vulnerabilities in dependencies
- 📋 Uses RustSec Advisory Database
- ⚠️ Fails when vulnerabilities are found

### Caching Strategy

```yaml
Cache contents:
  - ~/.cargo/registry  # Cargo registry
  - ~/.cargo/git       # Git dependencies
  - target             # Build artifacts

Cache key: cargo-{{ checksum "Cargo.lock" }}-v1
Fallback key: cargo-v1
```

**Performance Impact**:
- First build: ~5-10 minutes
- Cached build: ~1-3 minutes
- Time saved: ~70-80%

## 🚀 Usage Guide

### Enable CircleCI

1. **Sign Up and Login**
   - Visit [CircleCI](https://circleci.com/)
   - Login with your GitHub account

2. **Add Project**
   - Find your repository in the project list
   - Click "Set Up Project"
   - CircleCI will automatically detect `.circleci/config.yml`

3. **Start Building**
   - First build triggers automatically
   - Subsequent builds run on every commit

### View Build Status

**CircleCI Dashboard**:
```
https://app.circleci.com/pipelines/github/<org>/<repo>
```

**Pull Request Checks**:
- GitHub PR pages show CircleCI check status
- Click "Details" to view detailed logs

### Add Status Badge

Add build status badge to `README.md`:

```markdown
[![CircleCI](https://circleci.com/gh/<org>/<repo>.svg?style=svg)](https://circleci.com/gh/<org>/<repo>)
```

Or use shields.io style:

```markdown
[![CircleCI](https://img.shields.io/circleci/build/github/<org>/<repo>/main?label=build&logo=circleci)](https://circleci.com/gh/<org>/<repo>)
```

### View Artifacts

1. Go to CircleCI project page
2. Select a specific workflow run
3. Click "Artifacts" tab
4. Available files:
   - `coverage/lcov.info` - LCOV coverage report
   - `coverage/coverage.txt` - Text coverage report
   - `doc/` - API documentation

### Integrate Coveralls (Configured)

**Coveralls** is a simple code coverage service, already enabled in the configuration.

#### Step 1: Enable Coveralls

1. Visit [Coveralls](https://coveralls.io/)
2. Login with GitHub
3. Add your repository
4. Copy `COVERALLS_REPO_TOKEN`

#### Step 2: Configure CircleCI

Add environment variable in CircleCI project settings:

```
Name: COVERALLS_REPO_TOKEN
Value: [token from Coveralls]
```

#### Step 3: Add Badge

Add to `README.md`:

```markdown
[![Coverage Status](https://coveralls.io/repos/github/<org>/<repo>/badge.svg?branch=main)](https://coveralls.io/github/<org>/<repo>?branch=main)
```

#### Detailed Documentation

See [Coveralls Setup Guide](COVERALLS_SETUP.md) for complete configuration instructions.

## 🧪 Local Testing

Before committing code, run these commands locally:

```bash
# 1. Format check
cargo fmt -- --check
# Fix format issues
cargo fmt

# 2. Lint check
cargo clippy --all-targets --all-features -- -D warnings
# Auto-fix some issues
cargo clippy --fix

# 3. Run tests
cargo test

# 4. Generate coverage report
./coverage.sh text

# 5. Security audit
cargo install cargo-audit
cargo audit

# 6. Generate documentation
cargo doc --no-deps --open
```

### One-Command Check Script

Use the provided `ci-check.sh` script:

```bash
./ci-check.sh
```

This will run all checks automatically.

## ⚡ Performance Optimization

### Cache Optimization

**First Build**:
- Download all dependencies (~3-5 minutes)
- Compile all crates (~2-3 minutes)
- Total: ~5-10 minutes

**With Cache**:
- Skip dependency download
- Only compile changed code (~1-2 minutes)
- Total: ~1-3 minutes

**Force Cache Refresh**:
```yaml
# Change cache version number
key: cargo-{{ checksum "Cargo.lock" }}-v2  # v1 -> v2
```

Or click "Clear Cache" in CircleCI project settings.

### Parallel Optimization

Current job dependency graph:

```
┌─────────────────┐
│  check_format   │ ──┐
└─────────────────┘   │
                      ├──> ┌───────┐
┌─────────────────┐   │    │ build │
│      lint       │ ──┘    └───┬───┘
└─────────────────┘            │
                               ├──> ┌──────┐     ┌──────────┐
                               │    │ test │ ──> │ coverage │
                               │    └──────┘     └──────────┘
                               │
                               ├──> ┌──────┐
                               │    │ doc  │
                               │    └──────┘
                               │
                               └──> ┌────────────────┐
                                    │ security_audit │
                                    └────────────────┘
```

- `check_format` and `lint` run in parallel
- `test`, `doc`, `security_audit` run in parallel (after build)
- `coverage` runs after `test` completes

### Resource Configuration

Currently using `medium` resource class (2 CPU, 4GB RAM).

To speed up builds, upgrade resources:

```yaml
executors:
  rust-executor:
    resource_class: large  # 3 CPU, 6GB RAM
    # or
    resource_class: xlarge # 8 CPU, 16GB RAM
```

**Cost Comparison** (relative to medium):
- `large`: 2x credits
- `xlarge`: 4x credits

## 🔧 Troubleshooting

### Q1: Why is the first build slow?

**A**: First build requires:
- Downloading all Rust dependencies
- Compiling all dependency crates
- Compiling the project itself

**Solution**: Subsequent builds use cache, 70-80% faster.

---

### Q2: How to skip CI build?

**A**: Add `[ci skip]` or `[skip ci]` to commit message:

```bash
git commit -m "docs: update README [ci skip]"
git commit -m "style: adjust formatting [skip ci]"
```

---

### Q3: Format check failed?

**A**: Run auto-formatter:

```bash
cargo fmt
git add .
git commit -m "style: format code"
```

---

### Q4: Clippy check failed?

**A**: View errors and fix:

```bash
# View issues
cargo clippy --all-targets --all-features

# Auto-fix some issues
cargo clippy --fix

# For expected warnings, add allow attribute
#[allow(clippy::some_lint_name)]
```

---

### Q5: Security audit failed?

**A**:

**Option 1 - Update dependencies**:
```bash
cargo update
cargo test  # Ensure still working
```

**Option 2 - Temporarily ignore** (not recommended):

Create `.cargo-audit.toml`:

```toml
[advisories]
ignore = [
    "RUSTSEC-YYYY-NNNN",  # Specific vulnerability ID
]
```

**Option 3 - Contact dependency maintainers**:
If dependency has vulnerability with no update available.

---

### Q6: Tests failed?

**A**:

1. **Reproduce locally**:
```bash
cargo test --verbose
```

2. **View detailed logs**:
```bash
RUST_BACKTRACE=1 cargo test
```

3. **Run specific test**:
```bash
cargo test test_name -- --exact --nocapture
```

---

### Q7: How to debug CircleCI config?

**A**: Use CircleCI CLI:

```bash
# Install CLI (macOS)
brew install circleci

# Install CLI (Linux)
curl -fLSs https://circle.ci/cli | bash

# Validate config file
circleci config validate

# Execute job locally (requires Docker)
circleci local execute --job build
```

---

### Q8: Build takes too long?

**A**:

1. **Check if cache is working**
2. **Upgrade resource class** (if budget allows)
3. **Optimize dependencies**: Remove unnecessary ones
4. **Split jobs**: Break long jobs into smaller parallel ones

---

### Q9: How to run only specific jobs?

**A**:

**Option 1 - Use API**:
```bash
curl -X POST \
  -H "Circle-Token: $CIRCLE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"branch":"main","parameters":{"run_coverage":false}}' \
  https://circleci.com/api/v2/project/gh/<org>/<repo>/pipeline
```

**Option 2 - Add workflow filters**:
```yaml
workflows:
  build_and_test:
    when:
      not:
        equal: [ scheduled_pipeline, << pipeline.trigger_source >> ]
    jobs:
      - build
```

---

### Q10: How to add environment variables?

**A**:

In CircleCI project settings:
1. Go to project settings
2. Select "Environment Variables"
3. Click "Add Environment Variable"
4. Enter name and value
5. Use in config: `$VARIABLE_NAME`

---

## 📚 Additional Resources

### Official Documentation

- [CircleCI Documentation](https://circleci.com/docs/)
- [CircleCI Rust Guide](https://circleci.com/docs/language-rust/)
- [CircleCI Configuration Reference](https://circleci.com/docs/configuration-reference/)

### Rust Tools

- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) - Coverage tool
- [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit) - Security audit
- [clippy](https://github.com/rust-lang/rust-clippy) - Lint tool
- [rustfmt](https://github.com/rust-lang/rustfmt) - Format tool

### Third-Party Integrations

- [Codecov](https://codecov.io/) - Coverage reporting
- [Coveralls](https://coveralls.io/) - Coverage reporting
- [RustSec Advisory Database](https://rustsec.org/) - Security vulnerability database

## 🔄 Maintenance Recommendations

### Regular Updates

**Monthly Check**:
```bash
# Update Rust toolchain
rustup update

# Update dependencies
cargo update

# Run tests
cargo test

# Check security issues
cargo audit
```

**Update Docker Image**:

Update image version in `config.yml`:
```yaml
executors:
  rust-executor:
    docker:
      - image: cimg/rust:1.75  # 1.70 -> 1.75
```

### Monitor Build Status

**Setup Notifications**:
1. Go to CircleCI project settings
2. Select "Notifications"
3. Configure email, Slack, or webhook notifications

**Monitor Metrics**:
- Build success rate
- Average build time
- Cache hit rate
- Dependency security status

### Optimization Tips

**As Project Grows**:
1. Consider splitting large test suites
2. Use parallel test execution
3. Add more cache paths
4. Upgrade resource class

**Best Practices**:
- Keep `Cargo.lock` in version control
- Update dependencies regularly
- Monitor security audit results
- Maintain code coverage above 90%

---

## 💬 Support

For questions or suggestions:

- 📧 Email: starfish.hu@gmail.com
- 🐛 Issues: [GitHub Issues](https://github.com/3-prism/rust-common/issues)
- 💡 Discussions: [GitHub Discussions](https://github.com/3-prism/rust-common/discussions)

---

**3-Prism Co. Ltd.** | Apache-2.0 License

