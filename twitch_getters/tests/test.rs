extern crate twitch_getters;

use twitch_getters::twitch_getters;

#[derive(Clone, Copy)]
struct UnsafeSlice;

impl UnsafeSlice {
    pub fn as_str<'a>(&self) -> &'a str { "test string, ok?" }
}

#[allow(unused)]
#[twitch_getters]
pub struct TestStruct {
    field: UnsafeSlice,
    #[csv]
    list: UnsafeSlice,
    optional: Option<UnsafeSlice>,
    vec: Vec<UnsafeSlice>,
    msg: String,
}

#[test]
fn test_generated_methods() {
    let msg = String::from("a quick brown fox jumped over the lazy dog");
    let t = TestStruct {
        field: UnsafeSlice,
        list: UnsafeSlice,
        optional: Some(UnsafeSlice),
        vec: vec![UnsafeSlice],
        msg,
    };
    assert_eq!(t.field(), "test string, ok?");
    assert_eq!(t.optional(), Some("test string, ok?"));
    assert_eq!(t.list().collect::<Vec<_>>(), vec!["test string", " ok?"]);
    assert_eq!(t.vec().collect::<Vec<_>>(), vec!["test string, ok?"]);
}
