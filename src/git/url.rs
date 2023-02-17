use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use tracing::instrument;

#[instrument]
pub fn parse_url(url: &str) -> Result<(String, String)> {
    if let Some((first, rest)) = url.split_once(':') {
        // Split host and path based on protocol
        let (host, path) = match first {
            "http" | "https" | "ssh" | "git" => rest[2..]
                .split_once('/')
                .wrap_err("URL should contain a path."),
            "ftp" | "ftps" => Err(eyre!("FTP protocol is not supported.")),
            _ => Ok((first, rest)),
        }?;

        // Remove user from host if it exists
        let host = host.split_once('@').map_or(host, |(_, h)| h);

        // Remove ".git" from end if it is there
        let path = path.split_once(".git").map_or(path, |(p, _)| p);

        Ok((host.to_string(), path.to_string()))
    } else {
        Err(eyre!("Local directories are not supported."))
    }
}
