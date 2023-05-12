//! https://www.cqgma.org/wwff/doc/DX_Spider_EN.pdf

use std::fmt;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tracing::instrument;

use crate::config::CqgmaConfig;

pub struct CqgmaState {
    /// CQGMA telnet connection management task
    pub handle: JoinHandle<io::Result<()>>,
    /// A channel to send content to CQGMA telnet
    pub telnet_tx: UnboundedSender<String>,
    /// A channel receiving content from CQGMA telnet
    pub telnet_rx: UnboundedReceiver<String>,
}

pub async fn cqgma_init(config: &CqgmaConfig) -> CqgmaState {
    let (telnet_rx, user_tx) = unbounded_channel();
    let (user_rx, telnet_tx) = unbounded_channel();
    let host = config.host.clone();
    let user = config.username.clone();
    let handle = tokio::spawn(async { manage_telnet(host, user, telnet_rx, telnet_tx).await });
    CqgmaState {
        handle,
        telnet_rx: user_tx,
        telnet_tx: user_rx,
    }
}

/// Keep telnet connection to CQGMA going.
#[instrument(skip(telnet_rx, telnet_tx))]
async fn manage_telnet<H>(
    host: H,
    username: String,
    telnet_rx: UnboundedSender<String>,
    mut telnet_tx: UnboundedReceiver<String>,
) -> io::Result<()>
where
    H: ToSocketAddrs + fmt::Debug,
{
    loop {
        // Pre-calculate next sleep duration
        let sleep_for = rand_sleep();

        let mut stream = match connect(&host).await {
            Ok(s) => s,
            Err(err) => {
                tracing::error!(
                    "Telnet connection failed: {err}. Will retry in {} seconds.",
                    sleep_for.as_secs()
                );
                tokio::time::sleep(sleep_for).await;
                continue;
            }
        };

        match login(&mut stream, &username).await {
            Ok(()) => (),
            Err(err) => {
                tracing::error!("Telnet login failed: {err}.");
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "couldn't login",
                ));
            }
        }

        let (rx, mut tx) = stream.split();
        let mut lines = BufReader::new(rx).lines();

        'select: loop {
            tokio::select! {
                v = lines.next_line() => match v {
                    Ok(Some(line)) => {
                        let line: String = line.trim_end().trim_end_matches('\x07').to_string();
                        tracing::debug!("telnet rx: ^{line}$");
                        if line_filter(&line) {
                            if let Err(err) = telnet_rx.send(line) {
                                tracing::error!("Error when trying to send to channel: {err:?}");
                                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "telnet channel (rx) closed"));
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::error!("No more lines to read from telnet. Connection dead?");
                        break 'select;
                    }
                    Err(err) => tracing::warn!("Invalid line from telnet: {err:?}"),
                },
                v = telnet_tx.recv() => match v {
                    Some(line) => {
                        tracing::debug!("telnet tx: ^{line}$");
                        let s = format!("{line}\n");
                        if let Err(err) = tx.write_all(s.as_bytes()).await {
                            tracing::error!("Error when trying to send to telnet: {err:?}.");
                            break 'select;
                        }
                    }
                    None => {
                        tracing::error!("Telnet TX channel closed. Going to close the telnet connection.");
                        return Err(io::Error::new(io::ErrorKind::BrokenPipe, "telnet channel (tx) closed"));
                    }
                }
            }
        }

        tracing::error!(
            "Probably lost telnet connection. Going to reconnect in {} seconds...",
            sleep_for.as_secs()
        );
        tokio::time::sleep(sleep_for).await;
    }
}

#[instrument]
async fn connect<H>(addr: H) -> io::Result<TcpStream>
where
    H: ToSocketAddrs + fmt::Debug,
{
    let addrs = addr.to_socket_addrs()?;
    for addr in addrs {
        let socket = match addr {
            SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4()?,
            SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6()?,
        };
        if let Ok(stream) = socket.connect(addr).await {
            stream.set_nodelay(true)?;
            tracing::debug!("Connected to {addr:?}");
            return Ok(stream);
        } else {
            tracing::trace!("Couldn't connect to {addr:?}. Trying next one.");
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        format!("couldn't connect to {addr:?}"),
    ))
}

#[instrument]
async fn login(stream: &mut TcpStream, username: &str) -> io::Result<()> {
    let (rx, mut tx) = stream.split();
    let mut rx = BufReader::new(rx);

    let mut buf = Vec::new();
    rx.read_until(b' ', &mut buf).await?;

    if let Ok(s) = std::str::from_utf8(&buf) {
        tracing::trace!("First line received: {s}");
        if s.starts_with("login:") {
            tx.write_all(format!("{username}\n").as_bytes()).await?;
            return Ok(());
        }
    }

    tracing::error!("Received line: {}", String::from_utf8_lossy(&buf));
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "received unexpected data from CQGMA telnet",
    ))
}

fn line_filter(line: &str) -> bool {
    let line = line.to_lowercase();

    // Line is not a cluster spot
    if !line.starts_with("dx de") {
        return false;
    }

    // Spots from OH and OG stations
    if line.starts_with("dx de oh") || line.starts_with("dx de og") {
        if line.chars().nth(8) >= Some('0') && line.chars().nth(8) <= Some('9') {
            return true;
        }
    }

    // WWFF spots
    if line.contains("ohff-") {
        return true;
    }

    // POTA spots
    if line.contains("oh-") {
        return true;
    }

    false
}

/// This provides [Duration] between [17, 34] seconds.
fn rand_sleep() -> Duration {
    use rand::distributions::Uniform;
    use rand::{thread_rng, Rng};

    const TIMEOUT: Duration = Duration::from_secs(17);
    let timeout_fuzz: Uniform<Duration> = Uniform::new_inclusive(Duration::from_secs(0), TIMEOUT);

    TIMEOUT + thread_rng().sample(timeout_fuzz)
}

#[cfg(test)]
mod tests {
    use super::line_filter;

    #[test]
    fn test_line_filter() {
        assert!(!line_filter(
            "DX de AD6VT:     14310.0  AD6VT        x04s W6/ND-101                 1959Z"
        ));
        assert!(line_filter(
            "DX de OH8HUB:    14310.0  AD6VT        x04s W6/ND-101                 1959Z"
        ));
        assert!(line_filter(
            "DX de OG0Z:      14310.0  AD6VT        x04s W6/ND-101                 1959Z"
        ));
    }
}
