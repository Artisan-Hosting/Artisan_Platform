use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

use nix::unistd::{chown, Gid, Uid};
use dusa_collection_utils::{errors::{ErrorArray, ErrorArrayItem, Errors as SE, OkWarning, UnifiedResult as uf, WarningArray, WarningArrayItem, Warnings}, functions::del_file, types::PathType};


/// Returns the path to the socket.
///
/// # Arguments
/// * `int` - A boolean indicating if initialization is needed.
/// * `errors` - An array of errors to be populated if any occur.
/// * `warnings` - An array of warnings to be populated if any occur.
///
/// # Returns
/// A unified result containing the path type or errors/warnings.
pub fn get_socket_path (
    int: bool,
    mut errors: ErrorArray,
    mut warnings: WarningArray,
) -> uf<OkWarning<PathType>> {
    let socket_file = PathType::Content(String::from("/var/run/ais.sock"));
    // let socket_file = PathType::Content(String::from("/home/dwhitfield/Developer/RUST/Dev/server/s.socket"));
    let _socket_dir = match socket_file.ancestors().next() {
        Some(d) => PathType::PathBuf(d.to_path_buf()),
        None => {
            errors.push(ErrorArrayItem::new(
                SE::InvalidFile,
                "Socket file not found".to_string(),
            ));
            return uf::new(Err(errors));
        }
    };

    if int {
        // Create the dir and the sock file
        if socket_file.exists() {
            match del_file(socket_file.clone(), errors.clone(), warnings.clone()).uf_unwrap() {
                Ok(_) => {
                    return uf::new(Ok(OkWarning {
                        data: socket_file,
                        warning: warnings,
                    }));
                }
                Err(_) => {
                    warnings.push(WarningArrayItem::new(Warnings::OutdatedVersion));
                }
            }
        }
    }

    uf::new(Ok(OkWarning {
        data: socket_file,
        warning: warnings,
    }))
}

pub fn set_socket_ownership(path: &PathBuf, uid: Uid, gid: Gid) -> Result<(), ErrorArrayItem> {
    if let Err(err) = chown(path, Some(uid), Some(gid)) {
        return Err(ErrorArrayItem::from(err))
    };

    Ok(())
}

pub fn set_socket_permission(socket_path: PathType) -> Result<(), ErrorArrayItem>{
    // Changing the permissions the socket
    let socket_metadata = match fs::metadata(socket_path.clone()) {
        Ok(d) => d,
        Err(e) => {
            return Err(ErrorArrayItem::from(e))
        }
    };

    let mut permissions = socket_metadata.permissions();
    permissions.set_mode(0o660); // Set desired permissions

    if let Err(err) = fs::set_permissions(socket_path.clone(), permissions) {
        return Err(ErrorArrayItem::from(err))
    }

    Ok(())
}