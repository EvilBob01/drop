use anyhow::{anyhow, Error};
use futures::stream::TryStreamExt;
use futures::{stream::FuturesUnordered, StreamExt};
use hex::ToHex as _;
use humansize::{format_size, BINARY};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    collections::HashMap,
    future::Future,
    ops::Not,
    path::Path,
    sync::{
        self,
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::io::AsyncWriteExt;
use tokio::{
    io::{AsyncReadExt as _, AsyncWrite},
    join,
    sync::Mutex,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub filename: String,
    pub start: usize,
    pub length: usize, // TODO: Replace with u64 for 32 bit clients
    pub permissions: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkData {
    pub files: Vec<FileEntry>,
    pub checksum: String,
    pub iv: [u8; 16],
}

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub chunks: HashMap<String, ChunkData>,
    pub size: u64,
    pub key: [u8; 16],
}

pub const CHUNK_SIZE: u64 = 1024 * 1024 * 64;
pub const MAX_FILE_COUNT: usize = 512;

use crate::versions::{
    create_backend_constructor,
    types::{VersionBackend, VersionFile},
};

pub async fn generate_manifest_rusty_v2<P, LogFn, ProgFn, FactoryFn, Writer, CloseFn>(
    dir: P,
    progress_sfn: ProgFn,
    log_sfn: LogFn,
    factory: FactoryFn,
    closer: CloseFn,
) -> anyhow::Result<Manifest>
where
    P: AsRef<Path>,
    LogFn: Fn(String),
    ProgFn: Fn(f32),
    Writer: AsyncWrite + Unpin,
    FactoryFn: AsyncFn(String) -> Writer,
    CloseFn: AsyncFn(Writer),
{
    let mut backend = create_backend_constructor(dir).ok_or(anyhow!(
        "Could not create backend for path. Is this structure supported?"
    ))?()?;
    let mut files = backend.list_files().await?;
    files.sort_by(|a, b| b.size.cmp(&a.size));

    log_sfn("organising files into chunks...".to_string());

    let chunks = organise_files(files, backend.require_whole_files());

    log_sfn(format!(
        "organized into {} chunks, generating checksums...",
        chunks.len()
    ));
    let manifest =
        read_chunks_and_generate_manifest(backend, chunks, progress_sfn, log_sfn, factory, closer)
            .await?;

    let mut key = [0u8; 16];
    getrandom::fill(&mut key).map_err(|err| anyhow!("failed to generate key: {:?}", err))?;

    let total_manifest_length = manifest
        .iter()
        .map(|(_, value)| value.files.iter().map(|f| f.length as u64).sum::<u64>())
        .sum::<u64>();

    Ok(Manifest {
        version: "2".to_string(),
        chunks: manifest,
        size: total_manifest_length,
        key,
    })
}

fn organise_files(
    files: Vec<VersionFile>,
    require_whole_files: bool,
) -> Vec<Vec<(VersionFile, u64, u64)>> {
    let mut chunks = Vec::new();
    let mut current_chunk = Vec::new();

    for version_file in files {
        if current_chunk.len() >= MAX_FILE_COUNT {
            // Pop current chunk
            chunks.push(std::mem::take(&mut current_chunk));
        }
        let current_chunk_size = current_chunk
            .iter()
            .map(|(_, _, length)| length)
            .sum::<u64>();
        let version_file_size = version_file.size;

        if require_whole_files {
            // If the current chunk is larger than chunk size, there's no point adding
            // it to the current_chunk. Just push it by itself
            if version_file_size >= CHUNK_SIZE {
                chunks.push(vec![(version_file, 0, version_file_size)]);
                continue;
            }

            current_chunk.push((version_file, 0, version_file_size));
            if current_chunk_size + version_file_size >= CHUNK_SIZE {
                // Pop current chunk
                chunks.push(std::mem::take(&mut current_chunk));
            }
        } else {
            // Enough space for it to be put in immediately
            if version_file_size + current_chunk_size < CHUNK_SIZE {
                current_chunk.push((version_file, 0, version_file_size));
                continue;
            }

            let remaining = CHUNK_SIZE - current_chunk_size;
            current_chunk.push((version_file.clone(), 0, remaining));

            // Loop over remaining data and create sufficient chunks to use it
            let mut offset = remaining;
            while offset < version_file_size {
                let length = CHUNK_SIZE.min(version_file_size - offset);
                if length == CHUNK_SIZE {
                    chunks.push(vec![(version_file.clone(), offset, length)]);
                } else {
                    current_chunk.push((version_file.clone(), offset, length));
                }
                offset += length;
            }
        }
    }
    if current_chunk.is_empty().not() {
        chunks.push(current_chunk);
    }
    chunks
}

async fn read_chunks_and_generate_manifest<LogFn, ProgFn, FactoryFn, Writer, CloseFn>(
    backend: Box<dyn VersionBackend + Send + Sync>,
    chunks: Vec<Vec<(VersionFile, u64, u64)>>,
    progress_sfn: ProgFn,
    log_sfn: LogFn,
    factory: FactoryFn,
    closer: CloseFn,
) -> anyhow::Result<HashMap<String, ChunkData>>
where
    LogFn: Fn(String),
    ProgFn: Fn(f32),
    Writer: AsyncWrite + Unpin,
    FactoryFn: AsyncFn(String) -> Writer,
    CloseFn: AsyncFn(Writer),
{
    let backend = Arc::new(sync::nonpoison::Mutex::new(backend));

    let futures = chunks.into_iter().enumerate().map(|(index, chunk)| async {
        let backend = backend.clone();

        let mut read_buf = vec![0; 1024 * 1024 * 64];

        let uuid = uuid::Uuid::new_v4().to_string();
        let mut hasher = Sha256::new();

        let mut iv = [0u8; 16];
        getrandom::fill(&mut iv).map_err(|err| anyhow!("failed to generate IV: {:?}", err))?;
        let mut chunk_data = ChunkData {
            files: Vec::new(),
            checksum: String::new(),
            iv,
        };
        let mut writer = (factory)(uuid.clone()).await;
        for (file, start, length) in chunk {
            chunk_data.files.push(
                read_and_generate_chunk_file_data(
                    backend.clone(),
                    &file,
                    start,
                    length,
                    &mut hasher,
                    &mut read_buf,
                    &mut writer,
                )
                .await?,
            );
        }
        (closer)(writer).await;

        let hash: String = hasher.finalize().encode_hex();
        chunk_data.checksum = hash;

        Ok((uuid, chunk_data))
    });
    let stream = futures::stream::iter(futures).buffer_unordered(4);
    let results = stream.try_collect().await;
    results
}
async fn read_and_generate_chunk_file_data<Writer>(
    backend: Arc<sync::nonpoison::Mutex<Box<dyn VersionBackend + Sync + Send>>>,
    file: &VersionFile,
    start: u64,
    length: u64,
    hasher: &mut Sha256,
    read_buf: &mut [u8],
    writer: &mut Writer,
) -> anyhow::Result<FileEntry>
where
    Writer: AsyncWrite + Unpin,
{
    let mut reader = {
        let mut backend_lock = backend.lock();
        let reader = backend_lock.reader(&file, start, start + length).await?;
        reader
    };

    loop {
        let amount = reader.read(read_buf).await?;

        if amount == 0 {
            break;
        }
        writer.write_all(&read_buf[0..amount]).await?;
        hasher.update(&read_buf[0..amount]);
    }

    Ok(FileEntry {
        filename: file.relative_filename.clone(),
        start: start.try_into().unwrap(),
        length: length.try_into().unwrap(),
        permissions: file.permission,
    })
}

pub async fn generate_manifest_rusty<T: Fn(String), V: Fn(f32)>(
    dir: &Path,
    progress_sfn: V,
    log_sfn: T,
) -> anyhow::Result<Manifest> {
    let mut backend =
        create_backend_constructor(dir).ok_or(anyhow!("Could not create backend for path."))?()?;

    let required_single_file = backend.require_whole_files();

    let mut files = backend.list_files().await?;
    files.sort_by(|a, b| b.size.cmp(&a.size));
    // Filepath to chunk data
    let mut chunks: Vec<Vec<(VersionFile, u64, u64)>> = Vec::new();
    let mut current_chunk: Vec<(VersionFile, u64, u64)> = Vec::new();

    log_sfn("organizing files into chunks...".to_string());

    if required_single_file {
        for version_file in files {
            if version_file.size >= CHUNK_SIZE {
                let size = version_file.size;
                chunks.push(vec![(version_file, 0, size)]);

                continue;
            }

            let mut current_size = current_chunk.iter().map(|v| v.2).sum::<u64>();

            let size = version_file.size;
            current_chunk.push((version_file, 0, size));

            current_size += size;

            if current_size >= CHUNK_SIZE {
                // Pop current and add, then reset
                let new_chunk = std::mem::take(&mut current_chunk);
                chunks.push(new_chunk);
            }

            if current_chunk.len() >= MAX_FILE_COUNT {
                chunks.push(std::mem::take(&mut current_chunk));
            }

            continue;
        }
    } else {
        for version_file in files {
            if current_chunk.len() >= MAX_FILE_COUNT {
                chunks.push(std::mem::take(&mut current_chunk));
            }

            let current_size = current_chunk.iter().map(|v| v.2).sum::<u64>();

            if version_file.size + current_size < CHUNK_SIZE {
                let size = version_file.size;
                current_chunk.push((version_file, 0, size));

                continue;
            }

            // Fill up current chunk
            let remaining = CHUNK_SIZE - current_size;
            current_chunk.push((version_file.clone(), 0, remaining));
            chunks.push(std::mem::replace(&mut current_chunk, Vec::new()));

            // This is our offset in our current file
            let mut offset = remaining;
            while offset < version_file.size {
                let length = CHUNK_SIZE.min(version_file.size - offset);
                if length == CHUNK_SIZE {
                    chunks.push(vec![(version_file.clone(), offset, length)]);
                } else {
                    current_chunk.push((version_file.clone(), offset, length));
                }
                offset += length;
            }
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    log_sfn(format!(
        "organized into {} chunks, generating checksums...",
        chunks.len()
    ));

    let manifest: Arc<Mutex<HashMap<String, ChunkData>>> = Arc::new(Mutex::new(HashMap::new()));
    let total_manifest_length = Arc::new(AtomicU64::new(0));

    let backend = Arc::new(Mutex::new(backend));

    let futures: FuturesUnordered<impl Future<Output = Result<(), Error>>> =
        FuturesUnordered::new();
    let (send_log, mut recieve_log) = tokio::sync::mpsc::channel(16);
    let chunks_length = chunks.len();
    for (index, chunk) in chunks.into_iter().enumerate() {
        let send_log = send_log.clone();
        let backend = backend.clone();
        let total_manifest_length = total_manifest_length.clone();
        let manifest = manifest.clone();
        futures.push(async move {
            let mut read_buf = vec![0; 1024 * 1024 * 64];

            let uuid = uuid::Uuid::new_v4().to_string();
            let mut hasher = Sha256::new();

            let mut iv = [0u8; 16];
            getrandom::fill(&mut iv).map_err(|err| anyhow!("failed to generate IV: {:?}", err))?;
            let mut chunk_data = ChunkData {
                files: Vec::new(),
                checksum: String::new(),
                iv,
            };

            let mut chunk_length = 0;

            for (file, start, length) in chunk {
                let mut reader = {
                    let mut backend_lock = backend.lock().await;
                    let reader = backend_lock.reader(&file, start, start + length).await?;
                    reader
                };

                loop {
                    let amount = reader.read(&mut read_buf).await?;
                    if amount == 0 {
                        break;
                    }
                    hasher.update(&read_buf[0..amount]);
                }

                chunk_length += length;

                chunk_data.files.push(FileEntry {
                    filename: file.relative_filename,
                    start: start.try_into().unwrap(),
                    length: length.try_into().unwrap(),
                    permissions: file.permission,
                });
            }

            send_log
                .send(format!(
                    "created chunk of size {} ({}b) from {} files (index {})",
                    format_size(chunk_length, BINARY),
                    chunk_length,
                    chunk_data.files.len(),
                    index
                ))
                .await?;

            total_manifest_length.fetch_add(chunk_length, Ordering::Relaxed);

            let hash: String = hasher.finalize().encode_hex();
            chunk_data.checksum = hash;
            {
                let mut manifest_lock = manifest.lock().await;
                manifest_lock.insert(uuid, chunk_data);
            };

            Ok(())
        });
    }
    drop(send_log);
    join!(
        async move {
            let mut current_progress = 0f32;
            let total_progress = chunks_length as f32;
            while let Some(message) = recieve_log.recv().await {
                log_sfn(message);
                current_progress += 1.0f32;
                progress_sfn((current_progress / total_progress) * 100.0f32)
            }
        },
        futures.collect::<Vec<Result<(), Error>>>()
    );

    let manifest = manifest.lock().await;
    let manifest = manifest.clone();

    let mut key = [0u8; 16];
    getrandom::fill(&mut key).map_err(|err| anyhow!("failed to generate key: {:?}", err))?;

    Ok(Manifest {
        version: "2".to_string(),
        chunks: manifest,
        size: total_manifest_length.fetch_add(0, Ordering::Relaxed),
        key,
    })
}
