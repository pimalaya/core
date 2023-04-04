use log::{debug, trace};
use std::io;

use crate::time::pomodoro::{Request, Response, ThreadSafeTimer};

pub trait ProtocolStream<T> {
    fn read(&self, stream: &T) -> io::Result<Request>;
    fn write(&self, stream: &mut T, res: Response) -> io::Result<()>;

    fn handle_stream(&self, timer: ThreadSafeTimer, stream: &mut T) -> io::Result<()> {
        let req = self.read(stream)?;
        let res = match req {
            Request::Start => {
                debug!("starting timer");
                timer.start()?;
                Response::Ok
            }
            Request::Get => {
                debug!("getting timer");
                let timer = timer.get()?;
                trace!("{timer:#?}");
                Response::Get(timer)
            }
            Request::Pause => {
                debug!("pausing timer");
                timer.pause()?;
                Response::Ok
            }
            Request::Resume => {
                debug!("resuming timer");
                timer.resume()?;
                Response::Ok
            }
            Request::Stop => {
                debug!("stopping timer");
                timer.stop()?;
                Response::Ok
            }
        };
        self.write(stream, res)?;
        Ok(())
    }
}

pub trait Protocol {
    fn bind(&self, timer: ThreadSafeTimer) -> io::Result<()>;
    fn send(&self, timer: ThreadSafeTimer) -> io::Result<()>;
}
