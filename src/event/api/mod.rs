use std::sync::Arc;

use error_stack::ResultExt;
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing_futures::Instrument;

use crate::{
    event::api::elasticsearch::{
        ElasticsearchApiHandler, ElasticsearchRequestEvent, ElasticsearchResponseEvent,
    },
    ElasticsearchConfig,
};

pub(crate) mod elasticsearch;

#[derive(Debug, Clone)]
pub(crate) enum RequestEvent {
    Elasticsearch(ElasticsearchRequestEvent),
}

#[derive(Debug, Clone)]
pub(crate) enum ResponseEvent {
    Elasticsearch(ElasticsearchResponseEvent),
}

#[derive(Clone)]
pub(crate) struct ApiHandler {
    elasticsearch: Arc<ElasticsearchApiHandler>,
}

#[derive(Debug, Error)]
pub(crate) enum ApiHandleError {
    #[error("elasticsearch api error")]
    Elasticsearch,
}

impl ApiHandler {
    pub(crate) fn new(
        elasticsearch_configs: Vec<ElasticsearchConfig>,
    ) -> error_stack::Result<Self, ApiHandleError> {
        Ok(Self {
            elasticsearch: Arc::new(
                ElasticsearchApiHandler::new(elasticsearch_configs)
                    .change_context(ApiHandleError::Elasticsearch)?,
            ),
        })
    }

    pub(crate) async fn run(self, mut rx: Receiver<RequestEvent>, res_tx: Sender<ResponseEvent>) {
        tracing::info!("ApiHandler running...");

        loop {
            let req = match rx.recv().await {
                Some(req) => {
                    tracing::debug!(?req, "Receive");
                    req
                }
                None => break,
            };

            self.dispatch(req, res_tx.clone());
        }

        tracing::info!("Done");
    }

    fn dispatch(&self, req: RequestEvent, res_tx: Sender<ResponseEvent>) {
        // Cloning the entire handler is inefficient, should find a better way.
        let this = self.clone();
        let task = async move {
            let result = match req {
                RequestEvent::Elasticsearch(req) => {
                    let span = tracing::info_span!("dispatch",api="elasticsearch",request=?req);
                    this.elasticsearch
                        .handle(req)
                        .instrument(span)
                        .await
                        .map(ResponseEvent::Elasticsearch)
                }
            };
            match result {
                Ok(res) => {
                    res_tx.send(res).await.ok();
                }
                Err(report) => tracing::error!("{report:?}"),
            }
        };

        tokio::spawn(task);
    }
}
