use libc::c_char;
use simple_error::bail;
use std::env;
use std::ffi::CStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

use crate::result::Result;

fn access<P: AsRef<Path>>(path: &P, amode: libc::c_int) -> Result<()> {
    let mut buf = [0u8; libc::PATH_MAX as usize];
    let path = path.as_ref().as_os_str().as_bytes();
    if path.len() >= libc::PATH_MAX as usize {
        bail!("invalid argument");
    }

    // TODO: Replace with bytes::copy_memory. rust-lang/rust#24028
    let cstr = unsafe {
        ptr::copy_nonoverlapping(path.as_ptr(), buf.as_mut_ptr(), path.len());
        CStr::from_ptr(buf.as_ptr() as *const c_char)
    };

    let res = unsafe { libc::access(cstr.as_ptr(), amode) };
    if res < 0 {
        bail!("access failed: {}", res)
    }
    Ok(())
}

pub fn which<P>(exe_name: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(&exe_name);
                let res = access(&full_path, libc::X_OK);
                if res.is_ok() {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}
