use libc::c_char;
use std::env;
use std::ffi::CStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

fn is_executable<P: AsRef<Path>>(path: &P) -> bool {
    let mut buf = [0u8; libc::PATH_MAX as usize];
    let path = path.as_ref().as_os_str().as_bytes();
    if path.len() >= libc::PATH_MAX as usize {
        return false;
    }

    let cstr = unsafe {
        ptr::copy_nonoverlapping(path.as_ptr(), buf.as_mut_ptr(), path.len());
        CStr::from_ptr(buf.as_ptr() as *const c_char)
    };

    let res = unsafe { libc::access(cstr.as_ptr(), libc::X_OK) };
    res == 0
}

pub(crate) fn which<P>(exe_name: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(&exe_name);
                if is_executable(&full_path) {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}
