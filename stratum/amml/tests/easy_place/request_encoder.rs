use alaydriem_stratum::easy_place::{RequestEncoder, SlotRef};

// one request; its id; the end of every request: no strings to filter (0) and their origin -1
const ONE_REQUEST: u8 = 0x01;
const END: [u8; 5] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF];
// zigzag(-100001) = 200001 and zigzag(-100003) = 200005, in 7-bit groups, low group first
const ID_100001: [u8; 3] = [0xC1, 0x9A, 0x0C];
const ID_100003: [u8; 3] = [0xC5, 0x9A, 0x0C];
// -100003 as a little-endian signed int: 0xFFFE795D
const ID_100003_INT: [u8; 4] = [0x5D, 0x79, 0xFE, 0xFF];

fn request(id: [u8; 3], body: &[&[u8]]) -> Vec<u8> {
    let mut bytes = vec![ONE_REQUEST];
    bytes.extend_from_slice(&id);
    for part in body {
        bytes.extend_from_slice(part);
    }
    bytes.extend_from_slice(&END);
    bytes
}

#[test]
fn a_swap_is_the_cpp_bytes_one_swap_action_from_the_inventory_into_the_hotbar() {
    let from = SlotRef { container: SlotRef::INVENTORY, slot: 12, net_id: 41 };
    let to = SlotRef { container: SlotRef::HOTBAR, slot: 3, net_id: 42 };

    let bytes = RequestEncoder::swap(-100001, from, to);

    let expected = request(
        ID_100001,
        &[
            // one action: Swap, at variant index 2, type 2
            &[0x01, 0x02, 0x02],
            // the source: inventory 29, no dynamic id, slot 12, net id 41
            &[29, 0x00, 12, 41, 0, 0, 0],
            // the target: hotbar 28, no dynamic id, slot 3, net id 42
            &[28, 0x00, 3, 42, 0, 0, 0],
        ],
    );
    assert_eq!(bytes, expected);
}

#[test]
fn a_place_into_an_empty_slot_is_the_cpp_bytes_with_the_whole_stack_count() {
    let from = SlotRef { container: SlotRef::INVENTORY, slot: 20, net_id: 9 };
    let to = SlotRef { container: SlotRef::HOTBAR, slot: 0, net_id: 0 };

    let bytes = RequestEncoder::place(-100001, 16, from, to);

    let expected = request(
        ID_100001,
        &[
            // one action: Place, at variant index 1, type 1, with the count 16
            &[0x01, 0x01, 0x01, 16],
            &[29, 0x00, 20, 9, 0, 0, 0],
            &[28, 0x00, 0, 0, 0, 0, 0],
        ],
    );
    assert_eq!(bytes, expected);
}

#[test]
fn a_pick_into_an_empty_slot_is_the_cpp_bytes_craft_creative_then_take() {
    let to = SlotRef { container: SlotRef::HOTBAR, slot: 4, net_id: 0 };

    let bytes = RequestEncoder::pick(-100003, 300, 64, to);

    let expected = request(
        ID_100003,
        &[
            // two actions
            &[0x02],
            // CraftCreative: variant index 12, type 14, the creative net id 300 as an unsigned varint, one craft
            &[12, 14, 0xAC, 0x02, 0x01],
            // Take: variant index 0, type 0, the count 64
            &[0x00, 0x00, 64],
            // from the created output 60, slot 50, whose net id is the request's own id
            &[60, 0x00, 50],
            &ID_100003_INT,
            // into the empty hotbar slot 4
            &[28, 0x00, 4, 0, 0, 0, 0],
        ],
    );
    assert_eq!(bytes, expected);
}

#[test]
fn a_pick_over_a_held_stack_destroys_it_first() {
    let held = SlotRef { container: SlotRef::HOTBAR, slot: 4, net_id: 77 };

    let bytes = RequestEncoder::pick_replacing(-100003, 300, 64, held, 10);

    let expected = request(
        ID_100003,
        &[
            // three actions
            &[0x03],
            // Destroy: variant index 4, type 4, the held count 10, from hotbar slot 4 with its net id 77
            &[4, 4, 10],
            &[28, 0x00, 4, 77, 0, 0, 0],
            &[12, 14, 0xAC, 0x02, 0x01],
            &[0x00, 0x00, 64],
            &[60, 0x00, 50],
            &ID_100003_INT,
            // the slot is empty after the destroy, so its net id is 0
            &[28, 0x00, 4, 0, 0, 0, 0],
        ],
    );
    assert_eq!(bytes, expected);
}

#[test]
fn a_zigzag_var_int_of_minus_one_zero_and_300() {
    let encode = |value: i32| {
        let mut out = Vec::new();
        RequestEncoder::write_var_int(&mut out, value);
        out
    };

    assert_eq!(encode(-1), vec![0x01]);
    assert_eq!(encode(0), vec![0x00]);
    // zigzag(300) = 600 = 0b100_1011000
    assert_eq!(encode(300), vec![0xD8, 0x04]);
}

#[test]
fn an_unsigned_var_int_takes_more_groups_as_it_grows() {
    let encode = |value: u32| {
        let mut out = Vec::new();
        RequestEncoder::write_unsigned_var_int(&mut out, value);
        out
    };

    assert_eq!(encode(0), vec![0x00]);
    assert_eq!(encode(127), vec![0x7F]);
    assert_eq!(encode(128), vec![0x80, 0x01]);
    assert_eq!(encode(u32::MAX), vec![0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
}
