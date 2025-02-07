use std::{ffi::CStr, sync::mpsc::Receiver, thread, time::Duration};

use anyhow::Context as _;
use tracing::error;

use nng_sus::*;
use nng_sys as nng_sus;

use crate::{AddressBook, Event};

struct Work {
    ctx: nng_ctx,
    aio: nng_aio,
}

pub struct NNGAioHandle {
    ctx: nng_ctx, // TODO: Should be a pointer ?
    aio: nng_aio,
}

impl Drop for NNGAioHandle {
    fn drop(&mut self) {
        // TODO: cancel the current operations
    }
}

fn nng_wrap_err(rv: ::std::os::raw::c_int) -> anyhow::Result<()> {
    if rv != 0 {
        Err(anyhow::anyhow!("nng_rep0_open failed")).with_context(|| {
            // Safety:
            // Strings returned from nng_strerror:
            //   - are properly null-terminated
            //   - are under 32 bytes large so they fit within `isize::MAX`
            //   - will not have their memory mutated while they're in use
            //     by our code since we then immediately clone them into
            //     memory we control.
            unsafe { CStr::from_ptr(nng_strerror(rv)) }
                .to_string_lossy()
                .into_owned()
        })
    } else {
        Ok(())
    }
}

pub fn start(ab: AddressBook, reciever: Receiver<Event>) -> anyhow::Result<()> {
    let mut socket = nng_socket::NNG_SOCKET_INITIALIZER;
    nng_wrap_err(
        // Safety:
        // - We don't end up using the `socket` variable if the function fails
        //   se we can't be using invalid memory.
        unsafe { nng_rep0_open(&mut socket as *mut nng_socket) },
    )?;

    Ok(())
}
