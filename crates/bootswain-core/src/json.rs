pub fn read_json_file<T>(path: impl AsRef<Path>) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| anyhow::anyhow!("failed to parse {}: {error}", path.display()))
}

pub fn write_pretty_json_file<T>(path: impl AsRef<Path>, value: &T) -> anyhow::Result<()>
where
    T: Serialize,
{
    let path = path.as_ref();
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| anyhow::anyhow!("failed to encode JSON: {error}"))?;
    fs::write(path, json)
        .map_err(|error| anyhow::anyhow!("failed to write {}: {error}", path.display()))
}
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::Path;
