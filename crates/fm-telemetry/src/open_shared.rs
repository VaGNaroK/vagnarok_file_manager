use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

/// Abre arquivo para leitura compartilhada sem bloquear operações concorrentes no Windows.
pub fn open_shared_read(path: &Path) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_SHARE_READ (1) | FILE_SHARE_WRITE (2) | FILE_SHARE_DELETE (4)
        opts.share_mode(0x7);
    }
    opts.open(path)
}
