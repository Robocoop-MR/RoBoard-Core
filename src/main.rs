mod messages;
mod nng_reciever;
mod usb_sender;
mod events;

use std::{
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread,
};

use events::{AddressBook, Event};

use tracing::{error, level_filters::LevelFilter};

use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, Layer as _};

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

fn main() -> anyhow::Result<()> {
    let terminal_layer = tracing_subscriber::fmt::layer()
        .compact()
        // TODO: allow configuring the displayed log level with
        // either cli arguments or environment variables
        .with_filter(if cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        })
        .boxed();

    tracing_subscriber::registry().with(terminal_layer).init();

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
        thread::spawn(|| nng_reciever::main(ab, nng_receiver_thread))
    };
    let usb_sender_thread = {
        let ab = ab.clone();
        thread::spawn(|| usb_sender::main(ab, usb_sender_thread))
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
