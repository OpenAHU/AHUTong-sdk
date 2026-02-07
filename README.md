# AHUTong-sdk

AHUTong 的 Rust 核心实现，提供 Android JNI 接口以及用于桌面端调试/独立运行的 HTTP 服务器。

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
