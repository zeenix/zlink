use crate::{connection, Call};

use super::service;

use alloc::collections::BTreeMap;

/// A service that aggregates multiple services, one per connection.
#[derive(Debug)]
pub struct AggregatorService<SrvInstantiator: ServiceInstantiator> {
    services: BTreeMap<usize, SrvInstantiator::Service>,
}

impl<SrvInstantiator> AggregatorService<SrvInstantiator>
where
    SrvInstantiator: ServiceInstantiator,
{
    /// Creates a new aggregator service.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<SrvInstantiator> service::Service<SrvInstantiator::Socket>
    for AggregatorService<SrvInstantiator>
where
    SrvInstantiator: ServiceInstantiator,
{
    type MethodCall<'de> =
        <SrvInstantiator::Service as service::Service<SrvInstantiator::Socket>>::MethodCall<'de>;
    type ReplyParams<'ser>
        = <SrvInstantiator::Service as service::Service<SrvInstantiator::Socket>>::ReplyParams<'ser>
    where
        Self: 'ser;
    type ReplyStreamParams =
        <SrvInstantiator::Service as service::Service<SrvInstantiator::Socket>>::ReplyStreamParams;
    type ReplyStream =
        <SrvInstantiator::Service as service::Service<SrvInstantiator::Socket>>::ReplyStream;
    type ReplyError<'ser>
        = <SrvInstantiator::Service as service::Service<SrvInstantiator::Socket>>::ReplyError<'ser>
    where
        Self: 'ser;

    async fn handle<'ser>(
        &'ser mut self,
        method: &'ser Call<Self::MethodCall<'_>>,
        conn: &mut connection::Connection<SrvInstantiator::Socket>,
        #[cfg(feature = "std")] fds: Vec<std::os::fd::OwnedFd>,
    ) -> service::HandleResult<Self::ReplyParams<'ser>, Self::ReplyStream, Self::ReplyError<'ser>>
    {
        let id = conn.id();
        let service = self
            .services
            .entry(id)
            .or_insert_with(|| SrvInstantiator::instantiate(conn));
        service.handle(method, conn, fds).await
    }
}

impl<SrvInstantiator> Default for AggregatorService<SrvInstantiator>
where
    SrvInstantiator: ServiceInstantiator,
{
    fn default() -> Self {
        Self {
            services: BTreeMap::new(),
        }
    }
}

/// TODO:
pub trait ServiceInstantiator {
    /// The socket type used by the service.
    type Socket: connection::Socket;
    /// The service type instantiated by this instantiator.
    type Service: service::Service<Self::Socket>;

    /// Instantiates a service instance for a given connection.
    fn instantiate(connection: &mut connection::Connection<Self::Socket>) -> Self::Service;
}
