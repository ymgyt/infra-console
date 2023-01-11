use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicI64, AtomicU64, Ordering},
        Arc, RwLock,
    },
};

use either::Either;
use error_stack::ResultExt;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    app::AppError,
    config::Config,
    event::api::{ApiHandler, RequestEvent, ResponseEvent},
};

#[derive(Debug, Default)]
pub(crate) struct TransportStats {
    pub(crate) in_flight_requests: AtomicI64,
    pub(crate) _error_requests: AtomicU64,
    history: RwLock<VecDeque<Either<RequestEvent, ResponseEvent>>>,
}

impl TransportStats {
    fn new() -> Self {
        Self::default()
    }
    pub(crate) fn latest_transport(&self) -> Option<Either<RequestEvent, ResponseEvent>> {
        self.history.read().unwrap().front().cloned()
    }
}

pub(super) struct TransportController {
    req_tx: Sender<RequestEvent>,
    res_rx: Receiver<ResponseEvent>,
    stats: Arc<TransportStats>,
}

impl TransportController {
    const HISTORY_SIZE: usize = 100;

    pub(super) fn init(config: Config) -> error_stack::Result<Self, AppError> {
        let (req_tx, req_rx) = mpsc::channel::<RequestEvent>(10);
        let (res_tx, res_rx) = mpsc::channel::<ResponseEvent>(10);
        let api_handler = ApiHandler::new(config.elasticsearch.unwrap_or_default())
            .change_context_lazy(|| AppError::ConfigureClient)?;

        tokio::spawn(api_handler.run(req_rx, res_tx));

        Ok(Self {
            req_tx,
            res_rx,
            stats: Arc::new(TransportStats::new()),
        })
    }

    pub(super) async fn send_requests(&self, reqs: impl Iterator<Item = RequestEvent>) {
        for req in reqs {
            self.send_request(req).await
        }
    }

    pub(super) async fn send_request(&self, req: RequestEvent) {
        self.stats
            .in_flight_requests
            .fetch_add(1, Ordering::Relaxed);

        self.save_transport(Either::Left(req.clone()));
        self.req_tx.send(req).await.ok();
    }

    pub(super) async fn recv_response(&mut self) -> Option<ResponseEvent> {
        match self.res_rx.recv().await {
            Some(res) => {
                self.stats
                    .in_flight_requests
                    .fetch_sub(1, Ordering::Relaxed);
                self.save_transport(Either::Right(res.clone()));
                Some(res)
            }
            None => None,
        }
    }

    pub(crate) fn stats(&self) -> Arc<TransportStats> {
        self.stats.clone()
    }

    fn save_transport(&self, transport: Either<RequestEvent, ResponseEvent>) {
        let mut q = self.stats.history.write().unwrap();
        q.push_front(transport);
        if q.len() > Self::HISTORY_SIZE * 2 {
            q.truncate(Self::HISTORY_SIZE);
        }
    }
}
