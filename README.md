# AHUTong-sdk

AHUTong 的 Rust 核心实现，提供 Android JNI 接口以及用于桌面端调试/独立运行的 HTTP 服务器。

## Apple / iOS 持久化接口

iOS 以 `staticlib` 链接本 crate，并通过 `src/ffi.rs` 暴露的 C ABI 使用本地服务器与 GuiXu：

- `ahutong_init_persistence`：打开或切换 GuiXu 数据库；第三个参数控制是否允许把 Rust Cookie 写入数据库。
- `ahutong_kv_put_string` / `ahutong_kv_get_string`：字符串 KV 读写。
- `ahutong_kv_remove` / `ahutong_kv_clear_box`：删除键或清空命名 box。
- `ahutong_persist_current_cookies`：仅在初始化时允许会话落盘的客户端生效。

所有返回字符串都是 `{ "ok": true, "value": ... }` 或 `{ "ok": false, "error": ... }` JSON，调用方必须使用 `ahutong_free_string` 释放。切换数据库时会先清空 crawler 内存 Cookie，再按 seed / 已保存会话恢复，避免账号串用。

iOS 客户端必须以 `persist_session = 0` 初始化：GuiXu 只保存非敏感业务缓存，Cookie、Token 和密码继续由 Keychain 管理。Android 的现有 JNI 路径使用 `persist_session = 1`，保持当前兼容行为。

## 开发与测试

### 1. 预备环境
- Rust (Cargo)
- Python 3 (用于运行测试脚本)

### 2. 运行独立服务器
服务器默认监听 **3000** 端口。

```bash
# 启用日志输出
$env:RUST_LOG="info"  # PowerShell
export RUST_LOG=info  # Bash
cargo run --bin ahutong-sdk --features server
```

启动时，服务器会生成一个随机的安全 Token，请留意日志输出：
```text
Token: <YOUR_TOKEN_HERE>
```

### 3. 自动化测试
我们提供了一个 Python 脚本，用于自动编译、运行并测试服务器接口。

```bash
python test_server.py
```

### 4. 手动 API 测试指南
请求时必须在 Header 中携带 `X-AHUTONG-TOKEN`。

> **⚠️ 注意 (Windows PowerShell 用户)**: 
> PowerShell 中 `curl` 是 `Invoke-WebRequest` 的别名。**必须使用 `curl.exe`** 才能运行以下命令。
> JSON 数据中的双引号需要转义（如 `\"`），或者使用单引号包裹整个 JSON 字符串。

#### 4.1 基础检查
**健康检查 (Health Check)**
```bash
curl.exe http://127.0.0.1:3000/health
```

#### 4.2 登录 (必须先执行)
登录成功后，Cookies 会自动保存在服务器内存中，供后续接口使用。
```bash
curl.exe -X POST http://127.0.0.1:3000/login `
  -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" `
  -H "Content-Type: application/json" `
  -d '{\"username\": \"你的学号\", \"password\": \"你的密码\"}'
```

#### 4.3 教务系统数据
**查询考试安排**
```bash
curl.exe -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" http://127.0.0.1:3000/exam
```

**查询课表**
```bash
curl.exe -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" http://127.0.0.1:3000/schedule
```

**查询成绩**
```bash
# 查询当前登录用户成绩
curl.exe -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" http://127.0.0.1:3000/grade

# 查询特定学号成绩 (需有权限)
curl.exe -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" "http://127.0.0.1:3000/grade?student_id=10086"
```

#### 4.4 一卡通 (YCard)
**查询余额**
```bash
curl.exe -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" http://127.0.0.1:3000/ycard/balance
```

**获取支付码**
```bash
curl.exe -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" http://127.0.0.1:3000/ycard/qrcode
```

#### 4.5 Cookie 管理
**查看当前 Cookies (JSON)**
```bash
curl.exe -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" http://127.0.0.1:3000/cookies/dump
```

**初始化 Cookies (从 Android 端导入)**
```bash
curl.exe -X POST http://127.0.0.1:3000/init `
  -H "X-AHUTONG-TOKEN: <YOUR_TOKEN>" `
  -H "Content-Type: application/json" `
  -d '{\"cookies_json\": \"[...你的cookies...]\"}'
```

## Android 编译构建

### 编译并输出到 jniLibs/arm64-v8a（推荐）
如果你希望编译后直接把 `*.so` 放到 Android 工程的 `jniLibs/arm64-v8a` 目录下，可以指定输出目录：
```bash
# 输出到当前仓库的 ./jniLibs/arm64-v8a
cargo ndk -t arm64-v8a -o ./jniLibs build --release

# 如需包含本地服务器功能
cargo ndk -t arm64-v8a -o ./jniLibs build --release --features server
```
输出文件示例：`./jniLibs/arm64-v8a/libahutong_rs.so`

### 包含本地服务器 (JNI + HTTP Server)
适用于需要通过 HTTP 接口调用 Rust 核心功能的场景。
```bash
cargo ndk -t arm64-v8a build --release --features server
```

### 仅 JNI 接口 (体积更小)
如果不使用本地服务器功能，可以省略 server 特性。
```bash
cargo ndk -t arm64-v8a build --release
```
