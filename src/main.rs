mod messages;

use std::{
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread,
    time::Duration,
};

use tracing::{error, info};
fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // --- Setting up inter thread communication

    let (
        Receivers {
            usb_sender_thread,
            nng_receiver_thread,
        },
        ab,
    ) = create_channels();

    // --- Starting threads

    let nng_receiver_thread = {
        let ab = ab.clone();
        thread::spawn(|| recieve_from_nng(ab, nng_receiver_thread))
    };
    let usb_sender_thread = {
        let ab = ab.clone();
        thread::spawn(|| send_to_usb(ab, usb_sender_thread))
    };
    // Do we need a second thread for receiving usb events if we even can have one ?
    // (maybe the USB gadget context can't be shared between two threads. But in that case,
    // we can probably use message queues like we're already doing)

    // --- Waiting for the process to close

    // TODO: handle threads unexpectedly closing
    // (close the rest of the threads or try to restart)

    if let Err(err) = nng_receiver_thread.join() {
        error!("NNG receiver thread closed: {err:?}");
    }
    if let Err(err) = usb_sender_thread.join() {
        error!("NNG receiver thread closed: {err:?}");
    }

    Ok(())
}

#[derive(Debug)]
enum Event {
    GracefulShutdown,
    Message(u32), // TODO: use the flatbuffers definitions
}

/// Holds all the [`SyncSender`]s for sending messages to the running threads
#[derive(Debug, Clone)]
struct AddressBook {
    usb_sender_thread: SyncSender<Event>,
    nng_receiver_thread: SyncSender<Event>,
}

struct Receivers {
    usb_sender_thread: Receiver<Event>,
    nng_receiver_thread: Receiver<Event>,
}

fn create_channels() -> (Receivers, AddressBook) {
    let (usb_sender_thread_sender, usb_sender_thread_receiver) = sync_channel(128);
    let (nng_receiver_thread_sender, nng_receiver_thread_receiver) = sync_channel(128);

    (
        Receivers {
            usb_sender_thread: usb_sender_thread_receiver,
            nng_receiver_thread: nng_receiver_thread_receiver,
        },
        AddressBook {
            usb_sender_thread: usb_sender_thread_sender,
            nng_receiver_thread: nng_receiver_thread_sender,
        },
    )
}

fn send_to_usb(ab: AddressBook, receiver: Receiver<Event>) -> anyhow::Result<()> {
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

fn recieve_from_nng(ab: AddressBook, receiver: Receiver<Event>) -> anyhow::Result<()> {
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
