use std::sync::Arc;
use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts, Registry};

use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web;

struct Store {
    http_request_total: IntCounterVec,
    http_request_duration: Histogram,
}

#[derive(Clone)]
pub struct SayHi {
    pub registry: Arc<Registry>,
    store: Arc<Store>,
}

impl SayHi {
    pub fn get_registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }

    pub fn create() -> Self {
        let registry: Registry = Registry::new();
        let http_request_total: IntCounterVec = IntCounterVec::new(
            Opts::new("http_requests_total", "Total number of HTTP requests").namespace("ntex"),
            &["method", "path", "status"],
        )
        .expect("metric can be created");
        let http_request_duration: Histogram = Histogram::with_opts(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .namespace("ntex")
            .buckets(vec![
                0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
        )
        .expect("histogram can be created");
        registry
            .register(Box::new(http_request_total.clone()))
            .expect("HTTP_REQUESTS_TOTAL can be registered");
        registry.register(Box::new(http_request_duration.clone())).expect("HTTP_REQUEST_DURATION can be registered");

        // do the init here
        Self {
            registry: Arc::from(registry),
            store: Arc::from(Store {
                http_request_total,
                http_request_duration,
            }),
        }
    }
}

impl<S> Middleware<S> for SayHi {
    type Service = SayHiMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        SayHiMiddleware {
            service,
            store: self.store.clone(),
        }
    }
}

pub struct SayHiMiddleware<S> {
    service: S,
    store: Arc<Store>,
}

impl<S, Err> Service<web::WebRequest<Err>> for SayHiMiddleware<S>
where
    S: Service<web::WebRequest<Err>, Response = web::WebResponse, Error = web::Error>,
    Err: web::ErrorRenderer,
{
    type Response = web::WebResponse;
    type Error = web::Error;

    ntex::forward_ready!(service);

    async fn call(
        &self,
        req: web::WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        // println!("Hi from start. You requested: {}", req.path());
        let start = ntex::time::now();
        let method = req.method().as_str().to_owned();
        let path = req.path().to_owned();
        let res = ctx.call(&self.service, req).await?;
        self.store
            .http_request_total
            .with_label_values(&[
                &method,
                &path,
                res.status().as_str(), // You'd update this with actual response status
            ])
            .inc();
        let duration = start.elapsed().as_secs_f64();
        self.store.http_request_duration.observe(duration);
        // println!("took -> {}", duration);
        Ok(res)
    }
}

pub fn prepare_prom() -> SayHi {
    SayHi::create()
}
