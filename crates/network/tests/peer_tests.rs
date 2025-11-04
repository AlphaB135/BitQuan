use std::io::{Cursor, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use bitquan_network::peer::{handshake, read_frame, HANDSHAKE_TIMEOUT_MS, MAX_MSG_BYTES};
use bitquan_types::error::Error;

#[test]
fn oversized_message_rejected_without_panic() {
    let len = (MAX_MSG_BYTES + 1) as u32;
    let mut data = Vec::with_capacity(4 + len as usize);
    data.extend_from_slice(&len.to_le_bytes());
    data.resize(4 + len as usize, 0xAA);
    let mut cursor = Cursor::new(data);
    let result = read_frame(&mut cursor);
    assert!(matches!(result, Err(Error::Invalid(msg)) if msg == "message too large"));
}

#[test]
fn empty_message_rejected() {
    let mut cursor = Cursor::new(0u32.to_le_bytes().to_vec());
    let result = read_frame(&mut cursor);
    assert!(matches!(result, Err(Error::Invalid(msg)) if msg == "empty frame"));
}

#[test]
fn handshake_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        // Delay longer than handshake timeout and never send expected byte.
        thread::sleep(Duration::from_millis(HANDSHAKE_TIMEOUT_MS + 200));
        let _ = stream.write(&[]);
    });

    let mut client = TcpStream::connect(addr).expect("connect");
    let result = handshake(&mut client);
    assert!(matches!(result, Err(Error::Timeout(msg)) if msg == "handshake timeout"));

    server.join().expect("join server thread");
}
