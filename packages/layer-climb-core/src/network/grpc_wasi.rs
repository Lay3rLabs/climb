#![allow(warnings)]
// PLACEHOLDER FOR WASI IMPL

use http::{header::HeaderName, HeaderMap, HeaderValue};
use http::{Request, Response};
use http_body::Body;
use httparse::{Status, EMPTY_HEADER};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tonic::body::Body as TonicBody;
use tonic::codegen::Bytes;
use tower_service::Service;

pub struct Client;

impl Service<Request<TonicBody>> for Client {
    type Response = Response<ResponseBody>;

    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<TonicBody>) -> Self::Future {
        unimplemented!();
        // Box::pin(async {
        // })
    }
}

pub struct EncodedBytes {}

impl EncodedBytes {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {})
    }
}

type Error = String;

/// Type to handle HTTP response
pub struct ResponseBody {}

impl ResponseBody {
    pub(crate) fn new() -> Result<Self, Error> {
        Ok(Self {})
    }

    fn read_stream(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        unimplemented!()
    }

    fn step(self: Pin<&mut Self>) -> Result<(), Error> {
        unimplemented!()
    }
}

impl Body for ResponseBody {
    type Data = Bytes;

    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        unimplemented!();
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self {}
    }
}
