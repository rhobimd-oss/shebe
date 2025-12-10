//! Middleware for the Shebe API
//!
//! Provides request logging with duration tracking.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{error, info};

/// Request logging middleware
///
/// Logs all incoming requests with method, URI, status code, and
/// duration. Successful requests are logged at INFO level, failed
/// requests at ERROR level.
///
/// # Arguments
///
/// * `request` - The incoming HTTP request
/// * `next` - The next middleware or handler in the chain
///
/// # Returns
///
/// The response from the next handler
pub async fn log_request(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    // Process request
    let response = next.run(request).await;

    let duration_ms = start.elapsed().as_millis();
    let status = response.status();

    // Log based on status
    if status.is_success() {
        info!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = %duration_ms,
            "Request completed"
        );
    } else {
        error!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = %duration_ms,
            "Request failed"
        );
    }

    response
}
