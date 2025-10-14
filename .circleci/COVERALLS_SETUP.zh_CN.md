# Coveralls 集成配置指南

本文档说明如何为 `prism3-rust-core` 项目配置 Coveralls.io 代码覆盖率服务。

## 📋 什么是 Coveralls？

[Coveralls](https://coveralls.io/) 是一个代码覆盖率分析和可视化服务，提供：

- 📊 **可视化覆盖率报告**：清晰的界面展示覆盖率数据
- 📈 **历史趋势追踪**：跟踪覆盖率随时间的变化
- 🔍 **文件级别详情**：查看每个文件的覆盖情况
- 🎯 **Pull Request 集成**：在 PR 中自动显示覆盖率变化
- 🆓 **开源项目免费**：公开仓库完全免费使用

## 🚀 快速设置（5 分钟）

### 步骤 1：注册 Coveralls 账号

1. 访问 [Coveralls.io](https://coveralls.io/)
2. 点击 "Sign in with GitHub"
3. 授权 Coveralls 访问您的 GitHub 账号

### 步骤 2：添加仓库

1. 登录后，点击 "Add Repos"
2. 找到 `3-prism/rust-common` 仓库
3. 点击切换按钮启用该仓库
4. 复制显示的 **repo token**（非常重要！）

### 步骤 3：配置 CircleCI 环境变量

1. 访问 CircleCI 项目设置：
   ```
   https://app.circleci.com/settings/project/github/3-prism/rust-common
   ```

2. 在左侧菜单选择 "Environment Variables"

3. 点击 "Add Environment Variable"

4. 添加以下变量：
   ```
   名称: COVERALLS_REPO_TOKEN
   值:   [从 Coveralls 复制的 repo token]
   ```

5. 点击 "Add Environment Variable" 保存

### 步骤 4：触发构建

提交任何代码或手动触发 CircleCI 构建：

```bash
# 方式 1：提交代码
git commit --allow-empty -m "chore: 触发 Coveralls 测试"
git push

# 方式 2：在 CircleCI 界面手动触发
```

### 步骤 5：查看结果

1. 等待 CircleCI 构建完成（约 5-8 分钟）
2. 访问 Coveralls 仪表板：
   ```
   https://coveralls.io/github/3-prism/rust-common
   ```
3. 查看覆盖率报告 🎉

## 📊 添加 Coveralls 徽章

### 在 README.md 中添加

获取徽章代码：

1. 访问 Coveralls 项目页面
2. 点击 "BADGE URLS" 或 "Settings"
3. 复制 Markdown 格式的徽章代码

**Markdown 格式**：
```markdown
[![Coverage Status](https://coveralls.io/repos/github/3-prism/rust-common/badge.svg?branch=main)](https://coveralls.io/github/3-prism/rust-common?branch=main)
```

**在 README 中的位置**：

```markdown
# Prism3 Core

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![CircleCI](https://circleci.com/gh/3-prism/rust-common.svg?style=svg)](https://circleci.com/gh/3-prism/rust-common)
[![Coverage Status](https://coveralls.io/repos/github/3-prism/rust-common/badge.svg?branch=main)](https://coveralls.io/github/3-prism/rust-common?branch=main)
```

## 🔧 配置详解

### CircleCI 配置

已在 `.circleci/config.yml` 中添加了 Coveralls 集成：

```yaml
- run:
    name: 上传覆盖率到 Coveralls
    command: |
      if [ -n "$COVERALLS_REPO_TOKEN" ]; then
        echo "📤 上传覆盖率报告到 Coveralls..."
        cd prism3-rust-core

        # 使用 coveralls CLI 上传 LCOV 报告
        curl -sL https://coveralls.io/coveralls-linux.tar.gz | tar -xz
        ./coveralls report ../coverage.lcov \
          --repo-token="$COVERALLS_REPO_TOKEN" \
          --service-name=circleci \
          --service-number="$CIRCLE_BUILD_NUM" \
          --commit="$CIRCLE_SHA1" \
          --branch="$CIRCLE_BRANCH"

        echo "✅ 覆盖率报告已上传到 Coveralls"
      else
        echo "⚠️  未设置 COVERALLS_REPO_TOKEN，跳过上传"
      fi
```

### 工作流程

```
1. 运行测试
   ↓
2. 生成 LCOV 覆盖率报告 (cargo-llvm-cov)
   ↓
3. 检查 COVERALLS_REPO_TOKEN 是否存在
   ↓
4. 下载 Coveralls CLI 工具
   ↓
5. 上传覆盖率报告到 Coveralls
   ↓
6. Coveralls 处理并显示报告
```

### 环境变量

| 变量名 | 说明 | 来源 |
|--------|------|------|
| `COVERALLS_REPO_TOKEN` | 仓库令牌（必需） | 手动添加 |
| `CIRCLE_BUILD_NUM` | 构建编号 | CircleCI 自动提供 |
| `CIRCLE_SHA1` | Git commit SHA | CircleCI 自动提供 |
| `CIRCLE_BRANCH` | 分支名称 | CircleCI 自动提供 |

## 📈 使用 Coveralls

### 查看覆盖率报告

**项目概览**：
```
https://coveralls.io/github/3-prism/rust-common
```

显示信息：
- 总体覆盖率百分比
- 覆盖率变化趋势
- 最近的构建历史
- 文件列表及各自覆盖率

**文件详情**：
- 点击任意文件查看详细覆盖情况
- 绿色高亮：已覆盖的代码
- 红色高亮：未覆盖的代码
- 行号旁显示执行次数

### Pull Request 集成

当创建 Pull Request 时，Coveralls 会：

1. ✅ 自动分析覆盖率变化
2. 💬 在 PR 中添加评论显示覆盖率变化
3. 📊 显示哪些文件的覆盖率提升/下降
4. 🎯 标记新增的未覆盖代码

**示例评论**：
```
Coverage Status: coverage increased (+0.5%) to 98.5%
when pulling abc123 into def456.

Changes Missing Coverage:
- src/new_feature.rs: 85.0%

Files with Coverage Reduction:
- src/old_module.rs: 95.0% (-2.0%)
```

### 覆盖率趋势

**查看历史趋势**：
1. 进入项目页面
2. 点击 "Builds" 标签
3. 查看覆盖率随时间的变化图表

**设置覆盖率阈值**：
1. 进入 "Settings"
2. 配置最低覆盖率要求
3. 覆盖率低于阈值时 PR 检查会失败

## 🔍 本地测试

在提交前本地测试覆盖率：

```bash
cd prism3-rust-core

# 生成 LCOV 报告
./coverage.sh lcov

# 查看生成的文件
ls -lh target/llvm-cov/lcov.info
```

## 🐛 故障排查

### 问题 1：上传失败 "Token not found"

**原因**：未设置 `COVERALLS_REPO_TOKEN` 或设置错误

**解决方案**：
1. 检查 CircleCI 环境变量是否正确设置
2. 确认 token 没有多余的空格或换行
3. 重新从 Coveralls 复制 token

---

### 问题 2：覆盖率为 0% 或显示不正确

**原因**：LCOV 报告路径问题或格式不正确

**解决方案**：
1. 检查 `coverage.lcov` 文件是否存在
2. 查看文件内容是否包含覆盖率数据：
   ```bash
   head -20 coverage.lcov
   ```
3. 确认路径在 CircleCI 配置中正确

---

### 问题 3：Coveralls 显示 "No builds yet"

**原因**：首次构建尚未完成或上传失败

**解决方案**：
1. 等待 CircleCI 构建完成
2. 检查 CircleCI 日志中的上传步骤
3. 确认没有错误消息

---

### 问题 4：私有仓库无法访问

**原因**：私有仓库需要 Coveralls Pro 订阅

**解决方案**：
- 使用 [Coveralls Pro](https://coveralls.io/pricing)（付费）
- 或使用 Codecov（提供免费的私有仓库支持）

---

### 问题 5：构建成功但未上传

**原因**：上传步骤被跳过或失败

**解决方案**：
1. 检查 CircleCI 日志中 "上传覆盖率到 Coveralls" 步骤
2. 确认 `COVERALLS_REPO_TOKEN` 存在
3. 查看是否有网络错误或超时

---

## 🆚 Coveralls vs Codecov

| 特性 | Coveralls | Codecov |
|------|-----------|---------|
| 开源项目 | ✅ 免费 | ✅ 免费 |
| 私有仓库 | 💰 付费 | 🎁 免费（有限额） |
| 界面 | 简洁直观 | 功能丰富 |
| Pull Request 集成 | ✅ | ✅ |
| 趋势分析 | ✅ 基础 | ✅ 高级 |
| 配置复杂度 | ⭐ 简单 | ⭐⭐ 中等 |
| 社区支持 | ⭐⭐⭐ | ⭐⭐⭐⭐ |

**推荐**：
- **开源项目**：两者都很好，Coveralls 更简单
- **私有项目**：选择 Codecov
- **团队使用**：Codecov 功能更强大

## 📚 同时使用 Coveralls 和 Codecov

如果需要同时使用两个服务，取消 `config.yml` 中 Codecov 的注释：

```yaml
# 上传到 Codecov
- run:
    name: 上传到 Codecov
    command: |
      bash <(curl -s https://codecov.io/bash) -f coverage.lcov
```

然后添加徽章：

```markdown
[![Coverage Status](https://coveralls.io/repos/github/3-prism/rust-common/badge.svg?branch=main)](https://coveralls.io/github/3-prism/rust-common?branch=main)
[![codecov](https://codecov.io/gh/3-prism/rust-common/branch/main/graph/badge.svg)](https://codecov.io/gh/3-prism/rust-common)
```

## 🔗 相关资源

### 官方文档

- [Coveralls 主页](https://coveralls.io/)
- [Coveralls 文档](https://docs.coveralls.io/)
- [CircleCI 集成指南](https://docs.coveralls.io/circleci)
- [Coveralls API](https://docs.coveralls.io/api-introduction)

### 工具和库

- [coveralls CLI](https://github.com/coverallsapp/coverage-reporter) - 官方上传工具
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) - Rust 覆盖率工具
- [LCOV](http://ltp.sourceforge.net/coverage/lcov.php) - 覆盖率数据格式

### 社区资源

- [Coveralls 社区论坛](https://community.coveralls.io/)
- [GitHub Issues](https://github.com/coverallsapp/coveralls-ruby/issues)
- [Stack Overflow - Coveralls 标签](https://stackoverflow.com/questions/tagged/coveralls)

## 📧 获取帮助

遇到问题？

- 📖 先查看本文档的故障排查部分
- 🔍 搜索 [Coveralls 文档](https://docs.coveralls.io/)
- 💬 在 [社区论坛](https://community.coveralls.io/) 提问
- 📧 联系项目维护者：starfish.hu@gmail.com

## ✅ 检查清单

设置完成后，确认以下事项：

- [ ] Coveralls 账号已创建并登录
- [ ] `rust-common` 仓库已在 Coveralls 中启用
- [ ] `COVERALLS_REPO_TOKEN` 已添加到 CircleCI
- [ ] 至少触发了一次成功的构建
- [ ] Coveralls 显示覆盖率数据
- [ ] README 中已添加覆盖率徽章
- [ ] Pull Request 中能看到覆盖率评论

## 🎉 完成

恭喜！您已成功配置 Coveralls 集成。现在每次提交代码时：

1. ✅ CircleCI 自动运行测试
2. 📊 生成覆盖率报告
3. 📤 自动上传到 Coveralls
4. 🎯 在 PR 中显示覆盖率变化
5. 📈 跟踪长期覆盖率趋势

---

**更新日期**：2025-10-13
**配置版本**：v1.0

