# AHUTong-sdk
### ① 桌面调试（先验证编译通过）
cargo build --features server --bin ahutong-server

### ② 桌面运行
PORT=9876 cargo run --features server --bin ahutong-server

### ③ 验证接口
curl http://127.0.0.1:9876/health
### 预期输出: ok

curl http://127.0.0.1:9876/api/version
### 预期输出: {"success":true,"data":"1.0.0 (HotFix)"}

### ④ Android NDK 编译（JNI + Server 混合模式）
cargo ndk -t arm64-v8a build --release --features server

### ⑤ Android NDK 编译（仅 JNI，不含 Server，体积更小）
cargo ndk -t arm64-v8a build --release