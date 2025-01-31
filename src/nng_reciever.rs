use std::{sync::mpsc::Receiver, thread, time::Duration};

use tracing::error;

use crate::{AddressBook, Event};

pub fn main(ab: AddressBook, receiver: Receiver<Event>) -> anyhow::Result<()> {
    // TODO: read events coming in from other threads (especially the graceful shutdown)
    // TODO: Set up IPC
    for i in 0..=10 {
        if let Err(err) = ab.usb_sender_thread.send(Event::Message(i)) {
            error!("Could not send a message. Got err: {err:?}");
        }

        thread::sleep(Duration::from_millis(50));
    }

    ab.usb_sender_thread.send(Event::GracefulShutdown)?;

    Ok(())
}
