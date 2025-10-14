# Code Coverage Testing Guide

This project uses `cargo-llvm-cov` for code coverage statistics.

## Install Dependencies

If you haven't installed `cargo-llvm-cov` yet, please install it first:

```bash
cargo install cargo-llvm-cov
```

## Quick Start

### Using Convenience Script (Recommended)

We provide a convenience script `coverage.sh` that can quickly generate coverage reports in various formats:

```bash
# Generate HTML report and open in browser (default)
./coverage.sh

# Or specify format
./coverage.sh html       # HTML report (opens in browser)
./coverage.sh text       # Terminal text report
./coverage.sh lcov       # LCOV format
./coverage.sh json       # JSON format
./coverage.sh cobertura  # Cobertura XML format
./coverage.sh all        # Generate all formats

# View help
./coverage.sh help
```

### Using cargo Commands

You can also use `cargo llvm-cov` commands directly:

```bash
# Clean old coverage data
cargo llvm-cov clean

# Generate HTML report and open in browser
cargo llvm-cov --html --open

# Generate text format report (output to terminal)
cargo llvm-cov

# Generate LCOV format report
cargo llvm-cov --lcov --output-path target/llvm-cov/lcov.info

# Generate JSON format report
cargo llvm-cov --json --output-path target/llvm-cov/coverage.json

# Generate Cobertura XML format report
cargo llvm-cov --cobertura --output-path target/llvm-cov/cobertura.xml
```

## Report Locations

Generated reports are saved in the following locations by default:

- **HTML Report**: `target/llvm-cov/html/index.html`
- **LCOV Report**: `target/llvm-cov/lcov.info`
- **JSON Report**: `target/llvm-cov/coverage.json`
- **Cobertura Report**: `target/llvm-cov/cobertura.xml`

## Testing Specific Modules Only

If you only want to test specific modules, you can use:

```bash
# Test only lang module
cargo llvm-cov --html --open -- lang::

# Test only specific test files
cargo llvm-cov --html --open --test lang_tests -- lang::argument
```

## Exclude Specific Files

In the `.llvm-cov.toml` configuration file, we have excluded the following files:

- `tests/*` - Test files
- `benches/*` - Benchmark files
- `examples/*` - Example files

If you need to modify exclusion rules, please edit the `.llvm-cov.toml` file.

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Code Coverage

on:
  push:
    branches: [ main, dev ]
  pull_request:
    branches: [ main, dev ]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true

      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov

      - name: Generate coverage
        run: |
          cd prism3-rust-concurrent
          cargo llvm-cov --lcov --output-path lcov.info

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: prism3-rust-concurrent/lcov.info
          flags: prism3-rust-concurrent
```

## Common Issues

### 1. Cannot find `cargo-llvm-cov` command

Make sure you have installed `cargo-llvm-cov`:

```bash
cargo install cargo-llvm-cov
```

### 2. Coverage data is inaccurate

Clean old coverage data first:

```bash
cargo llvm-cov clean
```

### 3. How to improve coverage?

- Write tests for all public APIs
- Test boundary conditions and exception cases
- Use coverage reports to identify untested code paths
- Write tests for complex logic branches

## Coverage Goals

Our recommended coverage goals:

- **Minimum requirement**: 60%
- **Good**: 75%
- **Excellent**: 85%+
- **Core modules**: 90%+

## References

- [cargo-llvm-cov GitHub](https://github.com/taiki-e/cargo-llvm-cov)
- [LLVM Coverage Mapping](https://llvm.org/docs/CoverageMappingFormat.html)
