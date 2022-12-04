use tokio::sync::{mpsc, oneshot};

use crate::error::Error;

enum RequestWrapper<Req, Resp> {
    WithResponse(Req, oneshot::Sender<Resp>),
    NoResponse(Req),
}

pub trait RequestHandler {
    type Request;
    type Response;
    fn handle_request(&mut self, request: Self::Request) -> Self::Response;
}

#[derive(Debug)]
pub struct Connection<Req, Resp> {
    sender: mpsc::Sender<RequestWrapper<Req, Resp>>,
}

impl<Req, Resp> Connection<Req, Resp> {
    pub async fn call_async(&self, request: Req) -> Result<Resp, Error> {
        let (sender, receiver) = oneshot::channel();
        let request = RequestWrapper::WithResponse(request, sender);
        self.sender
            .send(request)
            .await
            .map_err(|_e| Error::RequestFailure)?;

        receiver.await.map_err(|_e| Error::ResponseFailure)
    }

    pub async fn call_async_no_response(&self, request: Req) -> Result<(), Error> {
        let request = RequestWrapper::NoResponse(request);
        self.sender
            .send(request)
            .await
            .map_err(|_e| Error::RequestFailure)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Entity<T>
where
    T: RequestHandler,
{
    object: T,
}

impl<T> Entity<T>
where
    T: RequestHandler,
{
    pub fn new(object: T) -> Self {
        Self { object }
    }
}

impl<T, Req, Resp> Entity<T>
where
    T: RequestHandler<Request = Req, Response = Resp> + Send + 'static,
    Req: Send + 'static,
    Resp: Send + 'static,
{
    pub fn listen(mut self) -> Connection<Req, Resp> {
        let (sender, mut receiver) = mpsc::channel::<RequestWrapper<Req, Resp>>(1);
        let msg_loop = async move {
            while let Some(wrapped_request) = receiver.recv().await {
                match wrapped_request {
                    RequestWrapper::WithResponse(request, response_sender) => {
                        let response = self.object.handle_request(request);
                        let _ = response_sender.send(response).map_err(|e| {
                            eprintln!("Problem sending response.");
                            e
                        });
                    }
                    RequestWrapper::NoResponse(request) => {
                        self.object.handle_request(request);
                    }
                }
            }
        };

        tokio::spawn(msg_loop);
        Connection { sender }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum ObjectMsg {
        Echo(String),
    }

    struct TestObject;

    impl RequestHandler for TestObject {
        type Request = ObjectMsg;
        type Response = ObjectMsg;

        fn handle_request(&mut self, request: Self::Request) -> Self::Response {
            match request {
                ObjectMsg::Echo(s) => ObjectMsg::Echo(s),
            }
        }
    }

    #[tokio::test]
    async fn create_entity() {
        let connection = Entity::new(TestObject {}).listen();
        let answer = connection
            .call_async(ObjectMsg::Echo("Hello".to_string()))
            .await
            .unwrap();
        assert_eq!(answer, ObjectMsg::Echo("Hello".to_string()));
    }
}
