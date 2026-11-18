use alaydriem_stratum::build::CommandPacket;
use alaydriem_stratum::errors::{CommandSendError, GameLayoutError};

use crate::support::FakePacket;
use crate::support::fake_packet::{CREATE_CALLS, SEND_CALLS};

#[test]
fn a_command_string_inside_the_packet_is_accepted() {
    // an MSVC std::string is 0x20 bytes: the text at 0x30 ends exactly at 0x50
    assert!(CommandPacket::new(FakePacket::calls(), 0x30, 0x50).is_ok());
    assert!(CommandPacket::new(FakePacket::calls(), 0x30, 0xA0).is_ok());
}

#[test]
fn a_command_string_that_ends_past_the_packet_is_refused() {
    let refused = CommandPacket::new(FakePacket::calls(), 0x30, 0x4F);

    assert_eq!(
        refused.map(|_| ()),
        Err(GameLayoutError::Misfit { entry: "CommandRequestPacket.mCommand", expected: 0x2F, found: 0x30 })
    );
}

#[test]
fn an_offset_that_overflows_is_refused() {
    assert!(CommandPacket::new(FakePacket::calls(), u64::MAX - 4, u64::MAX).is_err());
}

#[test]
#[allow(unsafe_code)]
fn without_the_game_heap_nothing_is_made_or_sent() {
    let packet = CommandPacket::new(FakePacket::calls(), 0x30, 0xA0).unwrap();

    // SAFETY: the fake client's vtable holds getPacketSender at the calls' slot
    let result = unsafe { packet.send(FakePacket::client(), "/setblock 0 0 0 stone") };

    assert_eq!(result, Err(CommandSendError::NoHeap));
    assert_eq!(FakePacket::count(&CREATE_CALLS), 0);
    assert_eq!(FakePacket::count(&SEND_CALLS), 0);
}

#[test]
#[allow(unsafe_code)]
fn a_null_client_sends_nothing() {
    let packet = CommandPacket::new(FakePacket::calls(), 0x30, 0xA0).unwrap();

    // SAFETY: a null client is allowed
    let result = unsafe { packet.send(std::ptr::null_mut(), "/gamerule sendcommandfeedback true") };

    assert!(result.is_err());
    assert_eq!(FakePacket::count(&SEND_CALLS), 0);
}
