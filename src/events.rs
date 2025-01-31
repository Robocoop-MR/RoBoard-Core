use std::sync::mpsc::SyncSender;

#[derive(Debug)]
pub enum Event {
    GracefulShutdown,
    Message(u32), // TODO: use the flatbuffers definitions
}

/// Holds all the [`SyncSender`]s for sending messages to the running threads
#[derive(Debug, Clone)]
pub struct AddressBook {
    pub usb_sender_thread: SyncSender<Event>,
    pub nng_receiver_thread: SyncSender<Event>,
}
