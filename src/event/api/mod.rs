use std::sync::Arc;

use error_stack::ResultExt;
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing_futures::Instrument;

use crate::{
    app::RequestId,
    event::api::elasticsearch::{
        ElasticsearchApiHandler, ElasticsearchRequestEvent, ElasticsearchResponseEvent,
    },
    ElasticsearchConfig,
};

pub(crate) mod elasticsearch;

#[derive(Debug, Clone)]
pub(crate) struct RequestEnvelope {
    pub(crate) request_id: RequestId,
    pub(crate) event: RequestEvent,
}

#[derive(Debug, Clone)]
pub(crate) enum RequestEvent {
    Elasticsearch(ElasticsearchRequestEvent),
}

#[derive(Debug)]
pub(crate) struct ResponseEnvelope {
    pub(crate) request_id: RequestId,
    pub(crate) result: error_stack::Result<ResponseEvent, ApiHandleError>,
}

#[derive(Debug, Clone)]
pub(crate) enum ResponseEvent {
    Elasticsearch(ElasticsearchResponseEvent),
}

#[derive(Clone)]
pub(crate) struct ApiHandler {
    elasticsearch: Arc<ElasticsearchApiHandler>,
}

#[derive(Clone, Debug, Error)]
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

    pub(crate) async fn run(
        self,
        mut rx: Receiver<RequestEnvelope>,
        res_tx: Sender<ResponseEnvelope>,
    ) {
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

    fn dispatch(&self, e: RequestEnvelope, res_tx: Sender<ResponseEnvelope>) {
        // Cloning the entire handler is inefficient, should find a better way.
        let this = self.clone();
        let task = async move {
            let result = match e.event {
                RequestEvent::Elasticsearch(req) => {
                    let span = tracing::info_span!("dispatch",api="elasticsearch",request=?req,id=?e.request_id);
                    this.elasticsearch
                        .handle(req)
                        .instrument(span)
                        .await
                        .map(ResponseEvent::Elasticsearch)
                }
            };
            // TODO: to chain by futures;
            res_tx
                .send(ResponseEnvelope {
                    request_id: e.request_id,
                    result,
                })
                .await
                .ok();
        };

        tokio::spawn(task);
    }
}
