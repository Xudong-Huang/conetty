use WireError;

/// raw request/response wrapper
#[derive(Serialize, Deserialize)]
pub struct Request {
    pub id: usize,
    pub data: Vec<u8>,
}

/// raw request/response wrapper
#[derive(Serialize, Deserialize)]
pub struct Response {
    pub id: usize,
    pub data: Result<Vec<u8>, WireError>,
}
