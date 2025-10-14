# CircleCI 配置说明

[English](README.md) | 简体中文

本目录包含了 `prism3-rust-core` 项目的 CircleCI 持续集成配置。

## 📋 配置概览

### 执行器

使用 `cimg/rust:1.70` Docker 镜像，这是一个官方的 Rust CircleCI 镜像，包含了 Rust 1.70+ 工具链。

### 工作流

#### 主工作流 (build_and_test)

在每次代码提交时自动运行，包含以下任务：

| 任务 | 说明 | 依赖 |
|------|------|------|
| **check_format** | 代码格式检查（cargo fmt） | 无 |
| **lint** | 代码质量检查（cargo clippy） | 无 |
| **build** | 构建 debug 和 release 版本 | check_format, lint |
| **test** | 运行所有测试 | build |
| **coverage** | 生成代码覆盖率报告 | test |
| **doc** | 生成 API 文档 | build |
| **security_audit** | 安全漏洞审计 | build |

#### 定时工作流 (nightly_security)

每天 UTC 时间 00:00 自动运行安全审计，仅在 `main` 或 `master` 分支上执行。

### 任务详情

#### 1. check_format - 代码格式检查

```bash
cargo fmt -- --check
```

- ✅ 检查代码格式是否符合 Rust 标准
- ❌ 如果格式不正确，构建将失败
- 💡 本地修复：`cargo fmt`

#### 2. lint - 代码质量检查

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

- ✅ 检查代码质量问题和潜在 bug
- ❌ 所有警告都会被视为错误
- 💡 本地修复：`cargo clippy --fix`

#### 3. build - 构建项目

```bash
cargo build --verbose
cargo build --release --verbose
```

- 🔨 构建 debug 和 release 版本
- 💾 构建产物会被缓存供后续任务使用
- ⚡ 使用 Cargo 缓存加速构建

#### 4. test - 运行测试

```bash
cargo test --verbose
```

- 🧪 运行所有单元测试和集成测试
- 📊 显示详细的测试输出
- ✅ 所有测试必须通过

#### 5. coverage - 代码覆盖率

```bash
cargo llvm-cov --package prism3-core --lcov --output-path coverage.lcov
cargo llvm-cov --package prism3-core
```

- 📈 生成代码覆盖率报告
- 📄 输出格式：LCOV（机器可读）和文本（人类可读）
- 💾 报告保存为 CircleCI artifacts
- 🎯 当前项目覆盖率：~98%

#### 6. doc - 生成文档

```bash
cargo doc --no-deps --verbose
```

- 📚 生成 API 文档
- 💾 文档保存为 CircleCI artifacts
- 🌐 可在 CircleCI 界面查看和下载

#### 7. security_audit - 安全审计

```bash
cargo audit
```

- 🔒 检查依赖中的已知安全漏洞
- 📋 使用 RustSec Advisory Database
- ⚠️ 发现漏洞时会失败

### 缓存策略

```yaml
缓存内容：
  - ~/.cargo/registry  # Cargo 注册表
  - ~/.cargo/git       # Git 依赖
  - target             # 构建产物

缓存键：cargo-{{ checksum "Cargo.lock" }}-v1
回退键：cargo-v1
```

**优化效果**：
- 首次构建：~5-10 分钟
- 缓存命中：~1-3 分钟
- 节省时间：~70-80%

## 🚀 使用指南

### 启用 CircleCI

1. **注册和登录**
   - 访问 [CircleCI](https://circleci.com/)
   - 使用 GitHub 账号登录

2. **添加项目**
   - 在项目列表中找到 `rust-common` 仓库
   - 点击 "Set Up Project"
   - CircleCI 会自动检测 `.circleci/config.yml`

3. **开始构建**
   - 自动触发首次构建
   - 后续每次提交都会自动构建

### 查看构建状态

**CircleCI 仪表板**：
```
https://app.circleci.com/pipelines/github/3-prism/rust-common
```

**Pull Request 检查**：
- GitHub PR 页面会显示 CircleCI 检查状态
- 点击 "Details" 查看详细日志

### 添加状态徽章

在 `README.md` 中添加构建状态徽章：

```markdown
[![CircleCI](https://circleci.com/gh/3-prism/rust-common.svg?style=svg)](https://circleci.com/gh/3-prism/rust-common)
```

或者使用 shields.io 风格：

```markdown
[![CircleCI](https://img.shields.io/circleci/build/github/3-prism/rust-common/main?label=build&logo=circleci)](https://circleci.com/gh/3-prism/rust-common)
```

效果预览：

![CircleCI Badge](https://img.shields.io/circleci/build/github/3-prism/rust-common/main?label=build&logo=circleci)

### 查看 Artifacts

1. 进入 CircleCI 项目页面
2. 选择具体的工作流运行
3. 点击 "Artifacts" 标签页
4. 可下载的文件：
   - `coverage/lcov.info` - LCOV 覆盖率报告
   - `coverage/coverage.txt` - 文本覆盖率报告
   - `doc/` - API 文档

### 集成 Coveralls（已配置）

**Coveralls** 是一个简单易用的代码覆盖率服务，已在配置中启用。

#### 步骤 1：启用 Coveralls

1. 访问 [Coveralls](https://coveralls.io/)
2. 使用 GitHub 登录
3. 添加 `rust-common` 仓库
4. 复制 `COVERALLS_REPO_TOKEN`

#### 步骤 2：配置 CircleCI

在 CircleCI 项目设置中添加环境变量：

```
名称：COVERALLS_REPO_TOKEN
值：[从 Coveralls 获取的 token]
```

#### 步骤 3：添加徽章

在 `README.md` 中添加：

```markdown
[![Coverage Status](https://coveralls.io/repos/github/3-prism/rust-common/badge.svg?branch=main)](https://coveralls.io/github/3-prism/rust-common?branch=main)
```

#### 详细文档

查看 [Coveralls 集成指南](.circleci/COVERALLS_SETUP.zh_CN.md) 获取完整配置说明。

---

### 集成 Codecov（可选）

**Codecov** 可以提供更详细的覆盖率分析和历史趋势。

#### 步骤 1：启用 Codecov

1. 访问 [Codecov](https://codecov.io/)
2. 使用 GitHub 登录
3. 添加 `rust-common` 仓库
4. 获取 `CODECOV_TOKEN`（可选）

#### 步骤 2：启用上传

取消 `config.yml` 中的注释：

```yaml
- run:
    name: 上传到 Codecov
    command: |
      bash <(curl -s https://codecov.io/bash) -f coverage.lcov
```

#### 步骤 3：添加徽章

在 `README.md` 中添加：

```markdown
[![codecov](https://codecov.io/gh/3-prism/rust-common/branch/main/graph/badge.svg)](https://codecov.io/gh/3-prism/rust-common)
```

#### 同时使用两者

Coveralls 和 Codecov 可以同时使用，互不冲突。两个服务各有特色：

- **Coveralls**：简单直观，开源项目免费
- **Codecov**：功能丰富，私有仓库有免费额度

## 🧪 本地测试

在提交代码前，建议在本地运行以下命令：

```bash
# 进入项目目录
cd prism3-rust-core

# 1. 格式检查
cargo fmt -- --check
# 修复格式问题
cargo fmt

# 2. Lint 检查
cargo clippy --all-targets --all-features -- -D warnings
# 自动修复部分问题
cargo clippy --fix

# 3. 运行测试
cargo test

# 4. 生成覆盖率报告
./coverage.sh text

# 5. 安全审计
cargo install cargo-audit
cargo audit

# 6. 生成文档
cargo doc --no-deps --open
```

### 一键检查脚本

创建 `check.sh` 脚本：

```bash
#!/bin/bash
set -e

echo "🔍 运行所有检查..."

cd prism3-rust-core

echo "✨ 1/5 格式检查..."
cargo fmt -- --check

echo "🔧 2/5 Lint 检查..."
cargo clippy --all-targets --all-features -- -D warnings

echo "🔨 3/5 构建..."
cargo build

echo "🧪 4/5 测试..."
cargo test

echo "📊 5/5 覆盖率..."
./coverage.sh text

echo "✅ 所有检查通过！"
```

使用方法：

```bash
chmod +x check.sh
./check.sh
```

## ⚡ 性能优化

### 缓存优化

**首次构建**：
- 下载所有依赖（~3-5 分钟）
- 编译所有 crate（~2-3 分钟）
- 总计：~5-10 分钟

**缓存命中后**：
- 跳过依赖下载
- 仅编译变更代码（~1-2 分钟）
- 总计：~1-3 分钟

**强制刷新缓存**：
```yaml
# 修改缓存版本号
key: cargo-{{ checksum "Cargo.lock" }}-v2  # v1 -> v2
```

或在 CircleCI 项目设置中点击 "Clear Cache"。

### 并行优化

当前任务依赖关系：

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

- `check_format` 和 `lint` 并行执行
- `test`、`doc`、`security_audit` 并行执行（在 build 后）
- `coverage` 在 `test` 完成后执行

### 资源配置

当前使用 `medium` 资源类别（2 CPU，4GB RAM）。

如需更快构建，可升级资源：

```yaml
executors:
  rust-executor:
    resource_class: large  # 3 CPU, 6GB RAM
    # 或
    resource_class: xlarge # 8 CPU, 16GB RAM
```

**成本对比**（相对于 medium）：
- `large`: 2x 积分消耗
- `xlarge`: 4x 积分消耗

## 🔧 常见问题

### Q1: 为什么首次构建很慢？

**A**: 首次构建需要：
- 下载所有 Rust 依赖（bigdecimal, chrono 等）
- 编译所有依赖 crate
- 编译项目本身

**解决方案**：后续构建会使用缓存，速度提升 70-80%。

---

### Q2: 如何跳过 CI 构建？

**A**: 在 commit 消息中添加 `[ci skip]` 或 `[skip ci]`：

```bash
git commit -m "docs: 更新 README [ci skip]"
git commit -m "style: 调整格式 [skip ci]"
```

---

### Q3: 格式检查失败怎么办？

**A**: 运行自动格式化：

```bash
cd prism3-rust-core
cargo fmt
git add .
git commit -m "style: 格式化代码"
```

---

### Q4: Clippy 检查失败怎么办？

**A**: 查看错误并修复：

```bash
# 查看问题
cargo clippy --all-targets --all-features

# 自动修复部分问题
cargo clippy --fix

# 如果某些警告是预期的，可以添加 allow 属性
#[allow(clippy::some_lint_name)]
```

---

### Q5: 安全审计失败怎么办？

**A**:

**方案 1 - 更新依赖**：
```bash
cargo update
cargo test  # 确保更新后仍然正常
```

**方案 2 - 临时忽略**（不推荐）：

创建 `prism3-rust-core/.cargo-audit.toml`：

```toml
[advisories]
ignore = [
    "RUSTSEC-YYYY-NNNN",  # 具体的漏洞 ID
]
```

**方案 3 - 联系依赖维护者**：
如果依赖本身有漏洞且无更新版本。

---

### Q6: 测试失败怎么办？

**A**:

1. **本地重现**：
```bash
cargo test --verbose
```

2. **查看详细日志**：
```bash
RUST_BACKTRACE=1 cargo test
```

3. **运行特定测试**：
```bash
cargo test test_name -- --exact --nocapture
```

---

### Q7: 如何调试 CircleCI 配置？

**A**: 使用 CircleCI CLI：

```bash
# 安装 CLI（macOS）
brew install circleci

# 安装 CLI（Linux）
curl -fLSs https://circle.ci/cli | bash

# 验证配置文件
circleci config validate

# 本地执行任务（需要 Docker）
circleci local execute --job build
```

---

### Q8: 构建时间太长怎么办？

**A**:

1. **检查缓存是否生效**
2. **升级资源类别**（如果预算允许）
3. **优化依赖**：移除不必要的依赖
4. **拆分任务**：将长任务拆分为多个小任务并行执行

---

### Q9: 如何只运行特定任务？

**A**:

**方案 1 - 使用 API**：
```bash
curl -X POST \
  -H "Circle-Token: $CIRCLE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"branch":"main","parameters":{"run_coverage":false}}' \
  https://circleci.com/api/v2/project/gh/3-prism/rust-common/pipeline
```

**方案 2 - 添加工作流过滤器**：
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

### Q10: 如何添加环境变量？

**A**:

在 CircleCI 项目设置中：
1. 进入项目设置
2. 选择 "Environment Variables"
3. 点击 "Add Environment Variable"
4. 输入名称和值
5. 在配置中使用：`$VARIABLE_NAME`

---

## 📚 更多资源

### 官方文档

- [CircleCI 文档](https://circleci.com/docs/)
- [CircleCI Rust 指南](https://circleci.com/docs/language-rust/)
- [CircleCI 配置参考](https://circleci.com/docs/configuration-reference/)

### Rust 工具

- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) - 覆盖率工具
- [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit) - 安全审计
- [clippy](https://github.com/rust-lang/rust-clippy) - Lint 工具
- [rustfmt](https://github.com/rust-lang/rustfmt) - 格式化工具

### 第三方集成

- [Codecov](https://codecov.io/) - 覆盖率报告
- [Coveralls](https://coveralls.io/) - 覆盖率报告
- [RustSec Advisory Database](https://rustsec.org/) - 安全漏洞数据库

## 🔄 维护建议

### 定期更新

**每月检查**：
```bash
# 更新 Rust 工具链
rustup update

# 更新依赖
cargo update

# 运行测试
cargo test

# 检查安全问题
cargo audit
```

**更新 Docker 镜像**：

在 `config.yml` 中更新镜像版本：
```yaml
executors:
  rust-executor:
    docker:
      - image: cimg/rust:1.75  # 1.70 -> 1.75
```

### 监控构建状态

**设置通知**：
1. 进入 CircleCI 项目设置
2. 选择 "Notifications"
3. 配置邮件、Slack 或 Webhook 通知

**监控指标**：
- 构建成功率
- 平均构建时间
- 缓存命中率
- 依赖安全状态

### 优化建议

**当项目增长时**：
1. 考虑拆分大型测试套件
2. 使用并行测试执行
3. 增加缓存路径
4. 升级资源类别

**最佳实践**：
- 保持 `Cargo.lock` 在版本控制中
- 定期更新依赖
- 监控安全审计结果
- 保持代码覆盖率高于 90%

---

## 💬 支持

如有任何问题或建议：

- 📧 邮件：starfish.hu@gmail.com
- 🐛 Issue：[GitHub Issues](https://github.com/3-prism/rust-common/issues)
- 💡 讨论：[GitHub Discussions](https://github.com/3-prism/rust-common/discussions)

---

**3-Prism Co. Ltd.** | Apache-2.0 License

