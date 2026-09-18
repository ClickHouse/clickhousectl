//! CLI-owned stdout treats a reader closing its pipe as successful delivery.
//! Keep running the command so a later API/child failure still wins. Other I/O
//! errors remain errors; infallible print macros defer them to the command tail.

use std::fmt;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};

// These shadow the standard macros throughout this binary. Declare this module
// before all other modules in main.rs so new human/JSON output shares the policy.
macro_rules! print {
    ($($arg:tt)*) => { $crate::stdout::print(format_args!($($arg)*)) };
}
macro_rules! println {
    () => { $crate::stdout::print(format_args!("\n")) };
    ($($arg:tt)*) => { $crate::stdout::print(format_args!("{}\n", format_args!($($arg)*))) };
}

struct PipeWriter<W> {
    inner: W,
    closed: bool,
}

impl<W: Write> Write for PipeWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.closed {
            return Ok(bytes.len());
        }
        match self.inner.write(bytes) {
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.closed = true;
                Ok(bytes.len())
            }
            result => result,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        match self.inner.flush() {
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.closed = true;
                Ok(())
            }
            result => result,
        }
    }
}

// Docker forwards native-client output asynchronously while feeding stdin.
// Keep the writer asynchronous so backpressure cannot stall that input task.
impl<W: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for PipeWriter<W> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        if self.closed {
            return std::task::Poll::Ready(Ok(bytes.len()));
        }
        match std::pin::Pin::new(&mut self.inner).poll_write(cx, bytes) {
            std::task::Poll::Ready(Err(error)) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.closed = true;
                std::task::Poll::Ready(Ok(bytes.len()))
            }
            result => result,
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        if self.closed {
            return std::task::Poll::Ready(Ok(()));
        }
        match std::pin::Pin::new(&mut self.inner).poll_flush(cx) {
            std::task::Poll::Ready(Err(error)) if error.kind() == io::ErrorKind::BrokenPipe => {
                self.closed = true;
                std::task::Poll::Ready(Ok(()))
            }
            result => result,
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

pub(crate) fn async_stdout() -> impl tokio::io::AsyncWrite + Unpin {
    PipeWriter {
        inner: tokio::io::stdout(),
        closed: false,
    }
}

fn writer() -> &'static Mutex<PipeWriter<io::Stdout>> {
    static WRITER: OnceLock<Mutex<PipeWriter<io::Stdout>>> = OnceLock::new();
    WRITER.get_or_init(|| {
        Mutex::new(PipeWriter {
            inner: io::stdout(),
            closed: false,
        })
    })
}

static PRINT_ERROR: Mutex<Option<io::Error>> = Mutex::new(None);

/// A stdout handle with the same closed-pipe policy as the print macros.
pub(crate) struct Stdout;

pub(crate) fn stdout() -> Stdout {
    Stdout
}

impl Write for Stdout {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        writer().lock().unwrap().write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        writer().lock().unwrap().flush()
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> io::Result<()> {
        writer().lock().unwrap().write_fmt(args)
    }
}

pub(crate) fn print(args: fmt::Arguments<'_>) {
    record(stdout().write_fmt(args));
}

/// Also used for clap, which owns its help/version output. Usage errors retain
/// clap's exit status regardless of whether their diagnostic can be written.
pub(crate) fn record(result: io::Result<()>) {
    if let Err(error) = result
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        PRINT_ERROR.lock().unwrap().get_or_insert(error);
    }
}

/// Flush before rendering the result and recording telemetry, even when the
/// final output had no newline. Never overwrite an independently failing result.
pub(crate) fn finish<T>(result: crate::error::Result<T>) -> crate::error::Result<T> {
    record(stdout().flush());
    let output_error = PRINT_ERROR.lock().unwrap().take();
    match (result, output_error) {
        (Ok(_), Some(error)) => Err(error.into()),
        (result, _) => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fails {
        write_error: Option<io::ErrorKind>,
        flush_error: Option<io::ErrorKind>,
        writes: usize,
    }

    impl Write for Fails {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            self.write_error
                .map_or(Ok(bytes.len()), |kind| Err(kind.into()))
        }
        fn flush(&mut self) -> io::Result<()> {
            self.flush_error.map_or(Ok(()), |kind| Err(kind.into()))
        }
    }

    impl tokio::io::AsyncWrite for Fails {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            bytes: &[u8],
        ) -> std::task::Poll<io::Result<usize>> {
            std::task::Poll::Ready(Write::write(&mut *self, bytes))
        }
        fn poll_flush(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            std::task::Poll::Ready(Write::flush(&mut *self))
        }
        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            self.poll_flush(cx)
        }
    }

    #[tokio::test]
    async fn async_forwarding_suppresses_only_broken_pipe() {
        use tokio::io::AsyncWriteExt;
        for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::PermissionDenied] {
            for flush in [false, true] {
                let mut writer = PipeWriter {
                    inner: Fails {
                        write_error: (!flush).then_some(kind),
                        flush_error: flush.then_some(kind),
                        writes: 0,
                    },
                    closed: false,
                };
                let result = match AsyncWriteExt::write_all(&mut writer, b"native output").await {
                    Ok(()) => AsyncWriteExt::flush(&mut writer).await,
                    error => error,
                };
                if kind == io::ErrorKind::BrokenPipe {
                    assert!(result.is_ok());
                    AsyncWriteExt::write_all(&mut writer, b"more output")
                        .await
                        .unwrap();
                    AsyncWriteExt::shutdown(&mut writer).await.unwrap();
                    assert_eq!(writer.inner.writes, 1);
                } else {
                    assert_eq!(result.unwrap_err().kind(), kind);
                }
            }
        }
    }

    #[test]
    fn closed_reader_discards_remaining_output() {
        let mut writer = PipeWriter {
            inner: Fails {
                write_error: Some(io::ErrorKind::BrokenPipe),
                flush_error: Some(io::ErrorKind::BrokenPipe),
                writes: 0,
            },
            closed: false,
        };
        writer.write_all(&vec![0; 1024 * 1024]).unwrap();
        writer.write_all(b"another line").unwrap();
        writer.flush().unwrap();
        assert_eq!(writer.inner.writes, 1);
    }

    #[test]
    fn only_broken_pipe_is_suppressed_on_write_or_flush() {
        for kind in [
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::WriteZero,
            io::ErrorKind::Other,
        ] {
            for flush in [false, true] {
                let mut writer = PipeWriter {
                    inner: Fails {
                        write_error: (!flush).then_some(kind),
                        flush_error: flush.then_some(kind),
                        writes: 0,
                    },
                    closed: false,
                };
                let result = writer.write_all(b"result").and_then(|()| writer.flush());
                if kind == io::ErrorKind::BrokenPipe {
                    assert!(result.is_ok());
                } else {
                    assert_eq!(result.unwrap_err().kind(), kind);
                }
            }
        }
    }
}
