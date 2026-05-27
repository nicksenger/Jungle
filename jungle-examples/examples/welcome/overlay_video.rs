use std::fs;
use std::io;
use std::path::PathBuf;

const AV_OVERLAY_FILE_NAME: &str = "mitb.mkv";
const AV_OVERLAY_BYTES: &[u8] = include_bytes!("assets/jungle.mkv");

pub fn ensure_av_overlay_media() -> Result<PathBuf, io::Error> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    let media_dir = PathBuf::from(home).join(".mitb");
    fs::create_dir_all(&media_dir)?;

    let media_path = media_dir.join(AV_OVERLAY_FILE_NAME);
    let should_write = match fs::metadata(&media_path) {
        Ok(metadata) => metadata.len() != AV_OVERLAY_BYTES.len() as u64,
        Err(err) if err.kind() == io::ErrorKind::NotFound => true,
        Err(err) => return Err(err),
    };

    if should_write {
        fs::write(&media_path, AV_OVERLAY_BYTES)?;
    }

    Ok(media_path)
}
