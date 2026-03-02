use std::{env, os::unix::fs::MetadataExt, path::PathBuf};

use droplet_rs::manifest::{ManifestWriterFactory, generate_manifest_rusty};
use tokio::runtime::Handle;

struct SinkFactory {}
#[async_trait::async_trait]
impl ManifestWriterFactory for SinkFactory {
    type Writer = tokio::io::Sink;
    async fn create(&self, _id: String) -> anyhow::Result<Self::Writer> {
        Ok(tokio::io::sink())
    }

    async fn close(&self, _writer: Self::Writer) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::main]
pub async fn main() {
    let mut args = env::args();
    let target_dir = PathBuf::from(args.nth(1).expect("Provide target directory"));

    let metrics = Handle::current().metrics();
    println!("using {} workers", metrics.num_workers());

    let manifest = generate_manifest_rusty(
        &target_dir,
        |progress| println!("PROGRESS: {}", progress),
        |message| {
            println!("{}", message);
        },
        Some(&SinkFactory {}),
        None,
    )
    .await
    .unwrap();


    return;

    // Sanity checks
    for (_, chunk_data) in manifest.chunks {
        for file in chunk_data.files {
            let path = target_dir.join(file.filename);
            if !path.exists() {
                panic!("{} doesn't exist", path.display());
            }

            let metadata = path.metadata().expect("failed to fetch metadata");
            let file_size = metadata.size();
            if file.start > file_size as usize {
                panic!(
                    "start for {} doesn't make sense: start: {}, size: {}",
                    path.display(),
                    file.start,
                    file_size
                );
            }

            let end_position = file.start + file.length;
            if end_position > file_size as usize {
                panic!(
                    "end for {} doesn't make sense: end: {}, size: {}",
                    path.display(),
                    end_position,
                    file_size
                );
            }
        }
    }
}
