use std::collections::HashMap;

use derive_more::Add;
use derive_more::Display;
use derive_more::From;
use derive_more::Into;
use derive_more::Sub;
use serde::Deserialize;
use serde::Deserializer;

#[derive(Deserialize, From, Into, Display, Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct IllustId(u32);

#[derive(Deserialize, From, Into, Display, Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct ProfileId(u32);

#[derive(
    Deserialize,
    From,
    Into,
    Add,
    Sub,
    Display,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    Debug,
)]
pub struct UgoiraDelay(u32);

#[derive(Deserialize, Debug)]
pub struct Profile {
    #[serde(deserialize_with = "de_profile_illusts")]
    pub illusts: Vec<IllustId>,
}

fn de_profile_illusts<'de, D>(deserializer: D) -> Result<Vec<IllustId>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    HashMap::<String, Option<()>>::deserialize(deserializer)?
        .keys()
        .map(|s| s.parse::<u32>())
        .map(|r| match r {
            Ok(o) => Ok(IllustId::from(o)),
            Err(e) => Err(D::Error::custom(e)),
        })
        .collect::<Result<_, _>>()
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct UgoiraMeta {
    pub original_src: String,
    pub frames: Vec<UgoiraMetaFrame>,
}

#[derive(Deserialize, Debug)]
pub struct UgoiraMetaFrame {
    pub file: String,
    pub delay: UgoiraDelay,
}
