# Coveralls Integration Guide

[简体中文](COVERALLS_SETUP.zh_CN.md) | English

This document explains how to configure Coveralls.io code coverage service for Rust projects.

## 📋 What is Coveralls?

[Coveralls](https://coveralls.io/) is a code coverage analysis and visualization service that provides:

- 📊 **Visual Coverage Reports**: Clear interface for coverage data
- 📈 **Historical Tracking**: Track coverage trends over time
- 🔍 **File-Level Details**: View coverage for each file
- 🎯 **Pull Request Integration**: Automatic coverage changes in PRs
- 🆓 **Free for Open Source**: Completely free for public repositories

## 🚀 Quick Setup (5 Minutes)

### Step 1: Sign Up for Coveralls

1. Visit [Coveralls.io](https://coveralls.io/)
2. Click "Sign in with GitHub"
3. Authorize Coveralls to access your GitHub account

### Step 2: Add Repository

1. After logging in, click "Add Repos"
2. Find your repository
3. Toggle to enable the repository
4. Copy the **repo token** (very important!)

### Step 3: Configure CircleCI Environment Variable

1. Visit CircleCI project settings:
   ```
   https://app.circleci.com/settings/project/github/<org>/<repo>
   ```

2. In left menu, select "Environment Variables"

3. Click "Add Environment Variable"

4. Add variable:
   ```
   Name: COVERALLS_REPO_TOKEN
   Value: [token from Coveralls]
   ```

5. Click "Add Environment Variable" to save

### Step 4: Trigger Build

Commit any code or manually trigger CircleCI build:

```bash
# Method 1: Commit code
git commit --allow-empty -m "chore: test Coveralls integration"
git push

# Method 2: Manually trigger in CircleCI interface
```

### Step 5: View Results

1. Wait for CircleCI build to complete (~5-8 minutes)
2. Visit Coveralls dashboard:
   ```
   https://coveralls.io/github/<org>/<repo>
   ```
3. View coverage report 🎉

## 📊 Add Coveralls Badge

### Add to README.md

Get badge code:

1. Visit Coveralls project page
2. Click "BADGE URLS" or "Settings"
3. Copy Markdown format badge code

**Markdown Format**:
```markdown
[![Coverage Status](https://coveralls.io/repos/github/<org>/<repo>/badge.svg?branch=main)](https://coveralls.io/github/<org>/<repo>?branch=main)
```

**In README**:

```markdown
# Project Name

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![CircleCI](https://circleci.com/gh/<org>/<repo>.svg?style=svg)](https://circleci.com/gh/<org>/<repo>)
[![Coverage Status](https://coveralls.io/repos/github/<org>/<repo>/badge.svg?branch=main)](https://coveralls.io/github/<org>/<repo>?branch=main)
```

## 🔧 Configuration Details

### CircleCI Configuration

Coveralls integration is already added in `.circleci/config.yml`:

```yaml
- run:
    name: Upload coverage to Coveralls
    command: |
      if [ -n "$COVERALLS_REPO_TOKEN" ]; then
        echo "📤 Uploading coverage report to Coveralls..."

        # Download and use Coveralls CLI to upload LCOV report
        curl -sL https://coveralls.io/coveralls-linux.tar.gz | tar -xz
        ./coveralls report coverage.lcov \
          --repo-token="$COVERALLS_REPO_TOKEN" \
          --service-name=circleci \
          --service-number="$CIRCLE_BUILD_NUM" \
          --commit="$CIRCLE_SHA1" \
          --branch="$CIRCLE_BRANCH"

        echo "✅ Coverage report uploaded to Coveralls"
      else
        echo "⚠️  COVERALLS_REPO_TOKEN not set, skipping upload"
      fi
```

### Workflow

```
1. Run tests
   ↓
2. Generate LCOV coverage report (cargo-llvm-cov)
   ↓
3. Check if COVERALLS_REPO_TOKEN exists
   ↓
4. Download Coveralls CLI tool
   ↓
5. Upload coverage report to Coveralls
   ↓
6. Coveralls processes and displays report
```

### Environment Variables

| Variable | Description | Source |
|----------|-------------|--------|
| `COVERALLS_REPO_TOKEN` | Repository token (required) | Manual setup |
| `CIRCLE_BUILD_NUM` | Build number | CircleCI auto-provided |
| `CIRCLE_SHA1` | Git commit SHA | CircleCI auto-provided |
| `CIRCLE_BRANCH` | Branch name | CircleCI auto-provided |

## 📈 Using Coveralls

### View Coverage Reports

**Project Overview**:
```
https://coveralls.io/github/<org>/<repo>
```

Shows:
- Overall coverage percentage
- Coverage trend over time
- Recent build history
- File list with individual coverage

**File Details**:
- Click any file to view detailed coverage
- Green highlight: Covered code
- Red highlight: Uncovered code
- Execution count shown next to line numbers

### Pull Request Integration

When creating a Pull Request, Coveralls will:

1. ✅ Automatically analyze coverage changes
2. 💬 Add comment in PR showing coverage changes
3. 📊 Show which files have coverage increases/decreases
4. 🎯 Highlight new uncovered code

**Example Comment**:
```
Coverage Status: coverage increased (+0.5%) to 98.5%
when pulling abc123 into def456.

Changes Missing Coverage:
- src/new_feature.rs: 85.0%

Files with Coverage Reduction:
- src/old_module.rs: 95.0% (-2.0%)
```

### Coverage Trends

**View Historical Trends**:
1. Go to project page
2. Click "Builds" tab
3. View coverage change graph over time

**Set Coverage Threshold**:
1. Go to "Settings"
2. Configure minimum coverage requirement
3. PR checks fail when coverage drops below threshold

## 🔍 Local Testing

Test coverage locally before committing:

```bash
# Generate LCOV report
./coverage.sh lcov

# View generated file
ls -lh target/llvm-cov/lcov.info
```

## 🐛 Troubleshooting

### Issue 1: Upload Failed "Token not found"

**Cause**: `COVERALLS_REPO_TOKEN` not set or incorrect

**Solution**:
1. Check CircleCI environment variable is correctly set
2. Ensure token has no extra spaces or newlines
3. Re-copy token from Coveralls

---

### Issue 2: Coverage is 0% or Incorrect

**Cause**: LCOV report path issue or wrong format

**Solution**:
1. Check `coverage.lcov` file exists
2. View file contents for coverage data:
   ```bash
   head -20 coverage.lcov
   ```
3. Verify path in CircleCI config is correct

---

### Issue 3: Coveralls Shows "No builds yet"

**Cause**: First build not completed or upload failed

**Solution**:
1. Wait for CircleCI build to complete
2. Check upload step in CircleCI logs
3. Verify no error messages

---

### Issue 4: Private Repository Not Accessible

**Cause**: Private repos need Coveralls Pro subscription

**Solution**:
- Use [Coveralls Pro](https://coveralls.io/pricing) (paid)
- Or use Codecov (offers free tier for private repos)

---

### Issue 5: Build Succeeds But No Upload

**Cause**: Upload step was skipped or failed

**Solution**:
1. Check "Upload coverage to Coveralls" step in CircleCI logs
2. Verify `COVERALLS_REPO_TOKEN` exists
3. Check for network errors or timeouts

---

## 🆚 Coveralls vs Codecov

| Feature | Coveralls | Codecov |
|---------|-----------|---------|
| Open Source | ✅ Free | ✅ Free |
| Private Repos | 💰 Paid | 🎁 Free (limited) |
| Interface | Simple & intuitive | Feature-rich |
| PR Integration | ✅ | ✅ |
| Trend Analysis | ✅ Basic | ✅ Advanced |
| Setup Complexity | ⭐ Simple | ⭐⭐ Moderate |
| Community Support | ⭐⭐⭐ | ⭐⭐⭐⭐ |

**Recommendation**:
- **Open Source Projects**: Both good, Coveralls is simpler
- **Private Projects**: Choose Codecov
- **Team Use**: Codecov has more features

## 📚 Related Resources

### Official Documentation

- [Coveralls Homepage](https://coveralls.io/)
- [Coveralls Documentation](https://docs.coveralls.io/)
- [CircleCI Integration Guide](https://docs.coveralls.io/circleci)
- [Coveralls API](https://docs.coveralls.io/api-introduction)

### Tools and Libraries

- [coveralls CLI](https://github.com/coverallsapp/coverage-reporter) - Official upload tool
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) - Rust coverage tool
- [LCOV](http://ltp.sourceforge.net/coverage/lcov.php) - Coverage data format

### Community Resources

- [Coveralls Community Forum](https://community.coveralls.io/)
- [GitHub Issues](https://github.com/coverallsapp/coveralls-ruby/issues)
- [Stack Overflow - Coveralls Tag](https://stackoverflow.com/questions/tagged/coveralls)

## 📧 Get Help

Having issues?

- 📖 Check troubleshooting section above
- 🔍 Search [Coveralls Documentation](https://docs.coveralls.io/)
- 💬 Ask on [Community Forum](https://community.coveralls.io/)
- 📧 Contact project maintainer: starfish.hu@gmail.com

## ✅ Setup Checklist

After setup, verify these items:

- [ ] Coveralls account created and logged in
- [ ] Repository enabled in Coveralls
- [ ] `COVERALLS_REPO_TOKEN` added to CircleCI
- [ ] At least one successful build triggered
- [ ] Coveralls displays coverage data
- [ ] Coverage badge added to README
- [ ] Pull requests show coverage comments

## 🎉 Complete

Congratulations! Your Coveralls integration is complete. Now on every commit:

1. ✅ CircleCI automatically runs tests
2. 📊 Generates coverage report
3. 📤 Automatically uploads to Coveralls
4. 🎯 Shows coverage changes in PRs
5. 📈 Tracks long-term coverage trends

---

**Last Updated**: 2025-10-13
**Configuration Version**: 1.0

