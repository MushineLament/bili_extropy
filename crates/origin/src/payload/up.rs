use api_req::Payload;
use serde::Serialize;

use crate::entity::UpperCid;

#[derive(Debug, Serialize, Payload)]
#[api_req(path = "/x/relation/followings")]
pub struct FollowingUpPayload {
    pub vmid: i64,
    pub pn: i64,
    pub ps: u8,
}

#[derive(Debug, Serialize, Payload)]
#[api_req(path = "/x/relation/stat")]
pub struct FollowingNumPayload {
    pub vmid: i64,
}

#[derive(Debug, Serialize, Payload)]
#[api_req(path = "/x/space/navnum")]
pub struct PublishNumPayload {
    pub mid: i64,
}

#[derive(Debug, Serialize, Payload)]
#[api_req(path = "/x/space/acc/info")]
pub struct UpperInfomation {
    pub mid: UpperCid,
}
