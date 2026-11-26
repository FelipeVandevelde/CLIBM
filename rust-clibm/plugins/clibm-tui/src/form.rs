//! A small text form for the TUI dialogs (no extra crate): text fields, read-only
//! fields and on/off toggles.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text,
    ReadOnly,
    Toggle(bool),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub label: &'static str,
    pub value: String,
    /// Cursor position in characters.
    pub cursor: usize,
    pub kind: FieldKind,
}

impl Field {
    fn editable(&self) -> bool {
        self.kind != FieldKind::ReadOnly
    }

    fn byte_index(&self, chars: usize) -> usize {
        self.value
            .char_indices()
            .nth(chars)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len())
    }

    fn len(&self) -> usize {
        self.value.chars().count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormOutcome {
    /// Keep editing.
    Continue,
    Submit,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct Form {
    pub title: String,
    pub fields: Vec<Field>,
    /// Index of the focused field (always an editable one when there is one).
    pub focus: usize,
    /// Last error from the core, shown inside the dialog.
    pub error: Option<String>,
}

impl Form {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), fields: Vec::new(), focus: 0, error: None }
    }

    fn push(mut self, label: &'static str, value: &str, kind: FieldKind) -> Self {
        let value = value.to_string();
        let cursor = value.chars().count();
        self.fields.push(Field { label, value, cursor, kind });
        if !self.fields[self.focus].editable() {
            self.focus = self.fields.len() - 1;
        }
        self
    }

    pub fn text(self, label: &'static str, value: &str) -> Self {
        self.push(label, value, FieldKind::Text)
    }

    pub fn read_only(self, label: &'static str, value: &str) -> Self {
        self.push(label, value, FieldKind::ReadOnly)
    }

    pub fn toggle(self, label: &'static str, on: bool) -> Self {
        self.push(label, "", FieldKind::Toggle(on))
    }

    /// Trimmed value of the field called `label` (empty when there is none).
    pub fn value(&self, label: &str) -> String {
        self.fields
            .iter()
            .find(|f| f.label == label)
            .map(|f| f.value.trim().to_string())
            .unwrap_or_default()
    }

    pub fn checked(&self, label: &str) -> bool {
        self.fields
            .iter()
            .any(|f| f.label == label && f.kind == FieldKind::Toggle(true))
    }

    fn step_focus(&mut self, forward: bool) {
        let n = self.fields.len();
        if n == 0 {
            return;
        }
        for _ in 0..n {
            self.focus = if forward { (self.focus + 1) % n } else { (self.focus + n - 1) % n };
            if self.fields[self.focus].editable() {
                return;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> FormOutcome {
        match key.code {
            KeyCode::Esc => return FormOutcome::Cancel,
            KeyCode::Enter => return FormOutcome::Submit,
            KeyCode::Tab | KeyCode::Down => self.step_focus(true),
            KeyCode::BackTab | KeyCode::Up => self.step_focus(false),
            _ => {}
        }
        let Some(field) = self.fields.get_mut(self.focus) else {
            return FormOutcome::Continue;
        };
        if let FieldKind::Toggle(on) = &mut field.kind {
            if key.code == KeyCode::Char(' ') {
                *on = !*on;
            }
            return FormOutcome::Continue;
        }
        if field.kind != FieldKind::Text {
            return FormOutcome::Continue;
        }
        match key.code {
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                field.value.clear();
                field.cursor = 0;
            }
            KeyCode::Char(c) if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                let at = field.byte_index(field.cursor);
                field.value.insert(at, c);
                field.cursor += 1;
            }
            KeyCode::Backspace if field.cursor > 0 => {
                let at = field.byte_index(field.cursor - 1);
                field.value.remove(at);
                field.cursor -= 1;
            }
            KeyCode::Delete if field.cursor < field.len() => {
                let at = field.byte_index(field.cursor);
                field.value.remove(at);
            }
            KeyCode::Left => field.cursor = field.cursor.saturating_sub(1),
            KeyCode::Right => field.cursor = (field.cursor + 1).min(field.len()),
            KeyCode::Home => field.cursor = 0,
            KeyCode::End => field.cursor = field.len(),
            _ => {}
        }
        FormOutcome::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn typing_editing_and_focus() {
        let mut f = Form::new("t")
            .read_only("Code", "APP")
            .text("Name", "ab")
            .toggle("Switch", false);
        assert_eq!(f.focus, 1, "read-only fields are skipped");
        f.handle_key(key(KeyCode::Left));
        f.handle_key(key(KeyCode::Char('é')));
        assert_eq!(f.value("Name"), "aéb");
        f.handle_key(key(KeyCode::Backspace));
        f.handle_key(key(KeyCode::End));
        f.handle_key(key(KeyCode::Char('c')));
        assert_eq!(f.value("Name"), "abc");
        f.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(f.value("Name"), "");

        f.handle_key(key(KeyCode::Tab));
        assert_eq!(f.focus, 2);
        f.handle_key(key(KeyCode::Char(' ')));
        assert!(f.checked("Switch"));
        f.handle_key(key(KeyCode::Tab));
        assert_eq!(f.focus, 1, "wraps around and skips the read-only field");

        assert_eq!(f.handle_key(key(KeyCode::Enter)), FormOutcome::Submit);
        assert_eq!(f.handle_key(key(KeyCode::Esc)), FormOutcome::Cancel);
    }
}
