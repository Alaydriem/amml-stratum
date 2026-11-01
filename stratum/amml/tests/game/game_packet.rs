use alaydriem_stratum::errors::PacketReadError;
use alaydriem_stratum::game::{GamePacket, PacketId};
use amml_game::{ContainerClosePacketLayout, InteractPacketLayout};

use crate::support::FakePacket;
use crate::support::fake_packet::{
    CREATE_CALLS, DELETE_CALLS, DESTROY_CALLS, PACKET_NAME, READ_BYTES, READ_CALLS, READ_OK, STREAM_DTOR_CALLS,
};

#[test]
fn dropping_a_packet_releases_its_control_block_once() {
    let calls = FakePacket::calls();

    let packet = GamePacket::create(&calls, PacketId::ITEM_STACK_REQUEST).expect("the fake factory made a packet");

    assert_eq!(FakePacket::count(&CREATE_CALLS), 1);
    assert_eq!(packet.name(), "ItemStackRequestPacket");
    assert_eq!(FakePacket::count(&DESTROY_CALLS), 0);
    drop(packet);
    assert_eq!(FakePacket::count(&DESTROY_CALLS), 1);
    assert_eq!(FakePacket::count(&DELETE_CALLS), 1);
}

#[test]
fn a_packet_of_another_name_is_released_once_and_not_given() {
    *PACKET_NAME.lock().unwrap() = "TextPacket";
    let calls = FakePacket::calls();

    let packet = GamePacket::create(&calls, PacketId::ITEM_STACK_REQUEST);

    assert!(packet.is_none());
    assert_eq!(FakePacket::count(&DESTROY_CALLS), 1);
    assert_eq!(FakePacket::count(&DELETE_CALLS), 1);
}

#[test]
fn a_read_feeds_the_bytes_through_one_stream_that_is_destroyed_once() {
    let calls = FakePacket::calls();
    let mut packet = GamePacket::create(&calls, PacketId::ITEM_STACK_REQUEST).unwrap();

    let read = packet.read_from(&[1, 2, 3, 4]);

    assert_eq!(read, Ok(()));
    assert_eq!(FakePacket::count(&READ_CALLS), 1);
    assert_eq!(*READ_BYTES.lock().unwrap(), vec![1, 2, 3, 4]);
    assert_eq!(FakePacket::count(&STREAM_DTOR_CALLS), 1);
}

#[test]
fn a_failed_read_is_an_error_with_the_read_pointer_and_the_stream_is_destroyed_once() {
    READ_OK.store(false, std::sync::atomic::Ordering::SeqCst);
    let calls = FakePacket::calls();
    let mut packet = GamePacket::create(&calls, PacketId::ITEM_STACK_REQUEST).unwrap();

    let read = packet.read_from(&[9, 9]);

    assert_eq!(read, Err(PacketReadError::Refused { read_to: 0, length: 2 }));
    assert_eq!(FakePacket::count(&STREAM_DTOR_CALLS), 1);
    drop(packet);
    assert_eq!(FakePacket::count(&DESTROY_CALLS), 1);
}

#[test]
fn a_payload_is_given_only_as_the_layout_of_the_packet_name() {
    *PACKET_NAME.lock().unwrap() = "InteractPacket";
    let calls = FakePacket::calls();
    let mut packet = GamePacket::create(&calls, PacketId::INTERACT).unwrap();

    assert!(packet.payload_mut::<ContainerClosePacketLayout>().is_none());
    let interact = packet.payload_mut::<InteractPacketLayout>().expect("the packet is an InteractPacket");
    interact.action = 6;
    assert_eq!(packet.payload_mut::<InteractPacketLayout>().unwrap().action, 6);
}
