#[derive(Debug)]
pub enum Error {
    RequestFailure,
    ResponseFailure,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Error::*;

        match self {
            RequestFailure => write!(f, "Problem peforming the request."),
            ResponseFailure => write!(f, "Failed to complete the response."),
        }
    }
}
