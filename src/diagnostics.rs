const PUBLIC_ERROR_CODES: &[&str] = &[
    "auth_code_unavailable",
    "campus_login_rejected",
    "campus_service_error",
    "campus_service_unavailable",
    "cas_parameter_parse_failed",
    "jwxt_redirect_mismatch",
    "ycard_redirect_failed",
    "ycard_ticket_decode_failed",
    "ycard_ticket_missing",
    "ycard_token_exchange_failed",
    "ycard_token_missing",
];

/// Returns only a fixed, caller-approved diagnostic code.
///
/// `anyhow` sources can contain request URLs, response fragments, file paths,
/// or authentication material. They must never cross an FFI boundary or be
/// interpolated into a production log. Exact allow-listed service codes are
/// retained so clients can preserve their existing status handling.
pub(crate) fn public_error_code(error: &anyhow::Error, fallback: &'static str) -> &'static str {
    for cause in error.chain() {
        let message = cause.to_string();
        if let Some(code) = PUBLIC_ERROR_CODES
            .iter()
            .copied()
            .find(|code| message == *code)
        {
            return code;
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::public_error_code;

    #[test]
    fn preserves_only_exact_allowlisted_service_codes() {
        let known = anyhow::anyhow!("ycard_ticket_missing");
        assert_eq!(
            public_error_code(&known, "campus_service_error"),
            "ycard_ticket_missing"
        );

        let contextual = known.context("request failed");
        assert_eq!(
            public_error_code(&contextual, "campus_service_error"),
            "ycard_ticket_missing"
        );

        let untrusted = anyhow::anyhow!("request failed for a private runtime value");
        assert_eq!(
            public_error_code(&untrusted, "campus_service_error"),
            "campus_service_error"
        );
    }
}
