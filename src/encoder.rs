use std::process::Stdio;

use anyhow::Result;
use bytes::Bytes;
use derive_more::Constructor;
use fraction::Fraction;
use futures::stream::BoxStream;
use par_stream::ParStreamExt;
use tempfile::tempdir;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::ugoira::UgoiraFrame;

pub trait EncodeParams {
    fn inspect_cmd(&self, cmd: &mut Command);
}

#[derive(Clone, Copy)]
pub struct WebmParams;
#[derive(Clone, Copy)]
pub struct GifParams;

impl EncodeParams for WebmParams {
    fn inspect_cmd(&self, cmd: &mut Command) {
        cmd.arg("-c:v");
        cmd.arg("libvpx-vp9");
        cmd.arg("-lossless");
        cmd.arg("1");
        cmd.arg("-f");
        cmd.arg("webm");
    }
}

impl EncodeParams for GifParams {
    fn inspect_cmd(&self, cmd: &mut Command) {
        cmd.arg("-vf");
        cmd.arg("split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse");
        cmd.arg("-loop");
        cmd.arg("0");
        cmd.arg("-f");
        cmd.arg("gif");
    }
}

#[derive(Constructor)]
struct UgoiraEncoder {
    data: Vec<UgoiraFrame>,
    params: Box<dyn EncodeParams + Send>,
}

impl UgoiraEncoder {
    fn calculate_fps(&self) -> Fraction {
        let num = self.data.len();
        let den = self
            .data
            .iter()
            .map(|frame| u32::from(frame.delay))
            .sum::<u32>();
        Fraction::from(1000) * Fraction::from(num) / Fraction::from(den)
    }

    async fn into(self) -> Result<Bytes> {
        // TODO: should be a better, non-Command way to do this;
        //       look into gstreamer or ffmpeg bindings
        // TODO: maybe use image2 instead of ffconcat

        let temp_dir = tempdir()?;

        let mut ffconcat = File::create(temp_dir.path().join("ffconcat.txt")).await?;
        ffconcat.write_all(b"ffconcat version 1.0\n").await?;

        for UgoiraFrame {
            file, data, delay, ..
        } in self.data.iter()
        {
            let duration = Fraction::from(u32::from(*delay)) / Fraction::from(1000);
            ffconcat
                .write_all(format!("\nfile {file}\nduration {duration:.3}\n").as_bytes())
                .await?;

            let mut frame_file = File::create(temp_dir.path().join(file)).await?;
            tokio::io::copy(&mut data.as_ref(), &mut frame_file).await?;
        }

        ffconcat.sync_all().await?;

        let mut cmd = Command::new("ffmpeg");

        cmd.arg("-y");
        cmd.arg("-i");
        cmd.arg("ffconcat.txt");
        self.params.inspect_cmd(&mut cmd);
        cmd.arg("-r");
        cmd.arg(format!("{}", self.calculate_fps()));
        cmd.arg("out");

        cmd.current_dir(&temp_dir);
        cmd.stderr(Stdio::null());
        cmd.stdout(Stdio::null());

        let mut proc = cmd.spawn()?;
        proc.wait().await?;

        let mut buf_writer = Vec::<u8>::new();
        let mut outfile = File::open(temp_dir.path().join("out")).await?;
        tokio::io::copy(&mut outfile, &mut buf_writer).await?;

        Ok(Bytes::from(buf_writer))
    }
}

#[derive(Constructor)]
pub struct StreamUgoiraEncoder<I>
where
    I: 'static,
{
    data_stream: BoxStream<'static, (I, Vec<UgoiraFrame>, Box<dyn EncodeParams + Send>)>,
}

impl<I> StreamUgoiraEncoder<I>
where
    I: Send,
{
    pub fn into_stream(self) -> BoxStream<'static, (I, Result<Bytes>)> {
        Box::pin(
            self.data_stream
                .par_then_unordered(None, |(i, data, params)| async move {
                    let encoder = UgoiraEncoder::new(data, params);
                    // let encoder = UgoiraEncoder::new(data);
                    let encode = encoder.into().await;
                    (i, encode)
                }),
        )
    }
}
