use alaydriem_stratum::errors::PacketReadError;
use alaydriem_stratum::game::{GamePacket, PacketId, PacketSender};

use crate::support::FakePacket;
use crate::support::fake_packet::{DELETE_CALLS, DESTROY_CALLS, READ_OK, SEND_CALLS, SENT, STREAM_DTOR_CALLS};

#[test]
#[allow(unsafe_code)]
fn the_sender_of_a_client_sends_the_packet_it_is_given() {
    let calls = FakePacket::calls();
    // SAFETY: the fake client's vtable holds getPacketSender at the calls' slot
    let sender = unsafe { PacketSender::of_client(FakePacket::client(), &calls) }.expect("the fake client has one");
    let packet = GamePacket::create(&calls, PacketId::ITEM_STACK_REQUEST).unwrap();

    sender.send(&packet);

    assert_eq!(FakePacket::count(&SEND_CALLS), 1);
    assert_eq!(*SENT.lock().unwrap(), packet.as_ptr() as usize);
}

#[test]
#[allow(unsafe_code)]
fn request_bytes_that_are_read_are_sent_once_and_released_once() {
    let calls = FakePacket::calls();
    // SAFETY: as above
    let sender = unsafe { PacketSender::of_client(FakePacket::client(), &calls) }.unwrap();

    let mut before = 0;

    let sent = sender.send_bytes(&calls, PacketId::ITEM_STACK_REQUEST, &[1, 2, 3], |_| before += 1);

    assert_eq!(sent, Ok(()));
    assert_eq!(before, 1);
    assert_eq!(FakePacket::count(&SEND_CALLS), 1);
    assert_eq!(FakePacket::count(&STREAM_DTOR_CALLS), 1);
    assert_eq!(FakePacket::count(&DESTROY_CALLS), 1);
    assert_eq!(FakePacket::count(&DELETE_CALLS), 1);
}

#[test]
#[allow(unsafe_code)]
fn request_bytes_the_packet_does_not_read_are_not_sent_and_released_once() {
    READ_OK.store(false, std::sync::atomic::Ordering::SeqCst);
    let calls = FakePacket::calls();
    // SAFETY: as above
    let sender = unsafe { PacketSender::of_client(FakePacket::client(), &calls) }.unwrap();

    let mut before = 0;

    let sent = sender.send_bytes(&calls, PacketId::ITEM_STACK_REQUEST, &[1, 2, 3], |_| before += 1);

    assert_eq!(sent, Err(PacketReadError::Refused { read_to: 0, length: 3 }));
    // no inventory open and no send
    assert_eq!(before, 0);
    assert_eq!(FakePacket::count(&SEND_CALLS), 0);
    assert_eq!(FakePacket::count(&STREAM_DTOR_CALLS), 1);
    assert_eq!(FakePacket::count(&DESTROY_CALLS), 1);
    assert_eq!(FakePacket::count(&DELETE_CALLS), 1);
}
