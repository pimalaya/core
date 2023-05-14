use log::{debug, trace};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{
    backend::maildir::{Error, Result},
    Envelope, Flags,
};

const BODY_DELIM: &[u8] = b"\r\n\r\n";

impl TryFrom<maildir::MailEntry> for Envelope {
    type Error = Error;

    fn try_from(entry: maildir::MailEntry) -> Result<Self> {
        debug!("trying to parse envelope from maildir entry");

        // opens email file and reads headers till body separator
        let headers = {
            let mut headers: Vec<u8> = Vec::new();

            let email_file = File::options()
                .read(true)
                .write(false)
                .open(entry.path())
                .map_err(|err| Error::OpenEmailFileError(err, entry.path().clone()))?;

            let mut email_reader = BufReader::new(email_file);

            loop {
                let num_bytes = email_reader
                    .read_until(b'\n', &mut headers)
                    .map_err(|err| Error::ReadEmailLineError(err, entry.path().clone()))?;

                if num_bytes == 0 {
                    break;
                }

                let begin = headers.len() - BODY_DELIM.len();
                let end = headers.len() - 1;
                let tail = &headers[begin..=end];

                if matches!(tail, BODY_DELIM) {
                    break;
                }
            }

            trace!("read headers: {:?}", String::from_utf8_lossy(&headers));
            headers
        };

        let mut envelope: Envelope = headers.as_slice().into();

        envelope.id = entry.id().to_owned();

        envelope.flags = Flags::from(&entry);

        trace!("maildir envelope: {envelope:#?}");
        Ok(envelope)
    }
}
