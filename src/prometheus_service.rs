use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts, Registry};
use reqwest::StatusCode;
use std::rc::Rc;

use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web;

struct Store {
    http_request_total: IntCounterVec,
    http_request_duration: Histogram,
}

#[derive(Clone)]
pub struct PrometheusMiddleware {
    registry: Rc<Registry>,
    store: Rc<Store>,
    path: String,
}

impl PrometheusMiddleware {
    pub fn get_registry(&self) -> Rc<Registry> {
        self.registry.clone()
    }

    pub fn new(path: &str) -> Self {
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
        registry
            .register(Box::new(http_request_duration.clone()))
            .expect("HTTP_REQUEST_DURATION can be registered");

        // do the init here
        Self {
            registry: Rc::from(registry),
            store: Rc::from(Store {
                http_request_total,
                http_request_duration,
            }),
            path: path.to_owned(),
        }
    }
}

impl<S> Middleware<S> for PrometheusMiddleware {
    type Service = PrometheusMiddlewareService<S>;

    fn create(&self, service: S) -> Self::Service {
        PrometheusMiddlewareService {
            service,
            store: self.store.clone(),
            encoder: prometheus::TextEncoder::new(),
            registry: self.registry.clone(),
            path: self.path.clone(),
        }
    }
}

pub struct PrometheusMiddlewareService<S> {
    service: S,
    store: Rc<Store>,
    encoder: prometheus::TextEncoder,
    registry: Rc<Registry>,
    path: String,
}

impl<S, Err> Service<web::WebRequest<Err>> for PrometheusMiddlewareService<S>
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
        let start = ntex::time::now();
        let method = req.method().as_str().to_owned();
        let path = req.path().to_owned();
        let res = ctx.call(&self.service, req).await?;

        let is_metrics_endpoint = method == "GET" && path == self.path;
        let status = res.status().as_str().to_owned();
        self.store
            .http_request_total
            .with_label_values(&[
                &method,
                &path,
                if is_metrics_endpoint {
                    "200"
                } else {
                    &status
                },
            ])
            .inc();
        let duration = start.elapsed().as_secs_f64();
        self.store.http_request_duration.observe(duration);
        if is_metrics_endpoint {
            return Ok(res.map_body(|head, _| {
                let m = self.registry.gather();
                let content = self.encoder.encode_to_string(&m).unwrap();
                head.status = StatusCode::OK;
                ntex::http::body::ResponseBody::<ntex::http::body::Body>::Body(
                    content.into(),
                )
            }));
        }
        Ok(res)
    }
}
