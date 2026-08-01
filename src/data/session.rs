use std::error::Error;
use std::fmt::{Display, Formatter};

use anyhow::Error as AnyError;
use reqwest::{StatusCode, Url};

pub const CAMPUS_SESSION_EXPIRED_CODE: &str = "campus_session_expired";

/// Stable, typed marker used across the SDK/server boundary. Its display value is
/// deliberately constant so neither upstream URLs nor credentials can leak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampusSessionExpired;

impl Display for CampusSessionExpired {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(CAMPUS_SESSION_EXPIRED_CODE)
    }
}

impl Error for CampusSessionExpired {}

pub fn session_expired_error() -> AnyError {
    AnyError::new(CampusSessionExpired)
}

pub fn is_session_expired(error: &AnyError) -> bool {
    error.downcast_ref::<CampusSessionExpired>().is_some()
}

pub(crate) fn ensure_authenticated_response(
    status: StatusCode,
    final_url: &Url,
    content_type: Option<&str>,
    redirect_location: Option<&str>,
    body: &str,
) -> anyhow::Result<()> {
    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
        || is_login_url(final_url)
        || redirect_location.is_some_and(is_login_redirect)
        || is_login_html(content_type, body)
    {
        return Err(session_expired_error());
    }

    Ok(())
}

fn is_login_url(url: &Url) -> bool {
    let path = url.path().trim_end_matches('/').to_ascii_lowercase();
    let school_host = url
        .host_str()
        .is_some_and(|host| host == "ahu.edu.cn" || host.ends_with(".ahu.edu.cn"));
    path.ends_with("/cas/login")
        || path.ends_with("/student/sso/login")
        || (school_host && path == "/login")
        || path.contains("/tologin")
        || path.ends_with("/refer")
}

fn is_login_redirect(location: &str) -> bool {
    let location = location.to_ascii_lowercase();
    location.contains("/cas/login")
        || location.contains("/student/sso/login")
        || location.contains("tologin")
        || location.contains("/refer")
}

fn is_login_html(content_type: Option<&str>, body: &str) -> bool {
    let trimmed = body.trim_start();
    let looks_like_html = content_type
        .is_some_and(|value| value.to_ascii_lowercase().contains("text/html"))
        || trimmed.starts_with("<!DOCTYPE html")
        || trimmed.starts_with("<!doctype html")
        || trimmed.starts_with("<html");

    if !looks_like_html {
        return false;
    }

    let lowercase = body.to_ascii_lowercase();
    lowercase.contains("id=\"loginform\"")
        || lowercase.contains("id='loginform'")
        || body.contains("<title>登入页面</title>")
        || ((lowercase.contains("name=\"username\"") || lowercase.contains("name='username'"))
            && (lowercase.contains("name=\"password\"") || lowercase.contains("name='password'"))
            && (lowercase.contains("/cas/") || lowercase.contains("login")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_form_html_is_session_expired() {
        let error = ensure_authenticated_response(
            StatusCode::OK,
            &Url::parse("https://jw.ahu.edu.cn/student/home").unwrap(),
            Some("text/html; charset=utf-8"),
            None,
            r#"<html><form id="loginForm"><input name="username"><input name="password"></form></html>"#,
        )
        .unwrap_err();

        assert!(is_session_expired(&error));
        assert_eq!(error.to_string(), CAMPUS_SESSION_EXPIRED_CODE);
    }

    #[test]
    fn final_cas_and_jwxt_login_urls_are_session_expired() {
        for url in [
            "https://one.ahu.edu.cn/cas/login?service=redacted",
            "https://jw.ahu.edu.cn/student/sso/login",
        ] {
            let error = ensure_authenticated_response(
                StatusCode::OK,
                &Url::parse(url).unwrap(),
                Some("text/html"),
                None,
                "<html></html>",
            )
            .unwrap_err();
            assert!(is_session_expired(&error));
        }
    }

    #[test]
    fn redirect_markers_are_session_expired() {
        for location in ["/tologin", "/student/refer", "/cas/login"] {
            let error = ensure_authenticated_response(
                StatusCode::FOUND,
                &Url::parse("https://jw.ahu.edu.cn/student/home").unwrap(),
                None,
                Some(location),
                "",
            )
            .unwrap_err();
            assert!(is_session_expired(&error));
        }
    }

    #[test]
    fn unrelated_failures_are_not_session_expired() {
        for status in [StatusCode::INTERNAL_SERVER_ERROR, StatusCode::BAD_GATEWAY] {
            assert!(
                ensure_authenticated_response(
                    status,
                    &Url::parse("https://jw.ahu.edu.cn/student/for-std/course-table").unwrap(),
                    Some("application/json"),
                    None,
                    r#"{"error":"upstream"}"#,
                )
                .is_ok()
            );
        }

        assert!(
            ensure_authenticated_response(
                StatusCode::OK,
                &Url::parse("https://jw.ahu.edu.cn/student/for-std/course-table").unwrap(),
                Some("text/html"),
                None,
                "<html><title>Changed response</title></html>",
            )
            .is_ok()
        );
    }

    #[test]
    fn protected_feature_responses_share_the_same_login_html_detection() {
        for path in [
            "/student/for-std/course-table",
            "/student/home/get-current-teach-week",
            "/student/for-std/exam-arrange",
            "/student/for-std/grade/sheet",
            "/xzxcard/yue",
            "/xzxcard/qrcode",
            "/berserker-auth/oauth/token",
        ] {
            let url = Url::parse(&format!("https://jw.ahu.edu.cn{path}")).unwrap();
            let error = ensure_authenticated_response(
                StatusCode::OK,
                &url,
                Some("text/html"),
                None,
                r#"<html><form id="loginForm"><input name="username"><input name="password"></form></html>"#,
            )
            .unwrap_err();
            assert!(is_session_expired(&error), "path={path}");
        }
    }

    #[test]
    fn only_remote_auth_statuses_are_session_expired() {
        let url = Url::parse("https://jw.ahu.edu.cn/student/home").unwrap();
        for status in [StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN] {
            let error =
                ensure_authenticated_response(status, &url, Some("application/json"), None, "{}")
                    .unwrap_err();
            assert!(is_session_expired(&error));
        }
        assert!(
            ensure_authenticated_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &url,
                Some("application/json"),
                None,
                "{}",
            )
            .is_ok()
        );
    }
}
