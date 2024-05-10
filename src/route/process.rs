use crate::Error;
use std::{io::Read, os::fd::OwnedFd};
use super::RouteResponse;
use hyper::StatusCode;
use tokio::{io, process::Child};

pub struct RouteProcess {
    child: Child,
    read_pipe: os_pipe::PipeReader,
    write_pipe_fd: Option<OwnedFd>
}

impl RouteProcess {
    pub fn new(child: Child, read_pipe: os_pipe::PipeReader, write_pipe_fd: OwnedFd) -> Self {
        RouteProcess { child, read_pipe, write_pipe_fd: Some(write_pipe_fd) }
    }

    pub async fn load_stdin<S>(&mut self, reader: &mut S) -> Result<&mut Self, Error>
        where S: io::AsyncRead + Unpin
    {
        let mut stdin = self.child.stdin.take()
            .ok_or(Error::RouteIoOpen)?;

        io::copy(reader, &mut stdin).await?;

        Ok(self)
    }

    pub async fn wait(&mut self) -> Result<RouteResponse, Error> {
        let status = self.child.wait().await
            .map_err(|e| Error::RouteWait(e))?;

        // close writer side of pipe to avoid blocking reader
        let write_pipe_fd = self.write_pipe_fd.take()
            .expect("write_pipe_fd should be set");
        drop(write_pipe_fd);

        let mut pipe_buf = String::new();
        self.read_pipe.read_to_string(&mut pipe_buf)?;

        let headers = pipe_buf.lines()
            .map(parse_header)
            .collect::<Result<Vec<_>, _>>()?;

        let status = match headers.iter().find(|(k, _)| k == "Status") {
            // use status header read from pipe
            Some((_, status)) => StatusCode::from_u16(
                status.parse().map_err(|_| Error::InvalidStatus(status.to_string()))?
            ).map_err(|_| Error::InvalidStatus(status.to_string()))?,
            // or derive status from process exit code
            None => match status.success() {
                true => StatusCode::OK,
                false => StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let stdout = self.child.stdout.take()
            .ok_or(Error::RouteIoOpen)?;

        Ok(RouteResponse { status, headers, stdout })
    }
}

fn parse_header(line: &str) -> Result<(String, String), Error> {
    let parts: Vec<_> = line.splitn(2, ':')
        .map(|s| s.trim())
        .collect();

    match &parts[..] {
        &[name, value, ..] => Ok((name.to_owned(), value.to_owned())),
        _ => Err(Error::InvalidHeader(line.to_owned()))
    }
}