use std::os::unix::net::UnixDatagram;

fn main() -> anyhow::Result<()> {
    let socket = UnixDatagram::unbound()?;

    println!("{:?}", socket.local_addr()?);

    socket.send_to(b"omelette au fromage", "test.sock")?;

    Ok(())
}
