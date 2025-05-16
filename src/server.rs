use tonic::{Request, Response, Result};

use crate::grpc::status_server::Status;
pub struct Server;

#[tonic::async_trait]
impl Status for Server {
    async fn ping(&self, _: Request<()>) -> Result<Response<()>, tonic::Status> {
        return Ok(Response::new(()));
    }
}
