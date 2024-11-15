// use std::io::{Read, Result, Write};

// use crate::{PollStartTls, StartTls};

// impl<S, T> StartTls<S, T, false>
// where
//     S: Read + Write,
//     T: PollStartTls<S, false, Output<()> = Result<()>>,
// {
//     pub fn prepare(self, stream: &mut S) -> Result<()> {
//         self.0.poll_start_tls(stream, None)
//     }
// }
