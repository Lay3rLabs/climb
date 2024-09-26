use crate::prelude::*;
use std::time::Duration;

#[derive(Clone)]
pub struct QueryRetryMiddleware {
    pub max_attempts: u32,
    pub backoff: Duration,
}

impl Default for QueryRetryMiddleware {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: Duration::from_millis(100),
        }
    }
}

impl QueryRetryMiddleware {
    pub async fn run<REQ: QueryRequest>(
        &self,
        req: REQ,
        client: QueryClient,
    ) -> Result<REQ::QueryResponse> {
        let mut attempts = 0;
        let mut backoff = self.backoff;

        loop {
            attempts += 1;
            match req.request(client.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    if attempts < self.max_attempts {
                        futures_timer::Delay::new(backoff).await;
                        backoff *= 2;
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }
}
