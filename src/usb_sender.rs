use std::sync::mpsc::Receiver;

use tracing::{error, info};

use crate::{AddressBook, Event};

pub fn main(ab: AddressBook, receiver: Receiver<Event>) -> anyhow::Result<()> {
    // TODO: advertise a USB gadget
    // TODO: forward events (and intercept control events)
    loop {
        let event = receiver
            .recv()
            .inspect_err(|err| error!("Main internal channel closed. Got err: {err:?}"))?;

        match event {
            Event::GracefulShutdown => return Ok(()),
            Event::Message(msg) => info!("Got message: {msg}"),
        }
    }
}
