pub mod account;
pub mod account_collection;
pub mod collection;
pub mod collection_media;
pub mod media;
pub mod status;
pub mod up;
pub mod up_account;
pub mod up_media;

use crate::table::head;

pub trait ToTableRecord<const N: usize> {
    fn to_record(self) -> [String; N];
}

impl ToTableRecord<3> for account::AccountModel {
    fn to_record(self) -> [String; 3] {
        [self.account_id.to_string(), self.name, self.state]
    }
}

impl ToTableRecord<4> for collection::CollectionModel {
    fn to_record(self) -> [String; 4] {
        [
            self.collection_id.to_string(),
            head(self.name, 20),
            self.count.to_string(),
            self.state,
        ]
    }
}

impl ToTableRecord<4> for status::StatusModel {
    fn to_record(self) -> [String; 4] {
        [
            head(self.id.unwrap_or(-1).to_string(), 5),
            self.state,
            self.name,
            self.path,
        ]
    }
}

impl ToTableRecord<3> for up::Model {
    fn to_record(self) -> [String; 3] {
        [self.up_id.to_string(), head(self.name, 20), self.state]
    }
}
