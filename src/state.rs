use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, UniqueIndex};

use crate::{
    launch::Launch,
    msg::{Config, ReplyInfo},
};

pub const CONFIG: Item<Config> = Item::new("config");
pub const REPLY: Item<ReplyInfo> = Item::new("reply");

const LAUNCH_NAMESPACE: &str = "launch";

pub struct LaunchIndexes<'a> {
    pub key: UniqueIndex<'a, u128, Launch>,
    pub status: MultiIndex<'a, String, Launch, u128>,
}

impl<'a> IndexList<Launch> for LaunchIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Launch>> + '_> {
        let v: Vec<&dyn Index<Launch>> = vec![&self.key, &self.key, &self.status];
        Box::new(v.into_iter())
    }
}

pub fn launch<'a>() -> IndexedMap<'a, u128, Launch, LaunchIndexes<'a>> {
    let indexes = LaunchIndexes {
        key: UniqueIndex::new(|d| d.idx.u128(), "launch__key"),
        status: MultiIndex::new(
            |_d, d| d.status.clone().to_string(),
            LAUNCH_NAMESPACE,
            "launch__status",
        ),
    };
    IndexedMap::new(LAUNCH_NAMESPACE, indexes)
}
