use anyhow;
use async_trait::async_trait;

use r53::{Request, Response};

#[async_trait]
pub trait Handler: Send + Clone + 'static {
    async fn resolve(&mut self, req: Request) -> anyhow::Result<Response>;
}
