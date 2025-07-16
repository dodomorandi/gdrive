use std::io::Write;
use std::pin::Pin;
use std::task::Poll;
use std::{io, task};

use pin_project_lite::pin_project;
use tokio::io::AsyncWrite;

pin_project! {
    pub struct Md5Writer<T> {
        #[pin]
        writer: T,
        context: md5::Context,
    }
}

impl<T> Md5Writer<T> {
    pub fn new(writer: T) -> Self {
        Self {
            writer,
            context: md5::Context::new(),
        }
    }

    pub fn md5(self) -> md5::Digest {
        self.context.compute()
    }
}

impl<T: Write> Write for Md5Writer<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let byte_count = self.writer.write(buf)?;
        self.context.consume(&buf[..byte_count]);
        Ok(byte_count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<T: AsyncWrite> AsyncWrite for Md5Writer<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        this.writer.poll_write(cx, buf).map(|result| {
            result.inspect(|&written_bytes| this.context.consume(&buf[..written_bytes]))
        })
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project().writer.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        self.project().writer.poll_shutdown(cx)
    }
}
