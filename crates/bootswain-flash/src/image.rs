use anyhow::{Context, Result, bail};
use bootswain_core::{CompressionKind, ImageInfo};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
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

pub fn sha256_file(path: &Path) -> Result<String> {
    let file =
        File::open(path).with_context(|| format!("failed to open image {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::{detect_compression, inspect_image};
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
