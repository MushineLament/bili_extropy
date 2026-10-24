# AI Agents 配置文档

> 本文档定义了 AI 助手在项目中的行为规范和工作流程

---

## 📋 概述

本文档为 AI Agents（包括 GitHub Copilot 和其他 AI 工具）提供项目上下文和行为指导，确保：
1. AI 能够准确理解项目结构
2. AI 在修改代码后自动维护文档
3. AI 遵循统一的代码规范
4. AI 能够高效协助开发工作

---

## 🎯 核心规则

### 规则 1: 文档同步（强制性）

**每次修改代码文件后，AI 必须检查是否需要更新 `PROJECT_ANALYSIS.md`**

**触发条件**：
- 添加/删除/重命名文件
- 添加新模块或 crate
- 修改数据库结构（添加/删除表或字段）
- 添加新命令
- 添加新实体或组件
- 修改 API 接口
- 修改项目依赖

**同步流程**：
1. 识别修改类型（文件变更、功能变更、结构变更等）
2. 定位 `PROJECT_ANALYSIS.md` 中相关章节
3. 更新文档内容
4. 验证文档准确性

**示例**：
```markdown
# AI 应该执行的操作

## 用户请求
添加一个新的命令 `user` 用于管理用户信息

## AI 执行步骤
1. 创建 `crates/origin/src/command/user.rs`
2. 在 `command/mod.rs` 中注册插件
3. ⚠️ 检查需要更新 PROJECT_ANALYSIS.md
4. 更新 "核心模块详解 > 命令系统" 表格，添加 `user` 命令
5. 更新 "关键代码位置" 章节
```

---

### 规则 2: 代码修改流程

**标准流程**：
1. **分析需求** → 理解用户意图
2. **查看现有代码** → 避免重复和冲突
3. **实现功能** → 编写高质量代码
4. **📄 同步文档** → 更新 `PROJECT_ANALYSIS.md`
5. **验证结果** → 确保代码正确性

**强制检查点**：
- 修改任何 `.rs` 文件后 → 检查是否需要更新文档
- 修改 `Cargo.toml` 后 → 检查是否需要更新依赖项分析
- 修改数据库迁移文件后 → 检查是否需要更新数据库设计章节
- 添加新命令后 → 检查是否需要更新命令系统章节
- 添加新实体后 → 检查是否需要更新数据库实体章节

---

### 规则 3: 文档更新优先级

| 优先级 | 修改类型 | 需要更新的章节 |
|-------|---------|--------------|
| 🔴 高 | 添加/删除 crate | 项目架构、工作空间配置 |
| 🔴 高 | 添加/删除数据库表 | 数据库设计、实体定义 |
| 🔴 高 | 添加新命令 | 命令系统、使用说明 |
| 🟡 中 | 添加新模块 | 核心模块详解、关键代码位置 |
| 🟡 中 | 修改项目依赖 | 依赖项分析 |
| 🟢 低 | 修改函数实现 | 不需要更新文档（除非是关键 API） |
| 🟢 低 | 修复 bug | 不需要更新文档 |

---

### 规则 4: Git 变更检测（强制性）

**每次检测到 Git 暂存区或提交变更时，AI 必须检查是否需要更新 `PROJECT_ANALYSIS.md`**

#### Git 变更检测时机

**暂存区检测**：
- 用户执行 `git add` 后
- 检测到文件被添加到暂存区
- 建议在提交前触发文档更新检查

**提交检测**：
- 用户执行 `git commit` 后
- 检测到新的提交记录
- 基于提交信息判断是否需要更新文档

#### Git 变更分析流程

```bash
# AI 应该执行的检测步骤

# 1. 检查暂存区变更
git diff --cached --name-only

# 2. 检查工作区变更
git status --short

# 3. 检查最近的提交
git log -1 --name-only

# 4. 分析变更文件类型
git diff --cached --stat
```

#### Commit Hash 追踪机制

**PROJECT_ANALYSIS.md** 中的 Git 追踪信息表格记录了：

| 字段 | 说明 |
|-----|------|
| **上次同步 Commit Hash** | 最后一次文档同步时的 Git 提交哈希 |
| **上次同步时间** | 文档同步的时间戳 |
| **文档版本** | 文档的版本号 |

**读取 Commit Hash**：
```bash
# 从 PROJECT_ANALYSIS.md 中提取 Commit Hash
grep "上次同步 Commit Hash" PROJECT_ANALYSIS.md | sed 's/.*|.*`\([^`]*\)`.*/\1/'
```

**更新 Commit Hash**：
```bash
# 获取当前 Commit Hash
CURRENT_COMMIT=$(git rev-parse HEAD)

# 获取当前时间
CURRENT_TIME=$(date +"%Y-%m-%d %H:%M:%S")

# 更新 PROJECT_ANALYSIS.md（由 AI 自动执行）
```

**变更检测逻辑**：
```rust
// 完整的变更检测逻辑
async fn check_git_changes_and_sync_doc() -> Result<bool> {
    // 1. 获取当前 Commit Hash
    let current_commit = get_current_commit_hash().await?;
    
    // 2. 从 PROJECT_ANALYSIS.md 读取上次同步的 Commit Hash
    let last_synced_commit = extract_commit_hash_from_doc("PROJECT_ANALYSIS.md").await?;
    
    // 3. 如果是首次初始化
    if last_synced_commit == "未初始化" {
        info!("首次初始化，记录当前 Commit Hash: {}", current_commit);
        update_commit_hash_in_doc("PROJECT_ANALYSIS.md", &current_commit).await?;
        return Ok(false);
    }
    
    // 4. 检查是否有新的提交
    if current_commit == last_synced_commit {
        info!("无新提交，无需检查");
        return Ok(false);
    }
    
    // 5. 获取自上次同步以来的提交
    let commits = get_commits_between(&last_synced_commit, &current_commit).await?;
    
    // 6. 分析每个提交的变更
    let mut needs_update = false;
    for commit in &commits {
        if check_commit_needs_doc_update(commit).await? {
            needs_update = true;
            break;
        }
    }
    
    // 7. 如果需要更新，执行更新并更新 Commit Hash
    if needs_update {
        sync_project_analysis().await?;
        update_commit_hash_in_doc("PROJECT_ANALYSIS.md", &current_commit).await?;
        Ok(true)
    } else {
        // 即使不需要更新文档，也要更新 Commit Hash
        update_commit_hash_in_doc("PROJECT_ANALYSIS.md", &current_commit).await?;
        Ok(false)
    }
}
```

**更新 Commit Hash 的模板**：
```markdown
## 📊 Git 追踪信息

| 项目 | 值 |
|-----|---|
| **上次同步 Commit Hash** | `{CURRENT_COMMIT}` |
| **上次同步时间** | `{CURRENT_TIME}` |
| **文档版本** | `v{VERSION}` |
```

#### Git 变更到文档更新的映射规则

| Git 变更类型 | 变更文件模式 | 触发的文档更新 |
|-------------|-------------|---------------|
| 添加源文件 | `**/*.rs` | 检查是否是新模块或命令 |
| 删除源文件 | `**/*.rs` | 检查是否需要从文档中移除 |
| 重命名文件 | `renamed:` | 更新关键代码位置 |
| 修改配置 | `**/Cargo.toml` | 更新依赖项分析 |
| 修改迁移 | `**/m2025*/**/*.rs` | 更新数据库设计 |
| 添加命令 | `**/command/*.rs` | 更新命令系统表格 |
| 添加实体 | `**/entity/*.rs` | 更新实体列表和表结构 |

#### 自动检测脚本（伪代码）

```rust
// AI 的自动检测逻辑
async fn check_git_changes_and_sync_doc() -> Result<bool> {
    // 1. 获取当前 Commit Hash
    let current_commit = get_current_commit_hash().await?;
    
    // 2. 读取上次同步的 Commit Hash
    let last_synced_commit = get_last_synced_commit_from_doc().await?;
    
    // 3. 检查是否有新的提交
    if current_commit == last_synced_commit {
        info!("无新提交，无需检查");
        return Ok(false);
    }
    
    // 4. 获取暂存区变更
    let staged_files = get_staged_files().await?;
    
    // 5. 获取工作区变更
    let modified_files = get_modified_files().await?;
    
    // 6. 获取自上次同步以来的提交
    let commits_since_last_sync = get_commits_since(&last_synced_commit).await?;
    
    // 7. 合并所有变更
    let all_changes = vec![staged_files, modified_files]
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();
    
    // 8. 分析变更类型
    let mut needs_update = false;
    
    for file in all_changes {
        if file.ends_with(".rs") {
            if file.contains("command/") {
                needs_update |= check_command_change(file).await?;
            } else if file.contains("entity/") {
                needs_update |= check_entity_change(file).await?;
            } else if file.contains("components/") {
                needs_update |= check_component_change(file).await?;
            } else {
                needs_update |= check_source_file_change(file).await?;
            }
        } else if file.contains("Cargo.toml") {
            needs_update = true; // 依赖变更总是需要更新
        } else if file.contains("migration/") {
            needs_update = true; // 迁移变更总是需要更新
        }
    }
    
    // 5. 如果需要更新，执行更新
    if needs_update {
        sync_project_analysis().await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
```

#### Git 钩子集成（可选）

建议配置 Git 钩子在提交前触发文档检查：

**pre-commit 钩子示例**：
```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "🔍 检查是否需要更新 PROJECT_ANALYSIS.md..."

# 检查是否有代码变更
if git diff --cached --name-only | grep -E "\.(rs|toml)$" > /dev/null; then
    echo "⚠️  检测到代码变更，请确认是否需要更新 PROJECT_ANALYSIS.md"
    echo "运行以下命令检查："
    echo "  git diff --cached --name-only"
fi
```

#### AI 检测提示语

当 AI 检测到 Git 变更时，应该使用以下提示语：

**暂存区变更提示**：
```
📝 检测到暂存区变更：
  - crates/origin/src/command/user.rs (新增)
  - crates/origin/src/entity/user.rs (新增)

⚠️  检测到可能需要更新 PROJECT_ANALYSIS.md 的变更
是否需要更新文档？[Y/n]
```

**提交变更提示**：
```
📝 检测到新提交：
  Commit: feat: add user management command
  Files:  crates/origin/src/command/user.rs (new)

⚠️  该提交可能需要更新 PROJECT_ANALYSIS.md
建议更新章节：命令系统、使用说明
```

#### 变更检测优先级

| 变更场景 | 优先级 | 是否强制更新 |
|---------|-------|------------|
| 暂存区 + 新命令文件 | 🔴 高 | 是 |
| 暂存区 + 新实体文件 | 🔴 高 | 是 |
| 暂存区 + 修改 Cargo.toml | 🟡 中 | 是 |
| 提交 + 重命名文件 | 🟡 中 | 建议 |
| 工作区 + 未暂存 | 🟢 低 | 不强制 |

#### 检测失败处理

如果 Git 检测失败（例如不在 Git 仓库中）：
1. 记录警告日志
2. 降级到文件系统变更检测
3. 不阻塞正常工作流程

```rust
// 降级检测逻辑
if !git_available() {
    warn!("Git 不可用，降级到文件系统检测");
    return check_filesystem_changes().await;
}
```

---

## 📁 项目上下文

### 项目基本信息
```yaml
名称: bili_extropy_ecs
类型: Bilibili 管理与下载工具
版本: 0.1.0
技术栈: Rust, Bevy ECS, SeaORM, SQLite, Tokio
```

### 核心架构
```
工作空间
├── origin crate (核心库)
│   ├── plugins (插件系统)
│   ├── command (命令系统)
│   ├── entity (数据库实体)
│   ├── components (ECS 组件)
│   └── db (数据库连接)
└── migration crate (数据库迁移)
```

### 设计模式
- **ECS 架构**: 使用 Bevy ECS 拆分功能
- **插件系统**: 每个功能模块都是 Bevy Plugin
- **异步优先**: 使用 Tokio 处理所有 IO 操作
- **ORM 映射**: 使用 SeaORM 操作数据库
- **REPL 交互**: 使用 rustyline 提供命令行界面

---

## 🔧 AI 行为规范

### 添加新功能

**步骤**：
1. 在现有模块中添加或创建新文件
2. 遵循现有代码风格和命名规范
3. 实现必要的错误处理
4. 添加适当的日志
5. **⚠️ 同步更新 PROJECT_ANALYSIS.md**

**检查清单**：
- [ ] 遵循 Rust 代码规范
- [ ] 使用 `cargo fmt` 格式化
- [ ] 使用 `cargo clippy` 检查
- [ ] 更新文档
- [ ] 验证编译通过

### 修改数据库结构

**步骤**：
1. 创建新的迁移文件
2. 使用 SeaORM 宏生成实体代码
3. 更新关系定义
4. **⚠️ 更新 PROJECT_ANALYSIS.md 的数据库设计章节**
5. **⚠️ 更新实体列表**

**检查清单**：
- [ ] 创建迁移文件
- [ ] 生成实体代码
- [ ] 更新中间表（如需要）
- [ ] 更新文档
- [ ] 测试迁移

### 添加新命令

**步骤**：
1. 在 `command/` 目录创建新模块
2. 实现 Bevy Plugin
3. 在 `command/mod.rs` 中注册
4. **⚠️ 在 PROJECT_ANALYSIS.md 中添加命令说明**
5. **⚠️ 在使用说明章节添加示例**

**检查清单**：
- [ ] 创建命令模块
- [ ] 实现 Plugin trait
- [ ] 注册插件
- [ ] 更新命令列表
- [ ] 添加使用示例

### 重构代码

**步骤**：
1. 分析现有代码结构
2. 制定重构计划
3. 执行重构
4. **⚠️ 检查是否需要更新文档**
5. 验证功能完整性

**检查清单**：
- [ ] 保持功能不变
- [ ] 更新相关文档
- [ ] 运行测试（如有）
- [ ] 检查编译警告

---

## 📝 文档同步操作指南

### 何时检查文档更新

AI 应该在以下操作**之后**立即检查是否需要更新 `PROJECT_ANALYSIS.md`：

```rust
// 示例伪代码：AI 的决策流程
if 文件被修改 {
    match 修改类型 {
        添加新文件 | 删除文件 => 检查文档更新_高优先级(),
        添加新模块 => 检查文档更新_中优先级(),
        修改函数实现 => 检查文档更新_低优先级(),
        修改 Cargo.toml => 检查依赖项更新(),
        修改数据库迁移 => 检查数据库设计更新(),
        添加新命令 => 检查命令系统更新(),
        _ => 不需要更新(),
    }
}

// Git 变更检测（规则 4 强制性）
if 检测到_Git_暂存区变更 || 检测到_Git_提交 {
    check_git_changes_and_sync_doc().await?;
}
```

#### 文件系统变更检查点

| 触发时机 | 检查内容 |
|---------|---------|
| 保存 `.rs` 文件 | 是否是新模块/命令/组件 |
| 保存 `Cargo.toml` | 是否修改了依赖项 |
| 保存迁移文件 | 是否修改了数据库结构 |
| 创建新文件 | 是否需要添加到文档 |
| 删除文件 | 是否需要从文档移除 |

#### Git 变更检查点

| 触发时机 | 检查内容 |
|---------|---------|
| 执行 `git add` | 暂存区文件类型和影响 |
| 执行 `git commit` | 提交信息和变更文件 |
| 检测到新提交 | 提交历史分析 |
| 暂存区有变更 | 是否需要同步文档 |

---

### 文档更新模板

#### 添加新文件时
```markdown
在相应章节添加文件信息：

### 模块名称 (`文件路径`)

简要描述模块功能
```

#### 添加新命令时
```markdown
在 "核心模块详解 > 命令系统" 表格中添加：

| 命令模块 | 功能 |
|---------|------|
| `新命令` | 新命令的功能描述 |

在 "使用说明" 中添加示例：

```bash
# 新命令示例
新命令 <参数>
```
```

#### 添加新实体时
```markdown
在 "数据库实体" 表格中添加：

| 实体 | 表名 | 说明 |
|-----|------|------|
| `新实体` | `新表名` | 实体描述 |

在 "主要表结构" 中添加详细结构
```

#### 修改依赖时
```markdown
在 "依赖项分析" 表格中更新：

| 依赖 | 版本 | 用途 |
|-----|------|------|
| `依赖名` | 新版本 | 更新后的用途描述 |
```

#### 更新 Commit Hash

每次更新 `PROJECT_ANALYSIS.md` 后，必须更新 Git 追踪信息：

```bash
# 1. 获取当前 Commit Hash
CURRENT_COMMIT=$(git rev-parse HEAD)

# 2. 获取当前时间
CURRENT_TIME=$(date +"%Y-%m-%d %H:%M:%S")

# 3. 更新文档中的追踪信息（由 AI 自动执行）
```

**更新示例**：
```markdown
## 📊 Git 追踪信息

| 项目 | 值 |
|-----|---|
| **上次同步 Commit Hash** | `a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0` |
| **上次同步时间** | `2026-09-09 14:30:45` |
| **文档版本** | `v1.1` |
```

**重要提醒**：
- 每次文档更新后，Commit Hash 必须同步更新
- Commit Hash 用于追踪文档与代码的同步状态
- 如果 Commit Hash 未更新，下次检测会误判需要更新文档

---

## 🎨 代码规范

### Rust 代码风格
- 使用 `cargo fmt` 格式化
- 使用 `cargo clippy` 检查代码质量
- 遵循 Rust API 指导原则
- 使用有意义的变量和函数名

### Bevy ECS 规范
- 每个 Plugin 职责单一
- Resource 用于全局状态
- Component 用于实体数据
- System 用于业务逻辑
- 使用 Schedule 控制执行顺序

### 数据库操作规范
- 使用 SeaORM 实体而不是原生 SQL
- 使用事务保证数据一致性
- 正确处理外键关系
- 使用迁移管理数据库结构

---

## 🔍 故障排查指南

### AI 检查清单

当用户报告问题时，AI 应该：

1. **查看错误信息**
   - 读取编译错误
   - 查看运行时错误
   - 检查日志输出

2. **定位问题**
   - 使用 grep_search 搜索相关代码
   - 查看调用栈
   - 分析数据流

3. **提供解决方案**
   - 解释问题原因
   - 提供修复方案
   - 预防措施

### 常见问题处理

| 问题类型 | 处理方法 |
|---------|---------|
| 编译错误 | 检查语法、类型、依赖 |
| 运行时错误 | 检查错误处理、资源管理 |
| 数据库错误 | 检查迁移、连接、SQL |
| 性能问题 | 检查异步操作、查询优化 |

---

## 📚 快速参考

### 关键文件位置
```
程序入口: src/main.rs
插件注册: crates/origin/src/plugins.rs
数据库连接: crates/origin/src/db/mod.rs
命令系统: crates/origin/src/command/mod.rs
实体定义: crates/origin/src/entity/mod.rs
数据库迁移: crates/migration/src/lib.rs
项目文档: PROJECT_ANALYSIS.md
```

### 重要概念
- **ECS**: Entity Component System，Bevy 的核心架构
- **Plugin**: Bevy 的插件系统，用于组织功能
- **Resource**: 全局共享数据
- **Component**: 实体的属性
- **System**: 处理逻辑的函数
- **Schedule**: 系统的执行顺序
- **SeaORM**: Rust 异步 ORM 框架
- **Migration**: 数据库结构迁移

---

## ✅ AI 工作流程总结

```
┌─────────────────────────────────────┐
│         1. 理解用户需求             │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│         2. 分析现有代码             │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│         3. 实现功能                 │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│    4. ⚠️ 检查是否需要更新文档       │
└─────────────────┬───────────────────┘
                  │
          ┌───────┴───────┐
          │ 需要更新?     │
          └───────┬───────┘
         是 │           │ 否
            │           │
┌───────────▼───┐   ┌───▼───────────┐
│ 5. 更新文档   │   │ 6. 验证结果   │
└───────────┬───┘   └───┬───────────┘
            │           │
            └─────┬─────┘
                  │
┌─────────────────▼───────────────────┐
│         7. 完成任务                 │
└─────────────────────────────────────┘
```

---

## 🚨 重要提醒

1. **文档同步是强制性的**，不是可选项
2. 每次修改代码后都要思考：这是否影响文档？
3. 如果不确定是否需要更新，宁可更新也不要遗漏
4. 保持文档和代码的一致性是 AI 的责任
5. 文档质量直接影响其他 AI 的理解能力

---

## 📞 联系与反馈

如果发现文档不准确或需要改进，请及时更新本文档。

---

## 🔄 文档维护记录

### 2026-09-09
- 创建 `AGENTS.md` 文档
- 定义 AI 行为规范和工作流程
- 在 `PROJECT_ANALYSIS.md` 中添加 AI 辅助章节链接
- 建立文档同步机制
- 添加 Git 变更检测机制（规则 4）
- 实现 Commit Hash 追踪机制
- 在 `PROJECT_ANALYSIS.md` 中添加 Git 追踪信息表格
- 完善文档更新和 Commit Hash 同步流程

---

**最后更新**: 2026-09-09  
**维护者**: AI Agent Team