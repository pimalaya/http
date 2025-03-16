use tracing::instrument;

use super::{Io, Request, Response, State};

#[derive(Debug)]
pub struct Flow {
    state: State,
}

impl Flow {
    #[instrument(skip_all)]
    pub fn new(request: impl Into<Request>) -> Self {
        let state = State::new(request);
        Self { state }
    }

    #[instrument(skip_all)]
    pub fn next(&mut self) -> Result<Response, Io> {
        match self.state.response.take() {
            Some(response) => Ok(response),
            None => Err(Io::Send),
        }
    }
}

impl AsMut<State> for Flow {
    fn as_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
