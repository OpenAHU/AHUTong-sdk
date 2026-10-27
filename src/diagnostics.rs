//! Optional in-process forwarding of existing Rust records to the Android feedback ring.
//! No records, credentials, or callback results are cached here.

use std::{
    cell::Cell,
    fmt::{self, Write},
    marker::PhantomData,
    rc::Rc,
};

pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;
const OVERSIZED: &str = "[Rust日志超过256KiB，正文未采集]";
const FORMAT_FAILED: &str = "[Rust日志格式化失败，正文未采集]";

thread_local! {
    static OPERATION_GENERATION: Cell<Option<Option<i64>>> = const { Cell::new(None) };
    static FORWARDING: Cell<bool> = const { Cell::new(false) };
}

#[cfg(not(target_arch = "wasm32"))]
tokio::task_local! {
    static REQUEST_GENERATION: Option<i64>;
}

/// Pins the account generation for one synchronous JNI operation, including its block_on future.
/// This guard stays on its originating thread and restores an enclosing operation on drop.
pub struct GenerationScope {
    previous: Option<Option<i64>>,
    _thread_bound: PhantomData<Rc<()>>,
}

impl GenerationScope {
    pub fn enter(generation: Option<i64>) -> Self {
        let previous = OPERATION_GENERATION.with(|current| current.replace(Some(generation)));
        Self {
            previous,
            _thread_bound: PhantomData,
        }
    }

    pub fn capture() -> Self {
        Self::enter(java_generation())
    }
}

impl Drop for GenerationScope {
    fn drop(&mut self) {
        OPERATION_GENERATION.with(|current| current.set(self.previous));
    }
}

/// Prevents a Java callback from recursively forwarding another Rust record on the same thread.
pub struct ForwardingGuard {
    _thread_bound: PhantomData<Rc<()>>,
}

impl ForwardingGuard {
    pub fn enter() -> Option<Self> {
        FORWARDING.with(|active| {
            if active.replace(true) {
                None
            } else {
                Some(Self {
                    _thread_bound: PhantomData,
                })
            }
        })
    }
}

impl Drop for ForwardingGuard {
    fn drop(&mut self) {
        FORWARDING.with(|active| active.set(false));
    }
}

pub fn generation_or(fallback: impl FnOnce() -> Option<i64>) -> Option<i64> {
    if let Some(generation) = OPERATION_GENERATION.with(Cell::get) {
        return generation;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(generation) = REQUEST_GENERATION.try_with(|generation| *generation) {
        // A request whose provider failed must not adopt a newer account halfway through.
        return generation;
    }
    fallback()
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn scope_generation<F: std::future::Future>(
    generation: Option<i64>,
    future: F,
) -> F::Output {
    REQUEST_GENERATION.scope(generation, future).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn scope_request<F: std::future::Future>(header: Option<&str>, future: F) -> F::Output {
    // Old clients omit the header. A malformed supplied value suppresses capture instead of adopting an account.
    let generation = match header {
        Some(value) => value.parse::<i64>().ok().filter(|value| *value >= 0),
        None => java_generation(),
    };
    scope_generation(generation, future).await
}

pub fn forwards_target(target: &str) -> bool {
    target == "ahutong_rs"
        || target.starts_with("ahutong_rs::")
        || target == "guixu"
        || target.starts_with("guixu::")
}

struct BoundedMessage {
    text: String,
    oversized: bool,
}

impl BoundedMessage {
    fn new() -> Self {
        Self {
            text: String::new(),
            oversized: false,
        }
    }

    fn finish(self, formatted: bool) -> String {
        if self.oversized {
            OVERSIZED.to_owned()
        } else if !formatted {
            FORMAT_FAILED.to_owned()
        } else {
            self.text
        }
    }
}

impl Write for BoundedMessage {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.oversized || value.len() > MAX_MESSAGE_BYTES - self.text.len() {
            self.oversized = true;
            return Err(fmt::Error);
        }
        self.text.push_str(value);
        Ok(())
    }
}

/// Oversized messages become a marker, never a prefix which might split a password field.
pub fn bounded_message(arguments: fmt::Arguments<'_>) -> String {
    let mut output = BoundedMessage::new();
    let formatted = fmt::write(&mut output, arguments).is_ok();
    output.finish(formatted)
}

/// Mirrors the old Android formatting while teeing only the message into bounded process memory.
pub fn format_android_log(writer: &mut dyn Write, record: &log::Record<'_>) -> fmt::Result {
    write!(writer, "{}: ", record.module_path().unwrap_or_default())?;
    if !forwards_target(record.target()) {
        return fmt::write(writer, *record.args());
    }
    struct Tee<'a> {
        android: &'a mut dyn Write,
        message: &'a mut BoundedMessage,
    }
    impl Write for Tee<'_> {
        fn write_str(&mut self, value: &str) -> fmt::Result {
            // Preserve the existing logger output even when feedback capture has reached its cap.
            let _ = self.message.write_str(value);
            self.android.write_str(value)
        }
    }
    let mut message = BoundedMessage::new();
    let result = fmt::write(
        &mut Tee {
            android: writer,
            message: &mut message,
        },
        *record.args(),
    );
    forward(
        record.level(),
        record.target(),
        &message.finish(result.is_ok()),
    );
    result
}

fn forward(level: log::Level, target: &str, message: &str) {
    if !forwards_target(target) {
        return;
    }
    let Some(_guard) = ForwardingGuard::enter() else {
        return;
    };
    let Some(generation) = generation_or(java_generation) else {
        return;
    };
    #[cfg(not(target_arch = "wasm32"))]
    android::forward(generation, level as i32, target, message);
    #[cfg(target_arch = "wasm32")]
    let _ = (generation, level, target, message);
}

fn java_generation() -> Option<i64> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        android::generation()
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

#[cfg(target_os = "android")]
pub(crate) use android::install as install_android_bridge;

// Compiled on desktop too, allowing cargo test/check to validate JNI API use without a JVM.
#[cfg(not(target_arch = "wasm32"))]
mod android {
    #[cfg(target_os = "android")]
    use jni::{JNIEnv, objects::JClass};
    use jni::{
        JavaVM,
        objects::{GlobalRef, JValue},
    };
    use std::sync::OnceLock;

    const CAPTURE: &str = "captureNativeLog";
    const CAPTURE_SIGNATURE: &str = "(JILjava/lang/String;Ljava/lang/String;)V";
    const GENERATION: &str = "nativeLogGeneration";
    const GENERATION_SIGNATURE: &str = "()J";

    struct Bridge {
        vm: JavaVM,
        class: GlobalRef,
    }
    static BRIDGE: OnceLock<Bridge> = OnceLock::new();

    #[cfg(target_os = "android")]
    pub(crate) fn install(env: &mut JNIEnv<'_>, class: &JClass<'_>) {
        if env.exception_check().unwrap_or(true) {
            return;
        }
        let bridge = (|| -> jni::errors::Result<Bridge> {
            env.get_static_method_id(class, CAPTURE, CAPTURE_SIGNATURE)?;
            env.get_static_method_id(class, GENERATION, GENERATION_SIGNATURE)?;
            Ok(Bridge {
                vm: env.get_java_vm()?,
                class: env.new_global_ref(class)?,
            })
        })();
        if let Ok(bridge) = bridge {
            let _ = BRIDGE.set(bridge);
        }
        // Missing callbacks are expected when a new SO is loaded by a legacy APK.
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }

    pub(super) fn generation() -> Option<i64> {
        let bridge = BRIDGE.get()?;
        let mut env = bridge.vm.attach_current_thread().ok()?;
        if env.exception_check().unwrap_or(true) {
            return None;
        }
        let result = env
            .call_static_method(&bridge.class, GENERATION, GENERATION_SIGNATURE, &[])
            .and_then(|value| value.j());
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
        result.ok()
    }

    pub(super) fn forward(generation: i64, level: i32, target: &str, message: &str) {
        let Some(bridge) = BRIDGE.get() else {
            return;
        };
        let Ok(mut env) = bridge.vm.attach_current_thread() else {
            return;
        };
        // A pending business exception belongs to its caller, not the diagnostic bridge.
        if env.exception_check().unwrap_or(true) {
            return;
        }
        let _: jni::errors::Result<()> = env.with_local_frame(4, |env| {
            let target = env.new_string(target)?;
            let message = env.new_string(message)?;
            env.call_static_method(
                &bridge.class,
                CAPTURE,
                CAPTURE_SIGNATURE,
                &[
                    JValue::Long(generation),
                    JValue::Int(level),
                    JValue::Object(target.as_ref()),
                    JValue::Object(message.as_ref()),
                ],
            )?
            .v()?;
            Ok(())
        });
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_messages_are_bounded_without_forwarding_a_partial_password() {
        let exact = "界".repeat(MAX_MESSAGE_BYTES / 3);
        assert_eq!(bounded_message(format_args!("{exact}")), exact);
        let oversized = "界".repeat(MAX_MESSAGE_BYTES / 3 + 1);
        let message = bounded_message(format_args!("{oversized}password=secret"));
        assert_eq!(message, OVERSIZED);
        assert!(message.len() <= MAX_MESSAGE_BYTES);
        assert!(!message.contains("secret"));
    }

    #[test]
    fn oversized_formatting_does_not_evaluate_later_display_arguments() {
        struct Later<'a>(&'a Cell<bool>);
        impl fmt::Display for Later<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.set(true);
                f.write_str("later")
            }
        }
        let evaluated = Cell::new(false);
        assert_eq!(
            bounded_message(format_args!(
                "{}{}",
                "x".repeat(MAX_MESSAGE_BYTES + 1),
                Later(&evaluated)
            )),
            OVERSIZED
        );
        assert!(!evaluated.get());
    }

    #[test]
    fn android_output_keeps_its_original_format_and_is_not_truncated() {
        let message = "x".repeat(MAX_MESSAGE_BYTES + 1);
        let mut output = String::new();
        format_android_log(
            &mut output,
            &log::Record::builder()
                .args(format_args!("{message}"))
                .target("ahutong_rs::core")
                .module_path(Some("ahutong_rs::core"))
                .level(log::Level::Info)
                .build(),
        )
        .unwrap();
        assert_eq!(output, format!("ahutong_rs::core: {message}"));
    }

    #[test]
    fn nested_operations_restore_their_generation_and_ignore_newer_fallbacks() {
        assert_eq!(generation_or(|| Some(9)), Some(9));
        {
            let _failed_provider = GenerationScope::enter(None);
            assert_eq!(generation_or(|| Some(9)), None);
        }
        {
            let _old_operation = GenerationScope::enter(Some(3));
            assert_eq!(generation_or(|| Some(9)), Some(3));
            {
                let _nested = GenerationScope::enter(Some(4));
                assert_eq!(generation_or(|| Some(9)), Some(4));
            }
            assert_eq!(generation_or(|| Some(9)), Some(3));
        }
        assert_eq!(generation_or(|| Some(9)), Some(9));
    }

    #[test]
    fn callback_reentrance_is_rejected_and_guard_drop_allows_later_logs() {
        let guard = ForwardingGuard::enter().unwrap();
        assert!(ForwardingGuard::enter().is_none());
        drop(guard);
        assert!(ForwardingGuard::enter().is_some());
    }

    #[test]
    fn dependencies_and_lookalike_targets_are_not_forwarded() {
        for target in [
            "ahutong_rs",
            "ahutong_rs::jni",
            "guixu",
            "guixu::persistence",
        ] {
            assert!(forwards_target(target));
        }
        for target in ["jni", "reqwest", "hyper", "ahutong_rs_extra", "guixu_other"] {
            assert!(!forwards_target(target));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn concurrent_requests_keep_their_generation_across_yields() {
        async fn read() -> Option<i64> {
            tokio::task::yield_now().await;
            generation_or(|| Some(99))
        }
        let (old, new) = tokio::join!(
            scope_generation(Some(3), read()),
            scope_generation(Some(4), read())
        );
        assert_eq!(old, Some(3));
        assert_eq!(new, Some(4));
        assert_eq!(scope_generation(None, read()).await, None);
        assert_eq!(generation_or(|| Some(99)), Some(99));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn queued_requests_keep_the_generation_of_the_client_that_sent_them() {
        async fn read() -> Option<i64> {
            tokio::task::yield_now().await;
            generation_or(|| Some(99))
        }
        assert_eq!(scope_request(Some("3"), read()).await, Some(3));
        assert_eq!(scope_request(Some("4"), read()).await, Some(4));
        assert_eq!(scope_request(Some("invalid"), read()).await, None);
        assert_eq!(scope_request(Some("-1"), read()).await, None);
    }
}
