use std::mem;

use tracing::{instrument, warn};

use super::{Request, Response};

/// The I/O state.
///
/// This struct represents the I/O state used by I/O handlers to take
/// and set data. It is usually held by flows themselves, and serve as
/// communication bridge between flows and I/O handlers.
#[derive(Debug, Default)]
pub struct State {
    pub(crate) request: Request,
    pub(crate) response: Option<Response>,
}

impl State {
    #[instrument(skip_all)]
    pub(crate) fn new(request: impl Into<Request>) -> Self {
        Self {
            request: request.into(),
            response: None,
        }
    }

    #[instrument(skip_all)]
    pub fn take_request(&mut self) -> Request {
        mem::take(&mut self.request)
    }

    #[instrument(skip_all)]
    pub fn set_response(&mut self, response: impl Into<Response>) {
        self.response.replace(response.into());
    }
}
