# ZPass — 本地密码管理器

基于 **Tauri v2 + Vue 3 + Rust** 的桌面密码管理工具。所有数据使用 **SQLite** 存储，并以 **AES-256-GCM** 加密，主密码通过 **Argon2id** 派生，**设置后不可更改**。

## 功能

- 首次使用设置主密码（加密/解密数据 + 解锁应用，不可更改）
- 所有密码加密存储（AES-256-GCM，每条记录独立 nonce）
- 常见分类：登录(Web)、数据库、邮箱、自定义，各带默认字段
- 录入时可自定义字段、标签、备注
- 长时间无操作自动锁屏（默认 30s，可配置）
- 右上角设置：主题样式、锁屏时间、存储路径、回收站管理
- 左侧边栏：类别统计、标签统计、回收站
- 搜索与按分类/标签筛选

## 安全设计

```
主密码 --Argon2id--> 256-bit 密钥 (AES-256-GCM)
                       │
                       ├─> 验证用哈希存 master_key 表
                       └─> 加密每个条目的 encrypted_data 字段
内存中仅保留密钥，锁屏即清除；无操作超时自动清除并跳转锁屏页。
```

## 开发 / 运行

```bash
# 1. 安装前端依赖
pnpm install

# 2. 开发模式（前端热更新 + Rust 编译运行）
pnpm tauri dev

# 3. 构建发布包
pnpm tauri build
```

> 要求：Node.js >= 18、Rust 工具链（cargo）、系统已安装 Tauri 依赖（webkit2gtk 等，见 https://tauri.app/start/prerequisites/）。

## 项目结构

```
src/                    前端 (Vue 3 + TS)
  ├─ api.ts             Tauri invoke 封装
  ├─ stores/app.ts      Pinia 状态（条目/标签/设置/锁状态）
  ├─ types.ts           类型 + 分类元数据
  ├─ views/             LockView / MainView / SettingsView
  └─ components/        Sidebar / EntryList / EntryDetail
src-tauri/src/          后端 (Rust)
  ├─ lib.rs             应用入口 + 自动锁屏定时器
  ├─ crypto.rs         Argon2 + AES-256-GCM
  ├─ db.rs             SQLite 建表、默认设置、路径
  ├─ state.rs          全局状态（密钥/活动时间）
  ├─ models.rs         数据模型 + 默认字段模板
  └─ commands.rs       Tauri Commands (CRUD/统计/设置)
```
