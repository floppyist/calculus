use crate::ui::clipboard::base64;

#[test]
fn test_base64_padding() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foob"), "Zm9vYg==");
    assert_eq!(base64(b"fooba"), "Zm9vYmE=");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn test_base64_binary() {
    assert_eq!(base64(&[0x00, 0xff, 0xfe]), "AP/+");
    assert_eq!(
        base64("sin(x)^-1 = asin(x)".as_bytes()),
        "c2luKHgpXi0xID0gYXNpbih4KQ=="
    );
}
