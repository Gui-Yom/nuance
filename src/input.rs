use crossterm::event::KeyCode;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget};

pub struct InputBox;

#[derive(Debug, Default)]
pub struct InputBoxState {
    text: Vec<char>,
    cursor: usize,
}

impl InputBoxState {
    pub fn process_event(&mut self, code: KeyCode) {
        match code {
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.text.remove(self.cursor);
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.text.len() {
                    self.text.remove(self.cursor);
                }
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor < self.text.len() {
                    self.cursor += 1;
                }
            }
            KeyCode::Char(char) => {
                self.text.insert(self.cursor, char);
                self.cursor += 1;
            }
            _ => {}
        }
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    pub fn text(&self) -> String {
        self.text.iter().collect::<String>()
    }
}

impl StatefulWidget for InputBox {
    type State = InputBoxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        let inner = block.inner(area);
        Paragraph::new(vec![Spans::from(vec![
            Span::raw("> "),
            Span::raw(state.text()),
        ])])
        .render(inner, buf);
        block.render(area, buf);
    }
}
