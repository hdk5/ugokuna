use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Deserializer;

#[derive(Deserialize, Debug)]
pub struct Response<T>
where
    T: DeserializeOwned,
{
    pub error: bool,
    pub message: String,

    #[serde(deserialize_with = "de_body")]
    pub body: Option<T>,
}

fn de_body<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Body<T> {
        Empty(Vec<()>),
        Value(T),
    }

    match Body::<T>::deserialize(deserializer)? {
        Body::Value(v) => Ok(Some(v)),
        Body::Empty(_) => Ok(None),
    }
}
