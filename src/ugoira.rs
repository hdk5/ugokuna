use std::io::Cursor;

use anyhow::Result;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use derive_more::Constructor;
use futures::stream::BoxStream;
use par_stream::ParStreamExt;
use zip::ZipArchive;

use crate::pixiv::types::IllustId;
use crate::pixiv::types::UgoiraDelay;
use crate::pixiv::types::UgoiraMetaFrame;
use crate::pixiv::CLIENT as PIXIV_CLIENT;

#[derive(Constructor)]
struct UgoiraDataProvider {
    illust_id: IllustId,
}

#[derive(Debug)]
pub struct UgoiraFrame {
    pub file: String,
    pub data: Bytes,
    pub delay: UgoiraDelay,
}

impl UgoiraDataProvider {
    pub async fn into(self) -> Result<Vec<UgoiraFrame>> {
        let client = &PIXIV_CLIENT;

        let meta = client.ugoira_meta(self.illust_id).await?;
        let data = client.download_ugoira(&meta).await?;

        let mut zip_archive = ZipArchive::new(Cursor::new(data))?;

        let result = meta
            .frames
            .into_iter()
            .map(|UgoiraMetaFrame { file, delay, .. }| {
                let mut zip_file = zip_archive.by_name(&file)?;
                let filesize: usize = zip_file.size().try_into()?;

                let data = {
                    let mut buf_writer = BytesMut::with_capacity(filesize).writer();
                    std::io::copy(&mut zip_file, &mut buf_writer)?;
                    buf_writer.into_inner().freeze()
                };

                Ok(UgoiraFrame { file, data, delay })
            })
            .collect::<Result<_>>()?;

        Ok(result)
    }
}

#[derive(Constructor)]
pub struct StreamUgoiraDataProvider<I>
where
    I: 'static,
{
    illust_ids: BoxStream<'static, (I, IllustId)>,
}

impl<I> StreamUgoiraDataProvider<I>
where
    I: Send,
{
    pub fn into_stream(self) -> BoxStream<'static, (I, Result<Vec<UgoiraFrame>>)> {
        Box::pin(
            self.illust_ids
                .par_then_unordered(None, |(i, illust_id)| async move {
                    let prov = UgoiraDataProvider::new(illust_id);
                    let data = prov.into().await;
                    (i, data)
                }),
        )
    }
}
