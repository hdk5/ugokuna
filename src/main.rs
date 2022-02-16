#![allow(dead_code)]

mod encoder;
mod illust;
mod pixiv;
mod save;
mod ugoira;
mod util;

use std::path::PathBuf;

use anyhow::Result;
use clap::ArgEnum;
use clap::Parser;
use futures::StreamExt;

use crate::encoder::EncodeParams;
use crate::encoder::StreamUgoiraEncoder;
use crate::illust::IllustIdProvider;
use crate::illust::MasterIllustIdProvider;
use crate::pixiv::types::IllustId;
use crate::pixiv::types::ProfileId;
use crate::save::SaveParams;
use crate::save::StreamUgoiraSaver;
use crate::ugoira::StreamUgoiraDataProvider;

#[derive(ArgEnum, Clone, Copy, Debug)]
enum Format {
    Webm,
    Gif,
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, arg_enum, default_value_t = Format::Gif)]
    format: Format,

    #[clap(short, long)]
    profile_ids: Vec<u32>,

    #[clap(short, long)]
    illust_ids: Vec<u32>,

    out_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let Args {
        format,
        profile_ids,
        illust_ids,
        out_path,
    } = Args::parse();

    let profile_ids = profile_ids
        .into_iter()
        .map(ProfileId::from)
        .collect::<Vec<_>>();
    let illust_ids = illust_ids
        .into_iter()
        .map(IllustId::from)
        .collect::<Vec<_>>();

    tokio::fs::create_dir_all(&out_path).await?;

    // 1. Get illustrations IDs
    let illust_id_provider = MasterIllustIdProvider::new(illust_ids, profile_ids);
    let illusts_stream = illust_id_provider.into_stream();
    let illusts_stream = Box::pin(illusts_stream.filter_map(|r| async move {
        match r {
            Ok(o) => {
                println!("Added illustration {o} to download queue");
                Some(o)
            }
            Err(e) => {
                eprintln!("Error while obtaining illustration id: {e}");
                None
            }
        }
    }));

    // 2. Download frames
    let ugoira_data_stream = illusts_stream;
    let ugoira_data_stream = Box::pin(ugoira_data_stream.map(|i| (i, i)));
    let ugoira_data_provider = StreamUgoiraDataProvider::new(ugoira_data_stream);
    let ugoira_data_stream = ugoira_data_provider.into_stream();
    let ugoira_data_stream = Box::pin(ugoira_data_stream.filter_map(|(i, r)| async move {
        match r {
            Ok(o) => {
                println!("Added illustration {i} to encode queue");
                Some((i, o))
            }
            Err(e) => {
                eprintln!("Error while obtaining illustration {i} ugoira frames: {e:?}");
                None
            }
        }
    }));

    // 3. Call ffmpeg
    let ugoira_encoder_stream = ugoira_data_stream;
    let ugoira_encoder_stream = Box::pin(ugoira_encoder_stream.map(move |(i, f)| {
        let params: Box<dyn EncodeParams + Send> = match format {
            Format::Webm => Box::new(crate::encoder::WebmParams),
            Format::Gif => Box::new(crate::encoder::GifParams),
        };
        (i, f, params)
    }));
    let ugoira_encoder = StreamUgoiraEncoder::new(ugoira_encoder_stream);
    let ugoira_encoder_stream = ugoira_encoder.into_stream();
    let ugoira_encoder_stream = Box::pin(ugoira_encoder_stream.filter_map(|(i, r)| async move {
        match r {
            Ok(o) => {
                println!("Added illustration {i} to save queue");
                Some((i, o))
            }
            Err(e) => {
                eprintln!("Error while encoding illustration {i}: {e:?}");
                None
            }
        }
    }));

    // 4. Save files
    let ugoira_saver_stream = ugoira_encoder_stream;
    let ugoira_saver_stream = Box::pin(ugoira_saver_stream.map(move |(i, b)| {
        let out_path = out_path.clone();
        let params: Box<dyn SaveParams + Send> = match format {
            Format::Webm => Box::new(crate::save::WebmParams),
            Format::Gif => Box::new(crate::save::GifParams),
        };
        (i, b, out_path, params)
    }));
    let ugoira_encoder = StreamUgoiraSaver::new(ugoira_saver_stream);
    let ugoira_saver_stream = ugoira_encoder.into_stream();
    let ugoira_saver_stream = Box::pin(ugoira_saver_stream.filter_map(|(i, r)| async move {
        match r {
            Ok(o) => {
                println!("Finished processing illustration {i}");
                Some((i, o))
            }
            Err(e) => {
                eprintln!("Error while saving illustration {i}: {e:?}");
                None
            }
        }
    }));

    // Run the pipeline until exhausted
    let mut pipeline = ugoira_saver_stream;
    while pipeline.next().await.is_some() {}

    Ok(())
}
