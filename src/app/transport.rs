use std::{
    collections::{HashMap, VecDeque},
    fmt::Write,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant},
};

use error_stack::{FrameKind, Report, ResultExt};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    app::AppError,
    config::Config,
    event::api::{ApiHandler, RequestEnvelope, RequestEvent, ResponseEnvelope, ResponseEvent},
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct RequestId(u64);

#[derive(Debug, Clone)]
pub(crate) struct TransportResult {
    pub(crate) request: RequestEvent,
    pub(crate) response: std::result::Result<ResponseEvent, String>,
    request_send: Instant,
    response_received: Instant,
}

impl TransportResult {
    pub(crate) fn elapsed(&self) -> Duration {
        self.response_received.duration_since(self.request_send)
    }
}

#[derive(Debug, Default)]
pub(crate) struct TransportStats {
    pub(crate) in_flight_requests: AtomicUsize,
    history: RwLock<VecDeque<TransportResult>>,
}

impl TransportStats {
    fn new() -> Self {
        Self::default()
    }
    pub(crate) fn latest_transport(&self) -> Option<TransportResult> {
        self.history.read().unwrap().front().cloned()
    }
}

pub(super) struct TransportController {
    req_tx: Sender<RequestEnvelope>,
    res_rx: Receiver<ResponseEnvelope>,
    stats: Arc<TransportStats>,
    in_flights: HashMap<RequestId, (Instant, RequestEvent)>,
    next_request_id: RequestId,
}

impl TransportController {
    const HISTORY_SIZE: usize = 100;

    pub(super) fn init(config: Config) -> error_stack::Result<Self, AppError> {
        let (req_tx, req_rx) = mpsc::channel::<RequestEnvelope>(10);
        let (res_tx, res_rx) = mpsc::channel::<ResponseEnvelope>(10);
        let api_handler = ApiHandler::new(config.elasticsearch.unwrap_or_default())
            .change_context_lazy(|| AppError::ConfigureClient)?;

        tokio::spawn(api_handler.run(req_rx, res_tx));

        Ok(Self {
            req_tx,
            res_rx,
            stats: Arc::new(TransportStats::new()),
            in_flights: HashMap::new(),
            next_request_id: RequestId(0),
        })
    }

    pub(super) async fn send_requests(&mut self, reqs: impl Iterator<Item = RequestEvent>) {
        for req in reqs {
            self.send_request(req).await
        }
    }

    pub(super) async fn send_request(&mut self, req: RequestEvent) {
        let request_id = self.request_id();
        let now = Instant::now();
        self.in_flights.insert(request_id, (now, req.clone()));
        self.stats
            .in_flight_requests
            .store(self.in_flights.len(), Ordering::Relaxed);

        self.req_tx
            .send(RequestEnvelope {
                request_id,
                event: req,
            })
            .await
            .ok();
    }

    pub(super) async fn recv_response(&mut self) -> Option<ResponseEnvelope> {
        match self.res_rx.recv().await {
            Some(res) => {
                let now = Instant::now();
                if let Some((requested_at, request)) = self.in_flights.remove(&res.request_id) {
                    self.stats
                        .in_flight_requests
                        .store(self.in_flights.len(), Ordering::Relaxed);

                    let r = match &res.result {
                        Ok(event) => Ok(event.clone()),
                        Err(report) => Err(format_err_msg(report)),
                    };

                    let t = TransportResult {
                        request,
                        response: r,
                        request_send: requested_at,
                        response_received: now,
                    };
                    self.save_transport(t);
                }
                Some(res)
            }
            None => None,
        }
    }

    pub(crate) fn stats(&self) -> Arc<TransportStats> {
        self.stats.clone()
    }

    fn save_transport(&self, transport: TransportResult) {
        let mut q = self.stats.history.write().unwrap();
        q.push_front(transport);
        if q.len() > Self::HISTORY_SIZE * 2 {
            q.truncate(Self::HISTORY_SIZE);
        }
    }

    fn request_id(&mut self) -> RequestId {
        let id = self.next_request_id;
        self.next_request_id = RequestId(id.0.saturating_add(1));
        id
    }
}

// Debug implementation of error_stack::Report may contain terminal control characters,
// so implement manually.
fn format_err_msg<T>(r: &Report<T>) -> String {
    r.frames()
        .filter_map(|frame| match frame.kind() {
            FrameKind::Context(context) => Some(context.to_string()),
            FrameKind::Attachment(_) => None,
        })
        .enumerate()
        .fold(String::new(), |mut s, (idx, frame)| {
            if idx == 0 {
                s.write_str(&frame).ok();
            } else {
                s.write_str(&format!(" | {frame}")).ok();
            }
            s
        })
}
