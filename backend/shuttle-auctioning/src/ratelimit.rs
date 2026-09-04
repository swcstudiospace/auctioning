//! Small in-process fixed-window rate limiter.
//!
//! Keyed by client IP (first `X-Forwarded-For` hop, then `X-Real-IP`, then
//! `Fly-Client-IP`, else the literal `anon`). Good enough for a single Shuttle
//! instance; swap for a Redis/Postgres bucket if the API scales out.

use crate::error::AppError;
use axum::http::{HeaderMap, Request};
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
use std::future::{ready, Future};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tower::{Layer, Service};

const WINDOW: Duration = Duration::from_secs(60);
const MAX_KEYS: usize = 50_000;

#[derive(Debug, Default)]
struct Buckets {
    map: HashMap<String, (u32, Instant)>,
}

/// Shared limiter. Clone is cheap (Arc).
#[derive(Debug, Clone, Default)]
pub struct RateLimiter {
    inner: Arc<Mutex<Buckets>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when the request is within budget.
    pub fn check(&self, key: &str, max_per_window: u32) -> bool {
        self.check_at(key, max_per_window, Instant::now())
    }

    fn check_at(&self, key: &str, max_per_window: u32, now: Instant) -> bool {
        let mut b = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if b.map.len() >= MAX_KEYS {
            b.map
                .retain(|_, (_, start)| now.duration_since(*start) < WINDOW);
        }
        let entry = b.map.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) >= WINDOW {
            *entry = (0, now);
        }
        entry.0 += 1;
        entry.0 <= max_per_window
    }
}

pub fn client_key(headers: &HeaderMap) -> String {
    for name in [
        "x-forwarded-for",
        "x-real-ip",
        "fly-client-ip",
        "cf-connecting-ip",
    ] {
        if let Some(v) = headers.get(name).and_then(|v| v.to_str().ok()) {
            let first = v.split(',').next().unwrap_or("").trim();
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    "anon".to_string()
}

/// Build a per-route tower layer: `post(handler).layer(ratelimit::layer(l, "name", 60))`.
pub fn layer(limiter: RateLimiter, bucket: &'static str, max_per_min: u32) -> RateLimitLayer {
    RateLimitLayer {
        limiter,
        bucket,
        max_per_min,
    }
}

#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: RateLimiter,
    bucket: &'static str,
    max_per_min: u32,
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: self.limiter.clone(),
            bucket: self.bucket,
            max_per_min: self.max_per_min,
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    limiter: RateLimiter,
    bucket: &'static str,
    max_per_min: u32,
}

impl<S, B> Service<Request<B>> for RateLimitService<S>
where
    S: Service<Request<B>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    B: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let key = format!("{}:{}", self.bucket, client_key(req.headers()));
        if !self.limiter.check(&key, self.max_per_min) {
            tracing::warn!(bucket = self.bucket, "rate limited");
            return Box::pin(ready(Ok(AppError::TooManyRequests.into_response())));
        }
        Box::pin(self.inner.call(req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_limit_then_blocks_then_resets() {
        let l = RateLimiter::new();
        let t0 = Instant::now();
        for _ in 0..3 {
            assert!(l.check_at("k", 3, t0));
        }
        assert!(!l.check_at("k", 3, t0 + Duration::from_secs(1)));
        assert!(l.check_at("k", 3, t0 + WINDOW));
        assert!(l.check_at("other", 3, t0));
    }

    #[test]
    fn client_key_prefers_forwarded_for_first_hop() {
        let mut h = HeaderMap::new();
        assert_eq!(client_key(&h), "anon");
        h.insert("x-real-ip", "10.0.0.2".parse().unwrap());
        assert_eq!(client_key(&h), "10.0.0.2");
        h.insert("x-forwarded-for", "203.0.113.9, 10.0.0.1".parse().unwrap());
        assert_eq!(client_key(&h), "203.0.113.9");
    }
}
