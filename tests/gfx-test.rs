use nano9::pico8::const_bitdepth::Gfx;
use std::fs;

#[test]
fn test_read_indexed_png_indices_from_file_8bit() {
    let data = fs::read("tests/gfx-test.png").expect("Failed to read test PNG file");

    let gfx = Gfx::<8, u8>::from_png(&data, None).unwrap();
    assert_eq!(gfx.get(0, 0).unwrap(), 0);
    assert_eq!(gfx.get(1, 0).unwrap(), 1);
    assert_eq!(gfx.get(0, 1).unwrap(), 2);
    assert_eq!(gfx.get(1, 1).unwrap(), 1);
    let v = gfx.data.into_vec();
    assert_eq!(v.len(), 4);
    assert_eq!(v[0], 0);
    assert_eq!(v[1], 1);
    assert_eq!(v[2], 2);
    assert_eq!(v[3], 1);
}

#[test]
fn test_read_indexed_png_indices_from_file_2bit() {
    let data = fs::read("tests/gfx-test.png").expect("Failed to read test PNG file");

    let gfx = Gfx::<2, u8>::from_png(&data, None).unwrap();
    assert_eq!(gfx.get(0, 0).unwrap(), 0);
    assert_eq!(gfx.get(1, 0).unwrap(), 1);
    assert_eq!(gfx.get(0, 1).unwrap(), 2);
    assert_eq!(gfx.get(1, 1).unwrap(), 1);
    let v = gfx.data.into_vec();
    assert_eq!(v.len(), 1);
    // assert_eq!(v[0], 0b00_01_10_01);
    assert_eq!(v[0], 0b01_10_01_00);
}

#[test]
fn test_read_indexed_png_indices_from_file_big_pal() {
    let data = fs::read("tests/gfx-big-palette.png").expect("Failed to read test PNG file");

    assert!(Gfx::<2, u8>::from_png(&data, None).is_err());
}
