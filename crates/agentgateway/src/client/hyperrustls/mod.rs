use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{fmt, io};

use http::Uri;
use hyper::rt;
use hyper_util::client::legacy::connect::{Connection, HttpConnector};
use hyper_util::rt::TokioIo;
use rustls_pki_types::ServerName;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tower::Service;

use crate::transport::stream;
use crate::transport::stream::Socket;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A Connector for the `https` scheme.
#[derive(Clone)]
pub struct HttpsConnector {
	pub http: HttpConnector,
	pub tls_config: Arc<rustls::ClientConfig>,
	pub server_name: ServerName<'static>,
}

impl Service<Uri> for HttpsConnector {
	type Response = Socket;
	type Error = BoxError;

	#[allow(clippy::type_complexity)]
	type Future = Pin<Box<dyn Future<Output = Result<Socket, BoxError>> + Send>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		match self.http.poll_ready(cx) {
			Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
			Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
			Poll::Pending => Poll::Pending,
		}
	}

	fn call(&mut self, dst: Uri) -> Self::Future {
		let cfg = self.tls_config.clone();
		let hostname = self.server_name.clone();

		let connecting_future = self.http.call(dst);
		Box::pin(async move {
			let tcp = connecting_future.await?.into_inner();
			let (ext, counter, tcp) = Socket::from_tcp(tcp)?.into_parts();
			let tls = TlsConnector::from(cfg)
				.connect(hostname, Box::new(tcp))
				.await
				.map_err(io::Error::other)?;
			let socket = Socket::from_tls(stream::Extension::new(), counter, tls.into())?;
			Ok(socket)
		})
	}
}

impl fmt::Debug for HttpsConnector {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("HttpsConnector").finish()
	}
}
