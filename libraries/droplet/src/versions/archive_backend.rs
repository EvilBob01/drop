use std::{path::PathBuf, task::Poll};

use anyhow::anyhow;
use async_trait::async_trait;
use libarchive_drop::{
    archive::{Entry, FileType, ReadCompression, ReadFormat},
    reader::{Builder, FileReader, Reader},
};
use tokio::io::AsyncRead;

use crate::versions::types::{MinimumFileObject, VersionBackend, VersionFile};

pub struct ZipVersionBackend {
    path: PathBuf,
}
impl ZipVersionBackend {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self { path })
    }

    fn open_archive(&self) -> Result<FileReader, anyhow::Error> {
        let mut archive = Builder::new();
        archive.support_format(ReadFormat::All)?;
        archive.support_compression(ReadCompression::All)?;
        let archive = archive.open_file(&self.path)?;

        Ok(archive)
    }
}

struct ArchiveReader {
    archive: FileReader,
}

impl AsyncRead for ArchiveReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let block = match self.archive.read_block() {
            Ok(v) => v,
            Err(err) => return Poll::Ready(Err(std::io::Error::other(err.to_string()))),
        };

        let block = match block {
            Some(v) => v,
            None => return Poll::Ready(Ok(())),
        };

        buf.put_slice(block);
        return Poll::Ready(Ok(()));
    }
}

#[async_trait]
impl VersionBackend for ZipVersionBackend {
    async fn list_files(&self) -> anyhow::Result<Vec<VersionFile>> {
        let mut archive = self.open_archive()?;
        let mut results = Vec::new();

        while let Some(header) = archive.next_header() {
            match header.filetype() {
                FileType::RegularFile => (),
                _ => {
                    continue;
                }
            }
            results.push(VersionFile {
                relative_filename: header.pathname().to_string(),
                permission: 0o744,
                size: header.size().try_into()?,
            });
        }

        Ok(results)
    }

    async fn reader(
        &self,
        file: &VersionFile,
        _start: u64,
        _end: u64,
    ) -> anyhow::Result<Box<dyn MinimumFileObject>> {
        let mut archive = self.open_archive()?;

        // Find entry in archive
        loop {
            let entry = match archive.next_header() {
                Some(v) => v,
                None => return Err(anyhow!("entry not found:{}", file.relative_filename)),
            };
            if entry.pathname() == file.relative_filename {
                break;
            }
        }

        Ok(Box::new(ArchiveReader { archive }))
    }

    async fn peek_file(&self, sub_path: String) -> anyhow::Result<VersionFile> {
        let files = self.list_files().await?;
        let file = files
            .iter()
            .find(|v| v.relative_filename == sub_path)
            .expect("file not found");

        Ok(file.clone())
    }

    fn require_whole_files(&self) -> bool {
        true
    }
}
