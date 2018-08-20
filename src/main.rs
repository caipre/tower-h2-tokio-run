extern crate bytes;
extern crate futures;
extern crate h2;
extern crate http;
#[macro_use]
extern crate log;
extern crate tokio;
extern crate tower_h2;
extern crate tower_service;

use bytes::Bytes;
use futures::*;
use h2::server::Builder;
use http::{Request, Response};
use tokio::net::TcpListener;
use tokio::reactor::Handle;
use tower_h2::{Body, RecvBody, Server};
use tower_service::{NewService, Service};

struct RespBody(Option<Bytes>);

impl RespBody {
    fn new(body: Bytes) -> Self {
        RespBody(Some(body))
    }

    fn empty() -> Self {
        RespBody(None)
    }
}

impl Body for RespBody {
    type Data = Bytes;

    fn is_end_stream(&self) -> bool {
        self.0.as_ref().map_or(false, |bs| bs.is_empty())
    }

    fn poll_data(&mut self) -> Poll<Option<Bytes>, h2::Error> {
        let data = self.0.take()
            .and_then(|bs| if bs.is_empty() { None } else { Some(bs) });
        Ok(Async::Ready(data))
    }
}

#[derive(Debug)]
struct FooService;

impl Service for FooService {
    type Request = Request<RecvBody>;
    type Response = Response<RespBody>;
    type Error = h2::Error;
    type Future = future::FutureResult<Response<RespBody>, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Self::Request) -> Self::Future {
        let mut resp = http::Response::builder();
        resp.version(http::Version::HTTP_2);
        let body = RespBody::new("hello world!".into());
        let resp = resp.status(200).body(body).unwrap();
        future::ok(resp)
    }
}

#[derive(Debug)]
struct FooNewService;

impl NewService for FooNewService {
    type Request = Request<RecvBody>;
    type Response = Response<RespBody>;
    type Error = h2::Error;
    type Service = FooService;
    type InitError = ::std::io::Error;
    type Future = future::FutureResult<FooService, Self::InitError>;

    fn new_service(&self) -> Self::Future {
        future::ok(FooService)
    }
}

fn main() {
    let h2 = Server::new(FooNewService, Builder::default(), Handle::default());
    let addr = "127.0.0.1:5050".parse().unwrap();
    let bind = TcpListener::bind(&addr).unwrap();
    let fut = bind.incoming()
        .for_each(move |sock| {
            let fut = h2.serve(sock)
                .map_err(|err| error!("h2 error: {:?}", err))
                ;
            tokio::spawn(fut);
            Ok(())
        })
        .map_err(|err| error!("server error: {:?}", err))
        ;
    tokio::run(fut);
}
