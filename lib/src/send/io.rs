/// The HTTP I/O request enum, emitted by flows and processed by
/// connectors.
///
/// This enum represents all the possible I/O requests that a HTTP
/// flow can emit. I/O connectors should be able to handle all
/// variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Io {
    /// I/O request that should be emitted by a flow needing bytes to
    /// be sent in order to continue its progression.
    ///
    /// When receiving this variant, I/O connectors need to send the
    /// given request and collect the response from the HTTP stream.
    ///
    /// [state buffer]: crate::State::get_read_buffer_mut
    /// [how many bytes]: crate::State::set_read_bytes_count
    Send,
}
