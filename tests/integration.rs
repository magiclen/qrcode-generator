#![cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr", feature = "url"))]

#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[path = "integration/common.rs"]
mod common;

#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[path = "integration/vectors.rs"]
mod vectors;

#[cfg(feature = "qr")]
#[path = "integration/model2.rs"]
mod model2;

#[cfg(feature = "micro-qr")]
#[path = "integration/micro.rs"]
mod micro;

#[cfg(feature = "rmqr")]
#[path = "integration/rmqr.rs"]
mod rmqr;

#[cfg(any(feature = "qr", feature = "micro-qr", feature = "rmqr"))]
#[path = "integration/renderer.rs"]
mod renderer;

#[cfg(all(feature = "qr", feature = "micro-qr"))]
#[path = "integration/auto.rs"]
mod auto;

#[cfg(feature = "url")]
#[path = "integration/url.rs"]
mod url;
