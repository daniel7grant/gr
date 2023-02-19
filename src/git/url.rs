use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub fn parse_url(url: &str) -> Result<(String, String)> {
    debug!("Parsing URL {url}.");
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

        debug!("Parsed remote URL to host {host} and path {path}.");

        Ok((host.to_string(), path.to_string()))
    } else {
        Err(eyre!("Local directories are not supported."))
    }
}
