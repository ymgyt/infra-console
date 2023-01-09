use std::fmt::{self, Display};

use ascii::AsAsciiStr;

use crate::view::component::elasticsearch::ElasticsearchComponentKind;

pub(crate) mod elasticsearch;
pub(crate) mod resource_tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComponentKind {
    ResourceTab,
    Elasticsearch(ElasticsearchComponentKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceKind {
    Elasticsearch,
    Mongo,
    RabbitMQ,
}

impl ResourceKind {
    pub(crate) fn variants() -> &'static [ResourceKind] {
        static VARIANTS: &[ResourceKind] = &[
            ResourceKind::Elasticsearch,
            ResourceKind::Mongo,
            ResourceKind::RabbitMQ,
        ];

        VARIANTS
    }
}

impl Display for ResourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ResourceKind::Elasticsearch => "elasticsearch",
            ResourceKind::Mongo => "mongo",
            ResourceKind::RabbitMQ => "rabbitmq",
        };
        f.write_str(s)
    }
}

trait StringUtil {
    fn capitalize(&self) -> String;
}

impl<T> StringUtil for T
where
    T: Display,
{
    fn capitalize(&self) -> String {
        let s = self.to_string();
        let a = s.as_str().as_ascii_str();
        match a {
            Ok(a) => format!("{}{}", a[0].to_ascii_uppercase(), &a[1..]),
            Err(_) => s,
        }
    }
}
