use crate::ui::input::{Key, KeyReader, LineEditor};

#[test]
fn test_editor_insert_and_cursor() {
    let mut editor = LineEditor::new();

    editor.insert('a');
    editor.insert('b');
    editor.move_left();
    editor.insert('x');

    assert_eq!(editor.text(), "axb");
    assert_eq!(editor.cursor(), 2);
}

#[test]
fn test_editor_backspace_and_delete() {
    let mut editor = LineEditor::new();

    for ch in "abc".chars() {
        editor.insert(ch);
    }

    editor.home();
    editor.delete(); // removes 'a'
    assert_eq!(editor.text(), "bc");

    editor.end();
    editor.backspace(); // removes 'c'
    assert_eq!(editor.text(), "b");
}

#[test]
fn test_editor_history() {
    let mut editor = LineEditor::new();

    editor.push_history("first".to_string());
    editor.push_history("second".to_string());

    editor.history_prev();
    assert_eq!(editor.text(), "second");

    editor.history_prev();
    assert_eq!(editor.text(), "first");

    editor.history_next();
    assert_eq!(editor.text(), "second");

    editor.history_next();
    assert_eq!(editor.text(), "");
}

#[test]
fn test_editor_set_and_read_history() {
    let mut editor = LineEditor::new();

    editor.set_history(vec!["a".to_string(), "b".to_string()]);

    assert_eq!(
        editor.history().to_vec(),
        vec!["a".to_string(), "b".to_string()]
    );

    editor.history_prev();
    assert_eq!(editor.text(), "b");
}

#[test]
fn test_key_reader_arrow_keys() {
    let mut reader = KeyReader::new();

    reader.feed(b"\x1b[A\x1b[B\x1b[C\x1b[D");

    assert_eq!(reader.next_key(), Some(Key::Up));
    assert_eq!(reader.next_key(), Some(Key::Down));
    assert_eq!(reader.next_key(), Some(Key::Right));
    assert_eq!(reader.next_key(), Some(Key::Left));
    assert_eq!(reader.next_key(), None);
}

#[test]
fn test_key_reader_utf8_and_special() {
    let mut reader = KeyReader::new();

    reader.feed("ä".as_bytes());
    reader.feed(b"\r");
    reader.feed(b"\x1b[3~");

    assert_eq!(reader.next_key(), Some(Key::Char('ä')));
    assert_eq!(reader.next_key(), Some(Key::Enter));
    assert_eq!(reader.next_key(), Some(Key::Delete));
}

#[test]
fn test_key_reader_incomplete_utf8_waits() {
    let mut reader = KeyReader::new();
    // First byte of a two-byte character
    reader.feed(&[0xc3]);
    assert_eq!(reader.next_key(), None);

    reader.feed(&[0xa4]);
    assert_eq!(reader.next_key(), Some(Key::Char('ä')));
}

#[test]
fn test_editor_word_motions() {
    let mut editor = LineEditor::new();

    for ch in "ab cd ef".chars() {
        editor.insert(ch);
    }

    editor.home();
    editor.move_word_forward();
    assert_eq!(editor.cursor(), 3);

    editor.move_word_forward();
    assert_eq!(editor.cursor(), 6);

    editor.move_word_back();
    assert_eq!(editor.cursor(), 3);
}

#[test]
fn test_editor_first_nonblank() {
    let mut editor = LineEditor::new();

    for ch in "  ab".chars() {
        editor.insert(ch);
    }

    editor.home();
    editor.first_nonblank();
    assert_eq!(editor.cursor(), 2);

    let mut blank = LineEditor::new();

    for ch in "   ".chars() {
        blank.insert(ch);
    }

    blank.home();
    blank.first_nonblank();
    assert_eq!(blank.cursor(), 0);
}

#[test]
fn test_editor_delete_to_end_and_clamp() {
    let mut editor = LineEditor::new();

    for ch in "abcd".chars() {
        editor.insert(ch);
    }

    editor.home();
    editor.move_right();
    editor.move_right();
    editor.delete_to_end();

    assert_eq!(editor.text(), "ab");

    editor.end();
    editor.clamp_normal();

    assert_eq!(editor.cursor(), 1);
}

#[test]
fn test_key_reader_ctrl_r() {
    let mut reader = KeyReader::new();

    reader.feed(&[0x12]);

    assert_eq!(reader.next_key(), Some(Key::CtrlR));
}

#[test]
fn test_editor_insert_str() {
    let mut editor = LineEditor::new();

    editor.insert('2');
    editor.insert_str("+x");

    assert_eq!(editor.text(), "2+x");
    assert_eq!(editor.cursor(), 3);
}
