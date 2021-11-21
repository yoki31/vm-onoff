#[derive(Debug, thiserror::Error)]
#[error("{status_code} status code")]
pub struct ServerError {
    pub status_code: u16,
}

pub fn check_status(res: &reqwest::Response) -> Result<(), ServerError> {
    let status = res.status();
    if !status.is_success() {
        return Err(ServerError {
            status_code: status.as_u16(),
        });
    }
    Ok(())
}
