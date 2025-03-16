use std::mem;

use memchr::memmem;
use stream_flows::{Io, State};

use crate::{
    request::{CR, CRLF, LF},
    Request, Response,
};

const CRLF_CRLF: [u8; 4] = [CR, LF, CR, LF];
const CONTENT_LENGTH: &[u8] = b"Content-Length";

#[derive(Debug)]
pub enum Step {
    SendRequest(Request),
    ReceiveResponse(Response),
}

#[derive(Debug)]
pub struct Send {
    step: Step,
    state: State,
}

impl Send {
    pub fn new(request: impl Into<Request>) -> Self {
        Self {
            step: Step::SendRequest(request.into()),
            state: State::default(),
        }
    }

    pub fn with_capacity(request: impl Into<Request>, capacity: usize) -> Self {
        Self {
            step: Step::SendRequest(request.into()),
            state: State::new(capacity),
        }
    }

    pub fn next(&mut self) -> Result<Response, Io> {
        match (&mut self.step, self.state.take_bytes_count()) {
            (Step::SendRequest(req), None) => {
                self.state.enqueue_bytes(req.as_ref());
                Err(Io::Write)
            }
            (Step::SendRequest(_), Some(_)) => {
                self.step = Step::ReceiveResponse(Response::default());
                Err(Io::Read)
            }
            (Step::ReceiveResponse(_), None) => {
                return Err(Io::Read);
            }
            (Step::ReceiveResponse(res), Some(0)) => {
                return Ok(mem::take(res));
            }
            (Step::ReceiveResponse(res), Some(n)) => {
                res.bytes.extend(self.state.get_read_bytes(n));

                if res.body_start == 0 {
                    let body_start = memmem::find(&res.bytes, &CRLF_CRLF);

                    if let Some(n) = body_start {
                        res.body_start = n + 4;
                    }
                }

                if res.body_start > 0 && res.body_length == 0 {
                    let mut content_length_start = None;

                    for crlf in memmem::find_iter(&res.bytes, &CRLF) {
                        if let Some(start) = content_length_start {
                            let length = &res.bytes[start..crlf];
                            let length = String::from_utf8_lossy(length);
                            res.body_length = length.trim().parse().unwrap();
                            break;
                        }

                        // take bytes after the found CRLF
                        let crlf = crlf + CRLF.len();
                        let bytes = &res.bytes[crlf..];

                        // break if length of bytes after CRLF is
                        // smaller than `Content-Length: 0`
                        let colon_space_digit = 3;
                        if bytes.len() < CONTENT_LENGTH.len() + colon_space_digit {
                            break;
                        }

                        // search for another CRLF if header does
                        // not match Content-Type
                        if !bytes[..CONTENT_LENGTH.len()].eq_ignore_ascii_case(CONTENT_LENGTH) {
                            continue;
                        }

                        content_length_start = Some(crlf + CONTENT_LENGTH.len() + 1);
                    }
                }

                if res.body_start > 0 && res.body_length > 0 {
                    let body_bytes = &res.bytes[res.body_start..];

                    if body_bytes.len() >= res.body_length {
                        return Ok(mem::take(res));
                    }
                }

                Err(Io::Read)
            }
        }
    }
}

impl AsMut<State> for Send {
    fn as_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
