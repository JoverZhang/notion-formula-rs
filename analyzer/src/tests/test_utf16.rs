use crate::source_map::byte_offset_to_utf16;

#[test]
fn test_byte_offset_to_utf16_ascii() {
    let s = "abc";
    assert_eq!(byte_offset_to_utf16(s, 0), 0);
    assert_eq!(byte_offset_to_utf16(s, 2), 2);
    assert_eq!(byte_offset_to_utf16(s, 3), 3);
    assert_eq!(byte_offset_to_utf16(s, 10), 3);
}

#[test]
fn test_byte_offset_to_utf16_chinese() {
    let s = "一二";
    assert_eq!(byte_offset_to_utf16(s, 0), 0);
    assert_eq!(byte_offset_to_utf16(s, 3), 1);
    assert_eq!(byte_offset_to_utf16(s, 4), 1);
    assert_eq!(byte_offset_to_utf16(s, 6), 2);
}

#[test]
fn test_byte_offset_to_utf16_emoji() {
    let s = "😀a";
    assert_eq!(byte_offset_to_utf16(s, 0), 0);
    assert_eq!(byte_offset_to_utf16(s, 2), 0);
    assert_eq!(byte_offset_to_utf16(s, 4), 2);
    assert_eq!(byte_offset_to_utf16(s, 5), 3);
}
