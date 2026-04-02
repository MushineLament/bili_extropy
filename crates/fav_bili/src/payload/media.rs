use api_req::Payload;
use serde::Serialize;

#[derive(Debug, Payload, Serialize)]
#[api_req(path = "/x/web-interface/wbi/view")]
pub struct MediaInfoAidPayload {
    pub aid: i64,
}

#[derive(Debug, Payload, Serialize)]
#[api_req(path = "/x/web-interface/wbi/view")]
pub struct MediaInfoBvidPayload {
    pub bvid: String,
}