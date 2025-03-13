use super::HulyStore;
use async_trait::async_trait;
use rustical_store::{Error, Subscription, SubscriptionStore};

#[async_trait]
impl SubscriptionStore for HulyStore {
    async fn get_subscriptions(&self, _topic: &str) -> Result<Vec<Subscription>, Error> {
        Err(Error::NotFound)
    }

    async fn get_subscription(&self, _id: &str) -> Result<Subscription, Error> {
        Err(Error::NotFound)
    }

    async fn upsert_subscription(&self, _sub: Subscription) -> Result<bool, Error> {
        Err(Error::NotFound)
    }

    async fn delete_subscription(&self, _id: &str) -> Result<(), Error> {
        Err(Error::NotFound)
    }
}
