//! Compile-time messages that name the type they refuse.

use core::str;

/// What a name cut short ends with.
const ELLIPSIS: &str = "…";

/// A message built in a const context, for a post-monomorphization refusal.
pub(crate) struct Message {
    /// The bytes written so far; only `len` of them are the message.
    bytes: [u8; 256],
    /// How many bytes are written.
    len: usize,
    /// Whether a piece was cut short at the buffer's end; nothing is appended after one.
    cut: bool,
}

impl Message {
    /// An empty message.
    pub(crate) const fn new() -> Self {
        Self { bytes: [0; 256], len: 0, cut: false }
    }

    /// Appends `text`, cut short at the last whole character the buffer holds.
    #[must_use]
    pub(crate) const fn text(self, text: &str) -> Self {
        let count = self.fit(text, 0);
        let mut message = self.append(text, count);
        message.cut |= count < text.len();
        message
    }

    /// Appends a type's `name`, cut short with `…` to leave `room` bytes for the text after it.
    #[must_use]
    pub(crate) const fn name(self, name: &str, room: usize) -> Self {
        if self.fit(name, room) == name.len() {
            return self.text(name);
        }
        let count = self.fit(name, room.saturating_add(ELLIPSIS.len()));
        self.append(name, count).text(ELLIPSIS)
    }

    /// How many bytes of `text`, ending on a character boundary, fit with `room` bytes to spare.
    const fn fit(&self, text: &str, room: usize) -> usize {
        let free = self.bytes.len().saturating_sub(self.len).saturating_sub(room);
        let mut count = if text.len() < free { text.len() } else { free };
        while !text.is_char_boundary(count) {
            count = count.saturating_sub(1);
        }
        count
    }

    /// Appends the first `count` bytes of `text`, which `fit` allows, unless a piece was cut.
    const fn append(mut self, text: &str, count: usize) -> Self {
        if self.cut {
            return self;
        }
        let (_, free) = self.bytes.split_at_mut(self.len);
        let (destination, _) = free.split_at_mut(count);
        let (source, _) = text.as_bytes().split_at(count);
        destination.copy_from_slice(source);
        self.len = self.len.saturating_add(count);
        self
    }

    /// Appends `value` in decimal.
    #[must_use]
    pub(crate) const fn number(self, value: u128) -> Self {
        let mut digits = [0_u8; 39];
        let mut start = digits.len();
        let mut rest = value;
        loop {
            start = start.saturating_sub(1);
            if let Some(slot) = digits.get_mut(start) {
                *slot = b'0'.saturating_add(rest.rem_euclid(10).wrapping_cast());
            }
            rest = rest.div_euclid(10);
            if rest == 0 {
                break;
            }
        }
        let (_, written) = digits.split_at(start);
        match str::from_utf8(written) {
            Ok(text) => self.text(text),
            Err(_) => self,
        }
    }

    /// The message, for `panic!`.
    pub(crate) const fn as_str(&self) -> &str {
        let (written, _) = self.bytes.split_at(self.len);
        match str::from_utf8(written) {
            Ok(text) => text,
            Err(_) => "atomiks: a refusal whose message could not be built",
        }
    }
}

/// Refuses the build with `message`.
#[expect(clippy::panic, reason = "a refusal evaluated at compile time, naming what was refused")]
pub(crate) const fn refuse(message: &Message) -> ! {
    panic!("{}", message.as_str())
}

#[cfg(test)]
mod tests {
    use core::str;

    use super::Message;

    #[test]
    fn text_and_numbers_are_appended_in_order() {
        let message = Message::new().text("`u64` has ").number(64).text(" bits");
        assert_eq!(message.as_str(), "`u64` has 64 bits", "the pieces in order");
    }

    #[test]
    fn zero_and_the_largest_number_print_in_full() {
        assert_eq!(Message::new().number(0).as_str(), "0", "zero is one digit");
        assert_eq!(
            Message::new().number(u128::MAX).as_str(),
            "340282366920938463463374607431768211455",
            "the largest needs all 39 digits"
        );
    }

    #[test]
    fn a_message_past_the_buffer_is_cut_not_corrupted() {
        let long = [b'x'; 300];
        let text = str::from_utf8(&long).expect("ASCII is UTF-8");
        assert_eq!(Message::new().text(text).as_str().len(), 256, "cut at the buffer's end");
    }

    #[test]
    fn a_character_the_buffer_cannot_hold_whole_is_left_out() {
        let short = [b'x'; 255];
        let text = str::from_utf8(&short).expect("ASCII is UTF-8");
        let message = Message::new().text(text).text("λ");
        assert_eq!(message.as_str(), text, "the two-byte `λ` left out, not the whole message");
    }

    #[test]
    fn nothing_is_appended_after_a_cut() {
        let short = [b'x'; 255];
        let text = str::from_utf8(&short).expect("ASCII is UTF-8");
        let message = Message::new().text(text).text("λ").text("y");
        assert_eq!(message.as_str(), text, "no `y` where the `λ` it follows is missing");
    }

    #[test]
    fn a_long_name_is_cut_to_keep_the_text_after_it() {
        let long = [b'n'; 300];
        let name = str::from_utf8(&long).expect("ASCII is UTF-8");
        let advice = ">`: use a type with a spare repr";
        let message = Message::new().text("`Option<").name(name, advice.len()).text(advice);
        let text = message.as_str();
        assert_eq!(text.len(), 256, "the name takes what the advice leaves");
        assert!(text.ends_with(advice), "the advice whole: {text}");
        assert!(text.contains("n…>"), "the name cut short with `…`: {text}");
        assert_eq!(Message::new().name("u64", 200).as_str(), "u64", "a name that fits, whole");
    }
}
