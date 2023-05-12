use std::fmt;
use std::io;
use std::path::Path;

use matrix_sdk::ruma::{OwnedDeviceId, OwnedRoomId, OwnedUserId};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub matrix: MatrixConfig,
    pub cqgma: CqgmaConfig,
}

#[derive(Deserialize)]
pub struct MatrixConfig {
    pub homeserver: url::Url,
    pub access_token: String,
    pub user_id: OwnedUserId,
    pub device_id: OwnedDeviceId,
    pub room_id: OwnedRoomId,
}

#[derive(Debug, Deserialize)]
pub struct CqgmaConfig {
    pub host: String,
    pub username: String,
}

impl Config {
    pub fn read_from_file<P: AsRef<Path>>(file: P) -> io::Result<Config> {
        let s = std::fs::read(file)?;
        let s = std::str::from_utf8(&s)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "config is not valid utf-8"))?;
        toml::from_str(s).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }
}

impl fmt::Debug for MatrixConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MatrixConfig")
            .field("homeserver", &self.homeserver)
            .field("access_token", &"<IS SECRET>")
            .field("user_id", &self.user_id)
            .field("device_id", &self.device_id)
            .field("room", &self.room_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn test_read_config() {
        let raw = r##"
        [matrix]
        homeserver = "https://matrix.pikaviestin.fi:8448"
        access_token = "abcdefghijklmnopqrstuvwxyz12345678901234567890"
        user_id = "@puskapupu:pikaviestin.fi"
        device_id = "puskapupu"
        room_id = "!hVUOVQnjnxUgSTCdCJ:pikaviestin.fi"

        [cqgma]
        host = "www.cqgma.org:7300"
        username = "oh9xxx-4"
        "##;

        let parsed: Config = toml::from_str(raw).unwrap();
        dbg!(parsed);
    }
}
