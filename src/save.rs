use std::path::PathBuf;

use anyhow::Result;
use bytes::Bytes;
use derive_more::Constructor;
use futures::stream::BoxStream;
use par_stream::ParStreamExt;
use tokio::fs::File;

use crate::pixiv::types::IllustId;

pub trait SaveParams {
    fn get_extension(&self) -> &'static str;
}

#[derive(Clone, Copy)]
pub struct WebmParams;
#[derive(Clone, Copy)]
pub struct GifParams;

impl SaveParams for WebmParams {
    fn get_extension(&self) -> &'static str {
        "webm"
    }
}

impl SaveParams for GifParams {
    fn get_extension(&self) -> &'static str {
        "gif"
    }
}

#[derive(Constructor)]
struct UgoiraSaver {
    illust_id: IllustId,
    data: Bytes,
    dir: PathBuf,
    params: Box<dyn SaveParams + Send>,
}

impl UgoiraSaver {
    async fn into(self) -> Result<File> {
        let file_ext = self.params.get_extension();
        let filename = format!("{i}.{file_ext}", i = self.illust_id);
        let filepath = self.dir.join(filename);

        let mut file = File::create(filepath).await?;
        tokio::io::copy(&mut self.data.as_ref(), &mut file).await?;
        file.sync_all().await?;

        Ok(file)
    }
}

#[derive(Constructor)]
pub struct StreamUgoiraSaver {
    data_stream: BoxStream<'static, (IllustId, Bytes, PathBuf, Box<dyn SaveParams + Send>)>,
}

impl StreamUgoiraSaver {
    pub fn into_stream(self) -> BoxStream<'static, (IllustId, Result<File>)> {
        Box::pin(self.data_stream.par_then_unordered(
            None,
            |(illust_id, data, dir, params)| async move {
                let saver = UgoiraSaver::new(illust_id, data, dir, params);
                let file = saver.into().await;
                (illust_id, file)
            },
        ))
    }
}
