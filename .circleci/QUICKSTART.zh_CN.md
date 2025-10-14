# CircleCI 快速开始指南

## 🚀 5 分钟快速设置

### 1. 启用 CircleCI（1 分钟）

1. 访问 https://circleci.com/
2. 使用 GitHub 登录
3. 选择 `rust-common` 项目
4. 点击 "Set Up Project"
5. ✅ 完成！自动开始构建

### 2. 配置 Coveralls（2 分钟，可选）

启用覆盖率报告服务：

1. 访问 [Coveralls.io](https://coveralls.io/)
2. 使用 GitHub 登录
3. 启用 `rust-common` 仓库
4. 复制 `COVERALLS_REPO_TOKEN`
5. 在 CircleCI 项目设置中添加环境变量：
   ```
   名称: COVERALLS_REPO_TOKEN
   值:   [您的 token]
   ```

**详细说明**：查看 [Coveralls 集成指南](COVERALLS_SETUP.zh_CN.md)

### 3. 添加徽章到 README（1 分钟）

在项目的 `README.md` 中添加：

```markdown
[![CircleCI](https://circleci.com/gh/3-prism/rust-common.svg?style=svg)](https://circleci.com/gh/3-prism/rust-common)
[![Coverage Status](https://coveralls.io/repos/github/3-prism/rust-common/badge.svg?branch=main)](https://coveralls.io/github/3-prism/rust-common?branch=main)
```

### 4. 本地测试（3 分钟）

在提交前运行本地检查：

```bash
cd prism3-rust-core
./ci-check.sh
```

## 📊 CI 流程一览

```
提交代码 → GitHub
    ↓
    ├── ✨ 格式检查 (30秒)
    ├── 🔧 Lint 检查 (30秒)
    ↓
    └── 🔨 构建项目 (2分钟)
        ↓
        ├── 🧪 运行测试 (1分钟)
        │   └── 📈 代码覆盖率 (2分钟)
        │       └── 📤 上传到 Coveralls (10秒)
        ├── 📚 生成文档 (1分钟)
        └── 🔒 安全审计 (30秒)
```

**总耗时**：首次 ~8-10 分钟，缓存后 ~2-3 分钟

## 🛠️ 日常使用

### 提交代码前

```bash
# 快速检查（推荐）
cd prism3-rust-core
./ci-check.sh

# 或分步检查
cargo fmt              # 格式化
cargo clippy --fix     # 修复 lint 问题
cargo test             # 运行测试
```

### 查看构建状态

- 在线查看：https://app.circleci.com/pipelines/github/3-prism/rust-common
- PR 页面会显示检查状态
- 失败时会收到邮件通知（如已配置）

### 下载构建产物

1. 进入 CircleCI 项目页面
2. 选择具体的工作流
3. 点击 "Artifacts" 标签
4. 下载：
   - 📊 `coverage/lcov.info` - 覆盖率
   - 📄 `coverage/coverage.txt` - 覆盖率文本
   - 📚 `doc/` - API 文档

## ⚡ 常用命令速查

| 任务 | 本地命令 | CI 自动运行 |
|------|---------|------------|
| 格式化 | `cargo fmt` | ✅ |
| 格式检查 | `cargo fmt -- --check` | ✅ |
| Lint | `cargo clippy` | ✅ |
| 构建 | `cargo build` | ✅ |
| 测试 | `cargo test` | ✅ |
| 覆盖率 | `./coverage.sh` | ✅ |
| 文档 | `cargo doc --open` | ✅ |
| 安全审计 | `cargo audit` | ✅ 每天 |

## 🐛 常见问题快速解决

### ❌ 格式检查失败
```bash
cargo fmt
git add .
git commit -m "style: 格式化代码"
```

### ❌ Clippy 警告
```bash
cargo clippy --fix
# 或手动修复后
git add .
git commit -m "fix: 修复 clippy 警告"
```

### ❌ 测试失败
```bash
# 查看详细信息
RUST_BACKTRACE=1 cargo test

# 修复后
cargo test
git add .
git commit -m "fix: 修复测试"
```

### ❌ 安全审计失败
```bash
# 更新依赖
cargo update
cargo test  # 确保正常
git add Cargo.lock
git commit -m "chore: 更新依赖修复安全问题"
```

## 🎯 跳过 CI（仅文档更新时）

```bash
git commit -m "docs: 更新文档 [ci skip]"
```

## 📱 设置通知

1. 进入 CircleCI 项目设置
2. 选择 "Notifications"
3. 配置：
   - ✉️ 邮件通知
   - 💬 Slack 通知
   - 🔗 Webhook

## 🔗 重要链接

- 📖 [完整文档](README.zh_CN.md)
- 🏠 [CircleCI 仪表板](https://app.circleci.com/pipelines/github/3-prism/rust-common)
- 📚 [项目文档](https://github.com/3-prism/rust-common)

## 💡 最佳实践

1. ✅ **提交前运行** `./ci-check.sh`
2. ✅ **小步提交**，便于定位问题
3. ✅ **查看 CI 日志**，了解失败原因
4. ✅ **保持依赖更新**，定期运行 `cargo update`
5. ✅ **关注安全审计**，及时修复漏洞

## 🆘 需要帮助？

- 📧 starfish.hu@gmail.com
- 🐛 [提交 Issue](https://github.com/3-prism/rust-common/issues)
- 💬 [讨论区](https://github.com/3-prism/rust-common/discussions)

---

**提示**：第一次构建会较慢（~10 分钟），后续构建会快很多（~2-3 分钟）。

