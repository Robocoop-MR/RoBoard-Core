use std::{
    collections::HashSet,
    os::linux::fs::MetadataExt as _,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, PoisonError},
};

use tokio::net::UnixDatagram;
use tracing::{error, info, trace};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SocketRegistryEntry {
    dev: u64,
    ino: u64,
}

static SOCKET_REGISTRY: LazyLock<Mutex<HashSet<SocketRegistryEntry>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// A socket managed by us which has the following guarantees:
///
/// # Duplicate prevention
/// We keep track of all the open [RoboSocket]s in this binary and refuse to open one if it is at
/// the same path as one that is still open.
///
/// # Automatic unlinking
/// Is automatically unlinked when [dropped](Drop) (Including when the program unwinds). Though,
/// that implementation had to be made blocking to ensure correctness. When destroying this socket
/// in a normal flow of operations, call [unlink](RoboSocket::unlink) to drop and unlink in a non blocking manner.
#[derive(Debug)]
pub struct RoboSocket {
    inner: UnixDatagram,
    path: PathBuf,
    reg_entry: SocketRegistryEntry,
    /// Used when dropping to ensure we don't try unlinking twice.
    is_unlinked: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum RoboSocketNewError {
    #[error("a socket to is already open in this executable")]
    AlreadyOpen,
    #[error("IO Error: {message} ({source})")]
    IO {
        source: std::io::Error,
        message: &'static str,
    },
    #[error("the lock to the internal socket registry has been poisoned")]
    Poisoned,
}

impl From<std::io::Error> for RoboSocketNewError {
    fn from(source: std::io::Error) -> Self {
        Self::IO {
            source,
            message: "",
        }
    }
}

impl<T> From<PoisonError<T>> for RoboSocketNewError {
    fn from(_: PoisonError<T>) -> Self {
        Self::Poisoned
    }
}

/// Gives you the [RoboSocket] back unchanged in case an error happens while
/// [unlinking](RoboSocket::unlink)
#[derive(thiserror::Error, Debug)]
#[error("couldn't unlink socket at {:?}", .socket.path)]
pub struct RoboSocketUnlinkError {
    socket: RoboSocket,
    source: std::io::Error,
}

impl RoboSocket {
    pub async fn new(path: &(impl AsRef<Path> + ?Sized)) -> Result<Self, RoboSocketNewError> {
        let path = path.as_ref().to_owned();

        let get_metadata = async || -> Result<SocketRegistryEntry, RoboSocketNewError> {
            let stats = tokio::fs::metadata(&path).await?;
            Ok(SocketRegistryEntry {
                dev: stats.st_dev(),
                ino: stats.st_ino(),
            })
        };

        let register_socket = async || -> Result<SocketRegistryEntry, RoboSocketNewError> {
            trace!("registering socket {path:?}");

            let entry = get_metadata().await?;

            let is_new = SOCKET_REGISTRY.lock()?.insert(entry.clone());

            if !is_new {
                error!("socket at {path:?} is still in the registry while absent from disk");
            }

            Ok(entry)
        };

        // Try binding the socket once
        // Only keep going if we couldn't bind the socket because it already exists
        match UnixDatagram::bind(&path) {
            Ok(inner) => {
                let reg_entry = register_socket().await?;

                return Ok(Self {
                    inner,
                    path,
                    reg_entry,
                    is_unlinked: false,
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => (), // Keep going
            Err(source) => {
                return Err(RoboSocketNewError::IO {
                    source,
                    message: "faild to bind socket",
                });
            }
        };

        // Is correct to call since we know the file exists
        let metadata = get_metadata().await?;

        // Check in OPEN_SOCKETS to see if we own the socket
        if SOCKET_REGISTRY.lock()?.contains(&metadata) {
            return Err(RoboSocketNewError::AlreadyOpen);
        }

        // If not, we assume it is leftover from the previous run and unlink it
        if let Err(source) = std::fs::remove_file(&path) {
            return Err(RoboSocketNewError::IO {
                source,
                message: "couldn't unlink previous socket",
            });
        };

        let inner = match UnixDatagram::bind(&path) {
            Ok(socket) => socket,
            Err(source) => {
                return Err(RoboSocketNewError::IO {
                    source,
                    message: "failed to bind socket",
                });
            }
        };

        let reg_entry = register_socket().await?;
        let socket = RoboSocket {
            inner,
            path,
            reg_entry,
            is_unlinked: false,
        };

        Ok(socket)
    }

    /// Unlinks gracefully without blocking.
    ///
    /// Exists only because we can't safely implement [Drop] to be non blocking.
    ///
    /// It is totally safe to just drop a [RoboSocket] however, though it incur more overhead.
    pub async fn unlink(mut self) -> Result<(), RoboSocketUnlinkError> {
        trace!("unlinking {:?} cooperatively", self.path);

        if let Err(source) = tokio::fs::remove_file(&self.path).await {
            Err(RoboSocketUnlinkError {
                socket: self,
                source,
            })
        } else {
            self.is_unlinked = true;
            drop(self);
            Ok(())
        }
    }
}

impl std::ops::Deref for RoboSocket {
    type Target = UnixDatagram;

    fn deref(&self) -> &UnixDatagram {
        &self.inner
    }
}

impl Drop for RoboSocket {
    fn drop(&mut self) {
        if let Ok(mut registry) = SOCKET_REGISTRY.lock() {
            let was_present = registry.remove(&self.reg_entry);
            if !was_present {
                error!(
                    path = ?self.path,
                    "socket shouldn't have been removed from the registry before calling drop"
                );
            }
        } else {
            error!(
                path = ?self.path,
                "failed to remove socket from the registry: Lock has been poisoned",
            );
        };

        if !self.is_unlinked {
            trace!("unlinking {:?} blockingly", self.path);
            // It is suboptimal that this blocks...
            // But we can't tokio::spawn a task since if that were to ever run outside of a tokio
            // runtime it would panic. And regardless, if we did manage to spawn a task, if drop was
            // called because the runtime is stopping then it would never even run.
            //
            // The unlink method exists as a solution to this issue.
            if let Err(err) = std::fs::remove_file(&self.path) {
                error!(
                    "failed to unlink socket at path {:?}. Got IO Error: {err}",
                    self.path
                );
            }
        }
    }
}

pub async fn testing() -> anyhow::Result<()> {
    let socket = RoboSocket::new("./test.sock").await?;

    assert!({
        let other_socket = RoboSocket::new("./src/../test.sock").await;
        other_socket.is_err()
    });

    let mut buf = [0; 100];
    let (size, sender) = socket.recv_from(&mut buf).await?;

    info!(
        "recieved {size} bytes from {sender:?}: {}",
        String::from_utf8_lossy(&buf)
    );

    std::fs::remove_file("test").or_else(|err| match err.kind() {
        std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(err),
    })?;

    socket.unlink().await?;

    Ok(())
}
