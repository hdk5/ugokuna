use anyhow::Result;
use async_stream::stream;
use async_stream::try_stream;
use derive_more::Constructor;
use futures::stream::BoxStream;

use crate::pixiv::types::IllustId;
use crate::pixiv::types::ProfileId;
use crate::pixiv::CLIENT as PIXIV_CLIENT;

pub trait IllustIdProvider {
    fn into_stream(self) -> BoxStream<'static, Result<IllustId>>;
}

#[derive(Constructor)]
struct SimpleIllustIdProvider {
    illust_id: IllustId,
}

#[derive(Constructor)]
struct ProfileIllustIdProvider {
    profile_id: ProfileId,
}

#[derive(Constructor)]
pub struct MasterIllustIdProvider {
    illust_ids: Vec<IllustId>,
    profile_ids: Vec<ProfileId>,
}

impl IllustIdProvider for SimpleIllustIdProvider {
    fn into_stream(self) -> BoxStream<'static, Result<IllustId>> {
        Box::pin(try_stream! {
            yield self.illust_id;
        })
    }
}

impl IllustIdProvider for ProfileIllustIdProvider {
    fn into_stream(self) -> BoxStream<'static, Result<IllustId>> {
        Box::pin(stream! {
            let client = &PIXIV_CLIENT;
            let resp = client.profile(self.profile_id).await?;

            let master_stream = futures::stream::select_all(
                resp.illusts
                    .into_iter()
                    .map(IllustId::from)
                    .map(SimpleIllustIdProvider::new)
                    .map(IllustIdProvider::into_stream)
            );

            for await illust_id in master_stream {
                yield illust_id;
            }
        })
    }
}

impl IllustIdProvider for MasterIllustIdProvider {
    fn into_stream(self) -> BoxStream<'static, Result<IllustId>> {
        Box::pin(stream! {
            let illusts_stream = futures::stream::select_all(
                self.illust_ids
                    .into_iter()
                    .map(SimpleIllustIdProvider::new)
                    .map(IllustIdProvider::into_stream),
            );

            let profiles_stream = futures::stream::select_all(
                self.profile_ids
                    .into_iter()
                    .map(ProfileIllustIdProvider::new)
                    .map(IllustIdProvider::into_stream),
            );

            let master_stream = futures::stream::select_all([illusts_stream, profiles_stream]);

            for await illust_id in master_stream {
                yield illust_id;
            }
        })
    }
}
