use axum::http::{Request, Response, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tokio::sync::Mutex;
use tower::{Layer, Service};

use axum::body::Body;

/// A simple token-bucket shared rate limiter implemented as a Clone-able
/// Tower Layer. It's intentionally small and suitable for a dev/test in-process
/// limiter. It is safe to use with the per-connection adapter because the
/// layer and its inner limiter are Clone via Arc.
struct TokenBucket {
	capacity: f64,
	tokens: f64,
	refill_per_sec: f64,
	last_refill: Instant,
}

impl TokenBucket {
	fn new(capacity: usize, refill_per_sec: u32) -> Self {
		let now = Instant::now();
		Self {
			capacity: capacity as f64,
			tokens: capacity as f64,
			refill_per_sec: refill_per_sec as f64,
			last_refill: now,
		}
	}

	fn try_consume(&mut self) -> bool {
		let now = Instant::now();
		let elapsed = now.duration_since(self.last_refill).as_secs_f64();
		self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
		self.last_refill = now;
		if self.tokens >= 1.0 {
			self.tokens -= 1.0;
			true
		} else {
			false
		}
	}
}

#[derive(Clone)]
struct SharedLimiter {
	inner: Arc<Mutex<TokenBucket>>,
}

impl SharedLimiter {
	fn new(capacity: usize, refill_per_sec: u32) -> Self {
		Self {
			inner: Arc::new(Mutex::new(TokenBucket::new(capacity, refill_per_sec))),
		}
	}

	async fn try_acquire(&self) -> bool {
		let mut b = self.inner.lock().await;
		b.try_consume()
	}
}

#[derive(Clone)]
pub struct SharedRateLimitLayer {
	limiter: SharedLimiter,
}

impl SharedRateLimitLayer {
	pub fn new(burst: usize, rps: u32) -> Self {
		Self {
			limiter: SharedLimiter::new(burst, rps),
		}
	}
}

#[derive(Clone)]
pub struct SharedRateLimitService<S> {
	inner: S,
	limiter: SharedLimiter,
}

impl<S> Layer<S> for SharedRateLimitLayer {
	type Service = SharedRateLimitService<S>;

	fn layer(&self, inner: S) -> Self::Service {
		SharedRateLimitService {
			inner,
			limiter: self.limiter.clone(),
		}
	}
}

type BoxF<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

impl<S, ReqBody> Service<Request<ReqBody>> for SharedRateLimitService<S>
where
	S: Service<Request<ReqBody>, Response = Response<Body>> + Clone + Send + 'static,
	S::Future: Send + 'static,
	S::Error: Send + 'static,
	ReqBody: Send + 'static,
{
	type Response = Response<Body>;
	type Error = S::Error;
	type Future = BoxF<Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}

	fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
		let mut inner = self.inner.clone();
		let limiter = self.limiter.clone();

		Box::pin(async move {
			if !limiter.try_acquire().await {
				let resp = Response::builder()
					.status(StatusCode::TOO_MANY_REQUESTS)
					.header("content-type", "text/plain")
					.body(Body::from("rate limit exceeded"))
					.expect("response builder should succeed");
				return Ok(resp);
			}

			inner.call(req).await
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use axum::body::Body;
	use axum::http::Request;
	use tower::service_fn;

	#[tokio::test]
	async fn limiter_allows_under_limit() {
		let layer = SharedRateLimitLayer::new(2, 10);

		// a trivial inner service that echoes 200 OK
		let svc = service_fn(|_req: Request<Body>| async move {
			Ok::<_, std::convert::Infallible>(Response::new(Body::from("ok")))
		});

		let mut svc = layer.layer(svc);
		let req = Request::builder().body(Body::empty()).unwrap();

		let _ = svc.call(req).await.unwrap();
	}

	#[tokio::test]
	async fn limiter_denies_when_exhausted() {
		// burst=1 and rps=0 means only a single immediate token is available
		let layer = SharedRateLimitLayer::new(1, 0);

		let svc = service_fn(|_req: Request<Body>| async move {
			Ok::<_, std::convert::Infallible>(Response::new(Body::from("ok")))
		});

		let mut svc = layer.layer(svc);
		let req = Request::builder().body(Body::empty()).unwrap();

		let first = svc.call(req).await.unwrap();
		assert_eq!(first.status(), StatusCode::OK);

		let req2 = Request::builder().body(Body::empty()).unwrap();
		let second = svc.call(req2).await.unwrap();
		assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
	}
}
