# twitch_getters

This is a proc macro for generating getters for `UnsafeSlice` fields in the `twitch` crate.

## Basic Usage 
Annotate a struct with `#[twitch_getters]`. All bare, Option, and Vec `UnsafeSlice` fields will be generated a getter.

```rust
use crate::util::UnsafeSlice;
use twitch_getters::twitch_getters;

 #[twitch_getters]
 struct TwitchStruct {
    // UnsafeSlice fields
    nick: UnsafeSlice,
    sub: Option<UnsafeSlice>,
    badges: Vec<UnsafeSlice>,
    // Any other fields
    some_other_vec: Vec<i32>,
    some_option: Option<String>
}

// Expands into this:
impl TwitchStruct {
    #[inline]
    pub fn nick(&self) -> &str {
        self.nick.as_str()
    }
    #[inline]
    pub fn sub(&self) -> Option<&str> {
        self.sub.as_ref().map(|v| v.as_str())
    }
    #[inline]
    pub fn badges(&self) -> impl Iterator<Item = &str> + '_ {
        self.badges.iter().map(|v| v.as_str())
    }
}
```