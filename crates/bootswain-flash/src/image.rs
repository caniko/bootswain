use anyhow::{Context, Result, bail};
use bootswain_core::{CompressionKind, ImageInfo};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub fn inspect_image(path: &Path) -> Result<ImageInfo> {
    let compression = detect_compression(path)?;
    let size_bytes = path
        .metadata()
        .with_context(|| format!("failed to read metadata for {}", path.display()))?
        .len();
    let sha256 = sha256_file(path)?;

    Ok(ImageInfo {
        path: path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
        compression,
        size_bytes,
        sha256,
    })
}

pub fn detect_compression(path: &Path) -> Result<CompressionKind> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        bail!(
            "image path has no valid UTF-8 file name: {}",
            path.display()
        );
    };

    if name.ends_with(".img.zst") {
        Ok(CompressionKind::Zstd)
    } else if name.ends_with(".img") {
        Ok(CompressionKind::None)
    } else {
        bail!(
            "unsupported image type for {}: expected .img or .img.zst",
            path.display()
        )
    }
}

/// sha2 0.11 dropped its `io::Write` impl; this keeps `io::copy` doing the buffering.
struct HashWriter<'a>(&'a mut Sha256);

impl io::Write for HashWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open image {}", path.display()))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut HashWriter(&mut hasher))
        .with_context(|| format!("failed to read {}", path.display()))?;

    Ok(hex::encode(hasher.finalize()))
}

/// Bytes a flash writes to the device: the image size, or for zstd the decoded size
/// recorded in the first frame header.
pub fn payload_size_bytes(image: &ImageInfo) -> Result<u64> {
    if image.compression == CompressionKind::None {
        return Ok(image.size_bytes);
    }

    // ZSTD_FRAMEHEADERSIZE_MAX is 18 bytes.
    let mut header = Vec::with_capacity(18);
    File::open(&image.path)
        .and_then(|file| file.take(18).read_to_end(&mut header))
        .with_context(|| format!("failed to read zstd header of {}", image.path.display()))?;
    match zstd::zstd_safe::get_frame_content_size(&header) {
        Ok(Some(size)) => Ok(size),
        Ok(None) => bail!(
            "{} does not record its decoded size, so device capacity cannot be checked; \
             recompress with a known content size (zstd does this for file input) or flash \
             through a manifest artifact that sets required_device_size_bytes",
            image.path.display()
        ),
        Err(error) => bail!(
            "{} has an unreadable zstd frame header: {error}",
            image.path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{detect_compression, inspect_image, payload_size_bytes};
    use bootswain_core::CompressionKind;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_img_and_img_zst() {
        let tmp = tempdir().expect("tempdir");
        let raw = tmp.path().join("sample.img");
        let zstd = tmp.path().join("sample.img.zst");

        fs::write(&raw, b"raw").expect("write raw");
        fs::write(&zstd, b"zst").expect("write zst");

        assert_eq!(
            detect_compression(&raw).expect("raw compression"),
            CompressionKind::None
        );
        assert_eq!(
            detect_compression(&zstd).expect("zstd compression"),
            CompressionKind::Zstd
        );
    }

    fn zstd_image(
        path: &std::path::Path,
        payload: &[u8],
        pledge_size: bool,
    ) -> bootswain_core::ImageInfo {
        let mut encoder = zstd::stream::Encoder::new(fs::File::create(path).expect("create"), 0)
            .expect("encoder");
        if pledge_size {
            encoder
                .set_pledged_src_size(Some(payload.len() as u64))
                .expect("pledge");
        }
        std::io::Write::write_all(&mut encoder, payload).expect("encode");
        encoder.finish().expect("finish");
        bootswain_core::ImageInfo {
            path: path.to_path_buf(),
            compression: CompressionKind::Zstd,
            size_bytes: fs::metadata(path).expect("metadata").len(),
            sha256: String::new(),
        }
    }

    #[test]
    fn zstd_payload_size_is_the_decoded_size() {
        let tmp = tempdir().expect("tempdir");
        let payload = vec![0_u8; 1 << 20];
        let image = zstd_image(&tmp.path().join("a.img.zst"), &payload, true);

        // Highly compressible: the file is tiny, the payload is 1 MiB.
        assert!(image.size_bytes < 1024);
        assert_eq!(payload_size_bytes(&image).expect("size"), 1 << 20);
    }

    #[test]
    fn zstd_without_content_size_is_an_error() {
        let tmp = tempdir().expect("tempdir");
        let image = zstd_image(&tmp.path().join("b.img.zst"), b"bootswain", false);

        let error = payload_size_bytes(&image).expect_err("unknown size");
        assert!(error.to_string().contains("does not record its decoded size"));
    }

    #[test]
    fn computes_sha256_and_size() {
        let tmp = tempdir().expect("tempdir");
        let image = tmp.path().join("artifact.img");
        fs::write(&image, b"bootswain").expect("write image");

        let info = inspect_image(&image).expect("inspect image");
        assert_eq!(info.size_bytes, 9);
        assert_eq!(
            info.sha256,
            "0c8bb5af83b12eb17b5bcdad45b7ea18fa2653749afa92c67a33fdd4ede619c1"
        );
    }
}
